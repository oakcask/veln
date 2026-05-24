//! Runtime-neutral typed IR.

pub mod lowering;
pub mod model;

pub use lowering::{IrLowerError, lower_checked_core};
pub use model::{
    IrCallTarget, IrDictEntry, IrExpr, IrExprKind, IrFunction, IrMatchArm, IrParam, IrPattern,
    IrPatternField, IrPatternKind, IrRecordField, IrStmt, IrStmtKind, TypedProgram,
};
