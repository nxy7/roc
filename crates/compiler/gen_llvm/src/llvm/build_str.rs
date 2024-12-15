use crate::llvm::build::Env;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue, StructValue};
use roc_builtins::bitcode;
use roc_mono::layout::{Builtin, InLayout, Layout, LayoutRepr, STLayoutInterner};

use super::bitcode::{call_str_bitcode_fn, BitcodeReturns};
use super::build::load_roc_value;
use super::build_list::pass_as_opaque;
use super::convert::zig_str_type;
use super::struct_::struct_from_fields;

pub static CHAR_LAYOUT: InLayout = Layout::U8;

pub(crate) fn decode_from_utf8_result<'a, 'ctx>(
    env: &Env<'a, 'ctx, '_>,
    layout_interner: &STLayoutInterner<'a>,
    pointer: PointerValue<'ctx>,
) -> BasicValueEnum<'ctx> {
    let layout =
        LayoutRepr::Struct(
            env.arena
                .alloc([Layout::U64, Layout::STR, Layout::BOOL, Layout::U8]),
        );

    load_roc_value(
        env,
        layout_interner,
        layout,
        pointer,
        "load_decode_from_utf8_result",
    )
}

/// Dec.toStr : Dec -> Str

/// Str.equal : Str, Str -> Bool
pub(crate) fn str_equal<'ctx>(
    env: &Env<'_, 'ctx, '_>,
    value1: BasicValueEnum<'ctx>,
    value2: BasicValueEnum<'ctx>,
) -> BasicValueEnum<'ctx> {
    call_str_bitcode_fn(
        env,
        &[value1, value2],
        &[],
        BitcodeReturns::Basic,
        bitcode::STR_EQUAL,
    )
}

// Gets a pointer to just after the refcount for a list or seamless slice.
// The value is just after the refcount so that normal lists and seamless slices can share code paths easily.
pub(crate) fn str_allocation_ptr<'ctx>(
    env: &Env<'_, 'ctx, '_>,
    value: BasicValueEnum<'ctx>,
) -> PointerValue<'ctx> {
    call_str_bitcode_fn(
        env,
        &[value],
        &[],
        BitcodeReturns::Basic,
        bitcode::STR_ALLOCATION_PTR,
    )
    .into_pointer_value()
}

pub(crate) fn store_str<'ctx>(
    env: &Env<'_, 'ctx, '_>,
    pointer_to_first_element: PointerValue<'ctx>,
    len: IntValue<'ctx>,
    cap: IntValue<'ctx>,
) -> StructValue<'ctx> {
    let ptr = pass_as_opaque(env, pointer_to_first_element);

    struct_from_fields(
        env,
        zig_str_type(env),
        [
            (Builtin::WRAPPER_PTR as usize, ptr),
            (Builtin::WRAPPER_LEN as usize, len.into()),
            (Builtin::WRAPPER_CAPACITY as usize, cap.into()),
        ]
        .into_iter(),
    )
}
