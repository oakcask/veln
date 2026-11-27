use veln_ast::{BinaryOp, PrefixOp};
use veln_core::CoreType;
use veln_literals::parse_integer_literal;

use crate::{IrExpr, IrExprKind};

pub(super) fn fold_expression_kind(ty: &CoreType, kind: IrExprKind) -> IrExprKind {
    record_visit();
    match kind {
        IrExprKind::Prefix { op, expr } => fold_prefix(ty, op, expr),
        IrExprKind::Binary { op, left, right } => fold_binary(ty, op, left, right),
        kind => kind,
    }
}

fn fold_prefix(ty: &CoreType, op: PrefixOp, expr: Box<IrExpr>) -> IrExprKind {
    let folded = match (op, ty, &expr.ty) {
        (PrefixOp::Negate, result, operand)
            if result == &CoreType::int() && operand == &CoreType::int() =>
        {
            int_value(&expr).map(|value| IrExprKind::IntConstant(value.wrapping_neg()))
        }
        (PrefixOp::BitwiseNot, result, operand)
            if result == &CoreType::int() && operand == &CoreType::int() =>
        {
            int_value(&expr).map(|value| IrExprKind::IntConstant(!value))
        }
        (PrefixOp::Not, result, operand)
            if result == &CoreType::bool() && operand == &CoreType::bool() =>
        {
            bool_value(&expr).map(|value| IrExprKind::BoolLiteral(!value))
        }
        _ => None,
    };
    folded.unwrap_or(IrExprKind::Prefix { op, expr })
}

fn fold_binary(ty: &CoreType, op: BinaryOp, left: Box<IrExpr>, right: Box<IrExpr>) -> IrExprKind {
    let folded = if left.ty == CoreType::int() && right.ty == CoreType::int() {
        fold_int_binary(ty, op, &left, &right)
    } else if left.ty == CoreType::bool() && right.ty == CoreType::bool() {
        fold_bool_binary(ty, op, &left, &right)
    } else {
        None
    };
    folded.unwrap_or(IrExprKind::Binary { op, left, right })
}

fn fold_int_binary(
    ty: &CoreType,
    op: BinaryOp,
    left: &IrExpr,
    right: &IrExpr,
) -> Option<IrExprKind> {
    let left = int_value(left)?;
    let right = int_value(right)?;
    let integer = |value| (ty == &CoreType::int()).then_some(IrExprKind::IntConstant(value));
    let boolean = |value| (ty == &CoreType::bool()).then_some(IrExprKind::BoolLiteral(value));
    match op {
        BinaryOp::Add => integer(left.wrapping_add(right)),
        BinaryOp::Subtract => integer(left.wrapping_sub(right)),
        BinaryOp::Multiply => integer(left.wrapping_mul(right)),
        BinaryOp::Divide if right != 0 => integer(left.wrapping_div(right)),
        BinaryOp::Remainder if right != 0 => integer(left.wrapping_rem(right)),
        BinaryOp::BitwiseAnd => integer(left & right),
        BinaryOp::BitwiseXor => integer(left ^ right),
        BinaryOp::BitwiseOr => integer(left | right),
        BinaryOp::ShiftLeft if (0..64).contains(&right) => integer(left.wrapping_shl(right as u32)),
        BinaryOp::ShiftRight if (0..64).contains(&right) => integer(left >> right),
        BinaryOp::ShiftRightLogical if (0..64).contains(&right) => {
            integer(((left as u64) >> right) as i64)
        }
        BinaryOp::Equal => boolean(left == right),
        BinaryOp::NotEqual => boolean(left != right),
        BinaryOp::Less => boolean(left < right),
        BinaryOp::LessEqual => boolean(left <= right),
        BinaryOp::Greater => boolean(left > right),
        BinaryOp::GreaterEqual => boolean(left >= right),
        _ => None,
    }
}

fn fold_bool_binary(
    ty: &CoreType,
    op: BinaryOp,
    left: &IrExpr,
    right: &IrExpr,
) -> Option<IrExprKind> {
    if ty != &CoreType::bool() {
        return None;
    }
    let left = bool_value(left)?;
    let right = bool_value(right)?;
    let result = match op {
        BinaryOp::And => left && right,
        BinaryOp::Or => left || right,
        BinaryOp::Equal => left == right,
        BinaryOp::NotEqual => left != right,
        _ => return None,
    };
    Some(IrExprKind::BoolLiteral(result))
}

fn int_value(expr: &IrExpr) -> Option<i64> {
    match &expr.kind {
        IrExprKind::IntLiteral(text) => parse_integer_literal(text).ok().map(|value| value.value),
        IrExprKind::IntConstant(value) => Some(*value),
        _ => None,
    }
}

fn bool_value(expr: &IrExpr) -> Option<bool> {
    match expr.kind {
        IrExprKind::BoolLiteral(value) => Some(value),
        _ => None,
    }
}

#[cfg(test)]
thread_local! {
    static VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_visit() {
    VISITS.with(|visits| visits.set(visits.get() + 1));
}

#[cfg(not(test))]
fn record_visit() {}

#[cfg(test)]
pub(super) fn take_visits() -> usize {
    VISITS.with(|visits| visits.replace(0))
}
