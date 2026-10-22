use std::sync::Arc;
use veln_ast::NodeId;
use veln_source::SourceSpan;

pub(crate) type FunctionKey = (Option<String>, String);

pub(crate) struct CallOrigin {
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) symbol: String,
    pub(crate) effects: Vec<String>,
    pub(crate) schema_encode: bool,
}

#[derive(Clone)]
pub(crate) struct EffectUse {
    pub(crate) effect: String,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) kind: &'static str,
    pub(crate) symbol: String,
}

#[cfg_attr(not(test), derive(Clone))]
pub(crate) struct Binding {
    pub(crate) name: String,
    pub(crate) ty: Type,
    pub(crate) transparent_alias_group: Option<usize>,
    pub(crate) stable_record_root: Option<usize>,
    pub(crate) stable_place: Option<usize>,
    pub(crate) type_origin: Option<TypeOrigin>,
    pub(crate) private_function_value: Option<FunctionKey>,
    pub(crate) schema_encode_callable: bool,
    pub(crate) is_diagnosed_unknown: bool,
}

#[derive(Clone)]
pub(crate) struct TypeOrigin {
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) source: ExpectedTypeSource,
    pub(crate) message: &'static str,
}

#[cfg(test)]
thread_local! {
    static BINDING_CLONES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MAX_SCOPED_BINDINGS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
impl Clone for Binding {
    fn clone(&self) -> Self {
        BINDING_CLONES.set(BINDING_CLONES.get() + 1);
        Self {
            name: self.name.clone(),
            ty: self.ty.clone(),
            transparent_alias_group: self.transparent_alias_group,
            stable_record_root: self.stable_record_root,
            stable_place: self.stable_place,
            type_origin: self.type_origin.clone(),
            private_function_value: self.private_function_value.clone(),
            schema_encode_callable: self.schema_encode_callable,
            is_diagnosed_unknown: self.is_diagnosed_unknown,
        }
    }
}

#[cfg(test)]
pub(crate) fn reset_binding_clone_count() {
    BINDING_CLONES.set(0);
    MAX_SCOPED_BINDINGS.set(0);
}

#[cfg(test)]
pub(crate) fn binding_clone_count() -> usize {
    BINDING_CLONES.get()
}

#[cfg(test)]
pub(crate) fn record_scoped_binding_count(count: usize) {
    MAX_SCOPED_BINDINGS.set(MAX_SCOPED_BINDINGS.get().max(count));
}

#[cfg(test)]
pub(crate) fn max_scoped_binding_count() -> usize {
    MAX_SCOPED_BINDINGS.get()
}

impl Binding {
    pub(crate) fn new(name: String, ty: Type) -> Self {
        Self {
            name,
            ty,
            transparent_alias_group: None,
            stable_record_root: None,
            stable_place: None,
            type_origin: None,
            private_function_value: None,
            schema_encode_callable: false,
            is_diagnosed_unknown: false,
        }
    }

    pub(crate) fn builtin_callsite() -> Self {
        Self {
            name: "callsite".to_string(),
            ty: Type::source_location(),
            transparent_alias_group: None,
            stable_record_root: None,
            stable_place: None,
            type_origin: None,
            private_function_value: None,
            schema_encode_callable: false,
            is_diagnosed_unknown: false,
        }
    }

    pub(crate) fn private_function_value(name: String, ty: Type, target: FunctionKey) -> Self {
        Self {
            name,
            ty,
            transparent_alias_group: None,
            stable_record_root: None,
            stable_place: None,
            type_origin: None,
            private_function_value: Some(target),
            schema_encode_callable: false,
            is_diagnosed_unknown: false,
        }
    }

    pub(crate) fn diagnosed_unknown(name: String, ty: Type) -> Self {
        Self {
            name,
            ty,
            transparent_alias_group: None,
            stable_record_root: None,
            stable_place: None,
            type_origin: None,
            private_function_value: None,
            schema_encode_callable: false,
            is_diagnosed_unknown: true,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ExpectedType {
    pub(crate) ty: Type,
    pub(crate) source: ExpectedTypeSource,
    pub(crate) origin_node_id: NodeId,
    pub(crate) origin_span: Option<SourceSpan>,
    pub(crate) origin_message: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) enum ExpectedTypeSource {
    DeclaredReturn,
    DeclaredParameter,
    SchemaEncodeParameter,
    LocalAnnotation,
    Inferred,
    Unknown,
}

impl ExpectedTypeSource {
    pub(crate) fn as_type_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn => "declared_return",
            Self::DeclaredParameter => "declared_parameter",
            Self::SchemaEncodeParameter => "declared_parameter",
            Self::LocalAnnotation => "local_annotation",
            Self::Inferred => "inferred_expression",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn as_hole_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn
            | Self::DeclaredParameter
            | Self::SchemaEncodeParameter
            | Self::LocalAnnotation => "declared",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg_attr(not(test), derive(Clone))]
#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    Unknown,
    Named {
        name: String,
        identity: String,
        args: Vec<Type>,
    },
    VariantRefinement {
        name: String,
        identity: String,
        args: Vec<Type>,
        variants: Arc<[String]>,
        unresolved_alternatives: Vec<(String, Vec<Type>, String)>,
    },
    Record(Vec<(String, Type)>),
    Function {
        params: Vec<Type>,
        variadic: Option<Box<Type>>,
        return_type: Box<Type>,
        effects: Vec<String>,
    },
}

#[cfg(test)]
thread_local! {
    static TYPE_CLONE_MEASUREMENT_DEPTH: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static MEASURED_TYPE_CLONE_NODES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
impl Clone for Type {
    fn clone(&self) -> Self {
        TYPE_CLONE_MEASUREMENT_DEPTH.with(|depth| {
            if depth.get() > 0 {
                MEASURED_TYPE_CLONE_NODES.set(MEASURED_TYPE_CLONE_NODES.get() + 1);
            }
        });
        match self {
            Self::Unknown => Self::Unknown,
            Self::Named {
                name,
                identity,
                args,
            } => Self::Named {
                name: name.clone(),
                identity: identity.clone(),
                args: args.clone(),
            },
            Self::VariantRefinement {
                name,
                identity,
                args,
                variants,
                unresolved_alternatives,
            } => Self::VariantRefinement {
                name: name.clone(),
                identity: identity.clone(),
                args: args.clone(),
                variants: variants.clone(),
                unresolved_alternatives: unresolved_alternatives.clone(),
            },
            Self::Record(fields) => Self::Record(fields.clone()),
            Self::Function {
                params,
                variadic,
                return_type,
                effects,
            } => Self::Function {
                params: params.clone(),
                variadic: variadic.clone(),
                return_type: return_type.clone(),
                effects: effects.clone(),
            },
        }
    }
}

#[cfg(test)]
pub(crate) fn measure_type_clone_nodes<T>(operation: impl FnOnce() -> T) -> (T, usize) {
    struct MeasurementGuard;

    impl Drop for MeasurementGuard {
        fn drop(&mut self) {
            TYPE_CLONE_MEASUREMENT_DEPTH.with(|depth| depth.set(depth.get() - 1));
        }
    }

    let is_outermost = TYPE_CLONE_MEASUREMENT_DEPTH.with(|depth| {
        let current = depth.get();
        depth.set(current + 1);
        current == 0
    });
    if is_outermost {
        MEASURED_TYPE_CLONE_NODES.set(0);
    }
    let guard = MeasurementGuard;
    let result = operation();
    drop(guard);
    let measured = if is_outermost {
        MEASURED_TYPE_CLONE_NODES.get()
    } else {
        0
    };
    (result, measured)
}

impl Type {
    const RECOVERY_SLOT_IDENTITY_PREFIX: &'static str = "internal:recovery-slot:";

    pub(crate) fn recovery_slot(index: usize) -> Self {
        Self::resolved_named(
            "",
            format!("{}{index}", Self::RECOVERY_SLOT_IDENTITY_PREFIX),
            Vec::new(),
        )
    }

    pub(crate) fn is_recovery_slot(&self) -> bool {
        let Self::Named { identity, args, .. } = self else {
            return false;
        };
        args.is_empty() && identity.starts_with(Self::RECOVERY_SLOT_IDENTITY_PREFIX)
    }

    pub(crate) fn named(name: impl Into<String>, args: Vec<Type>) -> Self {
        let name = name.into();
        Self::Named {
            identity: name.clone(),
            name,
            args,
        }
    }

    pub(crate) fn resolved_named(
        name: impl Into<String>,
        identity: impl Into<String>,
        args: Vec<Type>,
    ) -> Self {
        Self::Named {
            name: name.into(),
            identity: identity.into(),
            args,
        }
    }

    pub(crate) fn variant_refinement(
        name: impl Into<String>,
        args: Vec<Type>,
        variants: Vec<String>,
    ) -> Self {
        let name = name.into();
        Self::VariantRefinement {
            identity: name.clone(),
            name,
            args,
            variants: variants.into(),
            unresolved_alternatives: Vec::new(),
        }
    }

    pub(crate) fn resolved_variant_refinement(
        name: impl Into<String>,
        identity: impl Into<String>,
        args: Vec<Type>,
        variants: Vec<String>,
    ) -> Self {
        Self::VariantRefinement {
            name: name.into(),
            identity: identity.into(),
            args,
            variants: variants.into(),
            unresolved_alternatives: Vec::new(),
        }
    }

    pub(crate) fn resolved_variant_refinement_shared(
        name: impl Into<String>,
        identity: impl Into<String>,
        args: Vec<Type>,
        variants: Arc<[String]>,
    ) -> Self {
        Self::VariantRefinement {
            name: name.into(),
            identity: identity.into(),
            args,
            variants,
            unresolved_alternatives: Vec::new(),
        }
    }

    pub(crate) fn unresolved_variant_union(
        name: String,
        args: Vec<Type>,
        variants: Vec<String>,
        unresolved_alternatives: Vec<(String, Vec<Type>, String)>,
    ) -> Self {
        Self::VariantRefinement {
            identity: name.clone(),
            name,
            args,
            variants: variants.into(),
            unresolved_alternatives,
        }
    }

    pub(crate) fn bool() -> Self {
        Self::named("Bool", Vec::new())
    }

    pub(crate) fn int() -> Self {
        Self::named("Int", Vec::new())
    }

    pub(crate) fn float() -> Self {
        Self::named("Float", Vec::new())
    }

    pub(crate) fn string() -> Self {
        Self::named("String", Vec::new())
    }

    pub(crate) fn unit() -> Self {
        Self::named("Unit", Vec::new())
    }

    pub(crate) fn wall_time() -> Self {
        Self::Record(vec![
            ("unix_seconds".to_string(), Self::int()),
            ("nanosecond".to_string(), Self::int()),
        ])
    }

    pub(crate) fn source_location() -> Self {
        Self::Record(vec![
            ("package".to_string(), Self::string()),
            ("module".to_string(), Self::string()),
            ("file".to_string(), Self::string()),
            ("start_line".to_string(), Self::int()),
            ("start_column".to_string(), Self::int()),
            ("start_offset".to_string(), Self::int()),
            ("end_line".to_string(), Self::int()),
            ("end_column".to_string(), Self::int()),
            ("end_offset".to_string(), Self::int()),
        ])
    }

    #[cfg(test)]
    pub(crate) fn result(value: Type, error: Type) -> Self {
        Self::named("Result", vec![value, error])
    }

    pub(crate) fn vec(item: Type) -> Self {
        Self::named("Vec", vec![item])
    }

    pub(crate) fn dict(key: Type, value: Type) -> Self {
        Self::named("Dict", vec![key, value])
    }

    pub(crate) fn function(params: Vec<Type>, return_type: Type, effects: Vec<String>) -> Self {
        Self::Function {
            params,
            variadic: None,
            return_type: Box::new(return_type),
            effects,
        }
    }

    pub(crate) fn variadic_function(
        params: Vec<Type>,
        variadic: Type,
        return_type: Type,
        effects: Vec<String>,
    ) -> Self {
        Self::Function {
            params,
            variadic: Some(Box::new(variadic)),
            return_type: Box::new(return_type),
            effects,
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Unknown => "unknown".to_string(),
            Self::Named { name, args, .. } if name == "Unit" && args.is_empty() => "()".to_string(),
            Self::Named { name, args, .. } => render_named_type(name, args),
            Self::VariantRefinement {
                name,
                args,
                variants,
                ..
            } => render_variant_refinement(name, args, variants),
            Self::Record(fields) => render_record_type(fields),
            Self::Function {
                params,
                variadic,
                return_type,
                effects,
            } => render_function_type(params, variadic.as_deref(), return_type, effects),
        }
    }

    pub(crate) fn vec_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args, .. } if name == "Vec" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub(crate) fn dict_parts(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Named { name, args, .. } if name == "Dict" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    pub(crate) fn record_field(&self, field_name: &str) -> Option<&Type> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find_map(|(name, ty)| (name == field_name).then_some(ty)),
            _ => None,
        }
    }

    pub(crate) fn function_parts(&self) -> Option<(&[Type], &Type)> {
        match self {
            Self::Function {
                params,
                variadic,
                return_type,
                ..
            } if variadic.is_none() => Some((params, return_type)),
            _ => None,
        }
    }

    pub(crate) fn callable_parts(&self) -> Option<(&[Type], Option<&Type>, &Type)> {
        match self {
            Self::Function {
                params,
                variadic,
                return_type,
                ..
            } => Some((params, variadic.as_deref(), return_type)),
            _ => None,
        }
    }

    pub(crate) fn function_effects(&self) -> Option<&[String]> {
        match self {
            Self::Function { effects, .. } => Some(effects),
            _ => None,
        }
    }
}

fn render_named_type(name: &str, args: &[Type]) -> String {
    if args.is_empty() {
        name.to_string()
    } else {
        format!("{name}<{}>", render_type_list(args))
    }
}

fn render_variant_refinement(name: &str, args: &[Type], variants: &[String]) -> String {
    let base = render_named_type(name, args);
    variants
        .iter()
        .map(|variant| format!("{base}::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn render_record_type(fields: &[(String, Type)]) -> String {
    let fields = fields
        .iter()
        .map(|(name, ty)| format!("{name}: {}", ty.render()))
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{{fields}}}")
}

fn render_function_type(
    params: &[Type],
    variadic: Option<&Type>,
    return_type: &Type,
    effects: &[String],
) -> String {
    let mut rendered_params = params.iter().map(Type::render).collect::<Vec<_>>();
    if let Some(variadic) = variadic {
        rendered_params.push(format!("...{}", variadic.render()));
    }
    let effects = if effects.is_empty() {
        String::new()
    } else {
        format!(" effects [{}]", effects.join(", "))
    };
    format!(
        "fn({}) -> {}{effects}",
        rendered_params.join(", "),
        return_type.render()
    )
}

fn render_type_list(types: &[Type]) -> String {
    types
        .iter()
        .map(Type::render)
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod type_clone_measurement_tests {
    use super::{Type, measure_type_clone_nodes};

    #[test]
    fn type_clone_measurement_counts_container_children() {
        let ty = Type::Function {
            params: vec![Type::Record(vec![(
                "field".to_string(),
                Type::named("Vec", vec![Type::Unknown]),
            )])],
            variadic: Some(Box::new(Type::VariantRefinement {
                name: "State".to_string(),
                identity: "State".to_string(),
                args: vec![Type::named("Int", Vec::new())],
                variants: vec!["Ready".to_string()].into(),
                unresolved_alternatives: vec![(
                    "Other".to_string(),
                    vec![Type::Unknown],
                    "Done".to_string(),
                )],
            })),
            return_type: Box::new(Type::named(
                "Result",
                vec![Type::Unknown, Type::named("String", Vec::new())],
            )),
            effects: Vec::new(),
        };

        let (cloned, measured_nodes) = measure_type_clone_nodes(|| ty.clone());

        assert_eq!(cloned, ty);
        assert_eq!(measured_nodes, 10);

        let (_, container_nodes) = measure_type_clone_nodes(|| vec![ty.clone()]);
        assert_eq!(container_nodes, 10);

        let (_, temporary_nodes) = measure_type_clone_nodes(|| {
            let _temporary = ty.clone();
        });
        assert_eq!(temporary_nodes, 10);
    }
}
