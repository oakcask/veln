use super::*;

impl<'a> Reader<'a> {
    pub(super) fn expr(&mut self) -> Result<Expr, String> {
        Ok(Expr {
            node_id: self.node_id()?,
            kind: self.expr_kind()?,
            span: self.span()?,
        })
    }

    fn expr_kind(&mut self) -> Result<ExprKind, String> {
        let tag = self.u8()?;
        match tag {
            0..=7 => self.scalar_expr_kind(tag),
            8..=9 => self.invocation_expr_kind(tag),
            10..=11 => self.effect_expr_kind(tag),
            12..=15 => self.schema_and_access_expr_kind(tag),
            16..=20 => self.aggregate_expr_kind(tag),
            21..=22 => self.operator_expr_kind(tag),
            value => Err(format!("invalid expr kind tag {value}")),
        }
    }

    fn scalar_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            0 => Ok(ExprKind::Missing),
            1 => Ok(ExprKind::Hole {
                name: self.option(Self::string)?,
                satisfy: self.option(Self::satisfy)?,
            }),
            2 => Ok(ExprKind::NamePath(self.vec(Self::string)?)),
            3 => Ok(ExprKind::StringLiteral(self.string()?)),
            4 => Ok(ExprKind::IntLiteral(self.string()?)),
            5 => Ok(ExprKind::FloatLiteral(self.string()?)),
            6 => Ok(ExprKind::BoolLiteral(self.bool()?)),
            7 => Ok(ExprKind::Unit),
            _ => unreachable!("non-scalar tag passed to scalar wire decoder"),
        }
    }

    fn invocation_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            8 => Ok(ExprKind::TypeApply {
                callee: Box::new(self.expr()?),
                type_args: self.vec(Self::string)?,
            }),
            9 => Ok(ExprKind::Call {
                callee: Box::new(self.expr()?),
                args: self.vec(Self::expr)?,
            }),
            _ => unreachable!("non-invocation tag passed to invocation wire decoder"),
        }
    }

    fn effect_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            10 => Ok(ExprKind::Perform {
                effect: self.vec(Self::string)?,
                effect_span: self.span()?,
                operation: self.string()?,
                operation_span: self.span()?,
                args: self.vec(Self::expr)?,
            }),
            11 => Ok(ExprKind::Handle {
                body: Box::new(self.expr()?),
                handler: self.vec(Self::string)?,
                handler_span: self.span()?,
                args: self.vec(Self::expr)?,
            }),
            _ => unreachable!("non-effect tag passed to effect wire decoder"),
        }
    }

    fn schema_and_access_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            12 => Ok(ExprKind::SchemaDecode {
                schema: self.vec(Self::string)?,
                input: Box::new(self.expr()?),
                base: Box::new(self.expr()?),
            }),
            13 => Ok(ExprKind::SchemaEncode {
                schema: self.vec(Self::string)?,
                value: Box::new(self.expr()?),
            }),
            14 => Ok(ExprKind::FieldAccess {
                base: Box::new(self.expr()?),
                field: self.string()?,
                field_span: self.span()?,
            }),
            15 => Ok(ExprKind::Try(Box::new(self.expr()?))),
            _ => unreachable!("non-schema or access tag passed to schema wire decoder"),
        }
    }

    fn aggregate_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            16 => Ok(ExprKind::Record(self.vec(Self::record_field)?)),
            17 => Ok(ExprKind::Dict(self.vec(Self::dict_entry)?)),
            18 => Ok(ExprKind::List(self.vec(Self::expr)?)),
            19 => Ok(ExprKind::Match {
                scrutinee: Box::new(self.expr()?),
                arms: self.vec(Self::match_arm)?,
            }),
            20 => Ok(ExprKind::If {
                condition: Box::new(self.expr()?),
                then_branch: Box::new(self.expr()?),
                else_if_branches: self.vec(Self::if_branch)?,
                else_branch: Box::new(self.expr()?),
            }),
            _ => unreachable!("non-aggregate tag passed to aggregate wire decoder"),
        }
    }

    fn operator_expr_kind(&mut self, tag: u8) -> Result<ExprKind, String> {
        match tag {
            21 => Ok(ExprKind::Prefix {
                op: self.prefix_op()?,
                expr: Box::new(self.expr()?),
            }),
            22 => Ok(ExprKind::Binary {
                op: self.binary_op()?,
                left: Box::new(self.expr()?),
                right: Box::new(self.expr()?),
            }),
            _ => unreachable!("non-operator tag passed to operator wire decoder"),
        }
    }

    fn satisfy(&mut self) -> Result<SatisfyClause, String> {
        Ok(SatisfyClause {
            candidate: self.option(Self::string)?,
            candidate_span: self.option(Self::span)?,
            predicate: self.string()?,
            span: self.span()?,
        })
    }

    fn record_field(&mut self) -> Result<RecordField, String> {
        Ok(RecordField {
            node_id: self.node_id()?,
            name: self.string()?,
            expr: self.expr()?,
            span: self.span()?,
        })
    }

    fn dict_entry(&mut self) -> Result<DictEntry, String> {
        Ok(DictEntry {
            node_id: self.node_id()?,
            key: self.expr()?,
            value: self.expr()?,
            span: self.span()?,
        })
    }

    fn match_arm(&mut self) -> Result<MatchArm, String> {
        Ok(MatchArm {
            node_id: self.node_id()?,
            pattern: self.pattern()?,
            expr: self.expr()?,
            span: self.span()?,
        })
    }

    fn if_branch(&mut self) -> Result<IfBranch, String> {
        Ok(IfBranch {
            node_id: self.node_id()?,
            condition: self.expr()?,
            expr: self.expr()?,
            span: self.span()?,
        })
    }

    fn prefix_op(&mut self) -> Result<PrefixOp, String> {
        match self.u8()? {
            0 => Ok(PrefixOp::Not),
            1 => Ok(PrefixOp::Negate),
            2 => Ok(PrefixOp::BitwiseNot),
            value => Err(format!("invalid prefix op tag {value}")),
        }
    }

    fn binary_op(&mut self) -> Result<BinaryOp, String> {
        match self.u8()? {
            0 => Ok(BinaryOp::PipeGreater),
            1 => Ok(BinaryOp::Or),
            2 => Ok(BinaryOp::And),
            3 => Ok(BinaryOp::BitwiseOr),
            4 => Ok(BinaryOp::BitwiseXor),
            5 => Ok(BinaryOp::BitwiseAnd),
            6 => Ok(BinaryOp::Equal),
            7 => Ok(BinaryOp::NotEqual),
            8 => Ok(BinaryOp::Less),
            9 => Ok(BinaryOp::LessEqual),
            10 => Ok(BinaryOp::Greater),
            11 => Ok(BinaryOp::GreaterEqual),
            12 => Ok(BinaryOp::ShiftLeft),
            13 => Ok(BinaryOp::ShiftRight),
            14 => Ok(BinaryOp::ShiftRightLogical),
            15 => Ok(BinaryOp::Add),
            16 => Ok(BinaryOp::Subtract),
            17 => Ok(BinaryOp::Multiply),
            18 => Ok(BinaryOp::Divide),
            value => Err(format!("invalid binary op tag {value}")),
        }
    }

    pub(super) fn pattern(&mut self) -> Result<Pattern, String> {
        Ok(Pattern {
            node_id: self.node_id()?,
            kind: self.pattern_kind()?,
            span: self.span()?,
        })
    }

    fn pattern_kind(&mut self) -> Result<PatternKind, String> {
        match self.u8()? {
            0 => Ok(PatternKind::Wildcard),
            1 => Ok(PatternKind::Binding(self.string()?)),
            2 => Ok(PatternKind::StringLiteral(self.string()?)),
            3 => Ok(PatternKind::IntLiteral(self.string()?)),
            4 => Ok(PatternKind::FloatLiteral(self.string()?)),
            5 => Ok(PatternKind::BoolLiteral(self.bool()?)),
            6 => Ok(PatternKind::Unit),
            7 => Ok(PatternKind::Record(self.vec(Self::pattern_field)?)),
            8 => Ok(PatternKind::Constructor {
                name: self.vec(Self::string)?,
                args: self.vec(Self::pattern)?,
            }),
            value => Err(format!("invalid pattern kind tag {value}")),
        }
    }

    fn pattern_field(&mut self) -> Result<PatternField, String> {
        Ok(PatternField {
            node_id: self.node_id()?,
            name: self.string()?,
            pattern: self.pattern()?,
            span: self.span()?,
        })
    }
}
