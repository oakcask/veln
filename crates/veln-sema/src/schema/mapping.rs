use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{
    BinaryOp, Expr, ExprKind, PrefixOp, SchemaDecl, SchemaMappingClause, SurfaceModule, TypeDecl,
    TypeVariantDecl, Visibility,
};
use veln_source::SourceSpan;

use crate::adt::{self, AdtConstructor, AdtRegistry, ConstructorLookup};
use crate::types::{
    FunctionSignature, Type, closed_dispatch_schema_primitive, extension_dispatch_schema_primitive,
    imported_module_for_path, imported_use_for_path, infer_function_body_effects, is_assignable,
    ordinary_function_signatures, parse_type_or_unknown, reserved_bits_schema_primitive,
    schema_decode_record_fields, schema_dispatch_case_type, selected_mappings_cover_dispatch_cases,
    supported_encode_reserved_bits,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMappingField {
    pub(crate) target: String,
    pub(crate) source: String,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMapping {
    pub(crate) selector: Option<SchemaDecodeMappingSelector>,
    pub(crate) fields: Vec<SchemaDecodeMappingField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMappingSelector {
    pub(crate) text: String,
    pub(crate) predicate: Option<SchemaMappingSelectorPredicate>,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaMappingSelectorPredicate {
    Comparison {
        field: String,
        op: SchemaMappingSelectorComparison,
        value: i64,
    },
    And(Box<Self>, Box<Self>),
    Or(Box<Self>, Box<Self>),
    Not(Box<Self>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchemaMappingSelectorComparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaMappingSelectorError {
    Unsupported,
}

impl SchemaMappingSelectorPredicate {
    pub(crate) fn collect_fields(&self, fields: &mut BTreeSet<String>) {
        match self {
            Self::Comparison { field, .. } => {
                fields.insert(field.clone());
            }
            Self::And(left, right) | Self::Or(left, right) => {
                left.collect_fields(fields);
                right.collect_fields(fields);
            }
            Self::Not(expr) => expr.collect_fields(fields),
        }
    }

    pub(crate) fn as_simple_comparison(
        &self,
    ) -> Option<(&str, SchemaMappingSelectorComparison, i64)> {
        match self {
            Self::Comparison { field, op, value } => Some((field, *op, *value)),
            _ => None,
        }
    }

    fn collect_literals(&self, literals: &mut BTreeMap<String, BTreeSet<i64>>) {
        match self {
            Self::Comparison { field, value, .. } => {
                literals.entry(field.clone()).or_default().insert(*value);
            }
            Self::And(left, right) | Self::Or(left, right) => {
                left.collect_literals(literals);
                right.collect_literals(literals);
            }
            Self::Not(expr) => expr.collect_literals(literals),
        }
    }

    fn eval(&self, assignment: &BTreeMap<String, i64>) -> bool {
        match self {
            Self::Comparison { field, op, value } => {
                let candidate = assignment.get(field).copied().unwrap_or_default();
                match op {
                    SchemaMappingSelectorComparison::Equal => candidate == *value,
                    SchemaMappingSelectorComparison::NotEqual => candidate != *value,
                    SchemaMappingSelectorComparison::Less => candidate < *value,
                    SchemaMappingSelectorComparison::LessEqual => candidate <= *value,
                    SchemaMappingSelectorComparison::Greater => candidate > *value,
                    SchemaMappingSelectorComparison::GreaterEqual => candidate >= *value,
                }
            }
            Self::And(left, right) => left.eval(assignment) && right.eval(assignment),
            Self::Or(left, right) => left.eval(assignment) || right.eval(assignment),
            Self::Not(expr) => !expr.eval(assignment),
        }
    }
}

pub(crate) fn schema_mapping_selector_predicate(
    selector: &veln_ast::SchemaMappingSelector,
) -> Result<SchemaMappingSelectorPredicate, SchemaMappingSelectorError> {
    schema_mapping_selector_expr_predicate(&selector.expr)
}

pub(crate) fn schema_mapping_selectors_overlap(
    left: &SchemaMappingSelectorPredicate,
    right: &SchemaMappingSelectorPredicate,
) -> bool {
    let mut literals = BTreeMap::<String, BTreeSet<i64>>::new();
    left.collect_literals(&mut literals);
    right.collect_literals(&mut literals);
    let fields = literals
        .into_iter()
        .map(|(field, values)| {
            let mut candidates = values.into_iter().collect::<Vec<_>>();
            let other = schema_mapping_selector_other_value(&candidates);
            candidates.push(other);
            (field, candidates)
        })
        .collect::<Vec<_>>();
    schema_mapping_selectors_overlap_inner(left, right, &fields, 0, &mut BTreeMap::new())
}

fn schema_mapping_selectors_overlap_inner(
    left: &SchemaMappingSelectorPredicate,
    right: &SchemaMappingSelectorPredicate,
    fields: &[(String, Vec<i64>)],
    index: usize,
    assignment: &mut BTreeMap<String, i64>,
) -> bool {
    if index == fields.len() {
        return left.eval(assignment) && right.eval(assignment);
    }
    let (field, values) = &fields[index];
    for value in values {
        assignment.insert(field.clone(), *value);
        if schema_mapping_selectors_overlap_inner(left, right, fields, index + 1, assignment) {
            return true;
        }
    }
    assignment.remove(field);
    false
}

fn schema_mapping_selector_other_value(values: &[i64]) -> i64 {
    let mut candidate = 0;
    while values.contains(&candidate) {
        candidate += 1;
    }
    candidate
}

fn schema_mapping_selector_expr_predicate(
    expr: &Expr,
) -> Result<SchemaMappingSelectorPredicate, SchemaMappingSelectorError> {
    match &expr.kind {
        ExprKind::Binary { op, left, right } => match op {
            BinaryOp::And => Ok(SchemaMappingSelectorPredicate::And(
                Box::new(schema_mapping_selector_expr_predicate(left)?),
                Box::new(schema_mapping_selector_expr_predicate(right)?),
            )),
            BinaryOp::Or => Ok(SchemaMappingSelectorPredicate::Or(
                Box::new(schema_mapping_selector_expr_predicate(left)?),
                Box::new(schema_mapping_selector_expr_predicate(right)?),
            )),
            BinaryOp::Equal
            | BinaryOp::NotEqual
            | BinaryOp::Less
            | BinaryOp::LessEqual
            | BinaryOp::Greater
            | BinaryOp::GreaterEqual => {
                let Some((field, op, value)) =
                    schema_mapping_selector_comparison_operands(left, *op, right)
                else {
                    return Err(SchemaMappingSelectorError::Unsupported);
                };
                Ok(SchemaMappingSelectorPredicate::Comparison { field, op, value })
            }
            _ => Err(SchemaMappingSelectorError::Unsupported),
        },
        ExprKind::Prefix {
            op: PrefixOp::Not,
            expr,
        } => Ok(SchemaMappingSelectorPredicate::Not(Box::new(
            schema_mapping_selector_expr_predicate(expr)?,
        ))),
        _ => Err(SchemaMappingSelectorError::Unsupported),
    }
}

fn schema_mapping_selector_comparison_operands(
    left: &Expr,
    op: BinaryOp,
    right: &Expr,
) -> Option<(String, SchemaMappingSelectorComparison, i64)> {
    if let Some((field, value)) =
        schema_mapping_selector_field(left).zip(schema_mapping_selector_int_literal(right))
    {
        return Some((field, schema_mapping_selector_comparison(op)?, value));
    }
    let (value, field) =
        schema_mapping_selector_int_literal(left).zip(schema_mapping_selector_field(right))?;
    Some((
        field,
        schema_mapping_selector_comparison_inverse(op)?,
        value,
    ))
}

fn schema_mapping_selector_comparison(op: BinaryOp) -> Option<SchemaMappingSelectorComparison> {
    match op {
        BinaryOp::Equal => Some(SchemaMappingSelectorComparison::Equal),
        BinaryOp::NotEqual => Some(SchemaMappingSelectorComparison::NotEqual),
        BinaryOp::Less => Some(SchemaMappingSelectorComparison::Less),
        BinaryOp::LessEqual => Some(SchemaMappingSelectorComparison::LessEqual),
        BinaryOp::Greater => Some(SchemaMappingSelectorComparison::Greater),
        BinaryOp::GreaterEqual => Some(SchemaMappingSelectorComparison::GreaterEqual),
        _ => None,
    }
}

fn schema_mapping_selector_comparison_inverse(
    op: BinaryOp,
) -> Option<SchemaMappingSelectorComparison> {
    match op {
        BinaryOp::Equal => Some(SchemaMappingSelectorComparison::Equal),
        BinaryOp::NotEqual => Some(SchemaMappingSelectorComparison::NotEqual),
        BinaryOp::Less => Some(SchemaMappingSelectorComparison::Greater),
        BinaryOp::LessEqual => Some(SchemaMappingSelectorComparison::GreaterEqual),
        BinaryOp::Greater => Some(SchemaMappingSelectorComparison::Less),
        BinaryOp::GreaterEqual => Some(SchemaMappingSelectorComparison::LessEqual),
        _ => None,
    }
}

fn schema_mapping_selector_field(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::NamePath(segments) if segments.len() == 1 => Some(segments[0].clone()),
        _ => None,
    }
}

fn schema_mapping_selector_int_literal(expr: &Expr) -> Option<i64> {
    match &expr.kind {
        ExprKind::IntLiteral(value) => value.parse().ok(),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaDecodeMappingExpr {
    Field(String),
    Literal(i64),
    FieldAccess {
        base: Box<SchemaDecodeMappingExpr>,
        field: String,
    },
    Record(Vec<SchemaDecodeMappingRecordField>),
    Constructor {
        name: Vec<String>,
        args: Vec<SchemaDecodeMappingExpr>,
    },
    Converter {
        function: String,
        inverse_function: Option<String>,
        args: Vec<SchemaDecodeMappingConverterArg>,
    },
    Prefix {
        op: PrefixOp,
        expr: Box<SchemaDecodeMappingExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<SchemaDecodeMappingExpr>,
        right: Box<SchemaDecodeMappingExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMappingConverterArg {
    pub(crate) ty: Type,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMappingRecordField {
    pub(crate) name: String,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaMappingTypedExpr {
    pub(crate) ty: Type,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaMappingConverterInput {
    SourceField(String),
    Expression(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaMappingExprError {
    Unsupported {
        text: String,
        span: SourceSpan,
    },
    UnknownSchemaField {
        name: String,
        span: SourceSpan,
    },
    UnresolvedConstructor {
        name: String,
        span: SourceSpan,
    },
    UnresolvedConverter {
        name: String,
        span: SourceSpan,
    },
    PrivateConverter {
        name: String,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ConstructorArity {
        name: String,
        expected: usize,
        actual: usize,
        span: SourceSpan,
    },
    ConverterArity {
        name: String,
        expected: usize,
        actual: usize,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ConverterInputType {
        name: String,
        expected: Box<Type>,
        actual: Box<Type>,
        input: SchemaMappingConverterInput,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ConverterReturnType {
        name: String,
        expected: Box<Type>,
        actual: Box<Type>,
        input: SchemaMappingConverterInput,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ImpureConverter {
        name: String,
        effects: Vec<String>,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    RecordField {
        name: String,
        span: SourceSpan,
    },
    MissingRecordField {
        name: String,
        span: SourceSpan,
    },
    TypeMismatch {
        expected: Box<Type>,
        actual: Box<Type>,
        text: String,
        span: SourceSpan,
    },
}

type SchemaMappingExprResult = Result<SchemaMappingTypedExpr, Box<SchemaMappingExprError>>;

struct SchemaMappingExprContext<'a> {
    module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    registry: &'a AdtRegistry,
    converter_functions: &'a [FunctionSignature],
    schema_fields: &'a BTreeMap<String, Type>,
}

const SCHEMA_MAPPING_CONVERTER_MIN_ARITY: usize = 1;

pub(crate) struct SchemaMappingTyper<'a> {
    module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    registry: AdtRegistry,
    converter_functions: Vec<FunctionSignature>,
}

impl<'a> SchemaMappingTyper<'a> {
    pub(crate) fn new(module: &'a SurfaceModule, schema: &'a SchemaDecl) -> Self {
        let registry = AdtRegistry::from_module(module);
        let mut converter_functions = ordinary_function_signatures(module);
        infer_function_body_effects(module, &mut converter_functions);
        Self {
            module,
            schema,
            registry,
            converter_functions,
        }
    }

    fn context<'b>(
        &'b self,
        schema_fields: &'b BTreeMap<String, Type>,
    ) -> SchemaMappingExprContext<'b> {
        SchemaMappingExprContext {
            module: self.module,
            schema: self.schema,
            registry: &self.registry,
            converter_functions: &self.converter_functions,
            schema_fields,
        }
    }

    pub(crate) fn expr_typed(
        &self,
        schema_fields: &BTreeMap<String, Type>,
        expr: &Expr,
        expected: &Type,
    ) -> SchemaMappingExprResult {
        let context = self.context(schema_fields);
        let typed = schema_mapping_expr_typed_unchecked(&context, expr, expected, true)?;
        if !is_assignable(expected, &typed.ty) {
            return Err(Box::new(SchemaMappingExprError::TypeMismatch {
                expected: Box::new(expected.clone()),
                actual: Box::new(typed.ty),
                text: schema_mapping_expr_render(expr),
                span: expr.span.clone(),
            }));
        }
        Ok(typed)
    }

    pub(crate) fn converter_selector_expr_typed(
        &self,
        schema_fields: &BTreeMap<String, Type>,
        expr: &Expr,
    ) -> SchemaMappingExprResult {
        if !matches!(expr.kind, ExprKind::Call { .. }) {
            return Err(Box::new(SchemaMappingExprError::Unsupported {
                text: schema_mapping_expr_render(expr),
                span: expr.span.clone(),
            }));
        }
        self.expr_typed(schema_fields, expr, &Type::bool())
    }

    pub(crate) fn assignment_expr_typed(
        &self,
        schema_fields: &BTreeMap<String, Type>,
        assignment: &veln_ast::SchemaMappingAssignment,
        expected: &Type,
    ) -> SchemaMappingExprResult {
        self.assignment_expr_typed_inner(schema_fields, assignment, expected)
    }

    pub(crate) fn assignment_expr_typed_for_codegen(
        &self,
        schema_fields: &BTreeMap<String, Type>,
        assignment: &veln_ast::SchemaMappingAssignment,
        expected: &Type,
    ) -> SchemaMappingExprResult {
        self.assignment_expr_typed_inner(schema_fields, assignment, expected)
    }

    fn assignment_expr_typed_inner(
        &self,
        schema_fields: &BTreeMap<String, Type>,
        assignment: &veln_ast::SchemaMappingAssignment,
        expected: &Type,
    ) -> SchemaMappingExprResult {
        let context = self.context(schema_fields);
        let typed =
            schema_mapping_expr_typed_unchecked(&context, &assignment.expr, expected, true)?;
        if !is_assignable(expected, &typed.ty) {
            return Err(Box::new(SchemaMappingExprError::TypeMismatch {
                expected: Box::new(expected.clone()),
                actual: Box::new(typed.ty),
                text: schema_mapping_expr_render(&assignment.expr),
                span: assignment.expr.span.clone(),
            }));
        }
        schema_mapping_expr_with_assignment_inverse(&context, typed, assignment, expected)
    }
}

pub(crate) fn schema_decode_mapping_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<SchemaDecodeMappingField>> {
    let decoded_fields = schema_decode_record_fields(module, schema)?;
    schema_decode_mapping_fields_from_decoded_fields(module, schema, &decoded_fields)
}

pub(crate) fn schema_decode_mappings(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<SchemaDecodeMapping>> {
    let decoded_fields = schema_decode_record_fields(module, schema)?;
    schema_decode_mappings_from_decoded_fields(module, schema, &decoded_fields)
}

pub(crate) fn schema_decode_mapping_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
) -> Option<Vec<(String, Type)>> {
    let [first_mapping, rest @ ..] = schema.mappings.as_slice() else {
        return None;
    };
    let target_fields = schema_mapping_target_record_fields(module, schema, first_mapping)?;
    let source_field_types = decoded_fields
        .iter()
        .map(|(name, ty, _)| (name.clone(), ty.clone()))
        .collect::<BTreeMap<_, _>>();
    let typer = SchemaMappingTyper::new(module, schema);
    let mapping_source_field_types =
        schema_mapping_source_field_types(module, schema, &source_field_types, first_mapping)?;
    validate_schema_decode_mapping_fields(
        &typer,
        &mapping_source_field_types,
        first_mapping,
        &target_fields,
    )?;
    for mapping in rest {
        mapping.selector.as_ref()?;
        if schema_mapping_target_record_fields(module, schema, mapping)? != target_fields {
            return None;
        }
        let mapping_source_field_types =
            schema_mapping_source_field_types(module, schema, &source_field_types, mapping)?;
        validate_schema_decode_mapping_fields(
            &typer,
            &mapping_source_field_types,
            mapping,
            &target_fields,
        )?;
    }
    Some(target_fields)
}

fn schema_decode_mapping_fields_from_decoded_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
) -> Option<Vec<SchemaDecodeMappingField>> {
    let [mapping] = schema.mappings.as_slice() else {
        return None;
    };
    schema_decode_mapping_fields_for_mapping(module, schema, decoded_fields, mapping)
}

fn schema_decode_mappings_from_decoded_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
) -> Option<Vec<SchemaDecodeMapping>> {
    if schema.mappings.is_empty() {
        return None;
    }
    let typer = SchemaMappingTyper::new(module, schema);
    let source_field_types = decoded_fields
        .iter()
        .map(|(name, ty, _)| (name.clone(), ty.clone()))
        .collect::<BTreeMap<_, _>>();
    schema
        .mappings
        .iter()
        .map(|mapping| {
            let fields = schema_decode_mapping_fields_for_mapping_with_typer(
                &typer,
                module,
                schema,
                decoded_fields,
                mapping,
            )?;
            let selector = mapping_selector_for_codegen(&typer, &source_field_types, mapping)?;
            Some(SchemaDecodeMapping { selector, fields })
        })
        .collect()
}

fn mapping_selector_for_codegen(
    typer: &SchemaMappingTyper<'_>,
    source_field_types: &BTreeMap<String, Type>,
    mapping: &SchemaMappingClause,
) -> Option<Option<SchemaDecodeMappingSelector>> {
    let Some(selector) = &mapping.selector else {
        return Some(None);
    };
    if let Ok(predicate) = schema_mapping_selector_predicate(selector) {
        let typed = typer
            .expr_typed(source_field_types, &selector.expr, &Type::bool())
            .ok()?;
        return Some(Some(SchemaDecodeMappingSelector {
            text: selector.text.clone(),
            predicate: Some(predicate),
            expr: typed.expr,
        }));
    }
    let typed = typer
        .converter_selector_expr_typed(source_field_types, &selector.expr)
        .ok()?;
    Some(Some(SchemaDecodeMappingSelector {
        text: selector.text.clone(),
        predicate: None,
        expr: typed.expr,
    }))
}

fn schema_decode_mapping_fields_for_mapping(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
    mapping: &SchemaMappingClause,
) -> Option<Vec<SchemaDecodeMappingField>> {
    let typer = SchemaMappingTyper::new(module, schema);
    schema_decode_mapping_fields_for_mapping_with_typer(
        &typer,
        module,
        schema,
        decoded_fields,
        mapping,
    )
}

fn schema_decode_mapping_fields_for_mapping_with_typer(
    typer: &SchemaMappingTyper<'_>,
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
    mapping: &SchemaMappingClause,
) -> Option<Vec<SchemaDecodeMappingField>> {
    let target_fields = schema_mapping_target_record_fields(module, schema, mapping)?;
    let source_field_types = decoded_fields
        .iter()
        .map(|(name, ty, _)| (name.clone(), ty.clone()))
        .collect::<BTreeMap<_, _>>();
    let source_field_types =
        schema_mapping_source_field_types(module, schema, &source_field_types, mapping)?;
    let mut fields = Vec::new();
    for (target, target_ty) in target_fields {
        let assignment = mapping
            .assignments
            .iter()
            .find(|assignment| assignment.target == target)?;
        let typed = typer
            .assignment_expr_typed_for_codegen(&source_field_types, assignment, &target_ty)
            .ok()?;
        fields.push(SchemaDecodeMappingField {
            target,
            source: assignment.source.clone(),
            expr: typed.expr,
        });
    }
    Some(fields)
}

pub(crate) fn schema_mapping_source_field_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
    mapping: &SchemaMappingClause,
) -> Option<BTreeMap<String, Type>> {
    let mut fields = schema_fields.clone();
    for (index, field) in schema.fields.iter().enumerate() {
        let Some(reserved) = reserved_bits_schema_primitive(&field.ty) else {
            continue;
        };
        supported_encode_reserved_bits(&schema.fields, index, reserved)?;
        fields.insert(field.name.clone(), Type::int());
    }
    let Some(selector) = &mapping.selector else {
        return Some(fields);
    };
    for field in &schema.fields {
        let Some(dispatch) = closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
        else {
            continue;
        };
        let selector_case =
            schema_mapping_selector_predicate(selector)
                .ok()
                .and_then(|predicate| {
                    predicate
                        .as_simple_comparison()
                        .map(|(field, op, value)| (field.to_string(), op, value))
                });
        let Some((selector_field, SchemaMappingSelectorComparison::Equal, selector_value)) =
            selector_case
        else {
            continue;
        };
        if dispatch.tag_field != selector_field
            || !selected_mappings_cover_dispatch_cases(schema, &dispatch)
        {
            continue;
        }
        let case = dispatch
            .cases
            .iter()
            .find(|case| case.tag == selector_value)?;
        let ty = schema_dispatch_case_type(module, schema, case, &mut Vec::new())?;
        fields.insert(field.name.clone(), ty);
    }
    Some(fields)
}

fn validate_schema_decode_mapping_fields(
    typer: &SchemaMappingTyper<'_>,
    source_field_types: &BTreeMap<String, Type>,
    mapping: &SchemaMappingClause,
    target_fields: &[(String, Type)],
) -> Option<()> {
    for (target, target_ty) in target_fields {
        let assignment = mapping
            .assignments
            .iter()
            .find(|assignment| assignment.target == *target)?;
        typer
            .assignment_expr_typed_for_codegen(source_field_types, assignment, target_ty)
            .ok()?;
    }
    Some(())
}

pub(crate) fn schema_mapping_expr_typed(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
    expr: &Expr,
    expected: &Type,
) -> SchemaMappingExprResult {
    SchemaMappingTyper::new(module, schema).expr_typed(schema_fields, expr, expected)
}

pub(crate) fn schema_mapping_assignment_expr_typed(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
    assignment: &veln_ast::SchemaMappingAssignment,
    expected: &Type,
) -> SchemaMappingExprResult {
    SchemaMappingTyper::new(module, schema).assignment_expr_typed(
        schema_fields,
        assignment,
        expected,
    )
}

fn schema_mapping_expr_with_assignment_inverse(
    context: &SchemaMappingExprContext<'_>,
    typed: SchemaMappingTypedExpr,
    assignment: &veln_ast::SchemaMappingAssignment,
    expected: &Type,
) -> SchemaMappingExprResult {
    let Some(inverse) = &assignment.inverse_converter else {
        return Ok(typed);
    };
    let SchemaDecodeMappingExpr::Converter {
        function,
        inverse_function: _,
        mut args,
    } = typed.expr
    else {
        return Err(Box::new(SchemaMappingExprError::Unsupported {
            text: format!(
                "{} inverse {}",
                schema_mapping_expr_render(&assignment.expr),
                inverse.name
            ),
            span: inverse.span.clone(),
        }));
    };
    let [arg] = args.as_mut_slice() else {
        return Err(Box::new(SchemaMappingExprError::Unsupported {
            text: format!(
                "{} inverse {}",
                schema_mapping_expr_render(&assignment.expr),
                inverse.name
            ),
            span: inverse.span.clone(),
        }));
    };
    let inverse_function =
        schema_mapping_inverse_converter_function(context, inverse, expected, &arg.ty)?
            .target_name
            .clone();
    Ok(SchemaMappingTypedExpr {
        ty: typed.ty,
        expr: SchemaDecodeMappingExpr::Converter {
            function,
            inverse_function: Some(inverse_function),
            args,
        },
    })
}

fn schema_mapping_inverse_converter_function<'a>(
    context: &'a SchemaMappingExprContext<'_>,
    inverse: &veln_ast::SchemaMappingInverseConverter,
    target_ty: &Type,
    arg_ty: &Type,
) -> Result<&'a FunctionSignature, Box<SchemaMappingExprError>> {
    let segments = inverse
        .name
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    let function = match schema_mapping_converter_function(context, &segments) {
        SchemaMappingConverterLookup::Found(function) => function,
        SchemaMappingConverterLookup::Private(function) => {
            return Err(Box::new(SchemaMappingExprError::PrivateConverter {
                name: inverse.name.clone(),
                span: inverse.span.clone(),
                function_span: function.span.clone(),
            }));
        }
        SchemaMappingConverterLookup::Missing => {
            return Err(Box::new(SchemaMappingExprError::UnresolvedConverter {
                name: inverse.name.clone(),
                span: inverse.span.clone(),
            }));
        }
    };
    if function.params.len() != 1 {
        return Err(Box::new(SchemaMappingExprError::ConverterArity {
            name: function.name.clone(),
            expected: 1,
            actual: function.params.len(),
            span: inverse.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    if !function.effects.is_empty() {
        return Err(Box::new(SchemaMappingExprError::ImpureConverter {
            name: function.name.clone(),
            effects: function.effects.clone(),
            span: inverse.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    if !is_assignable(&function.params[0], target_ty) {
        return Err(Box::new(SchemaMappingExprError::ConverterInputType {
            name: function.name.clone(),
            expected: Box::new(function.params[0].clone()),
            actual: Box::new(target_ty.clone()),
            input: SchemaMappingConverterInput::Expression("mapped target value".to_string()),
            span: inverse.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    if !is_assignable(arg_ty, &function.return_type) {
        return Err(Box::new(SchemaMappingExprError::ConverterReturnType {
            name: function.name.clone(),
            expected: Box::new(arg_ty.clone()),
            actual: Box::new(function.return_type.clone()),
            input: SchemaMappingConverterInput::Expression("mapped target value".to_string()),
            span: inverse.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    Ok(function)
}

fn schema_mapping_expr_typed_unchecked(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    expected: &Type,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    match &expr.kind {
        ExprKind::NamePath(segments) => schema_mapping_name_expr(context, expr, segments, expected),
        ExprKind::FieldAccess { base, field, .. } => {
            let typed_base = schema_mapping_expr_inferred(context, base, allow_converter_calls)?;
            let Some(field_ty) = typed_base.ty.record_field(field) else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            Ok(SchemaMappingTypedExpr {
                ty: field_ty.clone(),
                expr: SchemaDecodeMappingExpr::FieldAccess {
                    base: Box::new(typed_base.expr),
                    field: field.clone(),
                },
            })
        }
        ExprKind::Record(fields) => {
            let Type::Record(expected_fields) = expected else {
                return Err(Box::new(SchemaMappingExprError::TypeMismatch {
                    expected: Box::new(expected.clone()),
                    actual: Box::new(schema_mapping_record_actual_type(context, fields)),
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            let mut seen = BTreeMap::<String, SourceSpan>::new();
            let mut record_fields = Vec::new();
            for field in fields {
                if seen
                    .insert(field.name.clone(), field.span.clone())
                    .is_some()
                {
                    return Err(Box::new(SchemaMappingExprError::RecordField {
                        name: field.name.clone(),
                        span: field.span.clone(),
                    }));
                }
                let Some((_, field_ty)) =
                    expected_fields.iter().find(|(name, _)| name == &field.name)
                else {
                    return Err(Box::new(SchemaMappingExprError::RecordField {
                        name: field.name.clone(),
                        span: field.span.clone(),
                    }));
                };
                let typed = schema_mapping_expr_typed_inner(
                    context,
                    &field.expr,
                    field_ty,
                    allow_converter_calls,
                )?;
                record_fields.push(SchemaDecodeMappingRecordField {
                    name: field.name.clone(),
                    expr: typed.expr,
                });
            }
            for (name, _) in expected_fields {
                if !seen.contains_key(name) {
                    return Err(Box::new(SchemaMappingExprError::MissingRecordField {
                        name: name.clone(),
                        span: expr.span.clone(),
                    }));
                }
            }
            Ok(SchemaMappingTypedExpr {
                ty: expected.clone(),
                expr: SchemaDecodeMappingExpr::Record(record_fields),
            })
        }
        ExprKind::Call { callee, args } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            if !allow_converter_calls && schema_mapping_name_can_be_converter(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            if allow_converter_calls {
                match schema_mapping_converter_function(context, segments) {
                    SchemaMappingConverterLookup::Found(function) => {
                        return schema_mapping_converter_expr(
                            context, expr, callee, args, function, expected,
                        );
                    }
                    SchemaMappingConverterLookup::Private(function) => {
                        return Err(Box::new(SchemaMappingExprError::PrivateConverter {
                            name: segments.join("::"),
                            span: callee.span.clone(),
                            function_span: function.span.clone(),
                        }));
                    }
                    SchemaMappingConverterLookup::Missing => {}
                }
            }
            if let [name] = segments.as_slice()
                && !schema_mapping_name_can_be_constructor(segments)
            {
                return Err(Box::new(SchemaMappingExprError::UnresolvedConverter {
                    name: name.clone(),
                    span: callee.span.clone(),
                }));
            }
            if segments.len() > 1 && schema_mapping_name_can_be_converter(segments) {
                return Err(Box::new(SchemaMappingExprError::UnresolvedConverter {
                    name: segments.join("::"),
                    span: callee.span.clone(),
                }));
            }
            if !schema_mapping_name_can_be_constructor(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            let constructor = schema_mapping_constructor(
                context.registry,
                context.module,
                context.schema,
                segments,
                expected,
            )
            .ok_or_else(|| {
                Box::new(SchemaMappingExprError::UnresolvedConstructor {
                    name: segments.join("::"),
                    span: callee.span.clone(),
                })
            })?;
            schema_mapping_constructor_expr(
                context,
                expr,
                args,
                expected,
                constructor,
                allow_converter_calls,
            )
        }
        ExprKind::Binary { op, left, right } => {
            schema_mapping_binary_expr(context, expr, *op, left, right, allow_converter_calls)
        }
        ExprKind::Prefix { op, expr: inner } => {
            schema_mapping_prefix_expr(context, expr, *op, inner, allow_converter_calls)
        }
        _ => Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        })),
    }
}

fn schema_mapping_expr_typed_inner(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    expected: &Type,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    let typed =
        schema_mapping_expr_typed_unchecked(context, expr, expected, allow_converter_calls)?;
    if !is_assignable(expected, &typed.ty) {
        return Err(Box::new(SchemaMappingExprError::TypeMismatch {
            expected: Box::new(expected.clone()),
            actual: Box::new(typed.ty),
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        }));
    }
    Ok(typed)
}

fn schema_mapping_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    match &expr.kind {
        ExprKind::NamePath(segments) => schema_mapping_name_expr_inferred(context, expr, segments),
        ExprKind::FieldAccess { base, field, .. } => {
            let typed_base = schema_mapping_expr_inferred(context, base, allow_converter_calls)?;
            let Some(field_ty) = typed_base.ty.record_field(field) else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            Ok(SchemaMappingTypedExpr {
                ty: field_ty.clone(),
                expr: SchemaDecodeMappingExpr::FieldAccess {
                    base: Box::new(typed_base.expr),
                    field: field.clone(),
                },
            })
        }
        ExprKind::Record(fields) => {
            let mut seen = BTreeMap::<String, SourceSpan>::new();
            let mut record_fields = Vec::new();
            let mut field_types = Vec::new();
            for field in fields {
                if seen
                    .insert(field.name.clone(), field.span.clone())
                    .is_some()
                {
                    return Err(Box::new(SchemaMappingExprError::RecordField {
                        name: field.name.clone(),
                        span: field.span.clone(),
                    }));
                }
                let typed =
                    schema_mapping_expr_inferred(context, &field.expr, allow_converter_calls)?;
                field_types.push((field.name.clone(), typed.ty));
                record_fields.push(SchemaDecodeMappingRecordField {
                    name: field.name.clone(),
                    expr: typed.expr,
                });
            }
            Ok(SchemaMappingTypedExpr {
                ty: Type::Record(field_types),
                expr: SchemaDecodeMappingExpr::Record(record_fields),
            })
        }
        ExprKind::Call { callee, args } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            if !allow_converter_calls && schema_mapping_name_can_be_converter(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            if allow_converter_calls {
                match schema_mapping_converter_function(context, segments) {
                    SchemaMappingConverterLookup::Found(function) => {
                        return schema_mapping_converter_expr_inferred(
                            context, expr, callee, args, function,
                        );
                    }
                    SchemaMappingConverterLookup::Private(function) => {
                        return Err(Box::new(SchemaMappingExprError::PrivateConverter {
                            name: segments.join("::"),
                            span: callee.span.clone(),
                            function_span: function.span.clone(),
                        }));
                    }
                    SchemaMappingConverterLookup::Missing => {}
                }
            }
            if !schema_mapping_name_can_be_constructor(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            let constructor = schema_mapping_constructor(
                context.registry,
                context.module,
                context.schema,
                segments,
                &Type::Unknown,
            )
            .ok_or_else(|| {
                Box::new(SchemaMappingExprError::UnresolvedConstructor {
                    name: segments.join("::"),
                    span: callee.span.clone(),
                })
            })?;
            schema_mapping_constructor_expr_inferred(
                context,
                expr,
                args,
                constructor,
                allow_converter_calls,
            )
        }
        ExprKind::Binary { op, left, right } => {
            schema_mapping_binary_expr(context, expr, *op, left, right, allow_converter_calls)
        }
        ExprKind::Prefix { op, expr: inner } => {
            schema_mapping_prefix_expr(context, expr, *op, inner, allow_converter_calls)
        }
        _ => Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        })),
    }
}

fn schema_mapping_binary_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    let (expected, left, right) = match op {
        BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
            let expected = Type::int();
            let left = schema_mapping_arithmetic_operand(
                context,
                expr,
                left,
                &expected,
                allow_converter_calls,
            )?;
            let right = schema_mapping_arithmetic_operand(
                context,
                expr,
                right,
                &expected,
                allow_converter_calls,
            )?;
            (expected, left, right)
        }
        BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual => {
            let operand_ty = Type::int();
            let left = schema_mapping_comparison_operand(
                context,
                expr,
                left,
                &operand_ty,
                allow_converter_calls,
            )?;
            let right = schema_mapping_comparison_operand(
                context,
                expr,
                right,
                &operand_ty,
                allow_converter_calls,
            )?;
            (Type::bool(), left, right)
        }
        BinaryOp::And | BinaryOp::Or => {
            let expected = Type::bool();
            let left = schema_mapping_bool_operand(context, expr, left, allow_converter_calls)?;
            let right = schema_mapping_bool_operand(context, expr, right, allow_converter_calls)?;
            (expected, left, right)
        }
        _ => {
            return Err(Box::new(SchemaMappingExprError::Unsupported {
                text: schema_mapping_expr_render(expr),
                span: expr.span.clone(),
            }));
        }
    };
    Ok(SchemaMappingTypedExpr {
        ty: expected,
        expr: SchemaDecodeMappingExpr::Binary {
            op,
            left: Box::new(left.expr),
            right: Box::new(right.expr),
        },
    })
}

fn schema_mapping_prefix_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    op: PrefixOp,
    inner: &Expr,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    if op != PrefixOp::Not {
        return Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        }));
    }
    let typed = schema_mapping_bool_operand(context, expr, inner, allow_converter_calls)?;
    Ok(SchemaMappingTypedExpr {
        ty: Type::bool(),
        expr: SchemaDecodeMappingExpr::Prefix {
            op,
            expr: Box::new(typed.expr),
        },
    })
}

fn schema_mapping_bool_operand(
    context: &SchemaMappingExprContext<'_>,
    whole_expr: &Expr,
    operand: &Expr,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    match &operand.kind {
        ExprKind::Binary {
            op:
                op @ (BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual
                | BinaryOp::And
                | BinaryOp::Or),
            left,
            right,
        } => schema_mapping_binary_expr(context, operand, *op, left, right, allow_converter_calls)
            .map_err(|error| match *error {
                SchemaMappingExprError::Unsupported { .. } => {
                    Box::new(SchemaMappingExprError::Unsupported {
                        text: schema_mapping_expr_render(whole_expr),
                        span: whole_expr.span.clone(),
                    })
                }
                other => Box::new(other),
            }),
        ExprKind::Prefix {
            op: PrefixOp::Not,
            expr,
        } => {
            schema_mapping_prefix_expr(context, operand, PrefixOp::Not, expr, allow_converter_calls)
                .map_err(|error| match *error {
                    SchemaMappingExprError::Unsupported { .. } => {
                        Box::new(SchemaMappingExprError::Unsupported {
                            text: schema_mapping_expr_render(whole_expr),
                            span: whole_expr.span.clone(),
                        })
                    }
                    other => Box::new(other),
                })
        }
        _ => Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(whole_expr),
            span: whole_expr.span.clone(),
        })),
    }
}

fn schema_mapping_arithmetic_operand(
    context: &SchemaMappingExprContext<'_>,
    whole_expr: &Expr,
    operand: &Expr,
    expected: &Type,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    match &operand.kind {
        ExprKind::NamePath(segments) => {
            let [name] = segments.as_slice() else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(whole_expr),
                    span: whole_expr.span.clone(),
                }));
            };
            let Some(ty) = context.schema_fields.get(name) else {
                return Err(Box::new(SchemaMappingExprError::UnknownSchemaField {
                    name: name.clone(),
                    span: operand.span.clone(),
                }));
            };
            if !is_assignable(expected, ty) {
                return Err(Box::new(SchemaMappingExprError::TypeMismatch {
                    expected: Box::new(expected.clone()),
                    actual: Box::new(ty.clone()),
                    text: schema_mapping_expr_render(operand),
                    span: operand.span.clone(),
                }));
            }
            Ok(SchemaMappingTypedExpr {
                ty: ty.clone(),
                expr: SchemaDecodeMappingExpr::Field(name.clone()),
            })
        }
        ExprKind::IntLiteral(value) => {
            let Some(value) = parse_schema_mapping_integer(value) else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(operand),
                    span: operand.span.clone(),
                }));
            };
            Ok(SchemaMappingTypedExpr {
                ty: Type::int(),
                expr: SchemaDecodeMappingExpr::Literal(value),
            })
        }
        ExprKind::Binary { op, left, right } => {
            let typed = schema_mapping_binary_expr(
                context,
                operand,
                *op,
                left,
                right,
                allow_converter_calls,
            )
            .map_err(|error| match *error {
                SchemaMappingExprError::Unsupported { .. } => {
                    Box::new(SchemaMappingExprError::Unsupported {
                        text: schema_mapping_expr_render(whole_expr),
                        span: whole_expr.span.clone(),
                    })
                }
                other => Box::new(other),
            })?;
            if !is_assignable(expected, &typed.ty) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(whole_expr),
                    span: whole_expr.span.clone(),
                }));
            }
            Ok(typed)
        }
        ExprKind::Call { callee, args } if allow_converter_calls => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(whole_expr),
                    span: whole_expr.span.clone(),
                }));
            };
            match schema_mapping_converter_function(context, segments) {
                SchemaMappingConverterLookup::Found(function) => schema_mapping_converter_expr(
                    context, operand, callee, args, function, expected,
                ),
                SchemaMappingConverterLookup::Private(function) => {
                    Err(Box::new(SchemaMappingExprError::PrivateConverter {
                        name: segments.join("::"),
                        span: callee.span.clone(),
                        function_span: function.span.clone(),
                    }))
                }
                SchemaMappingConverterLookup::Missing => {
                    if !schema_mapping_name_can_be_converter(segments) {
                        return Err(Box::new(SchemaMappingExprError::Unsupported {
                            text: schema_mapping_expr_render(whole_expr),
                            span: whole_expr.span.clone(),
                        }));
                    }
                    Err(Box::new(SchemaMappingExprError::UnresolvedConverter {
                        name: segments.join("::"),
                        span: callee.span.clone(),
                    }))
                }
            }
        }
        _ => Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(whole_expr),
            span: whole_expr.span.clone(),
        })),
    }
}

fn schema_mapping_comparison_operand(
    context: &SchemaMappingExprContext<'_>,
    whole_expr: &Expr,
    operand: &Expr,
    expected: &Type,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    schema_mapping_arithmetic_operand(
        context,
        whole_expr,
        operand,
        expected,
        allow_converter_calls,
    )
}

fn parse_schema_mapping_integer(text: &str) -> Option<i64> {
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

enum SchemaMappingConverterLookup<'a> {
    Found(&'a FunctionSignature),
    Private(&'a FunctionSignature),
    Missing,
}

fn schema_mapping_converter_function<'a>(
    context: &SchemaMappingExprContext<'a>,
    segments: &[String],
) -> SchemaMappingConverterLookup<'a> {
    match segments {
        [name] => context
            .converter_functions
            .iter()
            .find(|function| {
                function.name == *name && schema_mapping_same_module(function, context.schema)
            })
            .map_or(
                SchemaMappingConverterLookup::Missing,
                SchemaMappingConverterLookup::Found,
            ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &context.module.uses,
                &segments[..segments.len() - 1],
                context.schema.module_name.as_deref(),
            ) else {
                return SchemaMappingConverterLookup::Missing;
            };
            let module_name = use_decl.name.as_str();
            let Some(function) = context.converter_functions.iter().find(|function| {
                function.name == *name && function.module_name.as_deref() == Some(module_name)
            }) else {
                return SchemaMappingConverterLookup::Missing;
            };
            if function.visibility == Visibility::Public {
                SchemaMappingConverterLookup::Found(function)
            } else {
                SchemaMappingConverterLookup::Private(function)
            }
        }
        _ => SchemaMappingConverterLookup::Missing,
    }
}

fn schema_mapping_same_module(function: &FunctionSignature, schema: &SchemaDecl) -> bool {
    if function.span.file == schema.span.file {
        return true;
    }
    match (
        function.module_name.as_deref(),
        schema.module_name.as_deref(),
    ) {
        (Some(function_module), Some(schema_module)) => function_module == schema_module,
        (None, None) => function.span.file == schema.span.file,
        _ => false,
    }
}

fn schema_mapping_converter_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    function: &FunctionSignature,
    expected: &Type,
) -> SchemaMappingExprResult {
    let (typed_args, input) =
        schema_mapping_converter_arg_exprs(context, expr, callee, args, function)?;
    if !is_assignable(expected, &function.return_type) {
        return Err(Box::new(SchemaMappingExprError::ConverterReturnType {
            name: function.name.clone(),
            expected: Box::new(expected.clone()),
            actual: Box::new(function.return_type.clone()),
            input,
            span: expr.span.clone(),
            function_span: function.span.clone(),
        }));
    }

    Ok(SchemaMappingTypedExpr {
        ty: function.return_type.clone(),
        expr: SchemaDecodeMappingExpr::Converter {
            function: function.target_name.clone(),
            inverse_function: None,
            args: typed_args,
        },
    })
}

fn schema_mapping_converter_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    function: &FunctionSignature,
) -> SchemaMappingExprResult {
    let (typed_args, _) =
        schema_mapping_converter_arg_exprs(context, expr, callee, args, function)?;
    Ok(SchemaMappingTypedExpr {
        ty: function.return_type.clone(),
        expr: SchemaDecodeMappingExpr::Converter {
            function: function.target_name.clone(),
            inverse_function: None,
            args: typed_args,
        },
    })
}

fn schema_mapping_converter_arg_exprs(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    function: &FunctionSignature,
) -> Result<
    (
        Vec<SchemaDecodeMappingConverterArg>,
        SchemaMappingConverterInput,
    ),
    Box<SchemaMappingExprError>,
> {
    if args.len() < SCHEMA_MAPPING_CONVERTER_MIN_ARITY {
        return Err(Box::new(SchemaMappingExprError::ConverterArity {
            name: function.name.clone(),
            expected: SCHEMA_MAPPING_CONVERTER_MIN_ARITY,
            actual: args.len(),
            span: expr.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    if function.params.len() != args.len() {
        return Err(Box::new(SchemaMappingExprError::ConverterArity {
            name: function.name.clone(),
            expected: function.params.len(),
            actual: args.len(),
            span: callee.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    if !function.effects.is_empty() {
        return Err(Box::new(SchemaMappingExprError::ImpureConverter {
            name: function.name.clone(),
            effects: function.effects.clone(),
            span: callee.span.clone(),
            function_span: function.span.clone(),
        }));
    }

    let first_input = schema_mapping_converter_input(&args[0]);
    let mut typed_args = Vec::with_capacity(args.len());
    for (arg, param_ty) in args.iter().zip(&function.params) {
        if schema_mapping_is_comparison_expr(arg) {
            return Err(Box::new(SchemaMappingExprError::Unsupported {
                text: schema_mapping_expr_render(arg),
                span: arg.span.clone(),
            }));
        }
        let input = schema_mapping_converter_input(arg);
        let typed_arg = match schema_mapping_expr_typed_unchecked(context, arg, param_ty, true) {
            Ok(typed) => typed,
            Err(error) => match *error {
                SchemaMappingExprError::TypeMismatch {
                    actual, span, text, ..
                } if text == schema_mapping_expr_render(arg) => {
                    return Err(Box::new(SchemaMappingExprError::ConverterInputType {
                        name: function.name.clone(),
                        expected: Box::new(param_ty.clone()),
                        actual,
                        input,
                        span,
                        function_span: function.span.clone(),
                    }));
                }
                other => return Err(Box::new(other)),
            },
        };
        if !is_assignable(param_ty, &typed_arg.ty) {
            return Err(Box::new(SchemaMappingExprError::ConverterInputType {
                name: function.name.clone(),
                expected: Box::new(param_ty.clone()),
                actual: Box::new(typed_arg.ty),
                input,
                span: arg.span.clone(),
                function_span: function.span.clone(),
            }));
        }
        typed_args.push(SchemaDecodeMappingConverterArg {
            ty: typed_arg.ty,
            expr: typed_arg.expr,
        });
    }
    Ok((typed_args, first_input))
}

fn schema_mapping_is_comparison_expr(expr: &Expr) -> bool {
    matches!(
        expr.kind,
        ExprKind::Binary {
            op: BinaryOp::Equal
                | BinaryOp::NotEqual
                | BinaryOp::Less
                | BinaryOp::LessEqual
                | BinaryOp::Greater
                | BinaryOp::GreaterEqual,
            ..
        }
    )
}

fn schema_mapping_converter_input(arg: &Expr) -> SchemaMappingConverterInput {
    if let ExprKind::NamePath(segments) = &arg.kind
        && let [source] = segments.as_slice()
    {
        return SchemaMappingConverterInput::SourceField(source.clone());
    }
    SchemaMappingConverterInput::Expression(schema_mapping_expr_render(arg))
}

fn schema_mapping_record_actual_type(
    context: &SchemaMappingExprContext<'_>,
    fields: &[veln_ast::RecordField],
) -> Type {
    Type::Record(
        fields
            .iter()
            .map(|field| {
                (
                    field.name.clone(),
                    schema_mapping_expr_actual_type(context, &field.expr),
                )
            })
            .collect(),
    )
}

fn schema_mapping_expr_actual_type(context: &SchemaMappingExprContext<'_>, expr: &Expr) -> Type {
    match &expr.kind {
        ExprKind::NamePath(segments) => {
            if let [name] = segments.as_slice()
                && let Some(ty) = context.schema_fields.get(name)
            {
                return ty.clone();
            }
            Type::Unknown
        }
        ExprKind::IntLiteral(_) => Type::int(),
        ExprKind::Record(fields) => schema_mapping_record_actual_type(context, fields),
        ExprKind::FieldAccess { base, field, .. } => schema_mapping_expr_actual_type(context, base)
            .record_field(field)
            .cloned()
            .unwrap_or(Type::Unknown),
        ExprKind::Binary { op, left, right }
            if matches!(
                op,
                BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide
            ) && schema_mapping_expr_actual_type(context, left) == Type::int()
                && schema_mapping_expr_actual_type(context, right) == Type::int() =>
        {
            Type::int()
        }
        ExprKind::Binary { op, left, right }
            if matches!(
                op,
                BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
            ) && schema_mapping_expr_actual_type(context, left) == Type::int()
                && schema_mapping_expr_actual_type(context, right) == Type::int() =>
        {
            Type::bool()
        }
        ExprKind::Binary { op, left, right }
            if matches!(op, BinaryOp::And | BinaryOp::Or)
                && schema_mapping_expr_actual_type(context, left) == Type::bool()
                && schema_mapping_expr_actual_type(context, right) == Type::bool() =>
        {
            Type::bool()
        }
        ExprKind::Prefix {
            op: PrefixOp::Not,
            expr,
        } if schema_mapping_expr_actual_type(context, expr) == Type::bool() => Type::bool(),
        _ => Type::Unknown,
    }
}

fn schema_mapping_name_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    segments: &[String],
    expected: &Type,
) -> SchemaMappingExprResult {
    if let [name] = segments {
        if let Some(ty) = context.schema_fields.get(name) {
            return Ok(SchemaMappingTypedExpr {
                ty: ty.clone(),
                expr: SchemaDecodeMappingExpr::Field(name.clone()),
            });
        }
        if let Some(constructor) = schema_mapping_constructor(
            context.registry,
            context.module,
            context.schema,
            segments,
            expected,
        ) && constructor.variant.payload_fields.is_empty()
        {
            return Ok(SchemaMappingTypedExpr {
                ty: expected.clone(),
                expr: SchemaDecodeMappingExpr::Constructor {
                    name: schema_mapping_constructor_name(constructor),
                    args: Vec::new(),
                },
            });
        }
        return Err(Box::new(SchemaMappingExprError::UnknownSchemaField {
            name: name.clone(),
            span: expr.span.clone(),
        }));
    }
    let constructor = schema_mapping_constructor(
        context.registry,
        context.module,
        context.schema,
        segments,
        expected,
    )
    .ok_or_else(|| {
        Box::new(SchemaMappingExprError::UnresolvedConstructor {
            name: segments.join("::"),
            span: expr.span.clone(),
        })
    })?;
    if !constructor.variant.payload_fields.is_empty() {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: segments.join("::"),
            expected: constructor.variant.payload_fields.len(),
            actual: 0,
            span: expr.span.clone(),
        }));
    }
    Ok(SchemaMappingTypedExpr {
        ty: expected.clone(),
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: Vec::new(),
        },
    })
}

fn schema_mapping_name_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    segments: &[String],
) -> SchemaMappingExprResult {
    if let [name] = segments
        && let Some(ty) = context.schema_fields.get(name)
    {
        return Ok(SchemaMappingTypedExpr {
            ty: ty.clone(),
            expr: SchemaDecodeMappingExpr::Field(name.clone()),
        });
    }
    let constructor = schema_mapping_constructor(
        context.registry,
        context.module,
        context.schema,
        segments,
        &Type::Unknown,
    )
    .ok_or_else(|| {
        Box::new(if segments.len() == 1 {
            SchemaMappingExprError::UnknownSchemaField {
                name: segments[0].clone(),
                span: expr.span.clone(),
            }
        } else {
            SchemaMappingExprError::UnresolvedConstructor {
                name: segments.join("::"),
                span: expr.span.clone(),
            }
        })
    })?;
    if !constructor.variant.payload_fields.is_empty() {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: segments.join("::"),
            expected: constructor.variant.payload_fields.len(),
            actual: 0,
            span: expr.span.clone(),
        }));
    }
    Ok(SchemaMappingTypedExpr {
        ty: adt::constructed_type(constructor, &[]),
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: Vec::new(),
        },
    })
}

fn schema_mapping_constructor_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    args: &[Expr],
    expected: &Type,
    constructor: AdtConstructor<'_>,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    let expected_count = constructor.variant.payload_fields.len();
    if args.len() != expected_count {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: schema_mapping_constructor_name(constructor).join("::"),
            expected: expected_count,
            actual: args.len(),
            span: expr.span.clone(),
        }));
    }
    let mut payload_exprs = Vec::new();
    let mut payload_types = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        let payload_ty = adt::payload_type(expected, constructor, index).unwrap_or(Type::Unknown);
        let typed =
            schema_mapping_expr_typed_inner(context, arg, &payload_ty, allow_converter_calls)?;
        payload_types.push(typed.ty);
        payload_exprs.push(typed.expr);
    }
    let ty = if adt::adt_args(expected, constructor.descriptor).is_some() {
        expected.clone()
    } else {
        adt::constructed_type(constructor, &payload_types)
    };
    Ok(SchemaMappingTypedExpr {
        ty,
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: payload_exprs,
        },
    })
}

fn schema_mapping_constructor_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    args: &[Expr],
    constructor: AdtConstructor<'_>,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    let expected_count = constructor.variant.payload_fields.len();
    if args.len() != expected_count {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: schema_mapping_constructor_name(constructor).join("::"),
            expected: expected_count,
            actual: args.len(),
            span: expr.span.clone(),
        }));
    }
    let expected_ty = adt::constructed_type(constructor, &vec![Type::Unknown; expected_count]);
    let mut payload_exprs = Vec::new();
    let mut payload_types = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        let payload_ty =
            adt::payload_type(&expected_ty, constructor, index).unwrap_or(Type::Unknown);
        let typed =
            schema_mapping_expr_typed_inner(context, arg, &payload_ty, allow_converter_calls)?;
        payload_types.push(typed.ty);
        payload_exprs.push(typed.expr);
    }
    Ok(SchemaMappingTypedExpr {
        ty: adt::constructed_type(constructor, &payload_types),
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: payload_exprs,
        },
    })
}

fn schema_mapping_constructor<'a>(
    registry: &'a AdtRegistry,
    module: &SurfaceModule,
    schema: &SchemaDecl,
    segments: &[String],
    expected: &Type,
) -> Option<AdtConstructor<'a>> {
    match registry.constructor(segments, schema.module_name.as_deref(), &module.uses) {
        ConstructorLookup::Found(constructor) => Some(constructor),
        ConstructorLookup::Ambiguous => {
            registry
                .descriptor_for_type(expected)
                .and_then(|descriptor| {
                    registry.constructor_for_descriptor(
                        segments,
                        descriptor,
                        schema.module_name.as_deref(),
                        &module.uses,
                    )
                })
        }
        ConstructorLookup::Missing => None,
    }
}

fn schema_mapping_constructor_name(constructor: AdtConstructor<'_>) -> Vec<String> {
    vec![
        constructor.descriptor.type_name.clone(),
        constructor.variant.name.clone(),
    ]
}

fn schema_mapping_name_can_be_constructor(segments: &[String]) -> bool {
    segments.len() > 1
        || segments
            .last()
            .and_then(|name| name.chars().next())
            .is_some_and(char::is_uppercase)
}

fn schema_mapping_name_can_be_converter(segments: &[String]) -> bool {
    segments
        .last()
        .and_then(|name| name.chars().next())
        .is_some_and(char::is_lowercase)
}

pub(crate) fn schema_mapping_expr_render(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Missing => "<missing>".to_string(),
        ExprKind::Hole { name, .. } => format!("_{}", name.as_deref().unwrap_or("")),
        ExprKind::NamePath(segments) => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::BoolLiteral(true) => "true".to_string(),
        ExprKind::BoolLiteral(false) => "false".to_string(),
        ExprKind::Unit => "()".to_string(),
        ExprKind::TypeApply { callee, type_args } => {
            format!(
                "{}<{}>",
                schema_mapping_expr_render(callee),
                type_args.join(", ")
            )
        }
        ExprKind::Call { callee, args } => {
            let args = args
                .iter()
                .map(schema_mapping_expr_render)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({args})", schema_mapping_expr_render(callee))
        }
        ExprKind::FieldAccess { base, field, .. } => {
            format!("{}.{field}", schema_mapping_expr_render(base))
        }
        ExprKind::Try(inner) => format!("{}?", schema_mapping_expr_render(inner)),
        ExprKind::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| {
                    format!(
                        "{}: {}",
                        field.name,
                        schema_mapping_expr_render(&field.expr)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        ExprKind::Dict(entries) => {
            let entries = entries
                .iter()
                .map(|entry| {
                    format!(
                        "{}: {}",
                        schema_mapping_expr_render(&entry.key),
                        schema_mapping_expr_render(&entry.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {entries} }}")
        }
        ExprKind::List(items) => {
            let items = items
                .iter()
                .map(schema_mapping_expr_render)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{items}]")
        }
        ExprKind::Match { .. } => "match".to_string(),
        ExprKind::If { .. } => "if".to_string(),
        ExprKind::Prefix { op, expr } => match op {
            veln_ast::PrefixOp::Not => format!("not {}", schema_mapping_expr_render(expr)),
            veln_ast::PrefixOp::Negate => format!("-{}", schema_mapping_expr_render(expr)),
        },
        ExprKind::Binary { op, left, right } => {
            format!(
                "{} {} {}",
                schema_mapping_expr_render(left),
                schema_mapping_binary_op_text(*op),
                schema_mapping_expr_render(right)
            )
        }
    }
}

fn schema_mapping_binary_op_text(op: veln_ast::BinaryOp) -> &'static str {
    match op {
        veln_ast::BinaryOp::PipeGreater => "|>",
        veln_ast::BinaryOp::Or => "or",
        veln_ast::BinaryOp::And => "and",
        veln_ast::BinaryOp::Equal => "==",
        veln_ast::BinaryOp::NotEqual => "!=",
        veln_ast::BinaryOp::Less => "<",
        veln_ast::BinaryOp::LessEqual => "<=",
        veln_ast::BinaryOp::Greater => ">",
        veln_ast::BinaryOp::GreaterEqual => ">=",
        veln_ast::BinaryOp::Add => "+",
        veln_ast::BinaryOp::Subtract => "-",
        veln_ast::BinaryOp::Multiply => "*",
        veln_ast::BinaryOp::Divide => "/",
    }
}

pub(crate) fn schema_mapping_target_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
) -> Option<Vec<(String, Type)>> {
    let target = mapping.target.as_ref()?;
    let target_decl = schema_mapping_target_type(module, schema, target)?;
    if target_decl.params.is_empty() && target_decl.variants.len() == 1 {
        return Some(type_variant_record_fields(&target_decl.variants[0]));
    }
    None
}

fn type_variant_record_fields(variant: &TypeVariantDecl) -> Vec<(String, Type)> {
    variant
        .fields
        .iter()
        .map(|field| (field.name.clone(), parse_type_or_unknown(Some(&field.ty))))
        .collect()
}

fn schema_mapping_target_type<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    target: &str,
) -> Option<&'a TypeDecl> {
    let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => module.types.iter().find(|type_decl| {
            type_decl.name.as_deref() == Some(name.as_str())
                && type_decl.module_name.as_deref() == schema.module_name.as_deref()
        }),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            )?;
            module.types.iter().find(|type_decl| {
                type_decl.name.as_deref() == Some(name.as_str())
                    && type_decl.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}
