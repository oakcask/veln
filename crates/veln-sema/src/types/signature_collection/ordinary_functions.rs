use std::collections::BTreeMap;

use veln_ast::{FunctionKind, SurfaceModule};

use crate::adt::registry::AdtRegistry;
use crate::name_recovery::normal_use_decls;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_or_unknown;
use crate::types::private_inference::function_signature_params;
use crate::types::signatures::{CompanionAccessTarget, EffectSignature, FunctionSignature};
use veln_ast::{Function, UseDecl};

use super::{canonical_declared_effects, canonicalize_type_effects_with_context};
use crate::types::TypeParameterIdentityContext;

#[derive(Clone, Copy)]
pub(in crate::types) enum FunctionSignaturePurpose {
    TypeChecking,
    PathClassification,
}

pub(in crate::types) fn ordinary_function_signatures(
    module: &SurfaceModule,
    purpose: FunctionSignaturePurpose,
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
        purpose,
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
    purpose: FunctionSignaturePurpose,
    uses: &'a [UseDecl],
    quarantined_uses: &'a [UseDecl],
    effects: &'a [EffectSignature],
    adts: &'a AdtRegistry,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
    name_counts: &'a BTreeMap<String, usize>,
}

impl OrdinarySignatureContext<'_> {
    fn signature_shape(&self, function: &Function) -> OrdinarySignatureShape {
        match self.purpose {
            FunctionSignaturePurpose::TypeChecking => self.typechecking_shape(function),
            FunctionSignaturePurpose::PathClassification => path_classification_shape(function),
        }
    }

    fn typechecking_shape(&self, function: &Function) -> OrdinarySignatureShape {
        let type_parameters = TypeParameterIdentityContext::function_parameters(function);
        let type_parameter_context = TypeParameterIdentityContext::for_function(function);
        let (params, variadic) = function_signature_params(function);
        OrdinarySignatureShape {
            type_parameters,
            params: params
                .into_iter()
                .map(|ty| self.canonicalize_for_function(ty, function, &type_parameter_context))
                .collect(),
            variadic: variadic
                .map(|ty| self.canonicalize_for_function(ty, function, &type_parameter_context)),
            return_type: self.canonicalize_for_function(
                parse_type_or_unknown(function.return_type.as_deref()),
                function,
                &type_parameter_context,
            ),
            effects: canonical_declared_effects(
                function.effects.clone().unwrap_or_default(),
                self.uses,
                self.quarantined_uses,
                function.module_name.as_deref(),
                function.package_name.as_deref(),
                self.effects,
                self.companion_effect_access_targets,
            ),
        }
    }

    fn canonicalize_for_function(
        &self,
        ty: Type,
        function: &Function,
        type_parameters: &TypeParameterIdentityContext<'_>,
    ) -> Type {
        canonicalize_type_effects_with_context(
            ty,
            self.uses,
            self.quarantined_uses,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
            self.effects,
            self.adts,
            self.companion_effect_access_targets,
            Some(type_parameters),
        )
    }
}

struct OrdinarySignatureShape {
    type_parameters: Vec<crate::types::signatures::FunctionTypeParameter>,
    params: Vec<Type>,
    variadic: Option<Type>,
    return_type: Type,
    effects: Vec<String>,
}

fn path_classification_shape(function: &Function) -> OrdinarySignatureShape {
    // Path roles need callable identity and arity, not annotation types.
    OrdinarySignatureShape {
        type_parameters: Vec::new(),
        params: function
            .params
            .iter()
            .filter(|param| !param.is_variadic)
            .map(|_| Type::Unknown)
            .collect(),
        variadic: function
            .params
            .iter()
            .any(|param| param.is_variadic)
            .then_some(Type::Unknown),
        return_type: Type::Unknown,
        effects: Vec::new(),
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
    let shape = context.signature_shape(function);
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
        type_parameters: shape.type_parameters,
        params: shape.params.into(),
        variadic: shape.variadic,
        return_type: shape.return_type,
        effects: shape.effects,
        callsite: function.callsite.is_some(),
        node_id: function.node_id,
        span: function.span.clone(),
    })
}
