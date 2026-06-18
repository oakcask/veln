//! Arena-backed surface AST and node handles.

mod lower;
mod model;
mod satisfy;

pub use lower::{lower_surface_ast, lower_surface_ast_with_module_identity};
pub use model::{
    BinaryOp, BodyLine, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationClause,
    CodecImplementationKind, Contract, ContractKind, DictEntry, Expr, ExprKind, Function,
    FunctionKind, MatchArm, ModuleHeader, NodeId, Param, Pattern, PatternField, PatternKind,
    PrefixOp, PublicAlias, PublicAliasKind, RecordField, ResultBinding, SatisfyClause, SchemaDecl,
    SchemaField, SchemaFieldWhereClause, SchemaFormatClause, SchemaMappingAssignment,
    SchemaMappingClause, SchemaMappingInverseConverter, SchemaMappingSelector,
    SchemaValidationClause, SurfaceModule, TypeDecl, TypeVariantDecl, TypeVariantField, UseDecl,
    Visibility,
};

#[cfg(test)]
mod tests;
