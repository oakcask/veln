//! Source-linked checked core representation.

pub mod model;
pub mod readiness;
pub mod types;

pub use model::{
    CheckedProgram, ContractObligationStatus, CoreCallTarget, CoreContract, CoreDictEntry,
    CoreEffectDecl, CoreEffectOperationDecl, CoreExpr, CoreExprKind, CoreFunction, CoreMatchArm,
    CoreParam, CorePattern, CorePatternField, CorePatternKind, CoreRecordField, CoreStmt,
    CoreStmtKind,
};
pub use readiness::{CoreBlocker, CoreReadiness};
pub use types::CoreType;
