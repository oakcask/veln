//! Runtime-neutral typed IR.

pub mod lowering;
pub mod model;

pub use lowering::{IrLowerError, lower_checked_core};
pub use model::{
    IrCallTarget, IrDictEntry, IrExpr, IrExprKind, IrFunction, IrParam, IrRecordField, IrStmt,
    IrStmtKind, TypedProgram,
};
