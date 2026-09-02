use super::*;

use crate::semantic_model::ExpectedTypeSource;
use crate::type_lowering::core_type;
use crate::type_relations::is_assignable;
use crate::type_syntax::{parse_type_annotation, parse_type_or_unknown};
use veln_core::CoreType;

#[test]
fn parses_empty_tuple_spelling_as_unit_type() {
    assert_eq!(parse_type_annotation("()"), Ok(Type::unit()));
    assert_eq!(
        parse_type_annotation("Result<(), AppError>"),
        Ok(Type::result(
            Type::unit(),
            Type::named("AppError", Vec::new())
        ))
    );
}

#[test]
fn renders_unit_type_with_empty_tuple_spelling() {
    assert_eq!(Type::unit().render(), "()");
    assert_eq!(
        Type::result(Type::unit(), Type::named("AppError", Vec::new())).render(),
        "Result<(), AppError>"
    );
}

#[test]
fn keeps_unit_name_as_compatibility_alias() {
    assert_eq!(parse_type_annotation("Unit"), Ok(Type::unit()));
}

#[test]
fn renders_record_and_function_types() {
    let record = Type::Record(vec![
        ("name".to_string(), Type::string()),
        ("scores".to_string(), Type::vec(Type::int())),
    ]);
    let pure_function = Type::Function {
        params: vec![Type::int(), Type::float()],
        variadic: None,
        return_type: Box::new(Type::bool()),
        effects: Vec::new(),
    };
    let effectful_function = Type::Function {
        params: vec![record.clone()],
        variadic: None,
        return_type: Box::new(Type::result(
            Type::unit(),
            Type::named("AppError", Vec::new()),
        )),
        effects: vec!["stdio".to_string(), "net".to_string()],
    };

    assert_eq!(record.render(), "{name: String, scores: Vec<Int>}");
    assert_eq!(pure_function.render(), "fn(Int, Float) -> Bool");
    assert_eq!(
        effectful_function.render(),
        "fn({name: String, scores: Vec<Int>}) -> Result<(), AppError> effects [stdio, net]"
    );
}

#[test]
fn exposes_type_parts_and_core_type_shape() {
    let function = Type::Function {
        params: vec![Type::vec(Type::int())],
        variadic: None,
        return_type: Box::new(Type::Record(vec![("ok".to_string(), Type::bool())])),
        effects: vec!["stdio".to_string()],
    };

    let (params, return_type) = function
        .function_parts()
        .expect("function type should expose parts");
    assert_eq!(params, &[Type::vec(Type::int())]);
    assert_eq!(
        return_type,
        &Type::Record(vec![("ok".to_string(), Type::bool())])
    );
    assert!(Type::string().function_parts().is_none());
    assert_eq!(
        core_type(&function),
        CoreType::Function {
            params: vec![CoreType::vec(CoreType::int())],
            variadic: None,
            return_type: Box::new(CoreType::Record(vec![("ok".to_string(), CoreType::bool())])),
            effects: vec!["stdio".to_string()],
        }
    );
}

#[test]
fn assignability_allows_unknowns_record_width_and_function_shapes() {
    let expected_record = Type::Record(vec![
        ("name".to_string(), Type::string()),
        (
            "meta".to_string(),
            Type::Record(vec![("count".to_string(), Type::int())]),
        ),
    ]);
    let actual_record = Type::Record(vec![
        ("name".to_string(), Type::string()),
        ("extra".to_string(), Type::bool()),
        (
            "meta".to_string(),
            Type::Record(vec![("count".to_string(), Type::int())]),
        ),
    ]);
    let wrong_record = Type::Record(vec![("name".to_string(), Type::int())]);
    let expected_pure_function = Type::Function {
        params: vec![Type::int()],
        variadic: None,
        return_type: Box::new(Type::bool()),
        effects: Vec::new(),
    };
    let actual_effectful_function = Type::Function {
        params: vec![Type::int()],
        variadic: None,
        return_type: Box::new(Type::bool()),
        effects: vec!["stdio".to_string()],
    };
    let expected_effectful_function = Type::Function {
        params: vec![Type::int()],
        variadic: None,
        return_type: Box::new(Type::bool()),
        effects: vec!["stdio".to_string()],
    };
    let actual_pure_function = Type::Function {
        params: vec![Type::int()],
        variadic: None,
        return_type: Box::new(Type::bool()),
        effects: Vec::new(),
    };
    let wrong_function = Type::Function {
        params: vec![Type::int(), Type::int()],
        variadic: None,
        return_type: Box::new(Type::bool()),
        effects: Vec::new(),
    };

    assert!(is_assignable(&Type::Unknown, &Type::string()));
    assert!(is_assignable(&Type::string(), &Type::Unknown));
    assert!(is_assignable(&expected_record, &actual_record));
    assert!(!is_assignable(&expected_record, &wrong_record));
    assert!(!is_assignable(
        &Type::named("Path", Vec::new()),
        &Type::string()
    ));
    assert!(!is_assignable(
        &Type::string(),
        &Type::named("Path", Vec::new())
    ));
    assert!(is_assignable(
        &expected_effectful_function,
        &actual_pure_function
    ));
    assert!(!is_assignable(
        &expected_pure_function,
        &actual_effectful_function
    ));
    assert!(!is_assignable(&expected_pure_function, &wrong_function));
    assert!(!is_assignable(&Type::int(), &Type::float()));
}

#[test]
fn parses_nested_type_annotations_with_whitespace() {
    assert_eq!(
        parse_type_annotation(
            " fn ( Vec< Int > , platform::Request ) -> Result < Dict < String , Int > , AppError > effects [ stdio , net ] "
        ),
        Ok(Type::Function {
            params: vec![
                Type::vec(Type::int()),
                Type::named("platform::Request", Vec::new()),
            ],
            variadic: None,
            return_type: Box::new(Type::result(
                Type::dict(Type::string(), Type::int()),
                Type::named("AppError", Vec::new())
            )),
            effects: vec!["stdio".to_string(), "net".to_string()],
        })
    );
    assert_eq!(
        parse_type_annotation("{ name: String, scores: Vec<Int> }"),
        Ok(Type::Record(vec![
            ("name".to_string(), Type::string()),
            ("scores".to_string(), Type::vec(Type::int())),
        ]))
    );
    assert_eq!(
        parse_type_annotation("{ name: String, scores: Vec<Int>, }"),
        Ok(Type::Record(vec![
            ("name".to_string(), Type::string()),
            ("scores".to_string(), Type::vec(Type::int())),
        ]))
    );
}

#[test]
fn parses_angle_bracket_type_annotations() {
    assert_eq!(
        parse_type_annotation(
            "fn(Vec<Int>, domain::Envelope<String, Result<(), AppError>>) -> Dict<String, Int>"
        ),
        Ok(Type::Function {
            params: vec![
                Type::vec(Type::int()),
                Type::named(
                    "domain::Envelope",
                    vec![
                        Type::string(),
                        Type::result(Type::unit(), Type::named("AppError", Vec::new())),
                    ],
                ),
            ],
            variadic: None,
            return_type: Box::new(Type::dict(Type::string(), Type::int())),
            effects: Vec::new(),
        })
    );
}

#[test]
fn parses_variadic_function_type_annotations() {
    assert_eq!(
        parse_type_annotation("fn(String, ...String) -> ()"),
        Ok(Type::Function {
            params: vec![Type::string()],
            variadic: Some(Box::new(Type::string())),
            return_type: Box::new(Type::unit()),
            effects: Vec::new(),
        })
    );
    assert_eq!(
        parse_type_annotation("fn(String, ...) -> List<String>"),
        Ok(Type::Function {
            params: vec![Type::string()],
            variadic: Some(Box::new(Type::Unknown)),
            return_type: Box::new(Type::named("List", vec![Type::string()])),
            effects: Vec::new(),
        })
    );
    assert_eq!(
        parse_type_annotation("fn(String, ...unknown) -> List<String>"),
        Ok(Type::Function {
            params: vec![Type::string()],
            variadic: Some(Box::new(Type::Unknown)),
            return_type: Box::new(Type::named("List", vec![Type::string()])),
            effects: Vec::new(),
        })
    );
    assert_eq!(
        parse_type_annotation("...String"),
        Err("expected type".to_string())
    );
    assert_eq!(
        parse_type_annotation("fn(...String, String) -> ()"),
        Err("variadic function type parameter must be the final parameter".to_string())
    );
}

#[test]
fn variadic_and_fixed_function_types_are_not_assignable() {
    let variadic = Type::Function {
        params: vec![Type::string()],
        variadic: Some(Box::new(Type::string())),
        return_type: Box::new(Type::unit()),
        effects: Vec::new(),
    };
    let fixed = Type::Function {
        params: vec![Type::string(), Type::string()],
        variadic: None,
        return_type: Box::new(Type::unit()),
        effects: Vec::new(),
    };

    assert!(!is_assignable(&variadic, &fixed));
    assert!(!is_assignable(&fixed, &variadic));
}

#[test]
fn parses_lowercase_schema_primitives_as_canonical_names() {
    let cases = [
        ("uint1", "UInt1"),
        ("uint8", "UInt8"),
        ("uint24be", "UInt24be"),
        ("uint31le", "UInt31le"),
    ];

    for (text, canonical) in cases {
        let primitive = lowercase_schema_primitive(text)
            .expect("lowercase spelling should be recognized")
            .expect("lowercase spelling should be accepted");
        assert_eq!(primitive.canonical_name(), canonical);
        assert_eq!(
            canonical_schema_primitive_name(text).as_deref(),
            Some(canonical)
        );
        assert_eq!(
            exact_width_schema_primitive_bit_width(text),
            exact_width_schema_primitive_bit_width(canonical)
        );
        assert_eq!(
            exact_width_schema_primitive_little_endian(text),
            exact_width_schema_primitive_little_endian(canonical)
        );
        assert_eq!(
            exact_width_schema_primitive_max_value(text),
            exact_width_schema_primitive_max_value(canonical)
        );
    }
}

#[test]
fn preserves_all_exact_width_uint_primitive_shapes() {
    let cases = [
        ("uint8", "UInt8", 1, 8, false, 0xff),
        ("uint16be", "UInt16be", 2, 16, false, 0xffff),
        ("uint16le", "UInt16le", 2, 16, true, 0xffff),
        ("uint24be", "UInt24be", 3, 24, false, 0xffffff),
        ("uint24le", "UInt24le", 3, 24, true, 0xffffff),
        ("uint32be", "UInt32be", 4, 32, false, 0xffffffff),
        ("uint32le", "UInt32le", 4, 32, true, 0xffffffff),
        ("uint40be", "UInt40be", 5, 40, false, 0xffffffffff),
        ("uint40le", "UInt40le", 5, 40, true, 0xffffffffff),
        ("uint48be", "UInt48be", 6, 48, false, 0xffffffffffff),
        ("uint48le", "UInt48le", 6, 48, true, 0xffffffffffff),
        ("uint56be", "UInt56be", 7, 56, false, 0xffffffffffffff),
        ("uint56le", "UInt56le", 7, 56, true, 0xffffffffffffff),
        ("uint64be", "UInt64be", 8, 64, false, i64::MAX),
        ("uint64le", "UInt64le", 8, 64, true, i64::MAX),
    ];

    for (text, canonical, width, bit_width, little_endian, max_value) in cases {
        assert_eq!(
            canonical_schema_primitive_name(text).as_deref(),
            Some(canonical),
            "{text}"
        );
        assert_eq!(exact_width_schema_primitive(text), Some(width), "{text}");
        assert_eq!(
            exact_width_schema_primitive_bit_width(text),
            Some(bit_width),
            "{text}"
        );
        assert_eq!(
            exact_width_schema_primitive_little_endian(text),
            little_endian,
            "{text}"
        );
        assert_eq!(
            exact_width_schema_primitive_max_value(text),
            Some(max_value),
            "{text}"
        );

        let repeated = repeat_schema_primitive(&format!("[{text}; count]"))
            .expect("replacement primitive should be repeatable");
        assert_eq!(repeated.count_field, "count", "{text}");
        assert_eq!(
            repeated.payload,
            SchemaRepeatPayload::Primitive {
                width,
                max_value,
                little_endian,
            },
            "{text}"
        );
        assert!(schema_repeat_payload_accepts_lowercase_primitive(text));

        let dispatch = closed_dispatch_schema_primitive(&format!("Dispatch(kind, 1 => {text})"))
            .expect("replacement primitive should be dispatchable");
        assert_eq!(dispatch.tag_field, "kind", "{text}");
        assert_eq!(dispatch.cases.len(), 1, "{text}");
        assert_eq!(
            dispatch.cases[0].payload,
            SchemaDispatchCasePayload::Primitive {
                width,
                little_endian,
            },
            "{text}"
        );
        assert!(schema_dispatch_payload_accepts_lowercase_primitive(text));
    }
}

#[test]
fn parses_lowercase_schema_reserves_as_reserved_bits() {
    let cases = [
        ("uint1 reserves 0", (1, 0)),
        ("uint8 reserves 255", (8, 255)),
        ("uint24be reserves 66051", (24, 66051)),
        ("uint64le reserves 42", (64, 42)),
    ];

    for (text, reserved) in cases {
        assert_eq!(
            lowercase_reserved_bits_schema_primitive(text),
            Some(Ok(reserved))
        );
        assert_eq!(reserved_bits_schema_primitive(text), Some(reserved));
        assert_eq!(canonical_schema_primitive_name(text), None);
    }
}

#[test]
fn accepts_bounded_subbyte_lowercase_reserved_dispatch_payloads() {
    for width in 1..=7 {
        assert!(schema_dispatch_payload_accepts_lowercase_primitive(
            &format!("uint{width} reserves 0")
        ));
    }
    assert!(schema_dispatch_payload_accepts_lowercase_primitive(
        "uint1 reserves 1"
    ));
    assert!(schema_dispatch_payload_accepts_lowercase_primitive(
        "uint2 reserves 3"
    ));
    assert!(schema_dispatch_payload_accepts_lowercase_primitive(
        "uint7 reserves 127"
    ));
    assert!(schema_dispatch_payload_accepts_lowercase_primitive(
        "uint16be reserves 0"
    ));
    assert!(!schema_dispatch_payload_accepts_lowercase_primitive(
        "uint1 reserves 2"
    ));
    assert!(!schema_dispatch_payload_accepts_lowercase_primitive(
        "uint7 reserves 128"
    ));
    assert!(!schema_dispatch_payload_accepts_lowercase_primitive(
        "uint8 reserves 256"
    ));
}

#[test]
fn rejects_malformed_lowercase_schema_primitives_with_focused_reasons() {
    let cases = [
        ("uint", LowercaseSchemaPrimitiveError::MissingWidth),
        ("uint16ne", LowercaseSchemaPrimitiveError::UnknownEndian),
        ("uint24", LowercaseSchemaPrimitiveError::MissingEndian),
        ("uint8be", LowercaseSchemaPrimitiveError::RedundantEndian),
        ("uint9", LowercaseSchemaPrimitiveError::UnsupportedWidth),
    ];

    for (text, reason) in cases {
        assert_eq!(lowercase_schema_primitive(text), Some(Err(reason)));
        assert_eq!(canonical_schema_primitive_name(text), None);
    }
    assert_eq!(lowercase_schema_primitive("uint_value"), None);
}

#[test]
fn rejects_malformed_lowercase_schema_reserves_with_focused_reasons() {
    let cases = [
        (
            "uint8 reserves",
            LowercaseSchemaPrimitiveError::ReservesValue,
        ),
        (
            "uint8 reserves value",
            LowercaseSchemaPrimitiveError::ReservesValue,
        ),
        (
            "uint8 reserves -1",
            LowercaseSchemaPrimitiveError::ReservesValue,
        ),
        (
            "uint24 reserves 0",
            LowercaseSchemaPrimitiveError::MissingEndian,
        ),
    ];

    for (text, reason) in cases {
        assert_eq!(
            lowercase_reserved_bits_schema_primitive(text),
            Some(Err(reason))
        );
        assert_eq!(reserved_bits_schema_primitive(text), None);
    }
    assert_eq!(lowercase_reserved_bits_schema_primitive("uint8"), None);
    assert_eq!(
        lowercase_reserved_bits_schema_primitive("count reserves 0"),
        None
    );
}

#[test]
fn rejects_malformed_type_annotations_with_specific_errors() {
    let cases = [
        ("", "expected type"),
        ("(Int)", "expected `)` for unit type `()`"),
        ("Int trailing", "unexpected `trailing`"),
        ("{ : Int }", "expected record field name"),
        ("{ name: String,, }", "expected record field name"),
        (
            "{ value: Int, value: String }",
            "duplicate record field `value`",
        ),
        ("{ value Int }", "expected `:`"),
        ("fn(Int) Int", "expected `->` in function type"),
        ("fn(Int -> Int", "expected `)`"),
        ("fn() -> () effects [,]", "expected effect name"),
        ("fn() -> () effects [stdio", "expected `]`"),
        ("Result(Int, String)", "unexpected `(Int, String)`"),
        ("Vec", "`Vec` expects 1 type argument(s), found 0"),
        ("Dict<String>", "`Dict` expects 2 type argument(s), found 1"),
        ("std::", "expected type"),
    ];

    for (text, message) in cases {
        assert_eq!(parse_type_annotation(text), Err(message.to_string()));
    }
    assert_eq!(parse_type_or_unknown(Some("Vec")), Type::Unknown);
    assert_eq!(parse_type_or_unknown(None), Type::Unknown);
}

#[test]
fn expected_type_sources_render_for_diagnostics_and_holes() {
    let cases = [
        (
            ExpectedTypeSource::DeclaredReturn,
            "declared_return",
            "declared",
        ),
        (
            ExpectedTypeSource::DeclaredParameter,
            "declared_parameter",
            "declared",
        ),
        (
            ExpectedTypeSource::LocalAnnotation,
            "local_annotation",
            "declared",
        ),
        (
            ExpectedTypeSource::Inferred,
            "inferred_expression",
            "inferred",
        ),
        (ExpectedTypeSource::Unknown, "unknown", "unknown"),
    ];

    for (source, type_source, hole_source) in cases {
        assert_eq!(source.as_type_source(), type_source);
        assert_eq!(source.as_hole_source(), hole_source);
    }
}

#[test]
fn schema_literal_positions_accept_binary_and_hexadecimal_values() {
    assert_eq!(
        reserved_bits_schema_primitive("ReservedBits(0b1000, 0xFF)"),
        Some((8, 255))
    );
    assert_eq!(
        byte_view_multiple_constraint("payload_count multiple of 0b100"),
        Some(ByteViewMultipleConstraint::Literal(4))
    );
    let dispatch =
        closed_dispatch_schema_primitive("Dispatch(kind, 0x0A => UInt8, 0b1011 => UInt16be)")
            .expect("prefixed dispatch tags should be accepted");
    assert_eq!(
        dispatch
            .cases
            .iter()
            .map(|case| case.tag)
            .collect::<Vec<_>>(),
        vec![10, 11]
    );
}

#[test]
fn schema_grammar_composes_nested_repeat_and_dispatch_payloads() {
    let repeat = repeat_schema_primitive("[ByteView(row_size + padding); row_count]")
        .expect("nested repeat payload should parse");
    assert_eq!(repeat.count_field, "row_count");
    assert_eq!(
        repeat.payload,
        SchemaRepeatPayload::ByteView {
            length_field: "row_size + padding".to_string(),
        }
    );

    let dispatch = extension_dispatch_schema_primitive(
        "ExtensionDispatch(kind, payload_length, 1 => uint16le, 2 => wire::Packet)",
    )
    .expect("nested dispatch payloads should parse");
    assert_eq!(dispatch.tag_field, "kind");
    assert_eq!(dispatch.length_field.as_deref(), Some("payload_length"));
    assert!(dispatch.preserves_unknown);
    assert_eq!(dispatch.cases.len(), 2);
    assert_eq!(
        dispatch.cases[0].payload,
        SchemaDispatchCasePayload::Primitive {
            width: 2,
            little_endian: true,
        }
    );
    assert_eq!(
        dispatch.cases[1].payload,
        SchemaDispatchCasePayload::Schema {
            schema_name: "wire::Packet".to_string(),
        }
    );
}

#[test]
fn reserved_bits_encode_supports_each_neighbor_direction() {
    let source = veln_source::SourceFile::new(
        "main.veln",
        concat!(
            "schema NeighborDirections\n",
            "  format binary\n",
            "  forward: ReservedBits(3, 5)\n",
            "  forward_value: UInt8\n",
            "  backward_value: UInt8\n",
            "  backward: ReservedBits(9, 0)\n",
            "  middle_high: UInt3\n",
            "  middle: ReservedBits(2, 1)\n",
            "  middle_low: UInt3\n",
            "  standalone: ReservedBits(8, 255)\n",
            "end\n",
        ),
    );
    let parsed = veln_syntax::parse(&source);
    let module = veln_ast::lower_surface_ast(&parsed.tree);
    let fields = &module.schemas[0].fields;

    for (index, reserved) in [(0, (3, 5)), (3, (9, 0)), (5, (2, 1)), (7, (8, 255))] {
        assert_eq!(
            supported_encode_reserved_bits(fields, index, reserved),
            Some((reserved.0 as u8, reserved.1)),
            "reserved field at index {index}"
        );
    }
}
