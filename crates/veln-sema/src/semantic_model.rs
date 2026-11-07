use std::ops::{Deref, DerefMut};
use std::sync::Arc;
use veln_ast::NodeId;
use veln_source::SourceSpan;

mod type_lifecycle;

pub(crate) use type_lifecycle::drop_type_iteratively;
#[cfg(test)]
pub(crate) use type_lifecycle::measure_type_clone_nodes;

pub(crate) type FunctionKey = (Option<String>, Option<String>, String);

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
    pub(crate) call_bound_effect_row: Option<String>,
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
            call_bound_effect_row: self.call_bound_effect_row.clone(),
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
            call_bound_effect_row: None,
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
            call_bound_effect_row: None,
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
            call_bound_effect_row: None,
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
            call_bound_effect_row: None,
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
    AdmittedAggregate,
    Unknown,
}

impl ExpectedTypeSource {
    pub(crate) fn as_type_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn => "declared_return",
            Self::DeclaredParameter => "declared_parameter",
            Self::SchemaEncodeParameter => "declared_parameter",
            Self::LocalAnnotation => "local_annotation",
            Self::Inferred | Self::AdmittedAggregate => "inferred_expression",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn as_hole_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn
            | Self::DeclaredParameter
            | Self::SchemaEncodeParameter
            | Self::LocalAnnotation => "declared",
            Self::Inferred | Self::AdmittedAggregate => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

// Deep admitted inference contexts repeatedly retain suffixes of named types.
// Share those immutable argument trees and detach only when unification writes.
#[derive(Clone, Debug)]
pub(crate) struct TypeArgs(Arc<Vec<Type>>);

impl PartialEq for TypeArgs {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl Eq for TypeArgs {}

impl std::hash::Hash for TypeArgs {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl TypeArgs {
    pub(crate) fn into_vec(self) -> Vec<Type> {
        Arc::try_unwrap(self.0).unwrap_or_else(|args| (*args).clone())
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl From<Vec<Type>> for TypeArgs {
    fn from(args: Vec<Type>) -> Self {
        Self(Arc::new(args))
    }
}

impl FromIterator<Type> for TypeArgs {
    fn from_iter<T: IntoIterator<Item = Type>>(iter: T) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}

impl From<TypeArgs> for Vec<Type> {
    fn from(args: TypeArgs) -> Self {
        args.into_vec()
    }
}

impl PartialEq<Vec<Type>> for TypeArgs {
    fn eq(&self, other: &Vec<Type>) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl PartialEq<TypeArgs> for Vec<Type> {
    fn eq(&self, other: &TypeArgs) -> bool {
        self.as_slice() == other.as_slice()
    }
}

impl Deref for TypeArgs {
    type Target = Vec<Type>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for TypeArgs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        Arc::make_mut(&mut self.0)
    }
}

impl IntoIterator for TypeArgs {
    type Item = Type;
    type IntoIter = std::vec::IntoIter<Type>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_vec().into_iter()
    }
}

impl<'a> IntoIterator for &'a TypeArgs {
    type Item = &'a Type;
    type IntoIter = std::slice::Iter<'a, Type>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> IntoIterator for &'a mut TypeArgs {
    type Item = &'a mut Type;
    type IntoIter = std::slice::IterMut<'a, Type>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(crate) enum Type {
    Unknown,
    Named {
        name: String,
        identity: String,
        args: TypeArgs,
    },
    VariantRefinement {
        name: String,
        identity: String,
        args: TypeArgs,
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

impl Type {
    const RECOVERY_SLOT_IDENTITY_PREFIX: &'static str = "internal:recovery-slot:";
    pub(crate) const COMPILER_OPTION_IDENTITY: &'static str = "internal:compiler::Option";
    pub(crate) const COMPILER_RESULT_IDENTITY: &'static str = "internal:compiler::Result";
    pub(crate) const COMPILER_LIST_IDENTITY: &'static str = "internal:compiler::List";
    pub(crate) const COMPILER_VEC_IDENTITY: &'static str = "internal:compiler:Vec";
    pub(crate) const COMPILER_DICT_IDENTITY: &'static str = "internal:compiler:Dict";

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
            args: args.into(),
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
            args: args.into(),
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
            args: args.into(),
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
            args: args.into(),
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
            args: args.into(),
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
            args: args.into(),
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

    pub(crate) fn option(value: Type) -> Self {
        Self::resolved_named("Option", Self::COMPILER_OPTION_IDENTITY, vec![value])
    }

    pub(crate) fn result(value: Type, error: Type) -> Self {
        Self::resolved_named("Result", Self::COMPILER_RESULT_IDENTITY, vec![value, error])
    }

    pub(crate) fn list(item: Type) -> Self {
        Self::resolved_named("List", Self::COMPILER_LIST_IDENTITY, vec![item])
    }

    pub(crate) fn vec(item: Type) -> Self {
        Self::resolved_named("Vec", Self::COMPILER_VEC_IDENTITY, vec![item])
    }

    pub(crate) fn dict(key: Type, value: Type) -> Self {
        Self::resolved_named("Dict", Self::COMPILER_DICT_IDENTITY, vec![key, value])
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
            Self::Named { identity, args, .. }
                if Self::is_compiler_vec_identity(identity) && args.len() == 1 =>
            {
                Some(&args[0])
            }
            _ => None,
        }
    }

    pub(crate) fn is_compiler_vec_identity(identity: &str) -> bool {
        identity == Self::COMPILER_VEC_IDENTITY
    }

    pub(crate) fn is_compiler_dict_identity(identity: &str) -> bool {
        identity == Self::COMPILER_DICT_IDENTITY
    }

    pub(crate) fn is_compiler_covariant_identity(identity: &str) -> bool {
        Self::compiler_covariant_name(identity).is_some()
    }

    pub(crate) fn compiler_covariant_name(identity: &str) -> Option<&'static str> {
        match identity {
            Self::COMPILER_OPTION_IDENTITY => Some("Option"),
            Self::COMPILER_RESULT_IDENTITY => Some("Result"),
            Self::COMPILER_LIST_IDENTITY => Some("List"),
            Self::COMPILER_VEC_IDENTITY => Some("Vec"),
            Self::COMPILER_DICT_IDENTITY => Some("Dict"),
            _ => None,
        }
    }

    pub(crate) fn dict_parts(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Named { identity, args, .. }
                if Self::is_compiler_dict_identity(identity) && args.len() == 2 =>
            {
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
