#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{
    BodyLine, BodyLineKind, DictEntry, Expr, ExprKind, Function, FunctionKind, IfBranch, MatchArm,
    Pattern, PatternKind, PublicAliasKind, RecordField, SurfaceModule, UseDecl, Visibility,
};

use crate::adt::{self, AdtRegistry};
use crate::name_recovery::{normal_use_decls, public_alias_has_invalid_target_leaf};
use crate::semantic_model::{Binding, FunctionKey, Type};
use crate::type_syntax::parse_type_or_unknown;
use crate::types::signatures::{FunctionSignature, MatchScrutineePatternInference};
use crate::types::symbols::imported_use_for_path;

mod aliases_and_bindings;
mod call_site_resolution;
mod call_site_traversal;
mod callback_constraints;
mod callback_discovery;
mod expression_inference;
mod orchestration;
mod reference_discovery;

pub(crate) use aliases_and_bindings::*;
pub(crate) use call_site_resolution::*;
pub(crate) use call_site_traversal::*;
pub(crate) use callback_constraints::*;
pub(crate) use callback_discovery::*;
pub(crate) use expression_inference::*;
pub(crate) use orchestration::*;
pub(crate) use reference_discovery::*;

fn valid_value_binding_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

type FunctionAstMap<'a> = BTreeMap<FunctionKey, &'a Function>;
type FunctionSignatureMap = BTreeMap<FunctionKey, FunctionSignature>;
type FunctionReturnMap = BTreeMap<FunctionKey, Type>;
type PrivateSlotOmissions = (Vec<bool>, bool);
type PrivateSlotMap = BTreeMap<FunctionKey, PrivateSlotOmissions>;
type PrivateReferenceMap = BTreeMap<FunctionKey, BTreeSet<FunctionKey>>;

fn function_ast_map(module: &SurfaceModule) -> FunctionAstMap<'_> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect()
}

pub(super) fn function_signature_params(
    function: &veln_ast::Function,
) -> (Vec<Type>, Option<Type>) {
    let mut params = Vec::new();
    let mut variadic = None;
    for param in &function.params {
        let ty = parse_type_or_unknown(param.ty.as_deref());
        if param.is_variadic {
            variadic = Some(ty);
        } else {
            params.push(ty);
        }
    }
    (params, variadic)
}
