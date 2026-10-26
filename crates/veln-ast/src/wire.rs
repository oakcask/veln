use veln_source::{GeneratedSpanOrigin, LineCol, SourcePath, SourceSpan};

use crate::{
    BinaryOp, BodyLine, BodyLineKind, Contract, ContractKind, DictEntry, EffectBinder, EffectDecl,
    EffectOperationDecl, Expr, ExprKind, Function, FunctionKind, HandlerDecl,
    HandlerOperationClauseDecl, IfBranch, InvalidName, MatchArm, ModuleHeader, ModuleValueDecl,
    NameClass, NameOccurrence, NodeId, Param, Pattern, PatternField, PatternKind,
    PredicateRemainder, PredicateTokenSpan, PrefixOp, PublicAlias, PublicAliasKind, RecordField,
    ResultBinding, SatisfyClause, SchemaDecl, SchemaField, SchemaFieldWhereClause,
    SchemaFormatClause, SchemaValidationClause, SurfaceModule, TypeAnnotationStructure, TypeDecl,
    TypePathSegments, TypeVariantDecl, TypeVariantField, UseDecl, UseOrigin,
    VariantRefinementAlternative, VariantRefinementType, VariantRefinementTypeArgument, Visibility,
};

const MAGIC: &[u8; 9] = b"VLNAST18\n";

mod decoder;
mod encoder;

pub use decoder::decode_surface_module;
pub use encoder::encode_surface_module;
