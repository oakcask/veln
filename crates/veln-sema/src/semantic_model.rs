use veln_ast::NodeId;
use veln_source::SourceSpan;

pub(crate) type FunctionKey = (Option<String>, String);

pub(crate) struct CallOrigin {
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) symbol: String,
    pub(crate) effects: Vec<String>,
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
    pub(crate) private_function_value: Option<FunctionKey>,
    pub(crate) is_diagnosed_unknown: bool,
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
            private_function_value: self.private_function_value.clone(),
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
            private_function_value: None,
            is_diagnosed_unknown: false,
        }
    }

    pub(crate) fn builtin_callsite() -> Self {
        Self {
            name: "callsite".to_string(),
            ty: Type::source_location(),
            private_function_value: None,
            is_diagnosed_unknown: false,
        }
    }

    pub(crate) fn private_function_value(name: String, ty: Type, target: FunctionKey) -> Self {
        Self {
            name,
            ty,
            private_function_value: Some(target),
            is_diagnosed_unknown: false,
        }
    }

    pub(crate) fn diagnosed_unknown(name: String, ty: Type) -> Self {
        Self {
            name,
            ty,
            private_function_value: None,
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
    LocalAnnotation,
    Inferred,
    Unknown,
}

impl ExpectedTypeSource {
    pub(crate) fn as_type_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn => "declared_return",
            Self::DeclaredParameter => "declared_parameter",
            Self::LocalAnnotation => "local_annotation",
            Self::Inferred => "inferred_expression",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn as_hole_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn | Self::DeclaredParameter | Self::LocalAnnotation => "declared",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    Unknown,
    Named {
        name: String,
        args: Vec<Type>,
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
    pub(crate) fn named(name: impl Into<String>, args: Vec<Type>) -> Self {
        Self::Named {
            name: name.into(),
            args,
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
            Self::Named { name, args } if name == "Unit" && args.is_empty() => "()".to_string(),
            Self::Named { name, args } if args.is_empty() => name.clone(),
            Self::Named { name, args } => {
                let args = args.iter().map(Type::render).collect::<Vec<_>>().join(", ");
                format!("{name}<{args}>")
            }
            Self::Record(fields) => {
                let fields = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", ty.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{fields}}}")
            }
            Self::Function {
                params,
                variadic,
                return_type,
                effects,
            } => {
                let mut rendered_params = params.iter().map(Type::render).collect::<Vec<_>>();
                if let Some(variadic) = variadic {
                    rendered_params.push(format!("...{}", variadic.render()));
                }
                let params = rendered_params.join(", ");
                let effects = if effects.is_empty() {
                    String::new()
                } else {
                    format!(" effects [{}]", effects.join(", "))
                };
                format!("fn({params}) -> {}{effects}", return_type.render())
            }
        }
    }

    pub(crate) fn vec_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args } if name == "Vec" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub(crate) fn dict_parts(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Named { name, args } if name == "Dict" && args.len() == 2 => {
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
