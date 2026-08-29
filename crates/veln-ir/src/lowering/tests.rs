use super::*;
use veln_ast::{
    BinaryOp, BodyLine, BodyLineKind, ContractKind, DictEntry, Expr, ExprKind, Function, MatchArm,
    Pattern, PatternField, PatternKind, PrefixOp, RecordField, SurfaceModule, Visibility,
    lower_surface_ast,
};
use veln_core::{
    ContractObligationStatus, CoreContract, CoreDictEntry, CoreFunction, CoreMatchArm, CoreParam,
    CorePattern, CorePatternField, CorePatternKind, CoreReadiness, CoreRecordField, CoreStmtKind,
    CoreType,
};
use veln_source::SourceFile;
use veln_syntax::parse;

mod failures;
mod successful_lowering;

fn lower_source(text: &str) -> SurfaceModule {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics: {:?}",
        parsed.diagnostics
    );
    lower_surface_ast(&parsed.tree)
}

fn core_expr(expr: &Expr, ty: CoreType, kind: CoreExprKind) -> CoreExpr {
    CoreExpr {
        node_id: expr.node_id,
        ty,
        kind,
        span: expr.span.clone(),
    }
}

fn core_stmt(line: &BodyLine, kind: CoreStmtKind) -> CoreStmt {
    CoreStmt {
        node_id: line.node_id,
        kind,
        span: line.span.clone(),
    }
}

fn local(expr: &Expr, name: &str, ty: CoreType) -> CoreExpr {
    core_expr(expr, ty, CoreExprKind::Local(name.to_string()))
}

fn function_shell(function: &Function) -> CoreFunction {
    CoreFunction {
        node_id: function.node_id,
        name: function.name.clone().expect("function should be named"),
        visibility: function.visibility,
        params: Vec::new(),
        return_binding: None,
        return_type: CoreType::unit(),
        effects: Vec::new(),
        contracts: Vec::new(),
        body: Vec::new(),
        span: function.span.clone(),
    }
}

fn complete_program(functions: Vec<CoreFunction>) -> CheckedProgram {
    CheckedProgram {
        functions,
        effects: Vec::new(),
        readiness: CoreReadiness::Complete,
    }
}

fn let_expr(line: &BodyLine) -> &Expr {
    let BodyLineKind::Let { expr, .. } = &line.kind else {
        panic!("expected let line");
    };
    expr
}

fn expr_line(line: &BodyLine) -> &Expr {
    let BodyLineKind::Expr { expr } = &line.kind else {
        panic!("expected expression line");
    };
    expr
}

fn call_parts(expr: &Expr) -> (&Expr, &[Expr]) {
    let ExprKind::Call { callee, args } = &expr.kind else {
        panic!("expected call expression");
    };
    (callee, args)
}

fn try_inner(expr: &Expr) -> &Expr {
    let ExprKind::Try(inner) = &expr.kind else {
        panic!("expected try expression");
    };
    inner
}

fn list_items(expr: &Expr) -> &[Expr] {
    let ExprKind::List(items) = &expr.kind else {
        panic!("expected list expression");
    };
    items
}

fn prefix_inner(expr: &Expr) -> &Expr {
    let ExprKind::Prefix { expr, .. } = &expr.kind else {
        panic!("expected prefix expression");
    };
    expr
}

fn binary_parts(expr: &Expr) -> (&Expr, &Expr) {
    let ExprKind::Binary { left, right, .. } = &expr.kind else {
        panic!("expected binary expression");
    };
    (left, right)
}

fn record_fields(expr: &Expr) -> &[RecordField] {
    let ExprKind::Record(fields) = &expr.kind else {
        panic!("expected record expression");
    };
    fields
}

fn named_field<'a>(fields: &'a [RecordField], name: &str) -> &'a RecordField {
    fields
        .iter()
        .find(|field| field.name == name)
        .expect("record field should exist")
}

fn dict_entries(expr: &Expr) -> &[DictEntry] {
    let ExprKind::Dict(entries) = &expr.kind else {
        panic!("expected dictionary expression");
    };
    entries
}

fn match_parts(expr: &Expr) -> (&Expr, &[MatchArm]) {
    let ExprKind::Match { scrutinee, arms } = &expr.kind else {
        panic!("expected match expression");
    };
    (scrutinee, arms)
}

fn core_pattern(pattern: &Pattern) -> CorePattern {
    CorePattern {
        node_id: pattern.node_id,
        kind: match &pattern.kind {
            PatternKind::Wildcard => CorePatternKind::Wildcard,
            PatternKind::Binding(name) => CorePatternKind::Binding(name.clone()),
            PatternKind::StringLiteral(value) => CorePatternKind::StringLiteral(value.clone()),
            PatternKind::IntLiteral(value) => CorePatternKind::IntLiteral(value.clone()),
            PatternKind::FloatLiteral(value) => CorePatternKind::FloatLiteral(value.clone()),
            PatternKind::BoolLiteral(value) => CorePatternKind::BoolLiteral(*value),
            PatternKind::Unit => CorePatternKind::Unit,
            PatternKind::Record(fields) => {
                CorePatternKind::Record(fields.iter().map(core_pattern_field).collect::<Vec<_>>())
            }
            PatternKind::Constructor { name, args } => CorePatternKind::Constructor {
                name: name.clone(),
                args: args.iter().map(core_pattern).collect(),
            },
        },
        span: pattern.span.clone(),
    }
}

fn core_pattern_field(field: &PatternField) -> CorePatternField {
    CorePatternField {
        node_id: field.node_id,
        name: field.name.clone(),
        pattern: core_pattern(&field.pattern),
        span: field.span.clone(),
    }
}

fn main_function(module: &SurfaceModule) -> &Function {
    module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("main"))
        .expect("main should exist")
}

fn fixture_ids() -> SurfaceModule {
    lower_source(concat!(
        "pub fn main(input: Int, mapper: Mapper) -> Result<(), AppError> effects [stdio]\n",
        "  let answer: Int = mapper(input)\n",
        "  stdio::println(\"done\")\n",
        "  Ok(())\n",
        "end\n",
    ))
}
