//! Runtime-neutral typed IR.

pub mod lowering;
pub mod model;

pub use lowering::{IrLowerError, lower_checked_core};
pub use model::{
    IrCallTarget, IrCallbackTarget, IrCleanupRegion, IrContract, IrDeferredBlock,
    IrDeferredCapture, IrDictEntry, IrExpr, IrExprKind, IrFunction, IrHandlerProvider, IrMatchArm,
    IrModuleValueDecl, IrParam, IrPattern, IrPatternField, IrPatternKind, IrRecordField,
    IrSchemaDecodeDispatch, IrSchemaDecodeDispatchCase, IrSchemaDecodeField, IrSchemaDecodeSpec,
    IrSchemaRefinementField, IrSchemaRefinementPlan, IrSchemaRefinementValidationSpec,
    IrSchemaRepeat, IrSchemaReservedBits, IrSchemaVariantRefinement, IrStmt, IrStmtKind,
    TypedProgram,
};
pub use veln_core::{
    ContractObligationStatus, CoreContractCall, CoreModuleValueIdentity, CoreType,
};
