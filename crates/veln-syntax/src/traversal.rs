use crate::{BodyLine, Expr, ExprChildren};

impl Expr {
    /// Visits immediate expression children in source order, without recursing.
    /// Body-line structure inside `begin` expressions is transparent to the visitor.
    pub fn for_each_child<'a>(&'a self, visitor: &mut impl FnMut(&'a Expr)) {
        match self.children() {
            ExprChildren::None => {}
            ExprChildren::One { child, .. } => visitor(child),
            ExprChildren::Pair { first, second, .. } => {
                visitor(first);
                visitor(second);
            }
            ExprChildren::Slice(children) => children.iter().for_each(visitor),
            ExprChildren::HeadAndSlice(head, children) => {
                visitor(head);
                children.iter().for_each(visitor);
            }
            ExprChildren::Record(fields) => {
                fields.iter().for_each(|field| visitor(&field.expr));
            }
            ExprChildren::Dict(entries) => {
                for entry in entries {
                    visitor(&entry.key);
                    visitor(&entry.value);
                }
            }
            ExprChildren::Match(scrutinee, arms) => {
                visitor(scrutinee);
                arms.iter().for_each(|arm| visitor(&arm.expr));
            }
            ExprChildren::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                visitor(condition);
                visitor(then_branch);
                for branch in else_if_branches {
                    visitor(&branch.condition);
                    visitor(&branch.expr);
                }
                visitor(else_branch);
            }
            ExprChildren::BeginBody(body) => visit_body_exprs(body, visitor),
        }
    }
}

fn visit_body_exprs<'a>(body: &'a [BodyLine], visitor: &mut impl FnMut(&'a Expr)) {
    for line in body {
        match line {
            BodyLine::Let { expr, .. } | BodyLine::Expr { expr, .. } => visitor(expr),
            BodyLine::Defer { body, .. } => visit_body_exprs(body, visitor),
        }
    }
}
