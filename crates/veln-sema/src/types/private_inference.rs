#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{
    BodyLine, BodyLineKind, DictEntry, Expr, ExprKind, Function, FunctionKind, IfBranch, MatchArm,
    Pattern, PatternKind, PublicAliasKind, RecordField, SurfaceModule, UseDecl, Visibility,
};

use crate::adt::{self, AdtRegistry};
use crate::name_recovery::public_alias_has_invalid_target_leaf;
use crate::semantic_model::{Binding, FunctionKey, Type};
use crate::type_syntax::parse_type_or_unknown;
use crate::types::signatures::{FunctionSignature, MatchScrutineePatternInference};
use crate::types::symbols::imported_use_for_path;

fn valid_value_binding_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

type FunctionAstMap<'a> = BTreeMap<FunctionKey, &'a Function>;
type FunctionSignatureMap = BTreeMap<FunctionKey, FunctionSignature>;
type FunctionReturnMap = BTreeMap<FunctionKey, Type>;
type PrivateSlotOmissions = (Vec<bool>, bool);
type PrivateSlotMap = BTreeMap<FunctionKey, PrivateSlotOmissions>;
type PrivateReferenceMap = BTreeMap<FunctionKey, BTreeSet<FunctionKey>>;

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

pub(crate) fn infer_private_function_body_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let mut changed = true;
    while changed {
        changed = false;
        let signatures_by_path = signatures_by_path(functions);
        let omitted_private_returns = omitted_private_returns_that_can_change(module, functions);
        if omitted_private_returns.is_empty() {
            return;
        }
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && private_function_key(function)
                    .is_some_and(|key| omitted_private_returns.contains(&key))
        }) {
            let Some(name) = &function.name else {
                continue;
            };
            let key = (function.module_name.clone(), name.clone());
            let inferred = infer_private_function_tail_type(
                function,
                &module.uses,
                &signatures_by_path,
                &returns_by_path,
                adts,
            );
            if inferred == Type::Unknown {
                continue;
            }
            let Some(signature) = functions
                .iter_mut()
                .find(|signature| signature.module_name == key.0 && signature.name == key.1)
            else {
                continue;
            };
            if signature.return_type == inferred {
                continue;
            }
            if !type_has_unknown(&signature.return_type) {
                continue;
            }
            signature.return_type = inferred;
            changed = true;
        }
    }
}

pub(crate) fn infer_private_function_call_site_signature_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let initial_omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
    if initial_omitted_private_slots.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_slot_omissions(&initial_omitted_private_slots),
        &initial_omitted_private_slots
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
    );
    let contributors = private_call_site_constraint_contributors(
        module,
        &initial_omitted_private_slots,
        &private_references,
    );
    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
        if omitted_private_slots.is_empty() {
            return;
        }
        let signatures_by_path = signatures_by_path_with_aliases(module, functions);
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_call_site_constraints(
                function,
                &mut PrivateCallSiteConstraintContext {
                    uses: &module.uses,
                    function_by_path: &function_by_path,
                    omitted_private_slots: &omitted_private_slots,
                    signatures_by_path: &signatures_by_path,
                    returns_by_path: &returns_by_path,
                    functions,
                    adts,
                    changed: &mut changed,
                },
            );
        }
    }
}

fn function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

fn private_function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

fn signature_for_key<'a>(
    functions: &'a [FunctionSignature],
    key: &FunctionKey,
) -> Option<&'a FunctionSignature> {
    functions
        .iter()
        .find(|signature| signature.module_name == key.0 && signature.name == key.1)
}

fn omitted_private_returns_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let can_change = signature_for_key(functions, &key)
                .is_some_and(|signature| type_has_unknown(&signature.return_type));
            can_change.then_some(key)
        })
        .collect()
}

fn omitted_private_slots_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> PrivateSlotMap {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.name.is_some()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            let omitted_params = function
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    if !parameter_annotation_is_omitted(param) {
                        return false;
                    }
                    if param.is_variadic {
                        signature.variadic.as_ref().is_some_and(type_has_unknown)
                    } else {
                        signature.params.get(index).is_some_and(type_has_unknown)
                    }
                })
                .collect::<Vec<_>>();
            let omitted_return =
                function.return_type.is_none() && type_has_unknown(&signature.return_type);
            (omitted_params.iter().any(|omitted| *omitted) || omitted_return)
                .then_some((key, (omitted_params, omitted_return)))
        })
        .collect()
}

fn modules_with_private_slot_omissions(
    omitted_private_slots: &PrivateSlotMap,
) -> BTreeSet<Option<String>> {
    omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect()
}

fn modules_with_private_return_omissions(
    omitted_private_returns: &BTreeSet<FunctionKey>,
) -> BTreeSet<Option<String>> {
    omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect()
}

fn omitted_private_returns_requiring_prelude_pass(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            (type_has_unknown(&signature.return_type)
                || private_tail_can_use_expected(function, &signature.return_type, uses, adts))
            .then_some(key)
        })
        .collect()
}

fn private_reference_map(
    module: &SurfaceModule,
    function_by_path: &FunctionAstMap<'_>,
    modules_with_omitted_slots: &BTreeSet<Option<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> PrivateReferenceMap {
    let candidates_by_module = private_reference_candidates_by_module(omitted_private_keys);
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter(|function| {
            private_function_needs_reference_index(
                function,
                function_by_path,
                &candidates_by_module,
                omitted_private_keys,
            )
        })
        .filter_map(|function| {
            let key = function_key(function)?;
            let mut references = BTreeSet::new();
            #[cfg(test)]
            private_inference_counters::record_private_reference_index_scan();
            collect_private_function_references(function, function_by_path, &mut references);
            Some((key, references))
        })
        .collect()
}

fn private_reference_candidates_by_module(
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> BTreeMap<Option<String>, BTreeSet<String>> {
    let mut candidates: BTreeMap<Option<String>, BTreeSet<String>> = BTreeMap::new();
    for (module_name, name) in omitted_private_keys {
        candidates
            .entry(module_name.clone())
            .or_default()
            .insert(name.clone());
    }
    candidates
}

fn private_function_needs_reference_index(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates_by_module: &BTreeMap<Option<String>, BTreeSet<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> bool {
    let Some(key) = function_key(function) else {
        return false;
    };
    if omitted_private_keys.contains(&key) {
        return true;
    }
    let Some(candidates) = candidates_by_module.get(&function.module_name) else {
        return false;
    };
    #[cfg(test)]
    private_inference_counters::record_private_reference_candidate_scan();
    private_function_mentions_candidate(function, function_by_path, candidates)
}

fn private_function_mentions_candidate(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
) -> bool {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        if private_line_mentions_candidate(
            line,
            current_module,
            function_by_path,
            candidates,
            &mut bindings,
        ) {
            return true;
        }
    }
    false
}

fn private_line_mentions_candidate(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &mut Vec<Binding>,
) -> bool {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            let mentions = private_expr_mentions_candidate(
                expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
            mentions
        }
        BodyLineKind::Expr { expr } => private_expr_mentions_candidate(
            expr,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
    }
}

fn private_expr_mentions_candidate(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &[Binding],
) -> bool {
    if private_expr_reference_target(expr, current_module, function_by_path, bindings)
        .is_some_and(|key| key.0.as_deref() == current_module && candidates.contains(&key.1))
    {
        return true;
    }
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            private_expr_mentions_candidate(
                item,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            private_expr_mentions_candidate(
                &entry.key,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                &entry.value,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            private_expr_mentions_candidate(
                &field.expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Call { callee, args } => {
            private_expr_mentions_candidate(
                callee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::Perform { args, .. } => args.iter().any(|arg| {
            private_expr_mentions_candidate(
                arg,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Handle { body, args, .. } => {
            private_expr_mentions_candidate(
                body,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_expr_mentions_candidate(
                input,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                base,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => private_expr_mentions_candidate(
            value,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
        ExprKind::Match { scrutinee, arms } => {
            private_expr_mentions_candidate(
                scrutinee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || arms.iter().any(|arm| {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                private_expr_mentions_candidate(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    &arm_bindings,
                )
            })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_expr_mentions_candidate(
                condition,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                then_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || else_if_branches.iter().any(|branch| {
                private_expr_mentions_candidate(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                ) || private_expr_mentions_candidate(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            }) || private_expr_mentions_candidate(
                else_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::Binary { left, right, .. } => {
            private_expr_mentions_candidate(
                left,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                right,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => false,
    }
}

fn private_reference_initial_bindings(function: &Function) -> Vec<Binding> {
    function
        .params
        .iter()
        .filter(|param| valid_value_binding_name(&param.name))
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect()
}

fn collect_private_reference_pattern_bindings(pattern: &Pattern, bindings: &mut Vec<Binding>) {
    match &pattern.kind {
        PatternKind::Binding(name) => {
            if valid_value_binding_name(name) {
                bindings.push(Binding::new(name.clone(), Type::Unknown));
            }
        }
        PatternKind::Record(fields) => {
            for field in fields {
                collect_private_reference_pattern_bindings(&field.pattern, bindings);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_private_reference_pattern_bindings(arg, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn collect_private_function_references(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
) {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        collect_private_line_references(
            line,
            current_module,
            function_by_path,
            references,
            &mut bindings,
        );
    }
}

fn collect_private_line_references(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            collect_private_expr_references(
                expr,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
        }
        BodyLineKind::Expr { expr } => collect_private_expr_references(
            expr,
            current_module,
            function_by_path,
            references,
            bindings,
        ),
    }
}

fn collect_private_expr_references(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &[Binding],
) {
    if let Some(key) =
        private_expr_reference_target(expr, current_module, function_by_path, bindings)
    {
        references.insert(key);
    }
    match &expr.kind {
        ExprKind::List(items) => {
            for item in items {
                collect_private_expr_references(
                    item,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_private_expr_references(
                    &entry.key,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &entry.value,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_private_expr_references(
                    &field.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_expr_references(
                callee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_expr_references(
                body,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_expr_references(
                input,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                base,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            collect_private_expr_references(
                value,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_expr_references(
                scrutinee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arm in arms {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                collect_private_expr_references(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    references,
                    &arm_bindings,
                );
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_expr_references(
                condition,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                then_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for branch in else_if_branches {
                collect_private_expr_references(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
            collect_private_expr_references(
                else_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_expr_references(
                left,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                right,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn private_expr_reference_target(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &expr.kind else {
        return None;
    };
    private_reference_name_path_target(segments, current_module, function_by_path, bindings)
}

fn private_reference_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == *name) {
        return binding.private_function_value.clone();
    }
    private_name_path_target(segments, current_module, function_by_path)
}

fn signatures_by_path(functions: &[FunctionSignature]) -> FunctionSignatureMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.clone(),
            )
        })
        .collect()
}

fn private_call_site_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_slots: &PrivateSlotMap,
    private_references: &PrivateReferenceMap,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_slots = omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            #[cfg(test)]
            private_inference_counters::record_call_site_discovery_scan();
            (omitted_private_slots.contains_key(&key)
                || private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_slots.contains_key(reference))
                }))
            .then_some(key)
        })
        .collect()
}

fn signatures_by_path_with_aliases(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> FunctionSignatureMap {
    let mut signatures = signatures_by_path(functions);
    for alias in function_alias_signatures(module, functions) {
        let key = (alias.module_name.clone(), alias.name.clone());
        signatures.entry(key).or_insert(alias);
    }
    signatures
}

fn returns_by_path(functions: &[FunctionSignature]) -> FunctionReturnMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect()
}

struct PrivateCallSiteConstraintContext<'a> {
    uses: &'a [UseDecl],
    function_by_path: &'a FunctionAstMap<'a>,
    omitted_private_slots: &'a PrivateSlotMap,
    signatures_by_path: &'a FunctionSignatureMap,
    returns_by_path: &'a FunctionReturnMap,
    functions: &'a mut [FunctionSignature],
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

struct PrivateCallSiteExprContext<'a, 'b> {
    current_module: Option<&'b str>,
    caller_key: Option<&'b FunctionKey>,
    bindings: &'b [Binding],
    constraints: &'b mut PrivateCallSiteConstraintContext<'a>,
}

fn collect_private_call_site_constraints(
    function: &Function,
    context: &mut PrivateCallSiteConstraintContext<'_>,
) {
    #[cfg(test)]
    private_inference_counters::record_call_site_scan();

    let current_module = function.module_name.as_deref();
    let caller_key = function
        .name
        .as_ref()
        .map(|name| (function.module_name.clone(), name.clone()));
    let mut bindings = private_function_body_bindings(function, context.signatures_by_path);
    let declared_return = function.return_type.as_deref().map_or_else(
        || {
            caller_key
                .as_ref()
                .and_then(|key| context.signatures_by_path.get(key))
                .map(|signature| signature.return_type.clone())
                .filter(|ty| !type_has_unknown(ty))
        },
        |return_type| Some(parse_type_or_unknown(Some(return_type))),
    );

    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_call_site_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            current_module,
                            context.function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        current_module,
                        context.uses,
                        &bindings,
                        context.returns_by_path,
                        context.adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_call_site_expr_constraints(
                    expr,
                    expected,
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
            }
        }
    }
}

fn collect_private_call_site_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_call_site_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_call_site_expr_constraints(&entry.key, key_expected, context);
                collect_private_call_site_expr_constraints(&entry.value, value_expected, context);
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_call_site_expr_constraints(&field.expr, field_expected, context);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_call_site_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_call_site_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_call_site_expr_constraints(body, expected, context);
            for arg in args {
                collect_private_call_site_expr_constraints(arg, None, context);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_call_site_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_call_site_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_call_site_expr_constraints(value, None, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_call_site_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
                arms,
                context.current_module,
                context.constraints.uses,
                context.constraints.adts,
            ) {
                MatchScrutineePatternInference::Inferred(ty) => Some(ty),
                MatchScrutineePatternInference::Uninferred
                | MatchScrutineePatternInference::Ambiguous(_) => None,
            };
            collect_private_call_site_expr_constraints(
                scrutinee,
                scrutinee_expected.as_ref(),
                context,
            );
            for arm in arms {
                collect_private_call_site_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_call_site_expr_constraints(condition, Some(&Type::bool()), context);
            collect_private_call_site_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_call_site_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_call_site_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_call_site_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_call_site_expr_constraints(left, expected, context);
            collect_private_call_site_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            collect_private_parameter_constraints(segments, expected, context);
            collect_private_function_value_constraints(segments, expected, context);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_parameter_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(expected) = expected.filter(|ty| !type_has_unknown(ty)) else {
        return;
    };
    let [name] = segments else {
        return;
    };
    let Some(caller_key) = context.caller_key else {
        return;
    };
    let Some((omitted_params, _)) = context.constraints.omitted_private_slots.get(caller_key)
    else {
        return;
    };
    let Some(function) = context.constraints.function_by_path.get(caller_key) else {
        return;
    };
    let Some(index) = function
        .params
        .iter()
        .position(|param| param.name == *name && parameter_annotation_is_omitted(param))
    else {
        return;
    };
    if !omitted_params.get(index).copied().unwrap_or(false) {
        return;
    }
    if function.params[index].is_variadic {
        let Some(item_type) = expected.vec_part().filter(|ty| !type_has_unknown(ty)) else {
            return;
        };
        update_private_signature_variadic(
            context.constraints.functions,
            caller_key,
            item_type.clone(),
            context.constraints.changed,
        );
    } else {
        update_private_signature_param(
            context.constraints.functions,
            caller_key,
            index,
            expected.clone(),
            context.constraints.changed,
        );
    }
}

fn collect_private_call_site_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(target_key) = private_same_module_call_target(
        callee,
        context.current_module,
        context.constraints.function_by_path,
    ) else {
        collect_private_call_site_non_target_call_args(callee, args, expected, context);
        return;
    };

    let is_recursive_edge = context.caller_key == Some(&target_key);
    if !is_recursive_edge
        && let Some((omitted_params, omitted_return)) =
            context.constraints.omitted_private_slots.get(&target_key)
    {
        if let Some(target_params) = context
            .constraints
            .signatures_by_path
            .get(&target_key)
            .map(|signature| signature.params.clone())
        {
            for (index, arg) in args.iter().enumerate() {
                if omitted_params.get(index).copied().unwrap_or(false) {
                    let actual = infer_private_signature_expr_type(
                        arg,
                        None,
                        context.current_module,
                        context.constraints.uses,
                        context.bindings,
                        context.constraints.returns_by_path,
                        context.constraints.adts,
                    );
                    if !type_has_unknown(&actual) {
                        update_private_signature_param(
                            context.constraints.functions,
                            &target_key,
                            index,
                            actual,
                            context.constraints.changed,
                        );
                    }
                }
                let arg_expected = target_params
                    .get(index)
                    .filter(|ty| private_expected_can_constrain(ty));
                collect_private_call_site_expr_constraints(arg, arg_expected, context);
            }
        }

        if *omitted_return
            && let Some(expected) = expected
            && !type_has_unknown(expected)
        {
            update_private_signature_return(
                context.constraints.functions,
                &target_key,
                expected.clone(),
                context.constraints.changed,
            );
        }
    }

    if context
        .constraints
        .omitted_private_slots
        .contains_key(&target_key)
    {
        return;
    }
    let Some(target_params) = context
        .constraints
        .signatures_by_path
        .get(&target_key)
        .map(|signature| signature.params.clone())
    else {
        return;
    };
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = target_params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

fn collect_private_call_site_non_target_call_args(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        for arg in args {
            collect_private_call_site_expr_constraints(arg, None, context);
        }
        return;
    };
    let params = private_call_site_non_target_params(segments, args, expected, context);
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

fn private_expected_can_constrain(ty: &Type) -> bool {
    if !type_has_unknown(ty) {
        return true;
    }
    matches!(
        ty,
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } if !variadic.as_deref().is_some_and(type_has_unknown)
            && (params.iter().any(|param| !type_has_unknown(param))
            || !type_has_unknown(return_type)
            || variadic.as_deref().is_some_and(|ty| !type_has_unknown(ty)))
    )
}

fn private_call_site_non_target_params(
    segments: &[String],
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Vec<Type> {
    if let crate::adt::ConstructorLookup::Found(constructor) = context.constraints.adts.constructor(
        segments,
        context.current_module,
        context.constraints.uses,
    ) {
        return expected
            .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
            .map(|_| {
                constructor
                    .variant
                    .payload_fields
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        expected
                            .and_then(|expected| adt::payload_type(expected, constructor, index))
                            .filter(|ty| !type_has_unknown(ty))
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    if let Some(signature) = private_call_site_declared_signature(
        segments,
        context.current_module,
        context.constraints.uses,
        context.constraints.signatures_by_path,
    )
    .filter(|signature| {
        context.current_module == Some("std::prelude")
            || signature.module_name.as_deref() != Some("std::prelude")
    }) {
        return signature.params.clone();
    }

    private_prelude_constraint_name(
        segments,
        context.current_module,
        context.constraints.function_by_path,
    )
    .and_then(|name| {
        let input_type = private_prelude_input_arg(args, name).map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.constraints.uses,
                context.bindings,
                context.constraints.returns_by_path,
                context.constraints.adts,
            )
        });
        let mut params =
            crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
                .map(|(params, _)| params)?;
        if name == "vec_try_map_with" {
            let context_type = args.first().map(|arg| {
                infer_private_signature_expr_type(
                    arg,
                    None,
                    context.current_module,
                    context.constraints.uses,
                    context.bindings,
                    context.constraints.returns_by_path,
                    context.constraints.adts,
                )
            });
            apply_vec_try_map_with_context_param(&mut params, context_type);
        }
        Some(params)
    })
    .unwrap_or_default()
}

fn private_prelude_input_arg<'a>(args: &'a [Expr], helper_name: &str) -> Option<&'a Expr> {
    match helper_name {
        "vec_try_map_with" | "dict_map_with" | "dict_filter_with" | "dict_fold_with"
        | "dict_try_map_with" => args.get(1),
        _ => args.first(),
    }
}

fn collect_private_function_value_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let expected = expected.filter(|ty| private_expected_can_constrain(ty));
    let Some(Type::Function {
        params,
        variadic,
        return_type,
        ..
    }) = expected
    else {
        return;
    };
    let Some(target_key) = private_function_value_target(segments, context) else {
        return;
    };
    if context.caller_key == Some(&target_key) {
        return;
    }
    let Some(target_function) = context.constraints.function_by_path.get(&target_key) else {
        return;
    };
    let Some((omitted_params, omitted_return)) =
        context.constraints.omitted_private_slots.get(&target_key)
    else {
        return;
    };
    for (index, param) in params.iter().enumerate() {
        if omitted_params.get(index).copied().unwrap_or(false) && !type_has_unknown(param) {
            update_private_signature_param(
                context.constraints.functions,
                &target_key,
                index,
                param.clone(),
                context.constraints.changed,
            );
        }
    }
    if let Some(variadic) = variadic.as_deref().filter(|ty| !type_has_unknown(ty))
        && let Some(index) = target_function
            .params
            .iter()
            .position(|param| param.is_variadic && parameter_annotation_is_omitted(param))
        && omitted_params.get(index).copied().unwrap_or(false)
    {
        update_private_signature_variadic(
            context.constraints.functions,
            &target_key,
            variadic.clone(),
            context.constraints.changed,
        );
    }
    if *omitted_return && !type_has_unknown(return_type) {
        update_private_signature_return(
            context.constraints.functions,
            &target_key,
            return_type.as_ref().clone(),
            context.constraints.changed,
        );
    }
}

fn private_function_value_target(
    segments: &[String],
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
    {
        return binding.private_function_value.clone();
    }
    Some((context.current_module.map(str::to_string), name.clone()))
}

fn private_call_site_declared_signature<'a>(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    signatures_by_path: &'a FunctionSignatureMap,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => signatures_by_path.get(&(current_module.map(str::to_string), name.clone())),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    signatures_by_path.get(&(Some(use_decl.name.clone()), name.clone()))
                })
                .filter(|signature| signature.visibility == Visibility::Public)
        }
        _ => None,
    }
}

fn parameter_annotation_is_omitted(param: &veln_ast::Param) -> bool {
    param
        .ty
        .as_deref()
        .is_none_or(|annotation| param.is_variadic && annotation.is_empty())
}

fn private_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    let key = (current_module.map(str::to_string), name.clone());
    let function = function_by_path.get(&key)?;
    (function.kind == FunctionKind::Function && function.visibility == Visibility::Private)
        .then_some(key)
}

fn private_same_module_call_target(
    callee: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    private_name_path_target(segments, current_module, function_by_path)
}

fn update_private_signature_param(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    index: usize,
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    let Some(current) = signature.params.get_mut(index) else {
        return;
    };
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}

fn update_private_signature_variadic(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    let Some(current) = signature.variadic.as_mut() else {
        return;
    };
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}

fn update_private_signature_return(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    if type_has_unknown(&signature.return_type) {
        signature.return_type = inferred;
        *changed = true;
    }
}

pub(crate) fn infer_private_prelude_callback_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut returns_by_path = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let initial_omitted_private_returns =
        omitted_private_returns_requiring_prelude_pass(module, functions, &module.uses, adts);
    if initial_omitted_private_returns.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_return_omissions(&initial_omitted_private_returns),
        &initial_omitted_private_returns,
    );
    let contributors = private_prelude_callback_constraint_contributors(
        module,
        &initial_omitted_private_returns,
        &returns_by_path,
        &function_by_path,
        &private_references,
        &module.uses,
        adts,
    );
    if contributors.is_empty() {
        return;
    }

    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_returns = initial_omitted_private_returns.clone();
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_prelude_callback_return_constraints(
                function,
                &module.uses,
                &function_by_path,
                &omitted_private_returns,
                &mut returns_by_path,
                adts,
                &mut changed,
            );
        }
        for function in functions.iter_mut() {
            let key = (function.module_name.clone(), function.name.clone());
            if !omitted_private_returns.contains(&key) {
                continue;
            }
            if let Some(inferred) = returns_by_path.get(&key)
                && inferred != &function.return_type
            {
                function.return_type = inferred.clone();
            }
        }
    }
}

fn collect_private_prelude_callback_return_constraints(
    function: &Function,
    uses: &[UseDecl],
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
    omitted_private_returns: &BTreeSet<(Option<String>, String)>,
    returns_by_path: &mut BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
    changed: &mut bool,
) {
    #[cfg(test)]
    private_inference_counters::record_prelude_callback_scan();

    let mut bindings = function
        .params
        .iter()
        .filter(|param| valid_value_binding_name(&param.name))
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            function.module_name.as_deref(),
                            function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    expected,
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
            }
        }
    }
}

fn private_prelude_callback_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    private_references: &PrivateReferenceMap,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_returns = omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_returns.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            if !omitted_private_returns.contains(&key)
                && !private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_returns.contains(reference))
                })
            {
                return None;
            }
            private_prelude_callback_function_can_constrain(
                function,
                &key,
                omitted_private_returns,
                returns_by_path,
                function_by_path,
                uses,
                adts,
            )
            .then_some(key)
        })
        .collect()
}

fn private_prelude_callback_function_can_constrain(
    function: &Function,
    key: &FunctionKey,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    if omitted_private_returns.contains(key)
        && returns_by_path.get(key).is_some_and(|return_type| {
            private_tail_can_use_expected(function, return_type, uses, adts)
        })
    {
        return true;
    }

    #[cfg(test)]
    private_inference_counters::record_prelude_callback_discovery_scan();

    let mut bindings = function
        .params
        .iter()
        .filter(|param| valid_value_binding_name(&param.name))
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(
                    expr,
                    annotation_type.as_ref(),
                    &reference_context,
                ) {
                    return true;
                }
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            function.module_name.as_deref(),
                            function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(expr, expected, &reference_context)
                {
                    return true;
                }
            }
        }
    }
    false
}

fn private_prelude_callback_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    if let ExprKind::NamePath(segments) = &expr.kind
        && expected.is_some_and(|expected| {
            private_callback_return_constraint_can_update(segments, expected, context)
        })
    {
        return true;
    }
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            let direct_reference =
                private_prelude_callback_call_references_slot(callee, args, expected, context);
            direct_reference
                || !matches!(callee.kind, ExprKind::NamePath(_))
                    && private_prelude_callback_expr_references_slot(callee, None, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::List(_) | ExprKind::Dict(_) | ExprKind::Record(_) => {
            private_prelude_callback_collection_references_slot(expr, expected, context)
        }
        ExprKind::Perform { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try(_)
        | ExprKind::Prefix { .. } => {
            private_prelude_callback_wrapped_expr_references_slot(expr, expected, context)
        }
        ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Binary { .. } => {
            private_prelude_callback_control_flow_references_slot(expr, expected, context)
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => false,
    }
}

fn private_prelude_callback_collection_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            let item_expected = expected.and_then(Type::vec_part);
            private_prelude_callback_expr_references_slot(item, item_expected, context)
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            private_prelude_callback_expr_references_slot(&entry.key, key_expected, context)
                || private_prelude_callback_expr_references_slot(
                    &entry.value,
                    value_expected,
                    context,
                )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            let field_expected = expected.and_then(|expected| expected.record_field(&field.name));
            private_prelude_callback_expr_references_slot(&field.expr, field_expected, context)
        }),
        _ => false,
    }
}

fn private_prelude_callback_wrapped_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Perform { args, .. } => args
            .iter()
            .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context)),
        ExprKind::Handle { body, args, .. } => {
            private_prelude_callback_expr_references_slot(body, expected, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_prelude_callback_expr_references_slot(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            ) || private_prelude_callback_expr_references_slot(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            private_prelude_callback_expr_references_slot(value, None, context)
        }
        _ => false,
    }
}

fn private_prelude_callback_control_flow_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            private_prelude_callback_expr_references_slot(scrutinee, None, context)
                || arms.iter().any(|arm| {
                    private_prelude_callback_expr_references_slot(&arm.expr, expected, context)
                })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_prelude_callback_expr_references_slot(condition, Some(&Type::bool()), context)
                || private_prelude_callback_expr_references_slot(then_branch, expected, context)
                || else_if_branches.iter().any(|branch| {
                    private_prelude_callback_expr_references_slot(
                        &branch.condition,
                        Some(&Type::bool()),
                        context,
                    ) || private_prelude_callback_expr_references_slot(
                        &branch.expr,
                        expected,
                        context,
                    )
                })
                || private_prelude_callback_expr_references_slot(else_branch, expected, context)
        }
        ExprKind::Binary { left, right, .. } => {
            private_prelude_callback_expr_references_slot(left, expected, context)
                || private_prelude_callback_expr_references_slot(right, expected, context)
        }
        _ => false,
    }
}

fn private_prelude_callback_call_references_slot(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return false;
    };
    let Some(name) =
        private_prelude_constraint_name(segments, context.current_module, context.function_by_path)
    else {
        return false;
    };
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    let Some((mut params, _)) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
    else {
        return false;
    };
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    args.iter()
        .zip(params.iter())
        .any(|(arg, param)| private_prelude_callback_arg_references_slot(arg, param, context))
}

struct PrivatePreludeCallbackReferenceContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    omitted_private_returns: &'a BTreeSet<FunctionKey>,
    returns_by_path: &'a FunctionReturnMap,
    function_by_path: &'a FunctionAstMap<'a>,
    adts: &'a AdtRegistry,
}

fn private_prelude_callback_arg_references_slot(
    expr: &Expr,
    expected: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::NamePath(segments) => {
            private_callback_return_constraint_can_update(segments, expected, context)
        }
        _ => private_prelude_callback_expr_references_slot(expr, Some(expected), context),
    }
}

fn private_callback_return_constraint_can_update(
    segments: &[String],
    expected_callback: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let Type::Function { return_type, .. } = expected_callback else {
        return false;
    };
    if type_has_unknown(return_type) {
        return false;
    }
    let [name] = segments else {
        return false;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return false;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return false;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return false;
    }
    context.returns_by_path.get(&key) != Some(return_type)
}

struct PrivatePreludeCallbackConstraintContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    function_by_path: &'a BTreeMap<(Option<String>, String), &'a Function>,
    omitted_private_returns: &'a BTreeSet<(Option<String>, String)>,
    returns_by_path: &'a mut BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

fn collect_private_prelude_callback_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_prelude_callback_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_prelude_callback_expr_constraints(
                    &entry.key,
                    key_expected,
                    context,
                );
                collect_private_prelude_callback_expr_constraints(
                    &entry.value,
                    value_expected,
                    context,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_prelude_callback_expr_constraints(
                    &field.expr,
                    field_expected,
                    context,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_prelude_callback_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_prelude_callback_expr_constraints(body, expected, context);
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_prelude_callback_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_prelude_callback_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_prelude_callback_expr_constraints(value, None, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_prelude_callback_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_prelude_callback_expr_constraints(scrutinee, None, context);
            for arm in arms {
                collect_private_prelude_callback_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_prelude_callback_expr_constraints(
                condition,
                Some(&Type::bool()),
                context,
            );
            collect_private_prelude_callback_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_prelude_callback_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_prelude_callback_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_prelude_callback_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_prelude_callback_expr_constraints(left, expected, context);
            collect_private_prelude_callback_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            if let Some(expected) = expected {
                collect_private_callback_return_constraint_for_segments(
                    segments, expected, context,
                );
            }
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_prelude_callback_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return;
    };
    let Some(name) =
        private_prelude_constraint_name(segments, context.current_module, context.function_by_path)
    else {
        return;
    };
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    let Some((mut params, _)) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
    else {
        return;
    };
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    for (arg, param) in args.iter().zip(params.iter()) {
        collect_private_callback_return_constraint(arg, param, context);
        collect_private_prelude_callback_expr_constraints(arg, Some(param), context);
    }
}

fn apply_vec_try_map_with_context_param(params: &mut [Type], context_type: Option<Type>) {
    let Some(context_type) = context_type else {
        return;
    };
    if let Some(param) = params.first_mut() {
        *param = context_type.clone();
    }
    let Some(Type::Function {
        params: callback_params,
        ..
    }) = params.get_mut(2)
    else {
        return;
    };
    if let Some(callback_context) = callback_params.first_mut() {
        *callback_context = context_type;
    }
}

fn private_prelude_constraint_name<'a>(
    segments: &'a [String],
    current_module: Option<&str>,
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
) -> Option<&'a str> {
    match segments {
        [name]
            if !function_by_path
                .contains_key(&(current_module.map(str::to_string), name.clone())) =>
        {
            Some(name)
        }
        [module, name] if module == "prelude" || module == "prelude_builtin" => Some(name),
        _ => None,
    }
}

fn collect_private_callback_return_constraint(
    arg: &Expr,
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let ExprKind::NamePath(segments) = &arg.kind else {
        return;
    };
    collect_private_callback_return_constraint_for_segments(segments, expected_callback, context);
}

fn collect_private_callback_return_constraint_for_segments(
    segments: &[String],
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let [name] = segments else {
        return;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return;
    }
    if context.returns_by_path.get(&key) == Some(return_type) {
        return;
    }
    context
        .returns_by_path
        .insert(key, return_type.as_ref().clone());
    *context.changed = true;
}

fn private_tail_can_use_expected(
    function: &Function,
    expected: &Type,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    let Some(BodyLineKind::Expr { expr }) = function.body.last().map(|line| &line.kind) else {
        return false;
    };
    tail_expr_can_use_expected(expr, expected, function.module_name.as_deref(), uses, adts)
}

fn tail_expr_can_use_expected(
    expr: &Expr,
    expected: &Type,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    match &expr.kind {
        ExprKind::List(_) => expected.vec_part().is_some(),
        ExprKind::Dict(_) => expected.dict_parts().is_some(),
        ExprKind::Record(fields) => {
            if fields.is_empty() && expected.dict_parts().is_some() {
                return true;
            }
            !fields.is_empty()
                && fields
                    .iter()
                    .all(|field| expected.record_field(&field.name).is_some())
        }
        ExprKind::NamePath(segments) => {
            matches!(
                adts.nullary_constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return false;
            };
            matches!(
                adts.constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Match { arms, .. } => arms
            .iter()
            .all(|arm| tail_expr_can_use_expected(&arm.expr, expected, current_module, uses, adts)),
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => std::iter::once(then_branch.as_ref())
            .chain(else_if_branches.iter().map(|branch| &branch.expr))
            .chain(std::iter::once(else_branch.as_ref()))
            .all(|branch| tail_expr_can_use_expected(branch, expected, current_module, uses, adts)),
        _ => false,
    }
}

fn infer_private_function_tail_type(
    function: &veln_ast::Function,
    uses: &[UseDecl],
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    #[cfg(test)]
    private_inference_counters::record_body_return_scan();

    let mut bindings = private_function_body_bindings(function, signatures_by_path);
    let mut tail = Type::unit();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                tail = infer_private_signature_expr_type(
                    expr,
                    None,
                    function.module_name.as_deref(),
                    uses,
                    &bindings,
                    returns_by_path,
                    adts,
                );
            }
        }
    }
    tail
}

fn private_function_body_bindings(
    function: &veln_ast::Function,
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
) -> Vec<Binding> {
    let signature = function
        .name
        .as_ref()
        .and_then(|name| signatures_by_path.get(&(function.module_name.clone(), name.clone())));
    function
        .params
        .iter()
        .enumerate()
        .filter(|(_, param)| valid_value_binding_name(&param.name))
        .map(|(index, param)| {
            let ty = if param.is_variadic {
                signature
                    .and_then(|signature| signature.variadic.clone())
                    .map(|ty| Type::named("List", vec![ty]))
                    .unwrap_or_else(|| function_body_param_type(param))
            } else {
                signature
                    .and_then(|signature| signature.params.get(index).cloned())
                    .unwrap_or_else(|| function_body_param_type(param))
            };
            Binding::new(param.name.clone(), ty)
        })
        .collect()
}

fn infer_private_signature_expr_type(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    let context = PrivateSignatureInferContext {
        current_module,
        uses,
        bindings,
        returns_by_path,
        adts,
    };
    match &expr.kind {
        ExprKind::Missing | ExprKind::Hole { .. } | ExprKind::TypeApply { .. } => Type::Unknown,
        ExprKind::StringLiteral(_) => Type::string(),
        ExprKind::IntLiteral(_) => Type::int(),
        ExprKind::FloatLiteral(_) => Type::float(),
        ExprKind::BoolLiteral(_) => Type::bool(),
        ExprKind::Unit => Type::unit(),
        ExprKind::NamePath(segments) => infer_private_signature_name_type(
            segments,
            expected,
            current_module,
            uses,
            bindings,
            returns_by_path,
            adts,
        ),
        ExprKind::List(items) => infer_private_list_type(items, expected, &context),
        ExprKind::Dict(entries) => infer_private_dict_type(entries, expected, &context),
        ExprKind::Record(fields) => infer_private_record_type(fields, expected, &context),
        ExprKind::Call { callee, args } => {
            infer_private_signature_call_type(callee, args, expected, &context)
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Type::Unknown
        }
        ExprKind::Handle { body, args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            context.infer(body, expected)
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            context.infer(input, Some(&Type::named("ByteView", Vec::new())));
            context.infer(base, Some(&Type::named("ByteOffset", Vec::new())));
            Type::Unknown
        }
        ExprKind::SchemaEncode { value, .. } => {
            context.infer(value, None);
            Type::Unknown
        }
        ExprKind::FieldAccess { base, field, .. } => context
            .infer(base, None)
            .record_field(field)
            .cloned()
            .unwrap_or(Type::Unknown),
        ExprKind::Try(inner) => expected.cloned().unwrap_or_else(|| {
            let inner_type = context.infer(inner, None);
            adt::result_parts(&inner_type).map_or(Type::Unknown, |(value, _)| value.clone())
        }),
        ExprKind::Match { scrutinee, arms } => {
            infer_private_match_type(scrutinee, arms, expected, &context)
        }
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => infer_private_if_result_type(
            then_branch,
            else_if_branches,
            else_branch,
            expected,
            &context,
        ),
        ExprKind::Prefix { expr, .. } => {
            context.infer(expr, expected);
            Type::Unknown
        }
        ExprKind::Binary { op, left, right } => {
            infer_private_binary_type(*op, left, right, expected, &context)
        }
    }
}

fn infer_private_list_type(
    items: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut item_type = expected
        .and_then(Type::vec_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    for item in items {
        let actual = context.infer(item, item_type_unknown_as_none(&item_type));
        if item_type == Type::Unknown {
            item_type = actual;
        }
    }
    Type::vec(item_type)
}

fn infer_private_dict_type(
    entries: &[DictEntry],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let (mut key_type, mut value_type) = expected
        .and_then(Type::dict_parts)
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    for entry in entries {
        let key_actual = context.infer(&entry.key, item_type_unknown_as_none(&key_type));
        if key_type == Type::Unknown {
            key_type = key_actual;
        }
        let value_actual = context.infer(&entry.value, item_type_unknown_as_none(&value_type));
        if value_type == Type::Unknown {
            value_type = value_actual;
        }
    }
    Type::dict(key_type, value_type)
}

fn infer_private_record_type(
    fields: &[RecordField],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if fields.is_empty()
        && let Some(expected) = expected
        && expected.dict_parts().is_some()
    {
        return expected.clone();
    }
    Type::Record(
        fields
            .iter()
            .map(|field| {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                (
                    field.name.clone(),
                    context.infer(&field.expr, field_expected),
                )
            })
            .collect(),
    )
}

fn infer_private_match_type(
    scrutinee: &Expr,
    arms: &[MatchArm],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
        arms,
        context.current_module,
        context.uses,
        context.adts,
    ) {
        MatchScrutineePatternInference::Inferred(ty) => Some(ty),
        MatchScrutineePatternInference::Uninferred
        | MatchScrutineePatternInference::Ambiguous(_) => None,
    };
    context.infer(scrutinee, scrutinee_expected.as_ref());
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for arm in arms {
        let actual = context.infer(&arm.expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

fn infer_private_if_result_type(
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for branch_expr in std::iter::once(then_branch)
        .chain(else_if_branches.iter().map(|branch| &branch.expr))
        .chain(std::iter::once(else_branch))
    {
        let actual = context.infer(branch_expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

fn infer_private_binary_type(
    op: veln_ast::BinaryOp,
    left: &Expr,
    right: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    match op {
        veln_ast::BinaryOp::Equal
        | veln_ast::BinaryOp::NotEqual
        | veln_ast::BinaryOp::Less
        | veln_ast::BinaryOp::LessEqual
        | veln_ast::BinaryOp::Greater
        | veln_ast::BinaryOp::GreaterEqual
        | veln_ast::BinaryOp::Or
        | veln_ast::BinaryOp::And => Type::bool(),
        veln_ast::BinaryOp::BitwiseOr
        | veln_ast::BinaryOp::BitwiseXor
        | veln_ast::BinaryOp::BitwiseAnd
        | veln_ast::BinaryOp::ShiftLeft
        | veln_ast::BinaryOp::ShiftRight
        | veln_ast::BinaryOp::ShiftRightLogical => Type::int(),
        veln_ast::BinaryOp::Add
        | veln_ast::BinaryOp::Subtract
        | veln_ast::BinaryOp::Multiply
        | veln_ast::BinaryOp::Divide => {
            let left = context.infer(left, expected);
            let right = context.infer(right, expected);
            if left == Type::float() || right == Type::float() {
                Type::float()
            } else {
                Type::int()
            }
        }
        veln_ast::BinaryOp::PipeGreater => Type::Unknown,
    }
}

fn item_type_unknown_as_none(ty: &Type) -> Option<&Type> {
    (ty != &Type::Unknown).then_some(ty)
}

pub(crate) fn infer_match_scrutinee_type_from_constructor_patterns(
    arms: &[MatchArm],
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> MatchScrutineePatternInference {
    let mut inferred: Option<(crate::adt::AdtConstructor<'_>, Vec<Type>)> = None;

    for arm in arms {
        let PatternKind::Constructor { name, args } = &arm.pattern.kind else {
            continue;
        };
        let candidates = adts.constructor_candidates(name, current_module, uses);
        if candidates.is_empty() {
            continue;
        }
        let descriptor_names = unique_constructor_descriptor_names(&candidates);
        if descriptor_names.len() != 1 {
            return MatchScrutineePatternInference::Ambiguous(descriptor_names);
        }
        let constructor = candidates[0];
        if let Some((previous, _)) = &inferred {
            if !same_constructor_descriptor(previous, &constructor) {
                let mut names = unique_constructor_descriptor_names(&[*previous, constructor]);
                names.sort();
                return MatchScrutineePatternInference::Ambiguous(names);
            }
        } else {
            inferred = Some((
                constructor,
                vec![Type::Unknown; constructor.descriptor.type_parameters.len()],
            ));
        }
        let Some((_, type_args)) = &mut inferred else {
            continue;
        };
        for (index, pattern) in args.iter().enumerate() {
            let Some(pattern_type) =
                infer_pattern_type_from_constructor_patterns(pattern, current_module, uses, adts)
            else {
                continue;
            };
            adt::merge_type_args_from_payload(type_args, constructor, index, &pattern_type);
        }
    }

    match inferred {
        Some((constructor, type_args)) => MatchScrutineePatternInference::Inferred(
            adt::constructed_type_from_args(constructor, &type_args),
        ),
        None => MatchScrutineePatternInference::Uninferred,
    }
}

fn infer_pattern_type_from_constructor_patterns(
    pattern: &Pattern,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> Option<Type> {
    match &pattern.kind {
        PatternKind::StringLiteral(_) => Some(Type::string()),
        PatternKind::IntLiteral(_) => Some(Type::int()),
        PatternKind::FloatLiteral(_) => Some(Type::float()),
        PatternKind::BoolLiteral(_) => Some(Type::bool()),
        PatternKind::Unit => Some(Type::unit()),
        PatternKind::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|field| {
                    (
                        field.name.clone(),
                        infer_pattern_type_from_constructor_patterns(
                            &field.pattern,
                            current_module,
                            uses,
                            adts,
                        )
                        .unwrap_or(Type::Unknown),
                    )
                })
                .collect(),
        )),
        PatternKind::Constructor { name, args } => {
            let candidates = adts.constructor_candidates(name, current_module, uses);
            let [constructor] = candidates.as_slice() else {
                return None;
            };
            let mut type_args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
            for (index, pattern) in args.iter().enumerate() {
                let Some(pattern_type) = infer_pattern_type_from_constructor_patterns(
                    pattern,
                    current_module,
                    uses,
                    adts,
                ) else {
                    continue;
                };
                adt::merge_type_args_from_payload(
                    &mut type_args,
                    *constructor,
                    index,
                    &pattern_type,
                );
            }
            Some(adt::constructed_type_from_args(*constructor, &type_args))
        }
        PatternKind::Wildcard | PatternKind::Binding(_) => None,
    }
}

fn unique_constructor_descriptor_names(
    constructors: &[crate::adt::AdtConstructor<'_>],
) -> Vec<String> {
    let mut names = Vec::new();
    for constructor in constructors {
        let name = constructor.descriptor.diagnostic_name.clone();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn same_constructor_descriptor(
    left: &crate::adt::AdtConstructor<'_>,
    right: &crate::adt::AdtConstructor<'_>,
) -> bool {
    left.descriptor.type_name == right.descriptor.type_name
        && left.descriptor.module_name == right.descriptor.module_name
        && left.descriptor.type_parameters.len() == right.descriptor.type_parameters.len()
}

fn type_has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(type_has_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_has_unknown(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_has_unknown)
                || variadic.as_deref().is_some_and(type_has_unknown)
                || type_has_unknown(return_type)
        }
    }
}

fn infer_private_signature_name_type(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    if let crate::adt::ConstructorLookup::Found(constructor) =
        adts.nullary_constructor(segments, current_module, uses)
    {
        return expected
            .and_then(|expected| {
                adt::adt_args(expected, constructor.descriptor).map(|_| expected.clone())
            })
            .unwrap_or_else(|| adt::constructed_type(constructor, &[]));
    }
    match segments {
        [name] => bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                returns_by_path
                    .get(&(current_module.map(str::to_string), name.clone()))
                    .cloned()
            })
            .unwrap_or(Type::Unknown),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                        .cloned()
                })
                .unwrap_or(Type::Unknown)
        }
        _ => Type::Unknown,
    }
}

struct PrivateSignatureInferContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    returns_by_path: &'a BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
}

impl PrivateSignatureInferContext<'_> {
    fn infer(&self, expr: &Expr, expected: Option<&Type>) -> Type {
        infer_private_signature_expr_type(
            expr,
            expected,
            self.current_module,
            self.uses,
            self.bindings,
            self.returns_by_path,
            self.adts,
        )
    }
}

fn infer_private_signature_call_type(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if let ExprKind::NamePath(segments) = &callee.kind {
        if let crate::adt::ConstructorLookup::Found(constructor) =
            context
                .adts
                .constructor(segments, context.current_module, context.uses)
        {
            let actual_args = args
                .iter()
                .map(|arg| context.infer(arg, None))
                .collect::<Vec<_>>();
            if expected
                .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
                .is_some()
            {
                return expected.cloned().unwrap_or(Type::Unknown);
            }
            return adt::constructed_type(constructor, &actual_args);
        }
        if let Some(name) = segments.last() {
            if let Some(return_type) = match segments.as_slice() {
                [name] => context
                    .returns_by_path
                    .get(&(context.current_module.map(str::to_string), name.clone())),
                [_, .., name] => imported_use_for_path(
                    context.uses,
                    &segments[..segments.len() - 1],
                    context.current_module,
                )
                .and_then(|use_decl| {
                    context
                        .returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                }),
                _ => None,
            } {
                return return_type.clone();
            }
            if let Some((params, return_type)) = crate::prelude::prelude_signature(name, expected) {
                for (arg, param) in args.iter().zip(params.iter()) {
                    context.infer(arg, Some(param));
                }
                return return_type;
            }
        }
    }
    Type::Unknown
}

pub(crate) fn function_body_param_type(param: &veln_ast::Param) -> Type {
    let ty = parse_type_or_unknown(param.ty.as_deref());
    if param.is_variadic {
        Type::named("List", vec![ty])
    } else {
        ty
    }
}

pub(crate) fn function_alias_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<FunctionSignature> {
    let companion_access_targets = BTreeMap::new();
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
                return None;
            }
            if public_alias_has_invalid_target_leaf(
                module,
                alias,
                Some(veln_ast::NameClass::Function),
            ) {
                return None;
            }
            let target = function_signature_path(
                &alias.target,
                &module.uses,
                functions,
                alias.module_name.as_deref(),
                &companion_access_targets,
            )?;
            Some(FunctionSignature {
                name,
                target_name: target.target_name.clone(),
                module_name: alias.module_name.clone(),
                visibility: Visibility::Public,
                params: target.params.clone(),
                variadic: target.variadic.clone(),
                return_type: target.return_type.clone(),
                effects: target.effects.clone(),
                node_id: alias.node_id,
                span: alias.span.clone(),
            })
        })
        .collect()
}

pub(super) fn function_signature_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    functions: &'a [FunctionSignature],
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => functions.iter().find(|function| function.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            functions.iter().find(|function| {
                function.name == *name
                    && function.module_name.as_deref() == Some(module_name)
                    && imported_function_is_visible(
                        function,
                        use_decl,
                        current_module,
                        companion_access_targets,
                    )
            })
        }
        _ => None,
    }
}

pub(super) fn collect_pattern_bindings(pattern: &Pattern, ty: &Type, bindings: &mut Vec<Binding>) {
    collect_let_pattern_bindings(pattern, ty, None, bindings);
}

fn collect_let_pattern_bindings(
    pattern: &Pattern,
    ty: &Type,
    private_function_value: Option<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => {
            if valid_value_binding_name(name) {
                bindings.push(match private_function_value {
                    Some(target) => {
                        Binding::private_function_value(name.clone(), ty.clone(), target)
                    }
                    None => Binding::new(name.clone(), ty.clone()),
                });
            }
        }
        PatternKind::Record(fields) => {
            for field in fields {
                let field_ty = ty.record_field(&field.name).unwrap_or(&Type::Unknown);
                collect_let_pattern_bindings(&field.pattern, field_ty, None, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit
        | PatternKind::Constructor { .. } => {}
    }
}

pub(super) fn imported_function_is_visible(
    function: &FunctionSignature,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    if function.visibility == Visibility::Public {
        return true;
    }
    if use_decl.package.is_none()
        && current_module.is_some_and(|module| module.starts_with("std::"))
        && function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    {
        return true;
    }
    use_decl.package.is_none()
        && current_module.is_some_and(|current_module| {
            function.module_name.as_ref().is_some_and(|target_module| {
                companion_access_targets
                    .get(current_module)
                    .is_some_and(|allowed| allowed == target_module)
            })
        })
}

#[cfg(test)]
pub(crate) mod private_inference_counters {
    use super::Cell;

    thread_local! {
        static BODY_RETURN_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_INDEX_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) body_return_scans: usize,
        pub(crate) call_site_discovery_scans: usize,
        pub(crate) call_site_scans: usize,
        pub(crate) private_reference_candidate_scans: usize,
        pub(crate) private_reference_index_scans: usize,
        pub(crate) prelude_callback_discovery_scans: usize,
        pub(crate) prelude_callback_scans: usize,
    }

    pub(crate) fn reset() {
        BODY_RETURN_SCANS.set(0);
        CALL_SITE_DISCOVERY_SCANS.set(0);
        CALL_SITE_SCANS.set(0);
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(0);
        PRIVATE_REFERENCE_INDEX_SCANS.set(0);
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(0);
        PRELUDE_CALLBACK_SCANS.set(0);
    }

    pub(crate) fn snapshot() -> Snapshot {
        Snapshot {
            body_return_scans: BODY_RETURN_SCANS.get(),
            call_site_discovery_scans: CALL_SITE_DISCOVERY_SCANS.get(),
            call_site_scans: CALL_SITE_SCANS.get(),
            private_reference_candidate_scans: PRIVATE_REFERENCE_CANDIDATE_SCANS.get(),
            private_reference_index_scans: PRIVATE_REFERENCE_INDEX_SCANS.get(),
            prelude_callback_discovery_scans: PRELUDE_CALLBACK_DISCOVERY_SCANS.get(),
            prelude_callback_scans: PRELUDE_CALLBACK_SCANS.get(),
        }
    }

    pub(super) fn record_body_return_scan() {
        BODY_RETURN_SCANS.set(BODY_RETURN_SCANS.get() + 1);
    }

    pub(super) fn record_call_site_discovery_scan() {
        CALL_SITE_DISCOVERY_SCANS.set(CALL_SITE_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_call_site_scan() {
        CALL_SITE_SCANS.set(CALL_SITE_SCANS.get() + 1);
    }

    pub(super) fn record_private_reference_candidate_scan() {
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(PRIVATE_REFERENCE_CANDIDATE_SCANS.get() + 1);
    }

    pub(super) fn record_private_reference_index_scan() {
        PRIVATE_REFERENCE_INDEX_SCANS.set(PRIVATE_REFERENCE_INDEX_SCANS.get() + 1);
    }

    pub(super) fn record_prelude_callback_discovery_scan() {
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(PRELUDE_CALLBACK_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_prelude_callback_scan() {
        PRELUDE_CALLBACK_SCANS.set(PRELUDE_CALLBACK_SCANS.get() + 1);
    }
}
