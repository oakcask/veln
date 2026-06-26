use veln_ast::{SchemaDecl, SchemaMappingAssignment, SchemaMappingClause, SchemaMappingSelector};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::diagnostics::span_json;
use crate::schema::mapping::{
    SchemaMappingConverterInput, SchemaMappingExprError, schema_mapping_expr_render,
};
use crate::types::Type;

pub(crate) fn schema_mapping_expr_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    assignment: &SchemaMappingAssignment,
    error: SchemaMappingExprError,
) -> Diagnostic {
    match error {
        SchemaMappingExprError::UnknownSchemaField { name, span } => {
            schema_mapping_source_diagnostic(schema, assignment, &name, span)
        }
        SchemaMappingExprError::Unsupported { text, span } => Diagnostic::new(
            "schema.mapping_expression_unsupported",
            Severity::Error,
            DiagnosticKind::Type,
            format!("schema mapping expression `{text}` is not supported"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unsupported_expression")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("expression", JsonValue::string(text)),
                ],
            ),
        ),
        SchemaMappingExprError::UnresolvedConstructor { name, span } => Diagnostic::new(
            "schema.mapping_constructor",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping constructor `{name}` is not resolved"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unresolved_constructor")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("constructor", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::UnresolvedConverter { name, span } => Diagnostic::new(
            "schema.mapping_converter",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping converter `{name}` is not resolved"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unresolved_converter")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("converter", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::PrivateConverter {
            name,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_visibility",
                Severity::Error,
                DiagnosticKind::Name,
                format!("schema mapping converter `{name}` is private"),
                Some(span),
                schema_mapping_assignment_details(
                    assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    assignment,
                    [
                        ("reason", JsonValue::string("private_converter")),
                        (
                            "mapping_target",
                            JsonValue::string(mapping.target.clone().unwrap_or_default()),
                        ),
                        ("converter", JsonValue::string(name)),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConstructorArity {
            name,
            expected,
            actual,
            span,
        } => Diagnostic::new(
            "schema.mapping_constructor_arity",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "schema mapping constructor `{name}` expects {expected} argument(s), but got {actual}"
            ),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("constructor_arity_mismatch")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("constructor", JsonValue::string(name)),
                    (
                        "expected_argument_count",
                        JsonValue::Number(expected as i64),
                    ),
                    ("actual_argument_count", JsonValue::Number(actual as i64)),
                ],
            ),
        ),
        SchemaMappingExprError::ConverterArity {
            name,
            expected,
            actual,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_arity",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "schema mapping converter `{name}` expects {expected} argument(s), but got {actual}"
                ),
                Some(span),
                schema_mapping_assignment_details(
                    assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    assignment,
                    [
                        ("reason", JsonValue::string("converter_arity_mismatch")),
                        (
                            "mapping_target",
                            JsonValue::string(mapping.target.clone().unwrap_or_default()),
                        ),
                        ("converter", JsonValue::string(name)),
                        (
                            "expected_argument_count",
                            JsonValue::Number(expected as i64),
                        ),
                        ("actual_argument_count", JsonValue::Number(actual as i64)),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConverterInputType {
            name,
            expected,
            actual,
            input,
            span,
            function_span,
        } => {
            let message = match &input {
                SchemaMappingConverterInput::SourceField(source) => format!(
                    "schema mapping converter `{name}` expects `{}`, but source field `{source}` decodes as `{}`",
                    expected.render(),
                    actual.render()
                ),
                SchemaMappingConverterInput::Expression(text) => format!(
                    "schema mapping converter `{name}` expects `{}`, but argument expression `{text}` has type `{}`",
                    expected.render(),
                    actual.render()
                ),
            };
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_input",
                Severity::Error,
                DiagnosticKind::Type,
                message,
                Some(span),
                schema_mapping_converter_details(SchemaMappingConverterDetails {
                    node_id: assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    mapping,
                    assignment,
                    reason: "converter_input_type_mismatch",
                    converter: &name,
                    input: &input,
                    expected: &expected,
                    actual: &actual,
                }),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConverterReturnType {
            name,
            expected,
            actual,
            input,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_return",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "schema mapping converter `{name}` returns `{}`, but target field `{}` expects `{}`",
                    actual.render(),
                    assignment.target,
                    expected.render()
                ),
                Some(span),
                schema_mapping_converter_details(SchemaMappingConverterDetails {
                    node_id: assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    mapping,
                    assignment,
                    reason: "converter_return_type_mismatch",
                    converter: &name,
                    input: &input,
                    expected: &expected,
                    actual: &actual,
                }),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ImpureConverter {
            name,
            effects,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_purity",
                Severity::Error,
                DiagnosticKind::Effect,
                format!("schema mapping converter `{name}` must be pure"),
                Some(span),
                schema_mapping_assignment_details(
                    assignment.node_id.display("schema-mapping-assignment"),
                    schema,
                    assignment,
                    [
                        ("reason", JsonValue::string("impure_converter")),
                        (
                            "mapping_target",
                            JsonValue::string(mapping.target.clone().unwrap_or_default()),
                        ),
                        ("converter", JsonValue::string(name)),
                        (
                            "effects",
                            JsonValue::array(effects.iter().cloned().map(JsonValue::string)),
                        ),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::RecordField { name, span } => Diagnostic::new(
            "schema.mapping_record_field",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping record field `{name}` is not expected"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("unexpected_record_field")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("record_field", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::MissingRecordField { name, span } => Diagnostic::new(
            "schema.mapping_record_field",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping record expression does not assign field `{name}`"),
            Some(span),
            schema_mapping_assignment_details(
                assignment.node_id.display("schema-mapping-assignment"),
                schema,
                assignment,
                [
                    ("reason", JsonValue::string("missing_record_field")),
                    (
                        "mapping_target",
                        JsonValue::string(mapping.target.clone().unwrap_or_default()),
                    ),
                    ("record_field", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::TypeMismatch {
            expected,
            actual,
            text,
            span,
        } => schema_mapping_type_diagnostic(
            schema, mapping, assignment, &expected, &actual, text, span,
        ),
    }
}

pub(crate) fn schema_mapping_selector_expr_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    selector: &SchemaMappingSelector,
    error: SchemaMappingExprError,
) -> Diagnostic {
    match error {
        SchemaMappingExprError::Unsupported { text, span } => Diagnostic::new(
            "schema.mapping_selection_unsupported",
            Severity::Error,
            DiagnosticKind::Type,
            format!("schema mapping selector expression `{text}` is not supported"),
            Some(span),
            schema_mapping_selector_details(
                selector.node_id.display("schema-mapping-selector"),
                schema,
                mapping,
                selector,
                [
                    (
                        "reason",
                        JsonValue::string("unsupported_selector_expression"),
                    ),
                    ("expression", JsonValue::string(text)),
                ],
            ),
        ),
        SchemaMappingExprError::UnknownSchemaField { name, span } => Diagnostic::new(
            "schema.mapping_selection",
            Severity::Error,
            DiagnosticKind::Type,
            format!("schema mapping selector field `{name}` is not declared"),
            Some(span),
            schema_mapping_selector_details(
                selector.node_id.display("schema-mapping-selector"),
                schema,
                mapping,
                selector,
                [
                    ("reason", JsonValue::string("unknown_selector_field")),
                    ("selector_field", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::UnresolvedConverter { name, span } => Diagnostic::new(
            "schema.mapping_converter",
            Severity::Error,
            DiagnosticKind::Name,
            format!("schema mapping converter `{name}` is not resolved"),
            Some(span),
            schema_mapping_selector_details(
                selector.node_id.display("schema-mapping-selector"),
                schema,
                mapping,
                selector,
                [
                    ("reason", JsonValue::string("unresolved_converter")),
                    ("converter", JsonValue::string(name)),
                ],
            ),
        ),
        SchemaMappingExprError::PrivateConverter {
            name,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_visibility",
                Severity::Error,
                DiagnosticKind::Name,
                format!("schema mapping converter `{name}` is private"),
                Some(span),
                schema_mapping_selector_details(
                    selector.node_id.display("schema-mapping-selector"),
                    schema,
                    mapping,
                    selector,
                    [
                        ("reason", JsonValue::string("private_converter")),
                        ("converter", JsonValue::string(name)),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConverterInputType {
            name,
            expected,
            actual,
            input,
            span,
            function_span,
        } => {
            let message = match &input {
                SchemaMappingConverterInput::SourceField(source) => format!(
                    "schema mapping converter `{name}` expects `{}`, but source field `{source}` decodes as `{}`",
                    expected.render(),
                    actual.render()
                ),
                SchemaMappingConverterInput::Expression(text) => format!(
                    "schema mapping converter `{name}` expects `{}`, but argument expression `{text}` has type `{}`",
                    expected.render(),
                    actual.render()
                ),
            };
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_input",
                Severity::Error,
                DiagnosticKind::Type,
                message,
                Some(span),
                schema_mapping_selector_converter_details(SchemaMappingSelectorConverterDetails {
                    node_id: selector.node_id.display("schema-mapping-selector"),
                    schema,
                    mapping,
                    selector,
                    reason: "converter_input_type_mismatch",
                    converter: &name,
                    input: &input,
                    expected: &expected,
                    actual: &actual,
                }),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ConverterReturnType {
            name,
            expected,
            actual,
            input,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_return",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "schema mapping converter `{name}` returns `{}`, but selector expects `{}`",
                    actual.render(),
                    expected.render()
                ),
                Some(span),
                schema_mapping_selector_converter_details(SchemaMappingSelectorConverterDetails {
                    node_id: selector.node_id.display("schema-mapping-selector"),
                    schema,
                    mapping,
                    selector,
                    reason: "converter_return_type_mismatch",
                    converter: &name,
                    input: &input,
                    expected: &expected,
                    actual: &actual,
                }),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::ImpureConverter {
            name,
            effects,
            span,
            function_span,
        } => {
            let mut diagnostic = Diagnostic::new(
                "schema.mapping_converter_purity",
                Severity::Error,
                DiagnosticKind::Effect,
                format!("schema mapping converter `{name}` must be pure"),
                Some(span),
                schema_mapping_selector_details(
                    selector.node_id.display("schema-mapping-selector"),
                    schema,
                    mapping,
                    selector,
                    [
                        ("reason", JsonValue::string("impure_converter")),
                        ("converter", JsonValue::string(name)),
                        (
                            "effects",
                            JsonValue::array(effects.iter().cloned().map(JsonValue::string)),
                        ),
                    ],
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("span", span_json(&function_span)),
                (
                    "message",
                    JsonValue::string("Converter declaration is here."),
                ),
            ]));
            diagnostic
        }
        SchemaMappingExprError::TypeMismatch {
            expected,
            actual,
            text,
            span,
        } => Diagnostic::new(
            "schema.mapping_selection_unsupported",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "schema mapping selector expression `{}` has type `{}`, but selector expects `{}`",
                text,
                actual.render(),
                expected.render()
            ),
            Some(span),
            schema_mapping_selector_details(
                selector.node_id.display("schema-mapping-selector"),
                schema,
                mapping,
                selector,
                [
                    ("reason", JsonValue::string("selector_type_mismatch")),
                    ("expression", JsonValue::string(text)),
                    ("expected", JsonValue::string(expected.render())),
                    ("actual", JsonValue::string(actual.render())),
                ],
            ),
        ),
        other => Diagnostic::new(
            "schema.mapping_selection_unsupported",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "schema mapping selector expression `{}` is not supported",
                schema_mapping_expr_render(&selector.expr)
            ),
            Some(selector.span.clone()),
            schema_mapping_selector_details(
                selector.node_id.display("schema-mapping-selector"),
                schema,
                mapping,
                selector,
                [(
                    "reason",
                    JsonValue::string(format!("unsupported_{other:?}")),
                )],
            ),
        ),
    }
}

fn schema_mapping_source_diagnostic(
    schema: &SchemaDecl,
    assignment: &SchemaMappingAssignment,
    source: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::new(
        "schema.mapping_source_field",
        Severity::Error,
        DiagnosticKind::Name,
        format!("schema mapping source field `{source}` is not declared"),
        Some(span),
        schema_mapping_assignment_details(
            assignment.node_id.display("schema-mapping-assignment"),
            schema,
            assignment,
            [
                ("reason", JsonValue::string("unknown_source_field")),
                (
                    "missing_source_field",
                    JsonValue::string(source.to_string()),
                ),
            ],
        ),
    )
}

fn schema_mapping_type_diagnostic(
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    assignment: &SchemaMappingAssignment,
    target_ty: &Type,
    source_ty: &Type,
    source_text: String,
    span: SourceSpan,
) -> Diagnostic {
    let message = if assignment.source == source_text {
        format!(
            "schema mapping target field `{}` expects `{}`, but source field `{}` decodes as `{}`",
            assignment.target,
            target_ty.render(),
            source_text,
            source_ty.render()
        )
    } else {
        format!(
            "schema mapping target field `{}` expects `{}`, but expression `{}` has type `{}`",
            assignment.target,
            target_ty.render(),
            source_text,
            source_ty.render()
        )
    };
    Diagnostic::new(
        "schema.mapping_type",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(span),
        schema_mapping_assignment_details(
            assignment.node_id.display("schema-mapping-assignment"),
            schema,
            assignment,
            [
                ("reason", JsonValue::string("field_type_mismatch")),
                (
                    "mapping_target",
                    JsonValue::string(mapping.target.clone().unwrap_or_default()),
                ),
                ("expected", JsonValue::string(target_ty.render())),
                ("actual", JsonValue::string(source_ty.render())),
                ("expression", JsonValue::string(source_text)),
            ],
        ),
    )
}

struct SchemaMappingConverterDetails<'a> {
    node_id: String,
    schema: &'a SchemaDecl,
    mapping: &'a SchemaMappingClause,
    assignment: &'a SchemaMappingAssignment,
    reason: &'static str,
    converter: &'a str,
    input: &'a SchemaMappingConverterInput,
    expected: &'a Type,
    actual: &'a Type,
}

fn schema_mapping_converter_details(details: SchemaMappingConverterDetails<'_>) -> JsonValue {
    let mut fields = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(details.node_id)),
        (
            "schema",
            JsonValue::string(details.schema.name.as_deref().unwrap_or("<missing>")),
        ),
        (
            "target_field",
            JsonValue::string(details.assignment.target.clone()),
        ),
        (
            "source_field",
            JsonValue::string(details.assignment.source.clone()),
        ),
        ("reason", JsonValue::string(details.reason)),
        (
            "mapping_target",
            JsonValue::string(details.mapping.target.clone().unwrap_or_default()),
        ),
        (
            "converter",
            JsonValue::string(details.converter.to_string()),
        ),
        ("expected", JsonValue::string(details.expected.render())),
        ("actual", JsonValue::string(details.actual.render())),
    ];
    match details.input {
        SchemaMappingConverterInput::SourceField(source) => {
            fields.push(("input_source_field", JsonValue::string(source.clone())));
        }
        SchemaMappingConverterInput::Expression(text) => {
            fields.push(("input_expression", JsonValue::string(text.clone())));
        }
    }
    JsonValue::object(fields)
}

struct SchemaMappingSelectorConverterDetails<'a> {
    node_id: String,
    schema: &'a SchemaDecl,
    mapping: &'a SchemaMappingClause,
    selector: &'a SchemaMappingSelector,
    reason: &'static str,
    converter: &'a str,
    input: &'a SchemaMappingConverterInput,
    expected: &'a Type,
    actual: &'a Type,
}

fn schema_mapping_selector_converter_details(
    details: SchemaMappingSelectorConverterDetails<'_>,
) -> JsonValue {
    let mut fields = schema_mapping_selector_detail_fields(
        details.node_id,
        details.schema,
        details.mapping,
        details.selector,
    );
    fields.extend([
        ("reason", JsonValue::string(details.reason)),
        (
            "converter",
            JsonValue::string(details.converter.to_string()),
        ),
        ("expected", JsonValue::string(details.expected.render())),
        ("actual", JsonValue::string(details.actual.render())),
    ]);
    match details.input {
        SchemaMappingConverterInput::SourceField(source) => {
            fields.push(("input_source_field", JsonValue::string(source.clone())));
        }
        SchemaMappingConverterInput::Expression(text) => {
            fields.push(("input_expression", JsonValue::string(text.clone())));
        }
    }
    JsonValue::object(fields)
}

fn schema_mapping_selector_details<const N: usize>(
    node_id: String,
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    selector: &SchemaMappingSelector,
    extra: [(&'static str, JsonValue); N],
) -> JsonValue {
    let mut fields = schema_mapping_selector_detail_fields(node_id, schema, mapping, selector);
    fields.extend(extra);
    JsonValue::object(fields)
}

fn schema_mapping_selector_detail_fields(
    node_id: String,
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
    selector: &SchemaMappingSelector,
) -> Vec<(&'static str, JsonValue)> {
    vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ),
        (
            "mapping_target",
            JsonValue::string(mapping.target.clone().unwrap_or_default()),
        ),
        (
            "selector_expression",
            JsonValue::string(selector.text.clone()),
        ),
    ]
}

fn schema_mapping_assignment_details<const N: usize>(
    node_id: String,
    schema: &SchemaDecl,
    assignment: &SchemaMappingAssignment,
    extra: [(&'static str, JsonValue); N],
) -> JsonValue {
    let mut fields = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ),
        ("target_field", JsonValue::string(assignment.target.clone())),
        ("source_field", JsonValue::string(assignment.source.clone())),
    ];
    fields.extend(extra);
    JsonValue::object(fields)
}
