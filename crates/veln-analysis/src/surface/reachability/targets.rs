use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct ReachableFunction {
    pub(super) kind: FunctionKind,
    pub(super) name: String,
    pub(super) module_name: Option<String>,
    pub(super) node_id: Option<veln_ast::NodeId>,
}

pub(super) struct FunctionTarget {
    pub(super) name: String,
    pub(super) module_name: Option<String>,
    pub(super) target_name: String,
    pub(super) target_module_name: Option<String>,
    pub(super) target_node_id: veln_ast::NodeId,
    pub(super) visibility: Visibility,
    pub(super) shape: FunctionShape,
    pub(super) bare_importable: bool,
    pub(super) requires_public_import: bool,
    pub(super) recovery: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct FunctionShape {
    pub(super) fixed_arity: usize,
    pub(super) variadic: Option<String>,
}

pub(super) fn function_alias_targets(
    inputs: &ReachabilityInputs<'_>,
    function_targets: &[FunctionTarget],
) -> Vec<FunctionTarget> {
    let uses = inputs.uses();
    inputs
        .aliases()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let recovery = !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase);
            let target = target_for_alias_path(
                &alias.target,
                &uses,
                function_targets,
                alias.module_name.as_deref(),
            )?;
            if companion_alias_targets_imported_private_function(alias, target) {
                return None;
            }
            if target.recovery {
                return None;
            }
            Some(FunctionTarget {
                name,
                module_name: alias.module_name.clone(),
                target_name: target.target_name.clone(),
                target_module_name: target.target_module_name.clone(),
                target_node_id: target.target_node_id,
                visibility: Visibility::Public,
                shape: target.shape.clone(),
                bare_importable: true,
                requires_public_import: false,
                recovery,
            })
        })
        .collect()
}

pub(super) fn companion_alias_targets_imported_private_function(
    alias: &veln_ast::PublicAlias,
    target: &FunctionTarget,
) -> bool {
    target.visibility != Visibility::Public
        && alias.module_name != target.target_module_name
        && classify_companion_source(alias.span.file.as_str()).is_some()
}

pub(super) fn target_for_alias_path<'a>(
    segments: &[String],
    uses: &[&UseDecl],
    function_targets: &'a [FunctionTarget],
    current_module: Option<&str>,
) -> Option<&'a FunctionTarget> {
    match segments {
        [name] => function_targets.iter().find(|target| target.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            function_targets.iter().find(|target| {
                target.name == *name
                    && target.module_name.as_deref() == Some(module_name)
                    && imported_target_is_visible(target, use_decl)
            })
        }
        _ => None,
    }
}
