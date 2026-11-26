use crate::builtin_type_syntax::BuiltinTypeSyntaxRegistry;
use crate::semantic_model::Type;
use crate::source_less_names::InvalidStandardSymbolCase;
use crate::type_annotation_parser::{
    parse_type_annotation_with_arity, parse_type_annotation_with_arity_and_recovery_slots,
};

fn with_builtin_type_syntax_registry<R>(
    lookup: impl FnOnce(&BuiltinTypeSyntaxRegistry) -> R,
) -> Result<R, InvalidStandardSymbolCase> {
    crate::source_less_lookup::with_builtin_type_syntax_registry(lookup)
}

pub(crate) fn builtin_type_syntax_arity(name: &str) -> Option<usize> {
    with_builtin_type_syntax_registry(|registry| registry.arity(name))
        .ok()
        .flatten()
}

pub(crate) fn compiler_known_type_annotation_arity(name: &str) -> Option<usize> {
    builtin_type_syntax_arity(name).or_else(|| match name {
        "Byte"
        | "ByteChunk"
        | "ByteView"
        | "ByteOffset"
        | "ByteCount"
        | "StreamInput"
        | "AcceptOutcome"
        | "StreamReadOutcome"
        | "StreamWriteOutcome"
        | "DecodeReadiness"
        | "DecodeError"
        | "EncodeError"
        | "WallTime"
        | "CancelToken"
        | "SendError"
        | "SelectError"
        | "JoinError"
        | "Deadline"
        | "SourceLocation"
        | "SchemaDispatchPayload"
        | "NetStream"
        | "Path"
        | "FsError"
        | "ProcessError"
        | "CancelOwner"
        | "CancellableWaitOutcome"
        | "IoOwner"
        | "IoReader"
        | "IoWriter" => Some(0),
        "DecodeStep" | "EncodeStep" | "Sender" | "Receiver" | "Task" => Some(1),
        _ => None,
    })
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
            .map_err(|failure| failure.diagnostic().message.to_owned_string())
    })
}

pub(crate) fn parse_type_annotation_with_recovery_slots(
    text: &str,
    recovery_slots: &[std::ops::Range<usize>],
) -> Result<Type, String> {
    parse_type_annotation_with_arity_and_recovery_slots(text, recovery_slots, &|name| {
        with_builtin_type_syntax_registry(|registry| registry.arity(name))
            .map_err(|failure| failure.diagnostic().message.to_owned_string())
    })
}

pub(crate) fn parse_type_annotation_structure(text: &str) -> Result<Type, String> {
    parse_type_annotation_with_arity(text, &|_| Ok(None))
}

fn collect_type_reference_paths(ty: &Type, paths: &mut Vec<Vec<String>>) {
    match ty {
        Type::Named { name, args, .. } => {
            paths.push(name.split("::").map(str::to_string).collect());
            for arg in args {
                collect_type_reference_paths(arg, paths);
            }
        }
        Type::VariantRefinement { name, args, .. } => {
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
    fn unicode_whitespace_in_type_annotations_preserves_character_boundaries() {
        for (annotation, expected) in [
            ("\u{2003}Int", "Int"),
            ("Int\u{00a0}", "Int"),
            ("Option<\u{3000}Int>", "Option<Int>"),
            ("(\u{2003})", "()"),
        ] {
            assert_eq!(
                parse_type_annotation(annotation).unwrap().render(),
                expected
            );
        }
    }

    #[test]
    fn wide_record_types_preserve_written_field_order() {
        for count in [32, 64, 65, 128] {
            let names = (0..count).map(|i| format!("field_{i}")).collect::<Vec<_>>();
            let annotation = format!(
                "{{{}}}",
                names
                    .iter()
                    .map(|name| format!("{name}: Int"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            let Type::Record(fields) = parse_type_annotation(&annotation).unwrap() else {
                panic!("record annotation must retain its fields");
            };
            assert_eq!(
                fields.into_iter().map(|(name, _)| name).collect::<Vec<_>>(),
                names
            );
        }
    }

    #[test]
    fn record_types_report_the_first_repeated_field_at_small_and_large_widths() {
        for count in [2, 16, 32, 33, 64, 65, 128] {
            let fields = (0..count)
                .map(|i| format!("field_{i}: Int"))
                .collect::<Vec<_>>()
                .join(", ");
            let annotation = format!("{{{fields}, field_0: String, field_1: Bool}}");
            assert_eq!(
                parse_type_annotation(&annotation).unwrap_err(),
                "duplicate record field `field_0`",
                "{count}"
            );
        }
    }

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
