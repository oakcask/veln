//! Runtime-neutral typed IR.

pub mod lowering;
pub mod model;

pub use lowering::{IrLowerError, lower_checked_core};
pub use model::{
    IrCallTarget, IrContract, IrDictEntry, IrExpr, IrExprKind, IrFunction, IrMatchArm, IrParam,
    IrPattern, IrPatternField, IrPatternKind, IrRecordField, IrSchemaDecodeDispatch,
    IrSchemaDecodeDispatchCase, IrSchemaDecodeField, IrSchemaDecodeMappingField,
    IrSchemaDecodeSpec, IrSchemaReservedBits, IrStmt, IrStmtKind, TypedProgram,
};
pub use veln_core::ContractObligationStatus;
