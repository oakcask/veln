//! Arena-backed surface AST and node handles.

mod lower;
mod model;
mod satisfy;

pub use lower::lower_surface_ast;
pub use model::{
    BinaryOp, BodyLine, BodyLineKind, Contract, ContractKind, Expr, ExprKind, Function,
    FunctionKind, ModuleHeader, NodeId, Param, PrefixOp, RecordField, SatisfyClause, SurfaceModule,
    UseDecl, Visibility,
};

#[cfg(test)]
mod tests;
