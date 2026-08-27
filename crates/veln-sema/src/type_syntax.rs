use crate::builtin_type_syntax::BuiltinTypeSyntaxRegistry;
use crate::semantic_model::Type;
use crate::source_less_names::InvalidStandardSymbolCase;
use crate::type_annotation_parser::parse_type_annotation_with_arity;

fn with_builtin_type_syntax_registry<R>(
    lookup: impl FnOnce(&BuiltinTypeSyntaxRegistry) -> R,
) -> Result<R, InvalidStandardSymbolCase> {
    crate::source_less_lookup::with_builtin_type_syntax_registry(lookup)
}

pub fn type_annotation_reference_names(text: &str) -> Result<Vec<String>, String> {
    Ok(type_annotation_reference_paths(text)?
        .into_iter()
        .flatten()
        .collect())
}

pub fn type_annotation_reference_paths(text: &str) -> Result<Vec<Vec<String>>, String> {
    let ty = parse_type_annotation(text)?;
    let mut paths = Vec::new();
    collect_type_reference_paths(&ty, &mut paths);
    Ok(paths)
}

pub(crate) fn parse_type_or_unknown(text: Option<&str>) -> Type {
    text.and_then(|text| parse_type_annotation(text).ok())
        .unwrap_or(Type::Unknown)
}

pub(crate) fn parse_type_annotation(text: &str) -> Result<Type, String> {
    parse_type_annotation_with_arity(text, &|name| {
        with_builtin_type_syntax_registry(|registry| registry.arity(name))
            .map_err(|failure| failure.diagnostic().message)
    })
}

fn collect_type_reference_paths(ty: &Type, paths: &mut Vec<Vec<String>>) {
    match ty {
        Type::Named { name, args } => {
            paths.push(name.split("::").map(str::to_string).collect());
            for arg in args {
                collect_type_reference_paths(arg, paths);
            }
        }
        Type::Record(fields) => {
            for (_, field_type) in fields {
                collect_type_reference_paths(field_type, paths);
            }
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            for param in params {
                collect_type_reference_paths(param, paths);
            }
            if let Some(variadic) = variadic {
                collect_type_reference_paths(variadic, paths);
            }
            collect_type_reference_paths(return_type, paths);
        }
        Type::Unknown => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_annotation_reference_names_ignore_record_fields() {
        let names =
            type_annotation_reference_names("{item: Int, payload: module::Packet}").unwrap();

        assert_eq!(names, vec!["Int", "module", "Packet"]);
    }

    #[test]
    fn type_annotation_reference_names_collect_nested_function_types() {
        let names = type_annotation_reference_names(
            "fn({input: Request}, ...stream::Chunk) -> Result<Response, error::AppError>",
        )
        .unwrap();

        assert_eq!(
            names,
            vec![
                "Request", "stream", "Chunk", "Result", "Response", "error", "AppError"
            ]
        );
    }

    #[test]
    fn type_annotation_reference_paths_preserve_qualified_names() {
        let paths = type_annotation_reference_paths(
            "fn({input: Request}, ...stream::Chunk) -> Result<Response, error::AppError>",
        )
        .unwrap();

        assert_eq!(
            paths,
            vec![
                vec!["Request"],
                vec!["stream", "Chunk"],
                vec!["Result"],
                vec!["Response"],
                vec!["error", "AppError"]
            ]
        );
    }
}
