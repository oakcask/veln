use super::type_text::canonical_type_text;
use crate::{
    BodyLine, Expr, ExprKind, SyntaxItem, SyntaxTree, TokenKind, VariantRefinementAlternative,
    VariantRefinementType, VariantRefinementTypeArgument,
};
use veln_source::SourceFile;

pub(super) fn prepare_structured_type_text(tree: &mut SyntaxTree, source: &str) {
    for item in &mut tree.items {
        match item {
            SyntaxItem::Function(function) => {
                prepare_params(&mut function.params, source);
                prepare_optional_type(
                    &mut function.return_type,
                    &function.return_type_refinements,
                    source,
                );
                prepare_body(&mut function.body, source);
            }
            SyntaxItem::Effect(effect) => {
                for operation in &mut effect.operations {
                    prepare_params(&mut operation.params, source);
                    prepare_optional_type(
                        &mut operation.return_type,
                        &operation.return_type_refinements,
                        source,
                    );
                }
            }
            SyntaxItem::Handler(handler) => {
                prepare_params(&mut handler.params, source);
                for clause in &mut handler.operation_clauses {
                    prepare_expr(&mut clause.body, source);
                }
            }
            SyntaxItem::Type(type_decl) => {
                for variant in &mut type_decl.variants {
                    for field in &mut variant.fields {
                        field.ty = structured_type_text(&field.ty, &field.ty_refinements, source);
                    }
                }
            }
            SyntaxItem::Schema(schema) => {
                for field in &mut schema.fields {
                    field.ty = structured_type_text(&field.ty, &field.ty_refinements, source);
                }
            }
            SyntaxItem::PublicAlias(_) => {}
        }
    }
}

fn prepare_params(params: &mut [crate::Param], source: &str) {
    for param in params {
        prepare_optional_type(&mut param.ty, &param.ty_refinements, source);
    }
}

fn prepare_optional_type(
    text: &mut Option<String>,
    refinements: &[VariantRefinementType],
    source: &str,
) {
    if let Some(text) = text {
        *text = structured_type_text(text, refinements, source);
    }
}

fn prepare_body(body: &mut [BodyLine], source: &str) {
    for line in body {
        match line {
            BodyLine::Let {
                annotation,
                annotation_refinements,
                expr,
                ..
            } => {
                prepare_optional_type(annotation, annotation_refinements, source);
                prepare_expr(expr, source);
            }
            BodyLine::Expr { expr, .. } => prepare_expr(expr, source),
            BodyLine::Defer { body, .. } => prepare_body(body, source),
        }
    }
}

fn prepare_expr(expr: &mut Expr, source: &str) {
    match &mut expr.kind {
        ExprKind::TypeApply {
            callee,
            type_args,
            type_arg_spans,
            type_arg_refinements,
            ..
        } => {
            for ((text, span), refinements) in type_args
                .iter_mut()
                .zip(type_arg_spans)
                .zip(type_arg_refinements)
            {
                *text = structured_type_text_at_span(text, span, refinements, source);
            }
            prepare_expr(callee, source);
        }
        ExprKind::Call { callee, args } => {
            prepare_expr(callee, source);
            prepare_exprs(args, source);
        }
        ExprKind::Perform { args, .. } | ExprKind::List(args) => prepare_exprs(args, source),
        ExprKind::Handle { body, args, .. } => {
            prepare_expr(body, source);
            prepare_exprs(args, source);
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            prepare_expr(input, source);
            prepare_expr(base, source);
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try { expr: value, .. }
        | ExprKind::Prefix { expr: value, .. } => prepare_expr(value, source),
        ExprKind::Record(fields) => {
            for field in fields {
                prepare_expr(&mut field.expr, source);
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                prepare_expr(&mut entry.key, source);
                prepare_expr(&mut entry.value, source);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            prepare_expr(scrutinee, source);
            for arm in arms {
                prepare_expr(&mut arm.expr, source);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            prepare_expr(condition, source);
            prepare_expr(then_branch, source);
            for branch in else_if_branches {
                prepare_expr(&mut branch.condition, source);
                prepare_expr(&mut branch.expr, source);
            }
            prepare_expr(else_branch, source);
        }
        ExprKind::Begin { body, .. } => prepare_body(body, source),
        ExprKind::Binary { left, right, .. } => {
            prepare_expr(left, source);
            prepare_expr(right, source);
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

fn structured_type_text_at_span(
    fallback: &str,
    span: &veln_source::SourceSpan,
    refinements: &[VariantRefinementType],
    source: &str,
) -> String {
    if refinements.is_empty() {
        return fallback.to_string();
    }
    let Some(mut cursor) = source.get(..span.start.offset).map(str::len) else {
        return fallback.to_string();
    };
    let mut output = String::new();
    for refinement in refinements {
        let Some(fragment) = source.get(cursor..refinement.span.start.offset) else {
            return fallback.to_string();
        };
        output.push_str(fragment);
        output.push_str(&render_refinement(refinement));
        cursor = refinement.span.end.offset;
    }
    let Some(fragment) = source.get(cursor..span.end.offset) else {
        return fallback.to_string();
    };
    output.push_str(fragment);
    output
}

fn prepare_exprs(expressions: &mut [Expr], source: &str) {
    for expression in expressions {
        prepare_expr(expression, source);
    }
}

fn structured_type_text(text: &str, refinements: &[VariantRefinementType], source: &str) -> String {
    let mut output = text.to_string();
    let mut search_start = 0;
    for refinement in refinements {
        let originals = source
            .get(refinement.span.start.offset..refinement.span.end.offset)
            .map(source_type_text_candidates)
            .unwrap_or_default();
        let Some((relative_start, original_len)) = originals
            .iter()
            .filter_map(|original| {
                output[search_start..]
                    .find(original)
                    .map(|start| (start, original.len()))
            })
            .min_by_key(|(start, _)| *start)
        else {
            continue;
        };
        let start = search_start + relative_start;
        let end = start + original_len;
        let replacement = render_refinement(refinement);
        output.replace_range(start..end, &replacement);
        search_start = start + replacement.len();
    }
    output
}

fn source_type_text_candidates(text: &str) -> Vec<String> {
    let tokens = source_type_tokens(text);
    let joined = canonical_tokenized_type_text(&tokens);
    let compact = canonical_type_text(&tokens.join(""));
    if joined == compact {
        vec![joined]
    } else {
        vec![joined, compact]
    }
}

fn source_type_tokens(text: &str) -> Vec<String> {
    let source = SourceFile::new("type.veln", text);
    crate::lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !token.kind.is_trivia() && token.kind != TokenKind::Eof)
        .map(|token| token.text)
        .collect()
}

fn canonical_tokenized_type_text(tokens: &[String]) -> String {
    let joined = tokens
        .join(" ")
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace("->(", "-> (")
        .replace(" )", ")")
        .replace(" . ", ".")
        .replace("[ ", "[")
        .replace(" ]", "]")
        .replace(" ,", ",")
        .replace(" ; ", "; ")
        .replace(" ;", ";")
        .replace(";  ", "; ")
        .replace(" <", "<")
        .replace("< ", "<")
        .replace(" >", ">");
    canonical_type_text(&joined)
}

fn render_refinement(refinement: &VariantRefinementType) -> String {
    refinement
        .alternatives
        .iter()
        .map(render_alternative)
        .collect::<Vec<_>>()
        .join(" | ")
}

fn render_alternative(alternative: &VariantRefinementAlternative) -> String {
    let mut text = alternative.base.segments.join("::");
    if !alternative.type_arguments.is_empty() {
        text.push('<');
        text.push_str(
            &alternative
                .type_arguments
                .iter()
                .map(render_type_argument)
                .collect::<Vec<_>>()
                .join(", "),
        );
        text.push('>');
    }
    text.push_str("::");
    text.push_str(&alternative.variant);
    text
}

fn render_type_argument(argument: &VariantRefinementTypeArgument) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if let Some(refinement) = argument.ty_refinements.get(index) {
            text.push_str(&render_refinement(refinement));
        }
    }
    canonical_tokenized_type_text(&source_type_tokens(&text))
}
