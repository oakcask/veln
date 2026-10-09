use std::collections::BTreeMap;

use veln_syntax::{BodyLine, Expr, ExprKind, SyntaxItem, SyntaxTree, VariantRefinementType};

use super::{SemanticToken, SemanticTokenModifiers, SemanticTokenType};

pub(super) fn apply_variant_refinement_classification(
    tree: &SyntaxTree,
    semantic_tokens: &mut [SemanticToken],
) {
    let mut classifications = BTreeMap::new();
    collect_tree_refinements(tree, &mut classifications);
    for token in semantic_tokens {
        if let Some(token_type) = classifications.get(&token.span.start.offset) {
            token.kind.token_type = *token_type;
            token.modifiers = SemanticTokenModifiers::empty();
        }
    }
}

fn collect_tree_refinements(
    tree: &SyntaxTree,
    classifications: &mut BTreeMap<usize, SemanticTokenType>,
) {
    for item in &tree.items {
        match item {
            SyntaxItem::Function(function) => {
                collect_params(&function.params, classifications);
                collect_refinements(&function.return_type_refinements, classifications);
                collect_body(&function.body, classifications);
            }
            SyntaxItem::Effect(effect) => {
                for operation in &effect.operations {
                    collect_params(&operation.params, classifications);
                    collect_refinements(&operation.return_type_refinements, classifications);
                }
            }
            SyntaxItem::Handler(handler) => {
                collect_params(&handler.params, classifications);
                for clause in &handler.operation_clauses {
                    collect_params(&clause.params, classifications);
                    collect_expr(&clause.body, classifications);
                }
            }
            SyntaxItem::Type(ty) => {
                for variant in &ty.variants {
                    for field in &variant.fields {
                        collect_refinements(&field.ty_refinements, classifications);
                    }
                }
            }
            SyntaxItem::Schema(schema) => {
                for field in &schema.fields {
                    collect_refinements(&field.ty_refinements, classifications);
                }
            }
            SyntaxItem::PublicAlias(_) => {}
        }
    }
}

fn collect_params(
    params: &[veln_syntax::Param],
    classifications: &mut BTreeMap<usize, SemanticTokenType>,
) {
    for param in params {
        collect_refinements(&param.ty_refinements, classifications);
    }
}

fn collect_body(body: &[BodyLine], classifications: &mut BTreeMap<usize, SemanticTokenType>) {
    for line in body {
        match line {
            BodyLine::Let {
                annotation_refinements,
                expr,
                ..
            } => {
                collect_refinements(annotation_refinements, classifications);
                collect_expr(expr, classifications);
            }
            BodyLine::Expr { expr, .. } => collect_expr(expr, classifications),
            BodyLine::Defer { body, .. } => collect_body(body, classifications),
        }
    }
}

fn collect_expr(expr: &Expr, classifications: &mut BTreeMap<usize, SemanticTokenType>) {
    match &expr.kind {
        ExprKind::TypeApply {
            callee,
            type_arg_refinements,
            ..
        } => {
            collect_expr(callee, classifications);
            for refinements in type_arg_refinements {
                collect_refinements(refinements, classifications);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_expr(callee, classifications);
            collect_exprs(args, classifications);
        }
        ExprKind::Perform { args, .. } | ExprKind::List(args) => {
            collect_exprs(args, classifications);
        }
        ExprKind::Handle { body, args, .. } => {
            collect_expr(body, classifications);
            collect_exprs(args, classifications);
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_expr(input, classifications);
            collect_expr(base, classifications);
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::Prefix { expr: value, .. }
        | ExprKind::Try { expr: value, .. }
        | ExprKind::FieldAccess { base: value, .. } => collect_expr(value, classifications),
        ExprKind::Record(fields) => {
            for field in fields {
                collect_expr(&field.expr, classifications);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_expr(&entry.key, classifications);
                collect_expr(&entry.value, classifications);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr(scrutinee, classifications);
            for arm in arms {
                collect_expr(&arm.expr, classifications);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_expr(condition, classifications);
            collect_expr(then_branch, classifications);
            for branch in else_if_branches {
                collect_expr(&branch.condition, classifications);
                collect_expr(&branch.expr, classifications);
            }
            collect_expr(else_branch, classifications);
        }
        ExprKind::Begin { body, .. } => collect_body(body, classifications),
        ExprKind::Binary { left, right, .. } => {
            collect_expr(left, classifications);
            collect_expr(right, classifications);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::NamePath { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit => {}
    }
}

fn collect_exprs(exprs: &[Expr], classifications: &mut BTreeMap<usize, SemanticTokenType>) {
    for expr in exprs {
        collect_expr(expr, classifications);
    }
}

fn collect_refinements(
    refinements: &[VariantRefinementType],
    classifications: &mut BTreeMap<usize, SemanticTokenType>,
) {
    for refinement in refinements {
        for alternative in &refinement.alternatives {
            for span in &alternative.base.segment_spans {
                classifications.insert(span.start.offset, SemanticTokenType::Type);
            }
            for argument in &alternative.type_arguments {
                collect_refinements(&argument.ty_refinements, classifications);
            }
            classifications.insert(
                alternative.variant_span.start.offset,
                SemanticTokenType::EnumMember,
            );
        }
        for span in &refinement.pipe_spans {
            classifications.insert(span.start.offset, SemanticTokenType::Operator);
        }
    }
}
