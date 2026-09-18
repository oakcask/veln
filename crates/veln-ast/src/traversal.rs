use crate::{Expr, ExprKind};
use std::ops::ControlFlow;

impl Expr {
    /// Visits immediate expression children in source order, without recursing.
    /// Binding scopes and non-expression metadata remain the caller's responsibility.
    pub fn for_each_child<'a>(&'a self, visitor: &mut impl FnMut(&'a Expr)) {
        let _: ControlFlow<()> = self.try_for_each_child(&mut |child| {
            visitor(child);
            ControlFlow::Continue(())
        });
    }

    /// Visits immediate expression children in source order, stopping at the first break.
    /// Binding scopes and non-expression metadata remain the caller's responsibility.
    pub fn try_for_each_child<'a, B>(
        &'a self,
        visitor: &mut impl FnMut(&'a Expr) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        match &self.kind {
            ExprKind::Call { callee, args }
            | ExprKind::Handle {
                body: callee, args, ..
            } => {
                visitor(callee)?;
                for arg in args {
                    visitor(arg)?;
                }
            }
            ExprKind::TypeApply { callee: expr, .. }
            | ExprKind::SchemaEncode { value: expr, .. }
            | ExprKind::FieldAccess { base: expr, .. }
            | ExprKind::Try(expr)
            | ExprKind::Prefix { expr, .. } => visitor(expr)?,
            ExprKind::Perform { args, .. } | ExprKind::List(args) => {
                for arg in args {
                    visitor(arg)?;
                }
            }
            ExprKind::SchemaDecode {
                input: left,
                base: right,
                ..
            }
            | ExprKind::Binary { left, right, .. } => {
                visitor(left)?;
                visitor(right)?;
            }
            ExprKind::Record(fields) => {
                for field in fields {
                    visitor(&field.expr)?;
                }
            }
            ExprKind::Dict(entries) => {
                for entry in entries {
                    visitor(&entry.key)?;
                    visitor(&entry.value)?;
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                visitor(scrutinee)?;
                for arm in arms {
                    visitor(&arm.expr)?;
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                visitor(condition)?;
                visitor(then_branch)?;
                for branch in else_if_branches {
                    visitor(&branch.condition)?;
                    visitor(&branch.expr)?;
                }
                visitor(else_branch)?;
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
        ControlFlow::Continue(())
    }
}
