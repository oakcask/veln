//! Arena-backed surface AST and node handles.

mod lower;
mod model;
mod satisfy;
mod wire;

pub use lower::{lower_surface_ast, lower_surface_ast_with_module_identity};
pub use model::{
    BinaryOp, BodyLine, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationClause,
    CodecImplementationKind, Contract, ContractKind, DictEntry, EffectBinder, EffectDecl,
    EffectOperationDecl, Expr, ExprKind, Function, FunctionKind, HandlerDecl,
    HandlerOperationClauseDecl, IfBranch, InvalidName, MatchArm, ModuleHeader, NameClass,
    NameOccurrence, NodeId, Param, Pattern, PatternField, PatternKind, PrefixOp, PublicAlias,
    PublicAliasKind, RecordField, ResultBinding, SatisfyClause, SchemaDecl, SchemaField,
    SchemaFieldWhereClause, SchemaFormatClause, SchemaValidationClause, SurfaceModule, TypeDecl,
    TypeVariantDecl, TypeVariantField, UseDecl, UseOrigin, Visibility,
};
pub use wire::{decode_surface_module, encode_surface_module};

#[cfg(test)]
mod tests;
