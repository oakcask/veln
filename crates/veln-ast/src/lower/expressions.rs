use super::*;

impl AstBuilder {
    pub(super) fn lower_expr(&mut self, expr: &SyntaxExpr) -> Expr {
        Expr {
            node_id: self.alloc(),
            kind: self.lower_expr_kind(expr),
            span: expr.span.clone(),
        }
    }

    fn lower_expr_kind(&mut self, expr: &SyntaxExpr) -> ExprKind {
        if let Some(kind) = self.lower_scalar_expr_kind(expr) {
            return kind;
        }
        if let Some(kind) = self.lower_call_like_expr_kind(expr) {
            return kind;
        }
        if let Some(kind) = self.lower_collection_expr_kind(expr) {
            return kind;
        }
        self.lower_operator_expr_kind(expr)
    }

    fn lower_scalar_expr_kind(&mut self, expr: &SyntaxExpr) -> Option<ExprKind> {
        match &expr.kind {
            SyntaxExprKind::Missing => Some(ExprKind::Missing),
            SyntaxExprKind::Hole { name, satisfy } => Some(ExprKind::Hole {
                name: name.clone(),
                satisfy: satisfy.as_ref().map(crate::satisfy::lower_satisfy_clause),
            }),
            SyntaxExprKind::NamePath {
                segments,
                segment_spans,
            } => Some(ExprKind::NamePath {
                segments: segments.clone(),
                segment_spans: segment_spans.clone(),
            }),
            SyntaxExprKind::StringLiteral(value) => Some(ExprKind::StringLiteral(value.clone())),
            SyntaxExprKind::IntLiteral(value) => Some(ExprKind::IntLiteral(value.clone())),
            SyntaxExprKind::FloatLiteral(value) => Some(ExprKind::FloatLiteral(value.clone())),
            SyntaxExprKind::BoolLiteral(value) => Some(ExprKind::BoolLiteral(*value)),
            SyntaxExprKind::Unit => Some(ExprKind::Unit),
            _ => None,
        }
    }

    fn lower_call_like_expr_kind(&mut self, expr: &SyntaxExpr) -> Option<ExprKind> {
        match &expr.kind {
            SyntaxExprKind::TypeApply { callee, type_args } => Some(ExprKind::TypeApply {
                callee: Box::new(self.lower_expr(callee)),
                type_args: type_args.clone(),
            }),
            SyntaxExprKind::Call { callee, args } => Some(ExprKind::Call {
                callee: Box::new(self.lower_expr(callee)),
                args: self.lower_exprs(args),
            }),
            SyntaxExprKind::Perform {
                effect,
                effect_span,
                operation,
                operation_span,
                args,
            } => Some(ExprKind::Perform {
                effect: effect.clone(),
                effect_span: effect_span.clone(),
                operation: operation.clone(),
                operation_span: operation_span.clone(),
                args: self.lower_exprs(args),
            }),
            SyntaxExprKind::Handle {
                body,
                handler,
                handler_span,
                args,
            } => Some(ExprKind::Handle {
                body: Box::new(self.lower_expr(body)),
                handler: handler.clone(),
                handler_span: handler_span.clone(),
                args: self.lower_exprs(args),
            }),
            SyntaxExprKind::SchemaDecode {
                schema,
                input,
                base,
            } => Some(ExprKind::SchemaDecode {
                schema: schema.clone(),
                input: Box::new(self.lower_expr(input)),
                base: Box::new(self.lower_expr(base)),
            }),
            SyntaxExprKind::SchemaEncode { schema, value } => Some(ExprKind::SchemaEncode {
                schema: schema.clone(),
                value: Box::new(self.lower_expr(value)),
            }),
            SyntaxExprKind::FieldAccess {
                base,
                field,
                field_span,
            } => Some(ExprKind::FieldAccess {
                base: Box::new(self.lower_expr(base)),
                field: field.clone(),
                field_span: field_span.clone(),
            }),
            SyntaxExprKind::Try(expr) => Some(ExprKind::Try(Box::new(self.lower_expr(expr)))),
            _ => None,
        }
    }

    fn lower_collection_expr_kind(&mut self, expr: &SyntaxExpr) -> Option<ExprKind> {
        match &expr.kind {
            SyntaxExprKind::Record(fields) => Some(ExprKind::Record(
                fields
                    .iter()
                    .map(|field| self.lower_record_field(field))
                    .collect(),
            )),
            SyntaxExprKind::Dict(entries) => Some(ExprKind::Dict(
                entries
                    .iter()
                    .map(|entry| self.lower_dict_entry(entry))
                    .collect(),
            )),
            SyntaxExprKind::List(items) => Some(ExprKind::List(self.lower_exprs(items))),
            SyntaxExprKind::Match { scrutinee, arms } => {
                Some(self.lower_match_expr(scrutinee, arms))
            }
            SyntaxExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => Some(ExprKind::If {
                condition: Box::new(self.lower_expr(condition)),
                then_branch: Box::new(self.lower_expr(then_branch)),
                else_if_branches: self.lower_if_branches(else_if_branches),
                else_branch: Box::new(self.lower_expr(else_branch)),
            }),
            _ => None,
        }
    }

    fn lower_operator_expr_kind(&mut self, expr: &SyntaxExpr) -> ExprKind {
        match &expr.kind {
            SyntaxExprKind::Prefix { op, expr } => ExprKind::Prefix {
                op: lower_prefix_op(*op),
                expr: Box::new(self.lower_expr(expr)),
            },
            SyntaxExprKind::Binary { op, left, right } => ExprKind::Binary {
                op: lower_binary_op(*op),
                left: Box::new(self.lower_expr(left)),
                right: Box::new(self.lower_expr(right)),
            },
            _ => {
                unreachable!("syntax expression variant should be handled before operator lowering")
            }
        }
    }

    fn lower_exprs(&mut self, exprs: &[SyntaxExpr]) -> Vec<Expr> {
        exprs.iter().map(|expr| self.lower_expr(expr)).collect()
    }

    fn lower_match_expr(
        &mut self,
        scrutinee: &SyntaxExpr,
        arms: &[veln_syntax::MatchArm],
    ) -> ExprKind {
        ExprKind::Match {
            scrutinee: Box::new(self.lower_expr(scrutinee)),
            arms: arms
                .iter()
                .map(|arm| MatchArm {
                    node_id: self.alloc(),
                    pattern: self.lower_pattern(&arm.pattern),
                    expr: self.lower_expr(&arm.expr),
                    span: arm.span.clone(),
                })
                .collect(),
        }
    }

    fn lower_if_branches(&mut self, branches: &[veln_syntax::IfBranch]) -> Vec<crate::IfBranch> {
        branches
            .iter()
            .map(|branch| crate::IfBranch {
                node_id: self.alloc(),
                condition: self.lower_expr(&branch.condition),
                expr: self.lower_expr(&branch.expr),
                span: branch.span.clone(),
            })
            .collect()
    }

    pub(super) fn lower_pattern(&mut self, pattern: &SyntaxPattern) -> Pattern {
        Pattern {
            node_id: self.alloc(),
            kind: match &pattern.kind {
                SyntaxPatternKind::Wildcard => PatternKind::Wildcard,
                SyntaxPatternKind::Binding(name) => PatternKind::Binding(name.clone()),
                SyntaxPatternKind::StringLiteral(value) => {
                    PatternKind::StringLiteral(value.clone())
                }
                SyntaxPatternKind::IntLiteral(value) => PatternKind::IntLiteral(value.clone()),
                SyntaxPatternKind::FloatLiteral(value) => PatternKind::FloatLiteral(value.clone()),
                SyntaxPatternKind::BoolLiteral(value) => PatternKind::BoolLiteral(*value),
                SyntaxPatternKind::Unit => PatternKind::Unit,
                SyntaxPatternKind::Record(fields) => PatternKind::Record(
                    fields
                        .iter()
                        .map(|field| PatternField {
                            node_id: self.alloc(),
                            name: field.name.clone(),
                            pattern: self.lower_pattern(&field.pattern),
                            span: field.span.clone(),
                        })
                        .collect(),
                ),
                SyntaxPatternKind::Constructor {
                    name,
                    name_spans,
                    args,
                } => PatternKind::Constructor {
                    name: name.clone(),
                    name_spans: name_spans.clone(),
                    args: args.iter().map(|arg| self.lower_pattern(arg)).collect(),
                },
            },
            span: pattern.span.clone(),
        }
    }

    fn lower_record_field(&mut self, field: &SyntaxRecordField) -> RecordField {
        RecordField {
            node_id: self.alloc(),
            name: field.name.clone(),
            expr: self.lower_expr(&field.expr),
            span: field.span.clone(),
        }
    }

    fn lower_dict_entry(&mut self, entry: &SyntaxDictEntry) -> DictEntry {
        DictEntry {
            node_id: self.alloc(),
            key: self.lower_expr(&entry.key),
            value: self.lower_expr(&entry.value),
            span: entry.span.clone(),
        }
    }
}
