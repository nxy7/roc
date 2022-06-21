use bumpalo::Bump;
use const_format::concatcp;
use home::home_dir;
use inkwell::context::Context;
use libloading::Library;
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, PromptInfo};
use rustyline::history::History;
use rustyline::validate::{self, ValidationContext, ValidationResult, Validator};
use rustyline_derive::{Completer, Helper, Hinter};
use std::borrow::Cow;
use std::io;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::{env, fs};
use target_lexicon::Triple;

use roc_build::link::module_to_dylib;
use roc_collections::all::MutSet;
use roc_gen_llvm::llvm::externs::add_default_roc_externs;
use roc_gen_llvm::{run_jit_function, run_jit_function_dynamic_type};
use roc_load::file::MonomorphizedModule;
use roc_mono::ir::OptLevel;
use roc_parse::ast::Expr;
use roc_parse::parser::{EExpr, ELambda, SyntaxError};
use roc_repl_eval::eval::jit_to_ast;
use roc_repl_eval::gen::{compile_to_mono, format_answer, ReplOutput};
use roc_repl_eval::{ReplApp, ReplAppMemory};
use roc_reporting::report::DEFAULT_PALETTE;
use roc_std::RocStr;
use roc_target::TargetInfo;
use roc_types::pretty_print::{content_to_string, name_all_type_vars};

const BLUE: &str = "\u{001b}[36m";
const PINK: &str = "\u{001b}[35m";
const END_COL: &str = "\u{001b}[0m";

pub const WELCOME_MESSAGE: &str = concatcp!(
    "\n  The rockin’ ",
    BLUE,
    "roc repl",
    END_COL,
    "\n",
    PINK,
    "────────────────────────",
    END_COL,
    "\n\n"
);
pub const INSTRUCTIONS: &str = "Enter an expression, or :help, or :exit/:q.\n";
pub const PROMPT: &str = concatcp!("\n", BLUE, "»", END_COL, " ");
pub const CONT_PROMPT: &str = concatcp!(BLUE, "…", END_COL, " ");

pub const HISTORY_PATH: &str = concatcp!(".roc_cache", MAIN_SEPARATOR, "repl_history.dat");
#[derive(Completer, Helper, Hinter)]
struct ReplHelper {
    validator: InputValidator,
    pending_src: String,
}

impl ReplHelper {
    pub(crate) fn new() -> ReplHelper {
        ReplHelper {
            validator: InputValidator::new(),
            pending_src: String::new(),
        }
    }
}

impl Highlighter for ReplHelper {
    fn has_continuation_prompt(&self) -> bool {
        true
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        info: PromptInfo<'_>,
    ) -> Cow<'b, str> {
        if info.line_no() > 0 {
            CONT_PROMPT.into()
        } else {
            prompt.into()
        }
    }
}

impl Validator for ReplHelper {
    fn validate(
        &self,
        ctx: &mut validate::ValidationContext,
    ) -> rustyline::Result<validate::ValidationResult> {
        self.validator.validate(ctx)
    }

    fn validate_while_typing(&self) -> bool {
        self.validator.validate_while_typing()
    }
}

struct InputValidator {}

impl InputValidator {
    pub(crate) fn new() -> InputValidator {
        InputValidator {}
    }
}

impl Validator for InputValidator {
    fn validate(&self, ctx: &mut ValidationContext) -> rustyline::Result<ValidationResult> {
        if ctx.input().is_empty() {
            Ok(ValidationResult::Incomplete)
        } else {
            let arena = bumpalo::Bump::new();
            let state = roc_parse::state::State::new(ctx.input().trim().as_bytes());

            match roc_parse::expr::parse_loc_expr(0, &arena, state) {
                // Special case some syntax errors to allow for multi-line inputs
                Err((_, EExpr::DefMissingFinalExpr(_), _))
                | Err((_, EExpr::DefMissingFinalExpr2(_, _), _))
                | Err((_, EExpr::Lambda(ELambda::Body(_, _), _), _)) => {
                    Ok(ValidationResult::Incomplete)
                }
                _ => Ok(ValidationResult::Valid(None)),
            }
        }
    }
}

struct CommandHistories {
    local_history: History,
    local_history_path: Option<PathBuf>,
    session_history: History,
    global_history: History,
    global_history_path: Option<PathBuf>,
}

impl CommandHistories {
    fn add_history_entry(&mut self, line: &str) {
        self.local_history.add(line);
        self.session_history.add(line);
        self.global_history.add(line);
    }

    fn save_session_history(&mut self, path: &str) -> Result<(), ReadlineError> {
        self.session_history.save(path)
    }
    fn append_histories(&mut self) -> Result<(), ReadlineError> {
        Ok(())
            .and_then(|()| match &self.global_history_path {
                Some(path) => self.global_history.append(path),
                None => Ok(()),
            })
            .and_then(|()| match &self.local_history_path {
                Some(path) => self.local_history.append(path),
                None => Ok(()),
            })
    }
    fn new() -> CommandHistories {
        CommandHistories {
            local_history: History::new(),
            local_history_path: None,
            session_history: History::new(),
            global_history: History::new(),
            global_history_path: None,
        }
    }
    fn load(local_path: Option<PathBuf>, global_path: Option<PathBuf>) -> CommandHistories {
        let mut histories = Self::new();
        local_path
            .clone()
            .map(|path| Some(histories.local_history.load(&path)));
        global_path
            .clone()
            .map(|path| Some(histories.global_history.load(&path)));
        CommandHistories {
            local_history_path: local_path,
            global_history_path: global_path,
            ..histories
        }
    }
}
fn local_history_path() -> PathBuf {
    match env::var_os("ROC_REPL_LOCAL_HISTORY_PATH") {
        None => PathBuf::from(HISTORY_PATH),
        Some(path) => PathBuf::from(path),
    }
}
fn global_history_path() -> Option<PathBuf> {
    match env::var_os("ROC_REPL_GLOBAL_HISTORY_PATH") {
        None => home_dir().map(|path| path.join(PathBuf::from(HISTORY_PATH))),
        Some(path) => Some(PathBuf::from(path)),
    }
}
fn touch(path: PathBuf) -> Result<PathBuf, std::io::Error> {
    match fs::File::options()
        .write(true)
        .create(true)
        .open(&local_history_path())
    {
        Ok(_) => Ok(path),
        Err(e) => Err(e),
    }
}

struct CliApp {
    lib: Library,
}

struct CliMemory;

impl<'a> ReplApp<'a> for CliApp {
    type Memory = CliMemory;

    /// Run user code that returns a type with a `Builtin` layout
    /// Size of the return value is statically determined from its Rust type
    fn call_function<Return, F>(&self, main_fn_name: &str, transform: F) -> Expr<'a>
    where
        F: Fn(&'a Self::Memory, Return) -> Expr<'a>,
        Self::Memory: 'a,
    {
        run_jit_function!(self.lib, main_fn_name, Return, |v| transform(&CliMemory, v))
    }

    /// Run user code that returns a struct or union, whose size is provided as an argument
    fn call_function_dynamic_size<T, F>(
        &self,
        main_fn_name: &str,
        ret_bytes: usize,
        transform: F,
    ) -> T
    where
        F: Fn(&'a Self::Memory, usize) -> T,
        Self::Memory: 'a,
    {
        run_jit_function_dynamic_type!(self.lib, main_fn_name, ret_bytes, |v| transform(
            &CliMemory, v
        ))
    }
}

macro_rules! deref_number {
    ($name: ident, $t: ty) => {
        fn $name(&self, addr: usize) -> $t {
            let ptr = addr as *const _;
            unsafe { *ptr }
        }
    };
}

impl ReplAppMemory for CliMemory {
    deref_number!(deref_bool, bool);

    deref_number!(deref_u8, u8);
    deref_number!(deref_u16, u16);
    deref_number!(deref_u32, u32);
    deref_number!(deref_u64, u64);
    deref_number!(deref_u128, u128);
    deref_number!(deref_usize, usize);

    deref_number!(deref_i8, i8);
    deref_number!(deref_i16, i16);
    deref_number!(deref_i32, i32);
    deref_number!(deref_i64, i64);
    deref_number!(deref_i128, i128);
    deref_number!(deref_isize, isize);

    deref_number!(deref_f32, f32);
    deref_number!(deref_f64, f64);

    fn deref_str(&self, addr: usize) -> &str {
        let reference: &RocStr = unsafe { std::mem::transmute(addr) };
        reference.as_str()
    }
}

fn gen_and_eval_llvm<'a>(
    src: &str,
    target: Triple,
    opt_level: OptLevel,
) -> Result<ReplOutput, SyntaxError<'a>> {
    let arena = Bump::new();
    let target_info = TargetInfo::from(&target);

    let loaded = match compile_to_mono(&arena, src, target_info, DEFAULT_PALETTE) {
        Ok(x) => x,
        Err(prob_strings) => {
            return Ok(ReplOutput::Problems(prob_strings));
        }
    };

    let MonomorphizedModule {
        procedures,
        entry_point,
        interns,
        exposed_to_host,
        mut subs,
        module_id: home,
        ..
    } = loaded;

    let context = Context::create();
    let builder = context.create_builder();
    let module = arena.alloc(roc_gen_llvm::llvm::build::module_from_builtins(
        &target, &context, "",
    ));

    debug_assert_eq!(exposed_to_host.values.len(), 1);
    let (main_fn_symbol, main_fn_var) = exposed_to_host.values.iter().next().unwrap();
    let main_fn_symbol = *main_fn_symbol;
    let main_fn_var = *main_fn_var;

    // pretty-print the expr type string for later.
    name_all_type_vars(main_fn_var, &mut subs);
    let content = subs.get_content_without_compacting(main_fn_var);
    let expr_type_str = content_to_string(content, &subs, home, &interns);

    let (_, main_fn_layout) = match procedures.keys().find(|(s, _)| *s == main_fn_symbol) {
        Some(layout) => *layout,
        None => {
            return Ok(ReplOutput::NoProblems {
                expr: "<function>".to_string(),
                expr_type: expr_type_str,
            });
        }
    };

    let module = arena.alloc(module);
    let (module_pass, function_pass) =
        roc_gen_llvm::llvm::build::construct_optimization_passes(module, opt_level);

    let (dibuilder, compile_unit) = roc_gen_llvm::llvm::build::Env::new_debug_info(module);

    // Compile and add all the Procs before adding main
    let env = roc_gen_llvm::llvm::build::Env {
        arena: &arena,
        builder: &builder,
        dibuilder: &dibuilder,
        compile_unit: &compile_unit,
        context: &context,
        interns,
        module,
        target_info,
        is_gen_test: true, // so roc_panic is generated
        // important! we don't want any procedures to get the C calling convention
        exposed_to_host: MutSet::default(),
    };

    // Add roc_alloc, roc_realloc, and roc_dealloc, since the repl has no
    // platform to provide them.
    add_default_roc_externs(&env);

    let (main_fn_name, main_fn) = roc_gen_llvm::llvm::build::build_procedures_return_main(
        &env,
        opt_level,
        procedures,
        entry_point,
    );

    env.dibuilder.finalize();

    // we don't use the debug info, and it causes weird errors.
    module.strip_debug_info();

    // Uncomment this to see the module's un-optimized LLVM instruction output:
    // env.module.print_to_stderr();

    if main_fn.verify(true) {
        function_pass.run_on(&main_fn);
    } else {
        panic!("Main function {} failed LLVM verification in build. Uncomment things nearby to see more details.", main_fn_name);
    }

    module_pass.run_on(env.module);

    // Uncomment this to see the module's optimized LLVM instruction output:
    // env.module.print_to_stderr();

    // Verify the module
    if let Err(errors) = env.module.verify() {
        panic!(
            "Errors defining module:\n{}\n\nUncomment things nearby to see more details.",
            errors.to_string()
        );
    }

    let lib = module_to_dylib(env.module, &target, opt_level)
        .expect("Error loading compiled dylib for test");

    let app = CliApp { lib };

    let res_answer = jit_to_ast(
        &arena,
        &app,
        main_fn_name,
        main_fn_layout,
        content,
        &env.interns,
        home,
        &subs,
        target_info,
    );

    let formatted = format_answer(&arena, res_answer, expr_type_str);
    Ok(formatted)
}

fn eval_and_format<'a>(src: &str) -> Result<String, SyntaxError<'a>> {
    let format_output = |output| match output {
        ReplOutput::NoProblems { expr, expr_type } => {
            format!("\n{} {}:{} {}", expr, PINK, END_COL, expr_type)
        }
        ReplOutput::Problems(lines) => format!("\n{}\n", lines.join("\n\n")),
    };

    gen_and_eval_llvm(src, Triple::host(), OptLevel::Normal).map(format_output)
}

fn report_parse_error(fail: SyntaxError) {
    println!("TODO Gracefully report parse error in repl: {:?}", fail);
}

pub fn main(with_history: bool) -> io::Result<()> {
    use rustyline::Editor;

    // To debug rustyline:
    // <UNCOMMENT> env_logger::init();
    // <RUN WITH:> RUST_LOG=rustyline=debug cargo run repl 2> debug.log
    print!("{}{}", WELCOME_MESSAGE, INSTRUCTIONS);

    let mut prev_line_blank = false;
    let mut editor = Editor::<ReplHelper>::new();

    let global_history_path = if with_history {
        global_history_path()
            .map(
                |path| match touch(path.clone()).map(|path| editor.load_history(&path)) {
                    Ok(_) => Some(path),
                    Err(_) => {
                        eprintln!("Failed to load or initialize global history");
                        None
                    }
                },
            )
            .flatten()
    } else {
        None
    };

    let local_path = if with_history {
        match touch(local_history_path()).map(|path| editor.load_history(&path)) {
            Ok(_) => Some(local_history_path()),
            Err(_) => {
                eprintln!("Failed to load or initialize local history");
                None
            }
        }
    } else {
        None
    };
    let mut histories = CommandHistories::load(local_path, global_history_path);

    let repl_helper = ReplHelper::new();
    editor.set_helper(Some(repl_helper));

    loop {
        let readline = editor.readline(PROMPT);

        match readline {
            Ok(line) => {
                let trim_line = line.trim();
                editor.add_history_entry(trim_line);
                if with_history {
                    histories.add_history_entry(trim_line);
                    histories
                        .append_histories()
                        .expect("failed to append histories");
                }

                let pending_src = &mut editor
                    .helper_mut()
                    .expect("Editor helper was not set")
                    .pending_src;

                match trim_line.to_lowercase().as_str() {
                    "" => {
                        if pending_src.is_empty() {
                            print!("\n{}", INSTRUCTIONS);
                        } else if prev_line_blank {
                            // After two blank lines in a row, give up and try parsing it
                            // even though it's going to fail. This way you don't get stuck.
                            match eval_and_format(pending_src.as_str()) {
                                Ok(output) => {
                                    println!("{}", output);
                                }
                                Err(fail) => {
                                    report_parse_error(fail);
                                }
                            }

                            pending_src.clear();
                        } else {
                            pending_src.push('\n');

                            prev_line_blank = true;
                            continue; // Skip the part where we reset prev_line_blank to false
                        }
                    }
                    ":help" => {
                        println!("Use :exit or :q to exit. Use :save <filename> to save your session to a file.");
                    }
                    ":exit" => {
                        break;
                    }
                    ":q" => {
                        break;
                    }
                    save_cmd if save_cmd.starts_with(":save") => {
                        match save_cmd.split_once(" ") {
                            Some((_, path)) => match histories.save_session_history(path) {
                                Ok(_) => println!("History saved"),
                                Err(err) => println!("Failed to save history: {err}"),
                            },
                            None => println!("Please supply a path to save your history to"),
                        };
                    }
                    _ => {
                        let result = if pending_src.is_empty() {
                            eval_and_format(trim_line)
                        } else {
                            pending_src.push('\n');
                            pending_src.push_str(trim_line);

                            eval_and_format(pending_src.as_str())
                        };

                        match result {
                            Ok(output) => {
                                println!("{}", output);
                                pending_src.clear();
                            }
                            //                            Err(Fail {
                            //                                reason: FailReason::Eof(_),
                            //                                ..
                            //                            }) => {}
                            Err(fail) => {
                                report_parse_error(fail);
                                pending_src.clear();
                            }
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                // If we hit an eof, and we're allowed to keep going,
                // append the str to the src we're building up and continue.
                // (We only need to append it here if it was empty before;
                // otherwise, we already appended it before calling eval_and_format.)
                let pending_src = &mut editor
                    .helper_mut()
                    .expect("Editor helper was not set")
                    .pending_src;

                if pending_src.is_empty() {
                    pending_src.push_str("");
                }
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }

        prev_line_blank = false;
    }

    Ok(())
}
