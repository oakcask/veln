use std::collections::BTreeMap;

use veln_ast::{FunctionKind, SurfaceModule};

use crate::adt::registry::AdtRegistry;
use crate::name_recovery::normal_use_decls;
use crate::type_syntax::parse_type_or_unknown;
use crate::types::private_inference::function_signature_params;
use crate::types::signatures::{CompanionAccessTarget, EffectSignature, FunctionSignature};
use veln_ast::{Function, UseDecl};

use super::{canonical_declared_effects, canonicalize_type_effects};

pub(in crate::types) fn ordinary_function_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<FunctionSignature> {
    let mut name_counts = BTreeMap::<String, usize>::new();
    for function in module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| function.name.as_ref())
    {
        *name_counts.entry(function.clone()).or_default() += 1;
    }
    let uses = normal_use_decls(module);
    let quarantined_uses = crate::name_recovery::quarantined_use_decls(module);
    let context = OrdinarySignatureContext {
        uses: &uses,
        quarantined_uses: &quarantined_uses,
        effects,
        adts,
        companion_effect_access_targets,
        name_counts: &name_counts,
    };
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| ordinary_function_signature(function, &context))
        .collect()
}

struct OrdinarySignatureContext<'a> {
    uses: &'a [UseDecl],
    quarantined_uses: &'a [UseDecl],
    effects: &'a [EffectSignature],
    adts: &'a AdtRegistry,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
    name_counts: &'a BTreeMap<String, usize>,
}

impl OrdinarySignatureContext<'_> {
    fn canonicalize(
        &self,
        ty: crate::semantic_model::Type,
        current_module: Option<&str>,
    ) -> crate::semantic_model::Type {
        canonicalize_type_effects(
            ty,
            self.uses,
            self.quarantined_uses,
            current_module,
            self.effects,
            self.adts,
            self.companion_effect_access_targets,
        )
    }
}

fn ordinary_function_signature(
    function: &Function,
    context: &OrdinarySignatureContext<'_>,
) -> Option<FunctionSignature> {
    let name = function.name.clone()?;
    if !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase) {
        return None;
    }
    let current_module = function.module_name.as_deref();
    let (params, variadic) = function_signature_params(function);
    Some(FunctionSignature {
        target_name: crate::standard_symbols::function_link_name(
            current_module,
            function.package_name.as_deref(),
            &name,
            context.name_counts.get(&name).copied().unwrap_or_default() > 1,
        ),
        name,
        module_name: function.module_name.clone(),
        package_name: function.package_name.clone(),
        visibility: function.visibility,
        params: params
            .into_iter()
            .map(|ty| context.canonicalize(ty, current_module))
            .collect(),
        variadic: variadic.map(|ty| context.canonicalize(ty, current_module)),
        return_type: context.canonicalize(
            parse_type_or_unknown(function.return_type.as_deref()),
            current_module,
        ),
        effects: canonical_declared_effects(
            function.effects.clone().unwrap_or_default(),
            context.uses,
            context.quarantined_uses,
            current_module,
            context.effects,
            context.companion_effect_access_targets,
        ),
        callsite: function.callsite.is_some(),
        node_id: function.node_id,
        span: function.span.clone(),
    })
}
