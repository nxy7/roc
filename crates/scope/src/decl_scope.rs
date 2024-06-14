/// Scope within an individual top-level declaration.
/// It has a reference to the entire top-level scope
/// (so, it can see other top-level declarations),
/// and it can contain nested scopes.
use crate::{
    ids::{LowercaseId, ModuleId, UppercaseId},
    problem::Problem,
    tl_scope::TopLevelScope,
};
use bumpalo::{collections::vec::Vec, Bump};
use core::fmt::Debug;

#[derive(Debug, Clone, Copy)]
enum IsUsed {
    Used,
    Unused,
}

/// TODO replace this with the real Vec4 that stores 1 length as u32 etc. (We can even do u16 len.)
type Vec4<'a, A, B, C, D> = Vec<'a, (A, B, C, D)>;

pub struct DeclScope<'a, LcStrId, UcStrId, ShorthandStrId, ModuleStrId, Region> {
    /// The top-level scope is used to verify shadowing, and as a fallback for lookups
    /// that fail here. It's valuable for correctness to store it in the structure itself
    /// rather than having each method accept it, because this ensures that it cannot
    /// be modified in between calls.
    ///
    /// Top-level scope should never be modified once we have begun canonicalizing individual
    /// declarations, because we need to be able to do lookups in top-level declarations that
    /// appear later in the file than those declarations.
    tl_scope: &'a TopLevelScope<'a, LcStrId, UcStrId, ShorthandStrId, ModuleStrId, Region>,

    // These are not affected by reset(), because we want them to be unique across the whole module.
    // They are stored as usize so that we can detect later if they overflowed.
    next_lc_id: u32,
    next_uc_id: u32,
    next_module_id: u32,

    /// Lowercase bindings may shadow, but uppercase ones and module imports may not.
    lc_bindings: Vec4<'a, ModuleId, LcStrId, Region, IsUsed>,
    uc_bindings: Vec4<'a, ModuleId, UcStrId, Region, IsUsed>,
    imports: Vec4<'a, Option<ShorthandStrId>, ModuleStrId, Region, IsUsed>,

    // Anything before this is considered out of scope, and the lookup will fail.
    // At the end of each declaration, we move these up to next_lc_id (etc.) so that
    // the new declaration can't "see" bindings from the previous one, yet we still
    // have the previous bindings around so that we can use a LowercaseId (etc.)
    // to look up its ModuleId and such.
    lc_start: u16,
    uc_start: u16,
    imports_start: u16,

    /// Whenever we pop the scope, we take the .last() of this and
    /// truncate that many entries from the end of bindings.
    scope_lengths: Vec<'a, ScopeLen>,
}

#[derive(Debug, Clone, Copy, Default)]
struct ScopeLen {
    lc_bindings: u32,
    uc_bindings: u32,
    imports: u32,
}

impl<
        'a,
        LcStrId: Copy + PartialEq + Debug,
        UcStrId: Copy + PartialEq + Debug,
        ShorthandStrId: Copy + PartialEq + Debug,
        ModuleStrId: Copy + PartialEq + Debug,
        Region: Copy + Debug,
    > DeclScope<'a, LcStrId, UcStrId, ShorthandStrId, ModuleStrId, Region>
{
    pub fn new(
        arena: &'a Bump,
        tl_scope: &'a TopLevelScope<'a, LcStrId, UcStrId, ShorthandStrId, ModuleStrId, Region>,
    ) -> Self {
        Self {
            tl_scope,
            next_lc_id: tl_scope.num_lc_bindings() as u32,
            next_uc_id: tl_scope.num_uc_bindings() as u32,
            next_module_id: tl_scope.num_imports() as u32,
            lc_bindings: Vec::new_in(arena),
            uc_bindings: Vec::new_in(arena),
            imports: Vec::new_in(arena),
            scope_lengths: Vec::new_in(arena),
            lc_start: 0,
            uc_start: 0,
            imports_start: 0,
        }
    }

    pub fn end_decl(
        &mut self,
        // Is a lowercase identifier used, other than the lookups we already knew about?
        // e.g. is either of these true?
        //     scope.abilities_store.is_specialization_name(symbol)
        //     symbol.is_exposed_for_builtin_derivers()
        lc_is_used: impl Fn(LowercaseId) -> bool,
        // Is an uppercase identifier used, other than the lookups we already knew about?
        // e.g. is symbol.is_exposed_for_builtin_derivers() true?
        uc_is_used: impl Fn(UppercaseId) -> bool,
        // Whether the given module exposes the given lowercase ident
        mod_exposes_lc: impl Fn(ShorthandStrId, ModuleStrId, LcStrId) -> bool,
        // Whether the given module exposes the given uppercase ident
        mod_exposes_uc: impl Fn(ShorthandStrId, ModuleStrId, UcStrId) -> bool,
        // Whether the given package exposes the given module
        pkg_exposes_module: impl Fn(ShorthandStrId, ModuleStrId) -> bool,
        mut report_unused_lc: impl FnMut(ModuleStrId, LcStrId, Region),
        mut report_unused_uc: impl FnMut(ModuleStrId, UcStrId, Region),
        mut report_unused_import: impl FnMut(Option<ShorthandStrId>, ModuleStrId, Region),
        mut report_lookup_of_unexposed_lc: impl FnMut(ShorthandStrId, ModuleStrId, LcStrId, Region),
        mut report_lookup_of_unexposed_uc: impl FnMut(ShorthandStrId, ModuleStrId, UcStrId, Region),
        mut report_import_of_unexposed_mod: impl FnMut(ShorthandStrId, ModuleStrId, Region),
    ) {
        // Report unused things
        {
            for (module_id, str_id, region, is_used) in self.lc_bindings {
                let mod_str_id = self.str_id_from_module_id(module_id);

                if let Some(shorthand_str_id) = self.shorthand_from_module_id(module_id) {
                    if !mod_exposes_lc(shorthand_str_id, mod_str_id, str_id) {
                        report_lookup_of_unexposed_lc(shorthand_str_id, mod_str_id, str_id, region);
                    }
                }

                if let IsUsed::Unused = is_used {
                    // It's not exposed and wasn't looked up; report it as unused!
                    report_unused_lc(mod_str_id, str_id, region);
                }
            }

            for (module_id, str_id, region, is_used) in self.uc_bindings {
                let mod_str_id = self.str_id_from_module_id(module_id);

                if let Some(shorthand_str_id) = self.shorthand_from_module_id(module_id) {
                    if !mod_exposes_uc(shorthand_str_id, mod_str_id, str_id) {
                        report_lookup_of_unexposed_uc(shorthand_str_id, mod_str_id, str_id, region);
                    }
                }

                if let IsUsed::Unused = is_used {
                    // It's not exposed and wasn't looked up; report it as unused!
                    report_unused_uc(mod_str_id, str_id, region);
                }
            }

            for (opt_shorthand_str_id, module_str_id, region, is_used) in self.imports {
                if let Some(shorthand_str_id) = opt_shorthand_str_id {
                    if !pkg_exposes_module(shorthand_str_id, module_str_id) {
                        report_import_of_unexposed_mod(shorthand_str_id, module_str_id, region);
                    }
                }

                if let IsUsed::Unused = is_used {
                    report_unused_import(opt_shorthand_str_id, module_str_id, region);
                }
            }
        }

        // Reset for the next binding
        {
            // Do exhaustive destructuring here so if we add a field later, we don't forget
            // to include it in the reset.
            let Self {
                lc_bindings,
                uc_bindings,
                imports,
                scope_lengths,
                tl_scope: _, // Do not reset TL scope; it doesn't change when decls end!
                next_lc_id,
                next_uc_id,
                next_module_id,
                lc_start,
                uc_start,
                imports_start: import_start,
            } = self;

            // Do not reset the next_* values; IDs should be unique across the whole module!
            // However, we do want to update the *_start values so that future lookups treat
            // all of these as out of scope (as they should be, since they are now in the previous decl.)
            *lc_start = *next_lc_id as u16;
            *uc_start = *next_uc_id as u16;
            *import_start = *next_module_id as u16;

            lc_bindings.truncate(0);
            uc_bindings.truncate(0);
            imports.truncate(0);
            scope_lengths.truncate(0);
        }
    }

    /// If this is from `exposes` in an `import`, this should be passed that module's ModuleId.
    /// Otherwise, this should be passed the current module's ModuleId.
    ///
    /// This can never fail because it's always allowed to shadow an existing binding.
    pub fn bind_lc(&mut self, module_id: ModuleId, str_id: LcStrId, region: Region) -> LowercaseId {
        // Increment the current scope lengths. This is needed so that when we pop
        // a scope, we pop the correct number of bindings.
        match self.scope_lengths.last_mut() {
            Some(lengths) => {
                // This increment should never overflow because it's u32 and we support at most
                // u16::MAX lines with u16::MAX bytes per line.
                *lengths = ScopeLen {
                    lc_bindings: lengths.lc_bindings + 1,
                    uc_bindings: lengths.uc_bindings,
                    imports: lengths.imports,
                };
            }
            None => {
                // TODO panic in debug mode because we're trying to add a binding with no nested scopes!
            }
        }

        // This could theoretically overflow, but if it does, we'll catch that in validate() later.
        let lc_id = LowercaseId(self.next_lc_id as u16);

        self.next_lc_id += 1;
        self.lc_bindings
            .push((module_id, str_id, region, IsUsed::Unused));

        lc_id
    }

    /// If this is from `exposes` in an `import`, this should be passed that module's ModuleId.
    /// Otherwise, this should be passed the current module's ModuleId.
    ///
    /// If this would shadow an existing binding that's already in scope, returns Err with
    /// the original binding's Region.
    pub fn bind_uc(
        &mut self,
        module_id: ModuleId,
        str_id: UcStrId,
        region: Region,
    ) -> Result<UppercaseId, Region> {
        // Increment the current scope lengths. This is needed so that when we pop
        // a scope, we pop the correct number of bindings.
        match self.scope_lengths.last_mut() {
            Some(lengths) => {
                // This increment should never overflow because it's u32 and we support at most
                // u16::MAX lines with u16::MAX bytes per line.
                *lengths = ScopeLen {
                    lc_bindings: lengths.lc_bindings,
                    uc_bindings: lengths.uc_bindings + 1,
                    imports: lengths.imports,
                };
            }
            None => {
                // TODO panic in debug mode because we're trying to add a binding with no nested scopes!
            }
        }

        // This lookup should usually fail; if it succeeds then we're shadowing something!
        match self.lookup_uc_unqualified(str_id) {
            None => {
                // This cast should not overflow because we only allow one uppercase binding per line syntactically,
                // and we also only allow u16::MAX total lines.
                let uc_id = UppercaseId(self.next_uc_id as u16);

                self.next_uc_id += 1;
                self.uc_bindings
                    .push((module_id, str_id, region, IsUsed::Unused));

                Ok(uc_id)
            }
            Some((_uc_id, region)) => Err(region),
        }
    }

    /// NOTE: When handling a module importing using `as`, always pass the `as`, not the
    /// original module name. The thing after `as` is what will be added to scope; scope
    /// does not care about (or want to know about) the original!
    ///
    /// If this would shadow an existing import that's already in scope, returns Err with
    /// the original import's Region.
    pub fn import(
        &mut self,
        shorthand: Option<ShorthandStrId>,
        str_id: ModuleStrId,
        region: Region,
    ) -> Result<ModuleId, Region> {
        // Increment the current scope lengths. This is needed so that when we pop
        // a scope, we pop the correct number of bindings.
        match self.scope_lengths.last_mut() {
            Some(lengths) => {
                // This increment should never overflow because it's u32 and we support at most
                // u16::MAX lines with u16::MAX bytes per line.
                *lengths = ScopeLen {
                    lc_bindings: lengths.lc_bindings,
                    uc_bindings: lengths.uc_bindings,
                    imports: lengths.imports + 1,
                };
            }
            None => {
                // TODO panic in debug mode because we're trying to add a binding with no nested scopes!
            }
        }

        // This lookup should usually fail; if it succeeds then we're shadowing something!
        match self.lookup_module(str_id) {
            None => {
                // This cast should not overflow because we only allow one uppercase binding per line syntactically,
                // and we also only allow u16::MAX total lines.
                let module_id = ModuleId(self.next_module_id as u16);

                self.next_module_id += 1;
                self.imports
                    .push((shorthand, str_id, region, IsUsed::Unused));

                Ok(module_id)
            }
            Some((_module_id, region)) => Err(region),
        }
    }

    pub fn lookup_lc_unqualified(&mut self, str_id: LcStrId) -> Option<(LowercaseId, Region)> {
        // Start searching at lc_start; anything before that is the previous decl's bindings!
        let lc_start = self.lc_start as usize;
        let bindings_in_decl = &mut self.lc_bindings[lc_start..];
        let mut index = self.tl_scope.num_lc_bindings() + self.lc_bindings.len(); // index starts at the very end.

        for (_haystack_module_id, haystack_str_id, region, is_used) in bindings_in_decl
            .iter_mut()
            // Search in reverse because of shadowing; if we find a match, prefer the *last* binding!
            .rev()
        {
            // Do this at the start of each iteration because index is initialized to len, not len - 1
            index -= 1;

            // Since this is unqualified, we ignore the module_id.
            if *haystack_str_id == str_id {
                // Since a lookup found it, this binding has now been used.
                *is_used = IsUsed::Used;

                return Some((LowercaseId(index as u16), *region));
            }
        }

        // We didn't find it in our decl, so fall back on searching the top-level scope.
        self.tl_scope.lookup_lc(ModuleId::HOME, str_id)
    }

    /// The caller should already have looked up the ModuleId based on the qualified
    /// module name string, and should have reported any "module not imported here"
    /// errors as necessary.
    pub fn lookup_lc_qualified(
        &self,
        module_id: ModuleId,
        str_id: LcStrId,
    ) -> Option<(LowercaseId, Region)> {
        self.tl_scope.lookup_lc(module_id, str_id)
    }

    pub fn lookup_uc_unqualified(&self, str_id: UcStrId) -> Option<(UppercaseId, Region)> {
        // Start searching at uc_start; anything before that is the previous decl's bindings!
        let uc_start = self.uc_start as usize;
        let bindings_in_decl = &mut self.uc_bindings[uc_start..];
        let mut index = self.tl_scope.num_lc_bindings() + uc_start;

        // Unlike lookup_lc_unqualified, we don't need to bother searching in reverse here,
        // because uppercase bindings can't be shadowed. The hardware is usually faster
        // at searching forward (maybe because of prefetching heuristics), so prefer that.
        for (_haystack_module_id, haystack_str_id, region, is_used) in bindings_in_decl.iter_mut() {
            // Since this is unqualified, we ignore the module_id.
            if *haystack_str_id == str_id {
                // Since a lookup found it, this binding has now been used.
                *is_used = IsUsed::Used;

                return Some((UppercaseId(index as u16), *region));
            }

            index += 1;
        }

        // We didn't find it in our decl, so fall back on searching the top-level scope.
        self.tl_scope.lookup_uc(ModuleId::HOME, str_id)
    }

    /// The caller should already have looked up the ModuleId based on the qualified
    /// module name string, and should have reported any "module not imported here"
    /// errors as necessary.
    pub fn lookup_uc_qualified(
        &self,
        module_id: ModuleId,
        str_id: UcStrId,
    ) -> Option<(UppercaseId, Region)> {
        self.tl_scope.lookup_uc(module_id, str_id)
    }

    pub fn lookup_module(&self, str_id: ModuleStrId) -> Option<(ModuleId, Region)> {
        // Start searching at imports_start; anything before that is the previous decl's bindings!
        let imports_start = self.imports_start as usize;
        let imports_in_decl = &mut self.imports[imports_start..];
        let mut index = self.tl_scope.num_imports() + imports_start;

        // Unlike lookup_lc_unqualified, we don't need to bother searching in reverse here,
        // because imported module names can't be shadowed. The hardware is usually faster
        // at searching forward (maybe because of prefetching heuristics), so prefer that.
        for (_shorthand, haystack_str_id, region, is_used) in imports_in_decl.iter_mut() {
            if *haystack_str_id == str_id {
                // Since a lookup found it, this import has now been used.
                *is_used = IsUsed::Used;

                return Some((
                    ModuleId(
                        (index + self.tl_scope.num_imports() + self.imports_start as usize) as u16,
                    ),
                    *region,
                ));
            }

            index += 1;
        }

        // We didn't find it in our decl, so fall back on searching the top-level scope.
        self.tl_scope.lookup_module(str_id)
    }

    pub fn region_from_lc_id(&self, lc_id: LowercaseId) -> Region {
        match self
            .lc_bindings
            .get(lc_id.to_index() + self.tl_scope.num_lc_bindings())
        {
            Some((_module_id, _str_id, region, _is_used)) => *region,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn str_id_from_lc_id(&self, lc_id: LowercaseId) -> LcStrId {
        match self
            .lc_bindings
            .get(lc_id.to_index() + self.tl_scope.num_lc_bindings())
        {
            Some((_module_id, str_id, _region, _is_used)) => *str_id,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn module_id_from_lc_id(&self, lc_id: LowercaseId) -> ModuleId {
        match self
            .lc_bindings
            .get(lc_id.to_index() + self.tl_scope.num_lc_bindings())
        {
            Some((module_id, _str_id, _region, _is_used)) => *module_id,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn region_from_uc_id(&self, uc_id: UppercaseId) -> Region {
        match self
            .uc_bindings
            .get(uc_id.to_index() + self.tl_scope.num_uc_bindings())
        {
            Some((_module_id, _str_id, region, _is_used)) => *region,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn str_id_from_uc_id(&self, uc_id: UppercaseId) -> UcStrId {
        match self
            .uc_bindings
            .get(uc_id.to_index() + self.tl_scope.num_uc_bindings())
        {
            Some((_module_id, str_id, _region, _is_used)) => *str_id,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn module_id_from_uc_id(&self, uc_id: UppercaseId) -> ModuleId {
        match self
            .uc_bindings
            .get(uc_id.to_index() + self.tl_scope.num_uc_bindings())
        {
            Some((module_id, _str_id, _region, _is_used)) => *module_id,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn shorthand_from_module_id(&self, module_id: ModuleId) -> Option<ShorthandStrId> {
        match self
            .imports
            .get(module_id.to_index() + self.tl_scope.num_imports())
        {
            Some((shorthand, _str_id, _region, _is_used)) => *shorthand,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn region_from_module_id(&self, module_id: ModuleId) -> Region {
        match self
            .imports
            .get(module_id.to_index() + self.tl_scope.num_imports())
        {
            Some((_shorthand, _str_id, region, _is_used)) => *region,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    pub fn str_id_from_module_id(&self, module_id: ModuleId) -> ModuleStrId {
        match self
            .imports
            .get(module_id.to_index() + self.tl_scope.num_imports())
        {
            Some((_shorthand, str_id, _region, _is_used)) => *str_id,
            None => unreachable!(), // TODO this should never happen!
        }
    }

    /// Make sure we didn't overflow anything - e.g. that we didn't hand
    /// out more LowercaseIds than we support (which would have caused
    /// errors due to u16 casting resulting in incorrect numbers).
    ///
    /// It's essentially inconceivable that this would come up in practice,
    /// but if it does (or if someone is fuzzing the compiler), we should
    /// still handle it with a graceful error!
    pub fn validate(
        &self,
        arena: &'a Bump,
    ) -> Vec<'a, Problem<LcStrId, UcStrId, ShorthandStrId, ModuleStrId, Region>> {
        let mut problems = Vec::new_in(arena);

        if self.next_lc_id > u16::MAX as u32 {
            problems.push(Problem::TooManyBindings);
        }

        // Note: module imports and uppercase bindings each go on their own lines,
        // syntactically, so they actually cannot overflow. Only lowercase bindings
        // can possibly overflow, which could theoretically happen due to top-level destructuring.
        problems
    }

    pub fn unused_lc(&'a self) -> impl Iterator<Item = LowercaseId> + 'a {
        self.lc_bindings.iter().enumerate().filter_map(
            |(index, (_, _, _, is_used))| match is_used {
                IsUsed::Used => None,
                IsUsed::Unused => Some(LowercaseId(index as u16)),
            },
        )
    }

    pub fn unused_uc(&'a self) -> impl Iterator<Item = UppercaseId> + 'a {
        self.uc_bindings.iter().enumerate().filter_map(
            |(index, (_, _, _, is_used))| match is_used {
                IsUsed::Used => None,
                IsUsed::Unused => Some(UppercaseId(index as u16)),
            },
        )
    }

    pub fn unused_imports(&'a self) -> impl Iterator<Item = ModuleId> + 'a {
        self.imports
            .iter()
            .enumerate()
            .filter_map(|(index, (_, _, _, is_used))| match is_used {
                IsUsed::Used => None,
                IsUsed::Unused => Some(ModuleId(index as u16)),
            })
    }
}
