extern crate pretty_assertions;

extern crate bumpalo;
extern crate indoc;
extern crate roc_collections;
extern crate roc_load;
extern crate roc_module;

#[cfg(test)]
mod cli_tests {
    use const_format::concatcp;
    use std::process::Command;
    use indoc::indoc;
    use roc_cli::{CMD_BUILD, CMD_CHECK, CMD_FORMAT, CMD_DEV, CMD_TEST};
    use cli_test_utils::helpers::{roc_file_to_exe, check_exe_with_valgrind, path_to_roc_binary, file_from_root, check_output_maybe_valgrind};
    use cli_test_utils::command::run_command;
    use cli_test_utils::run_roc::{run_roc_exe, run_roc_maybe_both_linkers, RocCmdParams};

    // used for valgrind
    #[cfg(all(unix, not(target_os = "macos")))]
    const ON_LINUX: bool = true;

    #[cfg(target_os = "macos")]
    const ON_LINUX: bool = false;

    #[cfg(windows)]
    const ON_LINUX: bool = false;

    // use valgrind (if supported on the current platform)
    #[derive(Debug, Clone, Copy)]
    enum UseValgrind {
        Yes,
        No,
    }

    const OPTIMIZE_FLAG: &str = concatcp!("--", roc_cli::FLAG_OPTIMIZE);
    const BUILD_HOST_FLAG: &str = concatcp!("--", roc_cli::FLAG_BUILD_HOST);
    const SUPPRESS_BUILD_HOST_WARNING_FLAG: &str =
        concatcp!("--", roc_cli::FLAG_SUPPRESS_BUILD_HOST_WARNING);
    const CHECK_FLAG: &str = concatcp!("--", roc_cli::FLAG_CHECK);
    #[allow(dead_code)]
    const TARGET_FLAG: &str = concatcp!("--", roc_cli::FLAG_TARGET);

    #[test]
    #[cfg_attr(windows, ignore)]
    fn platform_switching_rust() {
        let roc_file_folder = "examples/platform-switching";
        let roc_file = "rocLovesRust.roc";
        let roc_file_path = file_from_root(roc_file_folder, roc_file);
        let expected_output = "Roc <3 Rust!\n";
        
        let roc_cmd_out =
            run_roc_maybe_both_linkers(
                RocCmdParams {
                    sub_command: CMD_BUILD,
                    args: all_to_os_string([
                        roc_file_path.to_str().unwrap(),
                        BUILD_HOST_FLAG,
                        SUPPRESS_BUILD_HOST_WARNING_FLAG,
                        ]),
                    ..Default::default()
                },
                ON_LINUX
            );
        
        roc_cmd_out.assert_clean_success();
        
        let roc_exe_path = file_from_root(roc_file_folder, &roc_file_to_exe(roc_file));
        
        check_output_maybe_valgrind(&roc_exe_path, expected_output, ON_LINUX)
    }
    
    
    /*#[test]
    #[cfg_attr(windows, ignore)]
    fn platform_switching_zig() {
        let roc_file_folder = "examples/platform-switching";
        let roc_file = "rocLovesZig.roc";
        let expected_output = "Roc <3 Zig!\n";
        
        let exec_cli = ExecCLI::new_roc()
            .set_mode(BuildAndRun)
            .arg(BUILD_HOST_FLAG)
            .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
            .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
            .with_valgrind(ON_LINUX)
            .arg(file_from_root("examples/platform-switching", "rocLovesZig.roc").as_path());

        let roc_cmd_out = run_command(roc_cmd, &[]);
        roc_cmd_out.assert_clean_success();
        
        let roc_exe_path = file_from_root(roc_file_folder, &roc_file_to_exe(roc_file));
        
        check_output_maybe_valgrind(&roc_exe_path, expected_output, ON_LINUX)
    }*/
    /*
    #[test]
    fn platform_switching_wasm() {
        // this is a web assembly example, but we don't test with JS at the moment
        // so let's just check it for now
        let exec_cli = ExecCLI::new_roc().arg(CMD_CHECK).arg(
            file_from_root("examples/platform-switching", "rocLovesWebAssembly.roc").as_path(),
        );

        let out = exec_cli.run();
        out.assert_clean_success();
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn test_module_imports_pkg_w_flag() {
        let expected_ending = indoc!(
            r#"
            0 failed and 1 passed in <ignored for test> ms.
            "#
        );
        let exec_cli = ExecCLI::new_roc()
            .arg(CMD_TEST)
            .with_valgrind(ON_LINUX)
            .add_args(["--main", "tests/module_imports_pkg/app.roc"])
            .arg(file_from_root("crates/cli/tests/module_imports_pkg", "Module.roc").as_path());

        let out = exec_cli.run();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn test_module_imports_pkg_no_flag() {
        let expected_ending = indoc!(
            r#"
            ── UNRECOGNIZED PACKAGE in tests/module_imports_pkg/Module.roc ─────────────────

            This module is trying to import from `pkg`:

            3│  import pkg.Foo
                       ^^^^^^^

            A lowercase name indicates a package shorthand, but I don't know which
            packages are available.

            When checking a module directly, I look for a `main.roc` app or
            package to resolve shorthands from.

            You can create it, or specify an existing one with the --main flag."#
        );
        let exec_cli = ExecCLI::new_roc()
            .arg(CMD_TEST)
            .with_valgrind(ON_LINUX)
            .arg(file_from_root("crates/cli/tests/module_imports_pkg", "Module.roc").as_path());

        let out = exec_cli.run();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn test_module_imports_unknown_pkg() {
        let expected_ending = indoc!(
            r#"
            ── UNRECOGNIZED PACKAGE in tests/module_imports_pkg/ImportsUnknownPkg.roc ──────

            This module is trying to import from `cli`:

            3│  import cli.Foo
                       ^^^^^^^

            A lowercase name indicates a package shorthand, but I don't recognize
            this one. Did you mean one of these?

                pkg

            Note: I'm using the following module to resolve package shorthands:

                tests/module_imports_pkg/app.roc

            You can specify a different one with the --main flag."#
        );
        let exec_cli = ExecCLI::new_roc()
            .arg(CMD_TEST)
            .with_valgrind(ON_LINUX)
            .add_args(["--main", "tests/module_imports_pkg/app.roc"])
            .arg(
                file_from_root(
                    "crates/cli/tests/module_imports_pkg",
                    "ImportsUnknownPkg.roc",
                )
                .as_path(),
            );

        let out = exec_cli.run();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    /// this tests that a platform can correctly import a package
    fn platform_requires_pkg() {
        let expected_ending = "from app from package🔨 Building host ...\n";
        let exec_cli = ExecCLI::new_roc()
            .arg(CMD_DEV)
            .arg(BUILD_HOST_FLAG)
            .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
            .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
            .arg(file_from_root("crates/cli/tests/platform_requires_pkg", "app.roc").as_path());

        let out = exec_cli.run();
        out.assert_clean_success();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn transitive_expects() {
        let expected_ending = indoc!(
            r#"
            0 failed and 3 passed in <ignored for test> ms.
            "#
        );
        let exec_cli = ExecCLI::new_roc()
            .arg(CMD_TEST)
            .with_valgrind(ON_LINUX)
            .arg(file_from_root("crates/cli/tests/expects_transitive", "main.roc").as_path());

        let out = exec_cli.run();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn transitive_expects_verbose() {
        let expected_ending = indoc!(
            r#"
            Compiled in <ignored for test> ms.

            Direct.roc:
                0 failed and 2 passed in <ignored for test> ms.

            Transitive.roc:
                0 failed and 1 passed in <ignored for test> ms.
            "#
        );
        let exec_cli = ExecCLI::new_roc()
            .arg(CMD_TEST)
            .with_valgrind(ON_LINUX)
            .arg("--verbose")
            .arg(file_from_root("crates/cli/tests/expects_transitive", "main.roc").as_path());

        let out = exec_cli.run();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(
        windows,
        ignore = "Flaky failure: Roc command failed with status ExitStatus(ExitStatus(3221225477))"
    )]
    fn fibonacci() {
        let expected_ending = "";
        let exec_cli = ExecCLI::new_roc()
            .set_mode(BuildAndRun)
            .arg(BUILD_HOST_FLAG)
            .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
            .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
            .with_valgrind(ON_LINUX)
            .arg(file_from_root("crates/cli/tests/algorithms", "fibonacci.roc").as_path());

        let out = exec_cli.run();
        out.assert_clean_success();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    fn quicksort() {
        let expected_ending =
            "[0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2]\n🔨 Building host ...\n";
        let exec_cli = ExecCLI::new_roc()
            .set_mode(BuildAndRun)
            .arg(BUILD_HOST_FLAG)
            .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
            .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
            .with_valgrind(ON_LINUX)
            .arg(file_from_root("crates/cli/tests/algorithms", "quicksort.roc").as_path());

        let out = exec_cli.run();
        out.assert_clean_success();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    // TODO: write a new test once mono bugs are resolved in investigation
    #[test]
    #[cfg(not(debug_assertions))] // https://github.com/roc-lang/roc/issues/4806
    fn check_virtual_dom_server() {
        ExecCLI::new_roc()
            .add_args([
                CMD_CHECK,
                file_from_root("examples/virtual-dom-wip", "example-server.roc")
                    .to_str()
                    .unwrap(),
            ])
            .run()
            .assert_clean_success();
    }

    // TODO: write a new test once mono bugs are resolved in investigation
    #[test]
    #[cfg(not(debug_assertions))] // https://github.com/roc-lang/roc/issues/4806
    fn check_virtual_dom_client() {
        ExecCLI::new_roc()
            .add_args([
                CMD_CHECK,
                file_from_root("examples/virtual-dom-wip", "example-client.roc")
                    .to_str()
                    .unwrap(),
            ])
            .run()
            .assert_clean_success();
    }

    #[test]
    #[cfg_attr(windows, ignore)]
    // tea = The Elm Architecture
    fn terminal_ui_tea() {
        let expected_ending = "Hello Worldfoo!\n🔨 Building host ...\n";
        let exec_cli = ExecCLI::new_roc()
            .set_mode(BuildAndRun)
            .arg(BUILD_HOST_FLAG)
            .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
            .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
            .with_valgrind(ON_LINUX)
            .arg(file_from_root("crates/cli/tests/tui", "main.roc").as_path())
            .with_stdin_vals(vec!["foo\n"]);

        let out = exec_cli.run();
        out.assert_clean_success();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    #[cfg_attr(
        any(target_os = "windows", target_os = "linux", target_os = "macos"),
        ignore = "Segfault, likely broken because of alias analysis: https://github.com/roc-lang/roc/issues/6544"
    )]
    fn false_interpreter() {
        // Test building
        let build_exec_cli = ExecCLI::new_roc()
            .arg(CMD_BUILD)
            .arg(BUILD_HOST_FLAG)
            .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
            .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
            .arg(file_from_root("crates/cli/tests/false-interpreter", "False.roc").as_path())
            .run();

        build_exec_cli.assert_clean_success();

        // Test running
        let exec_cli = ExecCLI::new_roc()
            .set_mode(BuildAndRun)
            .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
            .with_valgrind(ON_LINUX)
            .arg(file_from_root("crates/cli/tests/false-interpreter", "False.roc").as_path())
            .add_args([
                "--",
                file_from_root("crates/cli/tests/false-interpreter/examples", "sqrt.false")
                    .as_path()
                    .to_str()
                    .unwrap(),
            ]);

        let expected_ending = "1414";

        let out = exec_cli.run();
        out.assert_clean_success();
        out.assert_stdout_and_stderr_ends_with(expected_ending);
    }

    mod test_platform_effects_zig {
        use super::*;
        use cli_utils::helpers::{file_from_root, ExecCLI};
        use roc_cli::{CMD_BUILD, CMD_DEV};

        static BUILD_PLATFORM_HOST: std::sync::Once = std::sync::Once::new();

        /// Build the platform host once for all tests in this module
        fn build_platform_host() {
            BUILD_PLATFORM_HOST.call_once(|| {
                let out = ExecCLI::new_roc()
                    .arg(CMD_BUILD)
                    .arg(BUILD_HOST_FLAG)
                    .arg(OPTIMIZE_FLAG)
                    .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
                    .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                    .arg(
                        file_from_root("crates/cli/tests/effects/platform/", "app-stub.roc")
                            .as_path(),
                    )
                    .run();
                out.assert_clean_success();
            });
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn interactive_effects() {
            build_platform_host();

            let expected_ending = "hi there!\nIt is known\n";
            let exec_cli = ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_from_root("crates/cli/tests/effects", "print-line.roc").as_path())
                .with_stdin_vals(vec!["hi there!"]);

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn combine_tasks_with_record_builder() {
            build_platform_host();

            let expected_ending = "For multiple tasks: {a: 123, b: \"abc\", c: [123]}\n";

            let exec_cli = ExecCLI::new_roc()
                .arg(CMD_DEV)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .arg(file_from_root("crates/cli/tests/effects", "combine-tasks.roc").as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn inspect_logging() {
            build_platform_host();

            let expected_ending = "(@Community {friends: [{2}, {2}, {0, 1}], people: [(@Person {age: 27, favoriteColor: Blue, firstName: \"John\", hasBeard: Bool.true, lastName: \"Smith\"}), (@Person {age: 47, favoriteColor: Green, firstName: \"Debby\", hasBeard: Bool.false, lastName: \"Johnson\"}), (@Person {age: 33, favoriteColor: (RGB (255, 255, 0)), firstName: \"Jane\", hasBeard: Bool.false, lastName: \"Doe\"})]})\n";
            let exec_cli = ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_from_root("crates/cli/tests/effects", "inspect-logging.roc").as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn module_params_pass_task() {
            build_platform_host();

            let exec_cli = ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_from_root("crates/cli/tests/module_params", "pass_task.roc").as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with("Hi, Agus!\n");
        }
    }

    mod test_platform_simple_zig {
        use super::*;
        use cli_utils::helpers::{file_from_root, ExecCLI};
        use indoc::indoc;
        use roc_cli::{CMD_BUILD, CMD_DEV, CMD_TEST};

        static BUILD_PLATFORM_HOST: std::sync::Once = std::sync::Once::new();

        /// Build the platform host once for all tests in this module
        fn build_platform_host() {
            BUILD_PLATFORM_HOST.call_once(|| {
                let out = ExecCLI::new_roc()
                    .arg(CMD_BUILD)
                    .arg(BUILD_HOST_FLAG)
                    .arg(OPTIMIZE_FLAG)
                    .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
                    .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                    .arg(
                        file_from_root("crates/cli/tests/test-platform-simple-zig", "app.roc")
                            .as_path(),
                    )
                    .run();
                out.assert_clean_success();
            });
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_multi_dep_str_unoptimized() {
            build_platform_host();

            let expected_ending = "I am Dep2.str2\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(
                    file_from_root("crates/cli/tests/fixtures/multi-dep-str", "Main.roc").as_path(),
                );

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_multi_dep_str_optimized() {
            build_platform_host();

            let expected_ending = "I am Dep2.str2\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .arg(OPTIMIZE_FLAG)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(
                    file_from_root("crates/cli/tests/fixtures/multi-dep-str", "Main.roc").as_path(),
                );

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_multi_dep_thunk_unoptimized() {
            build_platform_host();

            let expected_ending = "I am Dep2.value2\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(
                    file_from_root("crates/cli/tests/fixtures/multi-dep-thunk", "Main.roc")
                        .as_path(),
                );

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(
            windows,
            ignore = "Flaky failure: Roc command failed with status ExitStatus(ExitStatus(3221225477))"
        )]
        fn run_multi_dep_thunk_optimized() {
            build_platform_host();

            let expected_ending = "I am Dep2.value2\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .arg(OPTIMIZE_FLAG)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(
                    file_from_root("crates/cli/tests/fixtures/multi-dep-thunk", "Main.roc")
                        .as_path(),
                );

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_packages_unoptimized() {
            build_platform_host();

            let expected_ending =
                "Hello, World! This text came from a package! This text came from a CSV package!\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_from_root("crates/cli/tests/fixtures/packages", "app.roc").as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_packages_optimized() {
            build_platform_host();

            let expected_ending =
                "Hello, World! This text came from a package! This text came from a CSV package!\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .arg(OPTIMIZE_FLAG)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_from_root("crates/cli/tests/fixtures/packages", "app.roc").as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_transitive_deps_app() {
            build_platform_host();

            let file_path = file_from_root(
                "crates/cli/tests/fixtures/transitive-deps",
                "direct-one.roc",
            );

            let expected_ending = "[One imports Two: From two]\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_path.as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_transitive_and_direct_dep_app() {
            build_platform_host();

            let file_path = file_from_root(
                "crates/cli/tests/fixtures/transitive-deps",
                "direct-one-and-two.roc",
            );

            let expected_ending = "[One imports Two: From two] | From two\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_path.as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn run_double_transitive_dep_app() {
            build_platform_host();

            let file_path = file_from_root(
                "crates/cli/tests/fixtures/transitive-deps",
                "direct-zero.roc",
            );

            let expected_ending = "[Zero imports One: [One imports Two: From two]]\n";
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .set_mode(BuildAndRun)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_path.as_path());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        fn expects_dev() {
            build_platform_host();

            let expected_ending = indoc!(
                r#"
                ── EXPECT FAILED in tests/expects/expects.roc ──────────────────────────────────

                This expectation failed:

                25│      expect words == []
                                ^^^^^^^^^^^

                When it failed, these variables had these values:

                words : List Str
                words = ["this", "will", "for", "sure", "be", "a", "large", "string", "so", "when", "we", "split", "it", "it", "will", "use", "seamless", "slices", "which", "affect", "printing"]

                Program finished!

                [<ignored for tests>:28] x = 42
                [<ignored for tests>:30] "Fjoer en ferdjer frieten oan dyn geve lea" = "Fjoer en ferdjer frieten oan dyn geve lea"
                [<ignored for tests>:32] "this is line 24" = "this is line 24"
                [<ignored for tests>:18] x = "abc"
                [<ignored for tests>:18] x = 10
                [<ignored for tests>:18] x = (A (B C))
                "#
            );
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .arg(CMD_DEV)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .with_valgrind(ON_LINUX)
                .arg(file_from_root("crates/cli/tests/expects", "expects.roc").as_path());

            let out = exec_cli.run();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        fn expects_test() {
            build_platform_host();

            let expected_ending = indoc!(
                r#"
                ── EXPECT FAILED in tests/expects/expects.roc ──────────────────────────────────

                This expectation failed:

                6│      expect a == 2
                               ^^^^^^

                When it failed, these variables had these values:

                a : Num *
                a = 1

                ── EXPECT FAILED in tests/expects/expects.roc ──────────────────────────────────

                This expectation failed:

                7│      expect a == 3
                               ^^^^^^

                When it failed, these variables had these values:

                a : Num *
                a = 1

                ── EXPECT FAILED in tests/expects/expects.roc ──────────────────────────────────

                This expectation failed:

                11│>  expect
                12│>      a = makeA
                13│>      b = 2i64
                14│>
                15│>      a == b

                When it failed, these variables had these values:

                a : Int Signed64
                a = 1

                b : I64
                b = 2


                1 failed and 0 passed in <ignored for test> ms.
                "#
            );
            let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                .arg(CMD_TEST)
                .with_valgrind(ON_LINUX)
                .arg(file_from_root("crates/cli/tests/expects", "expects.roc").as_path());

            let out = exec_cli.run();
            out.assert_stdout_and_stderr_ends_with(expected_ending);
        }
    }

    // TODO not sure if this cfg should still be here: #[cfg(not(debug_assertions))]
    // this is for testing the benchmarks, to perform proper benchmarks see crates/cli/benches/README.md
    mod test_benchmarks {
        use super::{
            UseValgrind, ON_LINUX, BUILD_HOST_FLAG, OPTIMIZE_FLAG,
            SUPPRESS_BUILD_HOST_WARNING_FLAG,
        };
        use cli_utils::helpers::{file_from_root, ExecCLI, Mode::*};
        use roc_cli::CMD_BUILD;

        // #[allow(unused_imports)]
        use std::sync::Once;

        static BUILD_PLATFORM_HOST: Once = Once::new();

        /// Build the platform host once for all tests in this module
        fn build_platform_host() {
            BUILD_PLATFORM_HOST.call_once(|| {
                let out = ExecCLI::new_roc()
                    .arg(CMD_BUILD)
                    .arg(BUILD_HOST_FLAG)
                    .arg(OPTIMIZE_FLAG)
                    .arg(SUPPRESS_BUILD_HOST_WARNING_FLAG)
                    .add_arg_if(super::LINKER_FLAG, super::TEST_LEGACY_LINKER)
                    .arg(
                        file_from_root("crates/cli/tests/benchmarks/platform", "app.roc").as_path(),
                    )
                    .run();
                out.assert_clean_success();
            });
        }

        fn test_benchmark(
            roc_filename: &str,
            stdin: Vec<&'static str>,
            expected_ending: &str,
            use_valgrind: UseValgrind,
        ) {
            let dir_name = "crates/cli/tests/benchmarks";
            let file_path = file_from_root(dir_name, roc_filename);

            build_platform_host();

            #[cfg(all(not(feature = "wasm32-cli-run"), not(feature = "i386-cli-run")))]
            {
                let exec_cli = cli_utils::helpers::ExecCLI::new_roc()
                    .set_mode(BuildAndRun)
                    .add_arg_if(super::LINKER_FLAG, super::TEST_LEGACY_LINKER)
                    .arg(file_path.as_path())
                    .with_valgrind(matches!(use_valgrind, UseValgrind::Yes) && ON_LINUX)
                    .with_stdin_vals(stdin);

                let out = exec_cli.run();
                out.assert_clean_success();
                out.assert_stdout_and_stderr_ends_with(expected_ending);
            }

            #[cfg(feature = "wasm32-cli-run")]
            check_output_wasm(file_path.as_path(), stdin, expected_ending);

            #[cfg(feature = "i386-cli-run")]
            check_output_i386(file_path.as_path(), stdin, expected_ending);
        }

        #[cfg(feature = "wasm32-cli-run")]
        fn check_output_wasm(file_name: &std::path::Path, stdin: Vec<&str>, expected_ending: &str) {
            // Check with and without optimizations
            check_wasm_output_with_stdin(file_name, stdin.clone(), &[], expected_ending);

            check_wasm_output_with_stdin(file_name, stdin, &[OPTIMIZE_FLAG], expected_ending);
        }

        #[cfg(feature = "wasm32-cli-run")]
        fn check_wasm_output_with_stdin(
            file: &std::path::Path,
            stdin: Vec<&str>,
            flags: &[&str],
            expected_ending: &str,
        ) {
            use super::{concatcp, TARGET_FLAG};

            let mut flags = flags.to_vec();
            flags.push(concatcp!(TARGET_FLAG, "=wasm32"));

            let out = ExecCLI::new_roc()
                .arg(CMD_BUILD)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .arg(file)
                .add_args(flags)
                .run();

            out.assert_clean_success();

            let stdout = crate::run_wasm(&file.with_extension("wasm"), stdin);

            if !stdout.ends_with(expected_ending) {
                panic!(
                    "expected output to end with {:?} but instead got {:#?}",
                    expected_ending, stdout
                );
            }
        }

        #[cfg(feature = "i386-cli-run")]
        fn check_output_i386(
            file_path: &std::path::Path,
            stdin: Vec<&'static str>,
            expected_ending: &str,
        ) {
            use super::{concatcp, TARGET_FLAG};

            let i386_target_arg = concatcp!(TARGET_FLAG, "=x86_32");

            let exec_cli = ExecCLI::new_roc()
                .arg(CMD_DEV)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .arg(i386_target_arg)
                .arg(file_path)
                .with_stdin_vals(stdin.clone());

            let out = exec_cli.run();
            out.assert_clean_success();
            out.assert_stdout_and_stderr_ends_with(expected_ending);

            let run_optimized = ExecCLI::new_roc()
                .arg(CMD_DEV)
                .add_arg_if(LINKER_FLAG, TEST_LEGACY_LINKER)
                .arg(i386_target_arg)
                .arg(OPTIMIZE_FLAG)
                .arg(file_path)
                .with_stdin_vals(stdin.clone());

            let out_optimized = run_optimized.run();
            out_optimized.assert_clean_success();
            out_optimized.assert_stdout_and_stderr_ends_with(expected_ending);
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn nqueens() {
            test_benchmark("nQueens.roc", vec!["6"], "4\n", UseValgrind::Yes)
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn cfold() {
            test_benchmark("cFold.roc", vec!["3"], "11 & 11\n", UseValgrind::Yes)
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn deriv() {
            test_benchmark(
                "deriv.roc",
                vec!["2"],
                "1 count: 6\n2 count: 22\n",
                UseValgrind::Yes,
            )
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn rbtree_ck() {
            test_benchmark("rBTreeCk.roc", vec!["100"], "10\n", UseValgrind::Yes)
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn rbtree_insert() {
            test_benchmark(
                "rBTreeInsert.roc",
                vec![],
                "Node Black 0 {} Empty Empty\n",
                UseValgrind::Yes,
            )
        }

        /*
        // rbtree_del does not work
        #[test]
        fn rbtree_del() {
            test_benchmark(
                "rBTreeDel.roc",
                &["420"],
                "30\n",
                UseValgrind::Yes,
            )
        }
        */

        #[test]
        #[cfg_attr(windows, ignore)]
        fn astar() {
            if cfg!(feature = "wasm32-cli-run") {
                eprintln!("WARNING: skipping testing benchmark testAStar.roc because it currently does not work on wasm32 due to dictionaries.");
            } else {
                test_benchmark("testAStar.roc", vec![], "True\n", UseValgrind::No)
            }
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn base64() {
            test_benchmark(
                "testBase64.roc",
                vec![],
                "encoded: SGVsbG8gV29ybGQ=\ndecoded: Hello World\n",
                UseValgrind::Yes,
            )
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn closure() {
            test_benchmark("closure.roc", vec![], "", UseValgrind::No)
        }

        #[test]
        #[cfg_attr(windows, ignore)]
        fn issue2279() {
            test_benchmark("issue2279.roc", vec![], "Hello, world!\n", UseValgrind::Yes)
        }

        #[test]
        fn quicksort_app() {
            eprintln!("WARNING: skipping testing benchmark quicksortApp.roc because the test is broken right now!");
            // test_benchmark(
            //     "quicksortApp.roc",
            //     vec![],
            //     "todo put the correct quicksort answer here",
            //     UseValgrind::Yes,
            // )
        }
    }

    #[test]
    fn known_type_error() {
        let expected_ending = indoc!(
            r#"

            ── TYPE MISMATCH in tests/known_bad/TypeError.roc ──────────────────────────────

            Something is off with the body of the main definition:

            3│  main : Str -> Task {} []
            4│  main = \_ ->
            5│      "this is a string, not a Task {} [] function like the platform expects."
                    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

            The body is a string of type:

                Str

            But the type annotation on main says it should be:

                Task {} []

            Tip: Add type annotations to functions or values to help you figure
            this out.

            ────────────────────────────────────────────────────────────────────────────────

            1 error and 0 warning found in <ignored for test> ms
            "#
        );

        ExecCLI::new_roc()
            .arg(CMD_CHECK)
            .arg(file_from_root(
                "crates/cli/tests/known_bad",
                "TypeError.roc",
            ))
            .run()
            .assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    fn known_type_error_with_long_path() {
        let expected_ending = indoc!(
            r#"

            ── UNUSED IMPORT in ...nown_bad/UnusedImportButWithALongFileNameForTesting.roc ─

            Symbol is imported but not used.

            3│  import Symbol exposing [Ident]
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

            Since Symbol isn't used, you don't need to import it.

            ────────────────────────────────────────────────────────────────────────────────

            0 error and 1 warning found in <ignored for test> ms
            "#
        );

        ExecCLI::new_roc()
            .arg(CMD_CHECK)
            .arg(file_from_root(
                "crates/cli/tests/known_bad",
                "UnusedImportButWithALongFileNameForTesting.roc",
            ))
            .run()
            .assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    fn exposed_not_defined() {
        let expected_ending = indoc!(
            r#"

            ── MISSING DEFINITION in tests/known_bad/ExposedNotDefined.roc ─────────────────

            bar is listed as exposed, but it isn't defined in this module.

            You can fix this by adding a definition for bar, or by removing it
            from exposes.

            ────────────────────────────────────────────────────────────────────────────────

            1 error and 0 warning found in <ignored for test> ms
            "#
        );

        ExecCLI::new_roc()
            .arg(CMD_CHECK)
            .arg(file_from_root(
                "crates/cli/tests/known_bad",
                "ExposedNotDefined.roc",
            ))
            .run()
            .assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    fn unused_import() {
        let expected_ending = indoc!(
            r#"

            ── UNUSED IMPORT in tests/known_bad/UnusedImport.roc ───────────────────────────

            Symbol is imported but not used.

            3│  import Symbol exposing [Ident]
                ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

            Since Symbol isn't used, you don't need to import it.

            ────────────────────────────────────────────────────────────────────────────────

            0 error and 1 warning found in <ignored for test> ms
            "#
        );

        ExecCLI::new_roc()
            .arg(CMD_CHECK)
            .arg(file_from_root(
                "crates/cli/tests/known_bad",
                "UnusedImport.roc",
            ))
            .run()
            .assert_stdout_and_stderr_ends_with(expected_ending);
    }

    #[test]
    fn format_check_good() {
        ExecCLI::new_roc()
            .arg(CMD_FORMAT)
            .arg(CHECK_FLAG)
            .arg(file_from_root("crates/cli/tests/fixtures/format", "Formatted.roc").as_path())
            .run()
            .assert_clean_success();
    }

    #[test]
    fn format_check_reformatting_needed() {
        ExecCLI::new_roc()
            .arg(CMD_FORMAT)
            .arg(CHECK_FLAG)
            .arg(file_from_root("crates/cli/tests/fixtures/format", "NotFormatted.roc").as_path())
            .run()
            .assert_nonzero_exit();
    }

    #[test]
    fn format_check_folders() {
        // This fails, because "NotFormatted.roc" is present in this folder
        ExecCLI::new_roc()
            .arg(CMD_FORMAT)
            .arg(CHECK_FLAG)
            .arg(dir_from_root("crates/cli/tests/fixtures/format").as_path())
            .run()
            .assert_nonzero_exit();

        // This doesn't fail, since only "Formatted.roc" and non-roc files are present in this folder
        ExecCLI::new_roc()
            .arg(CMD_FORMAT)
            .arg(CHECK_FLAG)
            .arg(dir_from_root("crates/cli/tests/fixtures/format/formatted_directory").as_path())
            .run()
            .assert_clean_success();
    }
}

#[cfg(feature = "wasm32-cli-run")]
fn run_wasm(wasm_path: &std::path::Path, stdin: Vec<&str>) -> String {
    use bumpalo::Bump;
    use roc_wasm_interp::{DefaultImportDispatcher, Instance, Value, WasiFile};

    let wasm_bytes = std::fs::read(wasm_path).unwrap();
    let arena = Bump::new();

    let mut instance = {
        let mut fake_stdin = vec![];
        let fake_stdout = vec![];
        let fake_stderr = vec![];
        for s in stdin {
            fake_stdin.extend_from_slice(s.as_bytes())
        }

        let mut dispatcher = DefaultImportDispatcher::default();
        dispatcher.wasi.files = vec![
            WasiFile::ReadOnly(fake_stdin),
            WasiFile::WriteOnly(fake_stdout),
            WasiFile::WriteOnly(fake_stderr),
        ];

        Instance::from_bytes(&arena, &wasm_bytes, dispatcher, false).unwrap()
    };

    let result = instance.call_export("_start", []);

    match result {
        Ok(Some(Value::I32(0))) => match &instance.import_dispatcher.wasi.files[1] {
            WasiFile::WriteOnly(fake_stdout) => String::from_utf8(fake_stdout.clone())
                .unwrap_or_else(|_| "Wasm test printed invalid UTF-8".into()),
            _ => unreachable!(),
        },
        Ok(Some(Value::I32(exit_code))) => {
            format!("WASI app exit code {}", exit_code)
        }
        Ok(Some(val)) => {
            format!("WASI _start returned an unexpected number type {:?}", val)
        }
        Ok(None) => "WASI _start returned no value".into(),
        Err(e) => {
            format!("WASI error {}", e)
        }
    }*/
}