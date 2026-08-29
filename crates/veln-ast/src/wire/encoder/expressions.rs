use super::*;

impl Writer {
    pub(super) fn expr(&mut self, value: &Expr) {
        self.node_id(value.node_id);
        self.expr_kind(&value.kind);
        self.span(&value.span);
    }

    fn expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => self.scalar_expr_kind(value),
            ExprKind::TypeApply { .. } | ExprKind::Call { .. } => {
                self.invocation_expr_kind(value);
            }
            ExprKind::Perform { .. } | ExprKind::Handle { .. } => self.effect_expr_kind(value),
            ExprKind::SchemaDecode { .. }
            | ExprKind::SchemaEncode { .. }
            | ExprKind::FieldAccess { .. }
            | ExprKind::Try(_) => self.schema_and_access_expr_kind(value),
            ExprKind::Record(_)
            | ExprKind::Dict(_)
            | ExprKind::List(_)
            | ExprKind::Match { .. }
            | ExprKind::If { .. } => self.aggregate_expr_kind(value),
            ExprKind::Prefix { .. } | ExprKind::Binary { .. } => self.operator_expr_kind(value),
        }
    }

    fn scalar_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Missing => self.u8(0),
            ExprKind::Hole { name, satisfy } => {
                self.u8(1);
                self.option(name, |writer, value| writer.string(value));
                self.option(satisfy, Self::satisfy);
            }
            ExprKind::NamePath(path) => {
                self.u8(2);
                self.vec(path, |writer, value| writer.string(value));
            }
            ExprKind::StringLiteral(value) => {
                self.u8(3);
                self.string(value);
            }
            ExprKind::IntLiteral(value) => {
                self.u8(4);
                self.string(value);
            }
            ExprKind::FloatLiteral(value) => {
                self.u8(5);
                self.string(value);
            }
            ExprKind::BoolLiteral(value) => {
                self.u8(6);
                self.bool(*value);
            }
            ExprKind::Unit => self.u8(7),
            _ => unreachable!("non-scalar expression passed to scalar wire encoder"),
        }
    }

    fn invocation_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::TypeApply { callee, type_args } => {
                self.u8(8);
                self.expr(callee);
                self.vec(type_args, |writer, value| writer.string(value));
            }
            ExprKind::Call { callee, args } => {
                self.u8(9);
                self.expr(callee);
                self.vec(args, Self::expr);
            }
            _ => unreachable!("non-invocation expression passed to invocation wire encoder"),
        }
    }

    fn effect_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Perform {
                effect,
                effect_span,
                operation,
                operation_span,
                args,
            } => {
                self.u8(10);
                self.vec(effect, |writer, value| writer.string(value));
                self.span(effect_span);
                self.string(operation);
                self.span(operation_span);
                self.vec(args, Self::expr);
            }
            ExprKind::Handle {
                body,
                handler,
                handler_span,
                args,
            } => {
                self.u8(11);
                self.expr(body);
                self.vec(handler, |writer, value| writer.string(value));
                self.span(handler_span);
                self.vec(args, Self::expr);
            }
            _ => unreachable!("non-effect expression passed to effect wire encoder"),
        }
    }

    fn schema_and_access_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::SchemaDecode {
                schema,
                input,
                base,
            } => {
                self.u8(12);
                self.vec(schema, |writer, value| writer.string(value));
                self.expr(input);
                self.expr(base);
            }
            ExprKind::SchemaEncode { schema, value } => {
                self.u8(13);
                self.vec(schema, |writer, value| writer.string(value));
                self.expr(value);
            }
            ExprKind::FieldAccess {
                base,
                field,
                field_span,
            } => {
                self.u8(14);
                self.expr(base);
                self.string(field);
                self.span(field_span);
            }
            ExprKind::Try(expr) => {
                self.u8(15);
                self.expr(expr);
            }
            _ => unreachable!("non-schema or access expression passed to schema wire encoder"),
        }
    }

    fn aggregate_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Record(fields) => {
                self.u8(16);
                self.vec(fields, Self::record_field);
            }
            ExprKind::Dict(entries) => {
                self.u8(17);
                self.vec(entries, Self::dict_entry);
            }
            ExprKind::List(items) => {
                self.u8(18);
                self.vec(items, Self::expr);
            }
            ExprKind::Match { scrutinee, arms } => {
                self.u8(19);
                self.expr(scrutinee);
                self.vec(arms, Self::match_arm);
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.u8(20);
                self.expr(condition);
                self.expr(then_branch);
                self.vec(else_if_branches, Self::if_branch);
                self.expr(else_branch);
            }
            _ => unreachable!("non-aggregate expression passed to aggregate wire encoder"),
        }
    }

    fn operator_expr_kind(&mut self, value: &ExprKind) {
        match value {
            ExprKind::Prefix { op, expr } => {
                self.u8(21);
                self.prefix_op(*op);
                self.expr(expr);
            }
            ExprKind::Binary { op, left, right } => {
                self.u8(22);
                self.binary_op(*op);
                self.expr(left);
                self.expr(right);
            }
            _ => unreachable!("non-operator expression passed to operator wire encoder"),
        }
    }

    fn satisfy(&mut self, value: &SatisfyClause) {
        self.option(&value.candidate, |writer, value| writer.string(value));
        self.option(&value.candidate_span, Self::span);
        self.string(&value.predicate);
        self.span(&value.span);
    }

    fn record_field(&mut self, value: &RecordField) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.expr(&value.expr);
        self.span(&value.span);
    }

    fn dict_entry(&mut self, value: &DictEntry) {
        self.node_id(value.node_id);
        self.expr(&value.key);
        self.expr(&value.value);
        self.span(&value.span);
    }

    fn match_arm(&mut self, value: &MatchArm) {
        self.node_id(value.node_id);
        self.pattern(&value.pattern);
        self.expr(&value.expr);
        self.span(&value.span);
    }

    fn if_branch(&mut self, value: &IfBranch) {
        self.node_id(value.node_id);
        self.expr(&value.condition);
        self.expr(&value.expr);
        self.span(&value.span);
    }

    fn prefix_op(&mut self, value: PrefixOp) {
        self.u8(match value {
            PrefixOp::Not => 0,
            PrefixOp::Negate => 1,
            PrefixOp::BitwiseNot => 2,
        });
    }

    fn binary_op(&mut self, value: BinaryOp) {
        self.u8(match value {
            BinaryOp::PipeGreater => 0,
            BinaryOp::Or => 1,
            BinaryOp::And => 2,
            BinaryOp::BitwiseOr => 3,
            BinaryOp::BitwiseXor => 4,
            BinaryOp::BitwiseAnd => 5,
            BinaryOp::Equal => 6,
            BinaryOp::NotEqual => 7,
            BinaryOp::Less => 8,
            BinaryOp::LessEqual => 9,
            BinaryOp::Greater => 10,
            BinaryOp::GreaterEqual => 11,
            BinaryOp::ShiftLeft => 12,
            BinaryOp::ShiftRight => 13,
            BinaryOp::ShiftRightLogical => 14,
            BinaryOp::Add => 15,
            BinaryOp::Subtract => 16,
            BinaryOp::Multiply => 17,
            BinaryOp::Divide => 18,
        });
    }

    pub(super) fn pattern(&mut self, value: &Pattern) {
        self.node_id(value.node_id);
        self.pattern_kind(&value.kind);
        self.span(&value.span);
    }

    fn pattern_kind(&mut self, value: &PatternKind) {
        match value {
            PatternKind::Wildcard => self.u8(0),
            PatternKind::Binding(value) => {
                self.u8(1);
                self.string(value);
            }
            PatternKind::StringLiteral(value) => {
                self.u8(2);
                self.string(value);
            }
            PatternKind::IntLiteral(value) => {
                self.u8(3);
                self.string(value);
            }
            PatternKind::FloatLiteral(value) => {
                self.u8(4);
                self.string(value);
            }
            PatternKind::BoolLiteral(value) => {
                self.u8(5);
                self.bool(*value);
            }
            PatternKind::Unit => self.u8(6),
            PatternKind::Record(fields) => {
                self.u8(7);
                self.vec(fields, Self::pattern_field);
            }
            PatternKind::Constructor { name, args } => {
                self.u8(8);
                self.vec(name, |writer, value| writer.string(value));
                self.vec(args, Self::pattern);
            }
        }
    }

    fn pattern_field(&mut self, value: &PatternField) {
        self.node_id(value.node_id);
        self.string(&value.name);
        self.pattern(&value.pattern);
        self.span(&value.span);
    }
}
