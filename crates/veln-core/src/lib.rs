//! Source-linked checked core representation.

pub mod model;
pub mod readiness;
pub mod types;

pub use model::{
    CheckedProgram, CoreCallTarget, CoreContract, CoreDictEntry, CoreExpr, CoreExprKind,
    CoreFunction, CoreMatchArm, CoreParam, CorePattern, CorePatternKind, CoreRecordField, CoreStmt,
    CoreStmtKind,
};
pub use readiness::{CoreBlocker, CoreReadiness};
pub use types::CoreType;
