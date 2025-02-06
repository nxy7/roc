const std = @import("std");
const base = @import("../../base.zig");
const cols = @import("../../collections.zig");
const Problem = @import("../../Problem.zig");
const types = @import("../../types.zig");

pub const IR = @This();

env: *base.ModuleEnv,
types: Type.List,
exprs: Expr.List,
expr_regions: cols.SafeList(base.Region),
typed_exprs: Expr.Typed.List,
patterns: Pattern.List,
typed_patterns: Pattern.Typed.List,
typed_idents: TypedIdent.List,
when_branches: WhenBranch.List,

pub fn init(env: *base.ModuleEnv, allocator: std.mem.Allocator) IR {
    return IR{
        .env = env,
        .types = Type.List.init(allocator),
        .exprs = Expr.List.init(allocator),
        .expr_regions = cols.SafeList(base.Region).init(allocator),
        .typed_exprs = Expr.Typed.List.init(allocator),
        .patterns = Pattern.List.init(allocator),
        .typed_patterns = Pattern.Typed.List.init(allocator),
        .typed_idents = TypedIdent.List.init(allocator),
        .when_branches = WhenBranch.List.init(allocator),
    };
}

pub fn deinit(self: *IR) void {
    self.types.deinit();
    self.exprs.deinit();
    self.expr_regions.deinit();
    self.typed_exprs.deinit();
    self.patterns.deinit();
    self.typed_patterns.deinit();
    self.typed_idents.deinit();
    self.when_branches.deinit();
}

pub const Type = union(enum) {
    Primitive: types.Primitive,
    Box: Type.Idx,
    List: Type.Idx,
    Struct: Type.NonEmptySlice,
    TagUnion: Type.NonEmptySlice,
    Func: struct {
        ret_then_args: Type.NonEmptySlice,
    },

    pub const List = cols.SafeList(@This());
    pub const Idx = List.Idx;
    pub const Slice = List.Slice;
    pub const NonEmptySlice = List.NonEmptySlice;
};

pub const Expr = union(enum) {
    Let: Def,
    Str: cols.StringLiteral,
    Number: base.Literal.Num,
    List: struct {
        elem_type: Type.Idx,
        elems: Expr.Slice,
    },
    Lookup: struct {
        ident: base.Module.Ident,
        type: Type.Idx,
    },

    Call: struct {
        fn_type: Type.Idx,
        fn_expr: Expr.Idx,
        args: Expr.Typed.Slice,
    },

    Lambda: struct {
        fn_type: Type.Idx,
        arguments: Pattern.Typed.Slice,
        body: Expr.Idx,
        recursive: base.Recursive,
    },

    Unit,

    /// A record literal or a tuple literal.
    /// These have already been sorted alphabetically.
    Struct: Expr.NonEmptySlice,

    /// Look up exactly one field on a record, tuple, or tag payload.
    /// At this point we've already unified those concepts and have
    /// converted (for example) record field names to indices, and have
    /// also dropped all fields that have no runtime representation (e.g. empty records).
    ///
    /// In a later compilation phase, these indices will be re-sorted
    /// by alignment and converted to byte offsets, but we in this
    /// phase we aren't concerned with alignment or sizes, just indices.
    StructAccess: struct {
        record_expr: Expr.Idx,
        record_type: Type.Idx,
        field_type: Type.Idx,
        field_id: cols.FieldName.Idx,
    },

    /// Same as SmallTag but with u16 discriminant instead of u8
    Tag: struct {
        discriminant: u16,
        tag_union_type: Type.Idx,
        args: Expr.Typed.Slice,
    },

    When: struct {
        /// The value being matched on
        value: Expr.Idx,
        /// The type of the value being matched on
        value_type: Type.Idx,
        /// The return type of all branches and thus the whole when expression
        branch_type: Type.Idx,
        /// The branches of the when expression
        branches: WhenBranch.NonEmptySlice,
    },

    CompilerBug: Problem.SpecializeTypes,

    pub const List = cols.SafeList(@This());
    pub const Idx = List.Idx;
    pub const Slice = List.Slice;
    pub const NonEmptySlice = List.NonEmptySlice;

    pub const Typed = struct {
        expr: Expr.Idx,
        type: Type.Idx,

        pub const List = cols.SafeMultiList(@This());
        pub const Slice = Typed.List.Slice;
    };
};

/// A definition, e.g. `x = foo`
pub const Def = struct {
    pattern: Pattern.Idx,
    /// Named variables in the pattern, e.g. `a` in `Ok a ->`
    pattern_vars: TypedIdent.Slice,
    expr: Expr.Idx,
    expr_type: Type.Idx,

    pub const List = cols.SafeMultiList(@This());
    pub const Slice = List.Slice;
};

pub const WhenBranch = struct {
    /// The pattern(s) to match the value against
    patterns: Pattern.NonEmptySlice,
    /// A boolean expression that must be true for this branch to be taken
    guard: ?Expr.Idx,
    /// The expression to produce if the pattern matches
    value: Expr.Idx,

    pub const List = cols.SafeMultiList(@This());
    pub const Slice = List.Slice;
};

pub const StructDestruct = struct {
    ident: base.Ident.Idx,
    field: cols.FieldName.Idx,
    kind: Kind,

    pub const Kind = union(enum) {
        Required,
        Guard: Pattern.Typed,
    };

    pub const List = cols.SafeMultiList(@This());
    pub const Slice = List.Slice;
};

pub const Pattern = union(enum) {
    Identifier: base.IdentId,
    As: struct {
        pattern: Pattern.Idx,
        ident: base.Ident.Idx,
    },
    StrLiteral: cols.StringLiteral.Idx,
    NumberLiteral: base.Literal.Num,
    AppliedTag: struct {
        tag_union_type: Type.Idx,
        tag_name: base.Ident.Idx,
        args: cols.SafeList(Pattern.Idx).Slice,
    },
    StructDestructure: struct {
        struct_type: Type.Idx,
        destructs: StructDestruct.Slice,
        opt_spread: ?Pattern.Typed,
    },
    List: struct {
        elem_type: Type.Idx,
        patterns: Pattern.Slice,

        /// Where a rest pattern splits patterns before and after it, if it does at all.
        /// If present, patterns at index >= the rest index appear after the rest pattern.
        /// For example:
        ///   [ .., A, B ] -> patterns = [A, B], rest = 0
        ///   [ A, .., B ] -> patterns = [A, B], rest = 1
        ///   [ A, B, .. ] -> patterns = [A, B], rest = 2
        /// Optionally, the rest pattern can be named - e.g. `[ A, B, ..others ]`
        opt_rest: ?.{ u16, ?base.Ident.Idx },
    },
    Underscore,
    CompilerBug: Problem.SpecializeTypes,

    pub const List = cols.SafeList(@This());
    pub const Idx = List.Idx;
    pub const Slice = List.Slice;
    pub const NonEmptySlice = List.NonEmptySlice;

    pub const Typed = struct {
        pattern: Pattern.Idx,
        type: Type.Idx,

        pub const List = cols.SafeMultiList(@This());
        pub const Slice = Typed.List.Slice;
    };
};

pub const TypedIdent = struct {
    pattern: Pattern.Idx,
    type: Type.Idx,

    pub const List = cols.SafeMultiList(@This());
    pub const Slice = List.Slice;
};
