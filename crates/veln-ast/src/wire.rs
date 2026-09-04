use veln_source::{LineCol, SourcePath, SourceSpan};

use crate::{
    BinaryOp, BodyLine, BodyLineKind, Contract, ContractKind, DictEntry, EffectBinder, EffectDecl,
    EffectOperationDecl, Expr, ExprKind, Function, FunctionKind, HandlerDecl,
    HandlerOperationClauseDecl, IfBranch, InvalidName, MatchArm, ModuleHeader, NameClass,
    NameOccurrence, NodeId, Param, Pattern, PatternField, PatternKind, PrefixOp, PublicAlias,
    PublicAliasKind, RecordField, ResultBinding, SatisfyClause, SchemaDecl, SchemaField,
    SchemaFieldWhereClause, SchemaFormatClause, SchemaValidationClause, SurfaceModule, TypeDecl,
    TypePathSegments, TypeVariantDecl, TypeVariantField, UseDecl, UseOrigin, Visibility,
};

const MAGIC: &[u8; 8] = b"VLNAST1\n";

mod decoder;
mod encoder;

pub use decoder::decode_surface_module;
pub use encoder::encode_surface_module;
