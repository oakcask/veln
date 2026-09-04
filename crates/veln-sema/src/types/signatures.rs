use veln_ast::{NodeId, SchemaField, Visibility};
use veln_source::SourceSpan;

use crate::semantic_model::Type;

#[derive(Clone)]
pub(crate) struct EffectSignature {
    pub(crate) name: String,
    pub(crate) qualified_name: String,
    pub(crate) module_name: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) span: SourceSpan,
    pub(crate) operations: Vec<EffectOperationSignature>,
}

#[derive(Clone)]
pub(crate) struct CompanionAccessTarget {
    pub(crate) companion_path: String,
    pub(crate) target_module: String,
}

pub(crate) enum UserEffectPathResolution<'a> {
    Found(&'a EffectSignature),
    PrivateCompanionTargetMismatch {
        effect: &'a EffectSignature,
        access: &'a CompanionAccessTarget,
    },
    QuarantinedImportTarget,
    Missing,
}

impl<'a> UserEffectPathResolution<'a> {
    pub(crate) fn found(self) -> Option<&'a EffectSignature> {
        match self {
            Self::Found(effect) => Some(effect),
            Self::PrivateCompanionTargetMismatch { .. }
            | Self::QuarantinedImportTarget
            | Self::Missing => None,
        }
    }
}

#[derive(Clone)]
pub(crate) struct EffectOperationSignature {
    pub(crate) name: String,
    pub(crate) params: Vec<Type>,
    pub(crate) return_type: Type,
    pub(crate) node_id: NodeId,
    pub(crate) name_span: SourceSpan,
}

#[derive(Clone)]
pub(crate) struct HandlerSignature {
    pub(crate) name: String,
    pub(crate) qualified_name: String,
    pub(crate) module_name: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) params: Vec<Type>,
    pub(crate) effect: String,
    pub(crate) effects: Vec<String>,
    pub(crate) operation_clauses: Vec<HandlerOperationClauseSignature>,
}

#[derive(Clone)]
pub(crate) struct HandlerOperationClauseSignature {
    pub(crate) operation: String,
    pub(crate) function: String,
    pub(crate) module_name: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SchemaReferenceErrorKind {
    Unresolved,
    Private,
    WrongKind,
}

pub(crate) struct SchemaReferenceError {
    pub(crate) kind: SchemaReferenceErrorKind,
    pub(crate) resolved_kind: Option<&'static str>,
}

pub(crate) struct UnsupportedSchemaEncodeField {
    pub(crate) schema_name: String,
    pub(crate) schema_span: SourceSpan,
    pub(crate) field: SchemaField,
}

#[derive(Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) name: String,
    pub(crate) target_name: String,
    pub(crate) module_name: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) params: Vec<Type>,
    pub(crate) variadic: Option<Type>,
    pub(crate) return_type: Type,
    pub(crate) effects: Vec<String>,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
}

pub(crate) const SCHEMA_DECODE_TARGET_PREFIX: &str = "schema-decode:";
pub(crate) const SCHEMA_DECODE_STEP_TARGET_PREFIX: &str = "schema-decode-step:";
pub(crate) const SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX: &str = "schema-neutral-decode:";
pub(crate) const SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX: &str = "schema-neutral-encode:";
pub(crate) const SCHEMA_ENCODE_TARGET_PREFIX: &str = "schema-encode:";
pub(crate) const SCHEMA_ENCODE_STEP_TARGET_PREFIX: &str = "schema-encode-step:";
pub(crate) const SCHEMA_VALIDATE_TARGET_PREFIX: &str = "schema-validate:";

pub(crate) enum FunctionLookup<'a> {
    Found(&'a FunctionSignature),
    Ambiguous,
    Missing,
}

pub(crate) enum HandlerPathResolution<'a> {
    Found(&'a HandlerSignature),
    PrivateCompanionTargetMismatch {
        handler: &'a HandlerSignature,
        access: &'a CompanionAccessTarget,
    },
    QuarantinedImportTarget,
    Missing,
}

pub(crate) enum MatchScrutineePatternInference {
    Uninferred,
    Inferred(Type),
    Ambiguous(Vec<String>),
}

impl<'a> FunctionLookup<'a> {
    pub(crate) fn found(self) -> Option<&'a FunctionSignature> {
        match self {
            Self::Found(function) => Some(function),
            Self::Ambiguous | Self::Missing => None,
        }
    }
}

impl FunctionSignature {
    pub(crate) fn ty(&self) -> Type {
        match &self.variadic {
            Some(variadic) => Type::variadic_function(
                self.params.clone(),
                variadic.clone(),
                self.return_type.clone(),
                self.effects.clone(),
            ),
            None => Type::function(
                self.params.clone(),
                self.return_type.clone(),
                self.effects.clone(),
            ),
        }
    }
}

pub(crate) fn synthetic_handler_clause_function_name(handler: &str, operation: &str) -> String {
    format!(
        "__handler_{}${handler}_{}${operation}",
        handler.len(),
        operation.len()
    )
}
