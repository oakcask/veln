use veln_ast::{BinaryOp, PrefixOp};

use crate::semantic_model::Type;
use crate::source_less_lookup::{
    compiler_adapter_symbol, prelude_builtin_module, prelude_symbol, standard_module,
};

mod byte_signatures;
mod core_signatures;
mod expected_types;
mod source_signatures;
mod surface_signatures;

pub(crate) use core_signatures::{
    core_prelude_signature, qualified_core_prelude_builtin_signature,
    qualified_core_prelude_signature,
};
use expected_types::ExpectedPreludeParts;
use surface_signatures::surface_prelude_signature;

pub(crate) fn prelude_signature(name: &str, expected: Option<&Type>) -> Option<(Vec<Type>, Type)> {
    prelude_signature_with_input(name, expected, None)
}

pub(crate) fn prelude_signature_with_input(
    name: &str,
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(Vec<Type>, Type)> {
    let descriptor = prelude_symbol(name)?;
    let expected = ExpectedPreludeParts::from_expected_and_input(expected, input);
    surface_prelude_signature(descriptor, &expected)
}

pub(crate) fn qualified_prelude_builtin_signature_with_input(
    segments: &[String],
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != prelude_builtin_module() {
        return None;
    }
    let descriptor = compiler_adapter_symbol(name)?;
    let expected = ExpectedPreludeParts::from_expected_and_input(expected, input);
    let (params, return_type) = surface_prelude_signature(descriptor, &expected)?;
    Some((name.clone(), params, return_type))
}

pub(crate) fn qualified_prelude_signature(
    segments: &[String],
    expected: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    qualified_prelude_signature_with_input(segments, expected, None)
}

pub(crate) fn qualified_prelude_signature_with_input(
    segments: &[String],
    expected: Option<&Type>,
    input: Option<&Type>,
) -> Option<(String, Vec<Type>, Type)> {
    let [module, name] = segments else {
        return None;
    };
    if module != standard_module() {
        return None;
    }
    let (params, return_type) = prelude_signature_with_input(name, expected, input)?;
    Some((name.clone(), params, return_type))
}

pub(crate) fn float_prefix_prelude_name(op: PrefixOp) -> Option<&'static str> {
    match op {
        PrefixOp::Negate => Some("float_negate"),
        _ => None,
    }
}

pub(crate) fn float_arithmetic_prelude_name(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Add => Some("float_add"),
        BinaryOp::Subtract => Some("float_subtract"),
        BinaryOp::Multiply => Some("float_multiply"),
        BinaryOp::Divide => Some("float_divide"),
        _ => None,
    }
}

pub(crate) fn float_comparison_prelude_name(op: BinaryOp) -> Option<&'static str> {
    match op {
        BinaryOp::Less => Some("float_less"),
        BinaryOp::LessEqual => Some("float_less_equal"),
        BinaryOp::Greater => Some("float_greater"),
        BinaryOp::GreaterEqual => Some("float_greater_equal"),
        _ => None,
    }
}

#[cfg(test)]
#[path = "prelude/tests.rs"]
mod tests;
