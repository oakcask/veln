//! Arena-backed surface AST and node handles.

mod lower;
mod model;
mod satisfy;

pub use lower::lower_surface_ast;
pub use model::{
    BinaryOp, BodyLine, BodyLineKind, Contract, ContractKind, Expr, ExprKind, Function, NodeId,
    Param, PrefixOp, RecordField, SatisfyClause, SurfaceModule, Visibility,
};

#[cfg(test)]
mod tests;
