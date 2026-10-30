//! Source-linked checked core representation.

pub mod model;
pub mod readiness;
pub mod types;

pub use model::{
    CheckedProgram, ContractObligationStatus, CoreCallTarget, CoreCallbackTarget,
    CoreCleanupRegion, CoreContract, CoreContractCall, CoreDeferredBlock, CoreDeferredCapture,
    CoreDictEntry, CoreEffectDecl, CoreEffectOperationDecl, CoreExpr, CoreExprKind, CoreFunction,
    CoreHandlerProvider, CoreMatchArm, CoreModuleValueDecl, CoreModuleValueIdentity, CoreParam,
    CorePattern, CorePatternField, CorePatternKind, CoreRecordField, CoreSchemaRefinementField,
    CoreSchemaRefinementPlan, CoreSchemaRefinementValidationSpec, CoreStmt, CoreStmtKind,
};
pub use readiness::{CoreBlocker, CoreReadiness};
pub use types::CoreType;
