mod schema_encode;

#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use veln_ast::{
    BodyLine, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationKind, DictEntry, Expr,
    ExprKind, Function, FunctionKind, IfBranch, MatchArm, NodeId, Pattern, PatternKind,
    PublicAliasKind, RecordField, SchemaDecl, SchemaField, SurfaceModule, UseDecl, Visibility,
};
use veln_literals::parse_integer_literal;
use veln_project::classify_companion_source;
use veln_source::SourceSpan;

use crate::adt::{self, AdtRegistry};
use crate::effects::{
    concurrency_effects, is_stdio_call, prelude_effects, standard_library_effects,
};
use crate::semantic_model::{Binding, FunctionKey, Type};
use crate::type_syntax::{parse_type_annotation, parse_type_or_unknown};

pub(crate) struct TypeEnvironment {
    functions: Vec<FunctionSignature>,
    codec_calls: Vec<CodecCallSignature>,
    effects: Vec<EffectSignature>,
    handlers: Vec<HandlerSignature>,
    schema_symbols: SchemaSymbolTable,
    type_symbols: Vec<NamedSymbol>,
    codec_symbols: Vec<NamedSymbol>,
    pub(crate) uses: Vec<UseDecl>,
    pub(crate) adts: AdtRegistry,
    companion_function_access_targets: BTreeMap<String, String>,
    companion_schema_access_targets: BTreeMap<String, String>,
    companion_effect_access_targets: BTreeMap<String, CompanionAccessTarget>,
}

#[derive(Clone)]
struct SchemaSymbolTable {
    schemas: Vec<SchemaSymbol>,
    aliases: Vec<SchemaAliasSymbol>,
}

#[derive(Clone)]
struct SchemaSymbol {
    name: String,
    module_name: Option<String>,
    visibility: Visibility,
    span: SourceSpan,
    unsupported_format_neutral_encode_field: Option<SchemaField>,
}

#[derive(Clone)]
struct SchemaAliasSymbol {
    name: String,
    module_name: Option<String>,
    target: Vec<String>,
}

struct ResolvedSchemaSymbol {
    name: String,
    module_name: Option<String>,
    span: SourceSpan,
    unsupported_format_neutral_encode_field: Option<SchemaField>,
}

struct SchemaAliasTarget {
    target: Vec<String>,
    module_name: Option<String>,
}

#[derive(Clone)]
struct NamedSymbol {
    name: String,
    module_name: Option<String>,
    visibility: Visibility,
}

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
    Missing,
}

impl<'a> UserEffectPathResolution<'a> {
    pub(crate) fn found(self) -> Option<&'a EffectSignature> {
        match self {
            Self::Found(effect) => Some(effect),
            Self::PrivateCompanionTargetMismatch { .. } | Self::Missing => None,
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
    pub(crate) providers: Vec<HandlerProviderSignature>,
}

#[derive(Clone)]
pub(crate) struct HandlerProviderSignature {
    pub(crate) operation: String,
    pub(crate) provider: Vec<String>,
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

#[derive(Clone)]
pub(crate) struct CodecCallSignature {
    pub(crate) name: String,
    pub(crate) target_name: String,
    pub(crate) boundary: CodecCallBoundary,
    pub(crate) module_name: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) params: Vec<Type>,
    pub(crate) return_type: Type,
    pub(crate) effects: Vec<String>,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodecCallBoundary {
    Direct,
    HandWrittenDecode,
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

type FunctionAstMap<'a> = BTreeMap<FunctionKey, &'a Function>;
type FunctionSignatureMap = BTreeMap<FunctionKey, FunctionSignature>;
type FunctionReturnMap = BTreeMap<FunctionKey, Type>;
type PrivateSlotOmissions = (Vec<bool>, bool);
type PrivateSlotMap = BTreeMap<FunctionKey, PrivateSlotOmissions>;
type PrivateReferenceMap = BTreeMap<FunctionKey, BTreeSet<FunctionKey>>;

#[cfg(test)]
pub(crate) mod private_inference_counters {
    use super::*;

    thread_local! {
        static BODY_RETURN_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static CALL_SITE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRIVATE_REFERENCE_INDEX_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static PRELUDE_CALLBACK_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) body_return_scans: usize,
        pub(crate) call_site_discovery_scans: usize,
        pub(crate) call_site_scans: usize,
        pub(crate) private_reference_candidate_scans: usize,
        pub(crate) private_reference_index_scans: usize,
        pub(crate) prelude_callback_discovery_scans: usize,
        pub(crate) prelude_callback_scans: usize,
    }

    pub(crate) fn reset() {
        BODY_RETURN_SCANS.set(0);
        CALL_SITE_DISCOVERY_SCANS.set(0);
        CALL_SITE_SCANS.set(0);
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(0);
        PRIVATE_REFERENCE_INDEX_SCANS.set(0);
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(0);
        PRELUDE_CALLBACK_SCANS.set(0);
    }

    pub(crate) fn snapshot() -> Snapshot {
        Snapshot {
            body_return_scans: BODY_RETURN_SCANS.get(),
            call_site_discovery_scans: CALL_SITE_DISCOVERY_SCANS.get(),
            call_site_scans: CALL_SITE_SCANS.get(),
            private_reference_candidate_scans: PRIVATE_REFERENCE_CANDIDATE_SCANS.get(),
            private_reference_index_scans: PRIVATE_REFERENCE_INDEX_SCANS.get(),
            prelude_callback_discovery_scans: PRELUDE_CALLBACK_DISCOVERY_SCANS.get(),
            prelude_callback_scans: PRELUDE_CALLBACK_SCANS.get(),
        }
    }

    pub(super) fn record_body_return_scan() {
        BODY_RETURN_SCANS.set(BODY_RETURN_SCANS.get() + 1);
    }

    pub(super) fn record_call_site_discovery_scan() {
        CALL_SITE_DISCOVERY_SCANS.set(CALL_SITE_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_call_site_scan() {
        CALL_SITE_SCANS.set(CALL_SITE_SCANS.get() + 1);
    }

    pub(super) fn record_private_reference_candidate_scan() {
        PRIVATE_REFERENCE_CANDIDATE_SCANS.set(PRIVATE_REFERENCE_CANDIDATE_SCANS.get() + 1);
    }

    pub(super) fn record_private_reference_index_scan() {
        PRIVATE_REFERENCE_INDEX_SCANS.set(PRIVATE_REFERENCE_INDEX_SCANS.get() + 1);
    }

    pub(super) fn record_prelude_callback_discovery_scan() {
        PRELUDE_CALLBACK_DISCOVERY_SCANS.set(PRELUDE_CALLBACK_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_prelude_callback_scan() {
        PRELUDE_CALLBACK_SCANS.set(PRELUDE_CALLBACK_SCANS.get() + 1);
    }
}

#[cfg(test)]
pub(crate) mod effect_inference_counters {
    use super::*;

    thread_local! {
        static DEPENDENCY_DISCOVERY_SCANS: Cell<usize> = const { Cell::new(0) };
        static FUNCTION_BODY_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
        static HANDLER_PROVIDER_EVALUATIONS: Cell<usize> = const { Cell::new(0) };
        static CHANGED_REEVALUATIONS: Cell<usize> = const { Cell::new(0) };
    }

    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) dependency_discovery_scans: usize,
        pub(crate) function_body_collections: usize,
        pub(crate) handler_provider_evaluations: usize,
        pub(crate) changed_reevaluations: usize,
    }

    pub(crate) fn reset() {
        DEPENDENCY_DISCOVERY_SCANS.set(0);
        FUNCTION_BODY_COLLECTIONS.set(0);
        HANDLER_PROVIDER_EVALUATIONS.set(0);
        CHANGED_REEVALUATIONS.set(0);
    }

    pub(crate) fn snapshot() -> Snapshot {
        Snapshot {
            dependency_discovery_scans: DEPENDENCY_DISCOVERY_SCANS.get(),
            function_body_collections: FUNCTION_BODY_COLLECTIONS.get(),
            handler_provider_evaluations: HANDLER_PROVIDER_EVALUATIONS.get(),
            changed_reevaluations: CHANGED_REEVALUATIONS.get(),
        }
    }

    pub(super) fn record_dependency_discovery_scan() {
        DEPENDENCY_DISCOVERY_SCANS.set(DEPENDENCY_DISCOVERY_SCANS.get() + 1);
    }

    pub(super) fn record_function_body_collection() {
        FUNCTION_BODY_COLLECTIONS.set(FUNCTION_BODY_COLLECTIONS.get() + 1);
    }

    pub(super) fn record_handler_provider_evaluation() {
        HANDLER_PROVIDER_EVALUATIONS.set(HANDLER_PROVIDER_EVALUATIONS.get() + 1);
    }

    pub(super) fn record_changed_reevaluation() {
        CHANGED_REEVALUATIONS.set(CHANGED_REEVALUATIONS.get() + 1);
    }
}

impl TypeEnvironment {
    pub(crate) fn from_module(module: &SurfaceModule) -> Self {
        let effects = effect_signatures(module);
        let adts = AdtRegistry::from_module(module);
        let companion_effect_access_targets = companion_access_target_infos(module);
        let mut handlers = handler_signatures(module, &effects, &companion_effect_access_targets);
        let mut functions =
            ordinary_function_signatures(module, &effects, &adts, &companion_effect_access_targets);
        infer_private_function_body_return_types(module, &mut functions, &adts);
        infer_private_function_call_site_signature_types(module, &mut functions, &adts);
        infer_private_function_body_return_types(module, &mut functions, &adts);
        infer_private_prelude_callback_return_types(module, &mut functions, &adts);
        functions.extend(schema_decode_function_signatures(module));
        functions.extend(schema_encode_function_signatures(module));
        functions.extend(schema_validate_function_signatures(module));
        infer_function_and_private_handler_effects(module, &mut functions, &effects, &mut handlers);
        let codec_calls = codec_call_signatures(module, &functions);
        let aliases = function_alias_signatures(module, &functions);
        functions.extend(aliases);
        Self {
            functions,
            codec_calls,
            effects,
            handlers,
            schema_symbols: SchemaSymbolTable::from_module(module),
            type_symbols: named_type_symbols(module),
            codec_symbols: named_codec_symbols(module),
            uses: module.uses.clone(),
            adts,
            companion_function_access_targets: companion_function_access_targets(module),
            companion_schema_access_targets: companion_access_targets(module),
            companion_effect_access_targets,
        }
    }

    pub(crate) fn function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.iter().find(|function| function.name == name)
    }

    pub(crate) fn canonicalize_type_annotation(
        &self,
        ty: Type,
        current_module: Option<&str>,
    ) -> Type {
        canonicalize_type_effects(
            ty,
            &self.uses,
            current_module,
            &self.effects,
            &self.adts,
            &self.companion_effect_access_targets,
        )
    }

    pub(crate) fn user_effect_by_label(
        &self,
        label: &str,
        current_module: Option<&str>,
    ) -> Option<&EffectSignature> {
        self.effects.iter().find(|effect| {
            effect.qualified_name == label
                || (effect.name == label && effect.module_name.as_deref() == current_module)
        })
    }

    pub(crate) fn user_effect_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&EffectSignature> {
        self.resolve_user_effect_path(segments, current_module)
            .found()
    }

    pub(crate) fn resolve_user_effect_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> UserEffectPathResolution<'_> {
        match segments {
            [name] => self.user_effect_by_label(name, current_module).map_or(
                UserEffectPathResolution::Missing,
                UserEffectPathResolution::Found,
            ),
            [_, .., name] => {
                let Some(use_decl) = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                ) else {
                    return UserEffectPathResolution::Missing;
                };
                let module_name = use_decl.name.as_str();
                let Some(effect) = self.effects.iter().find(|effect| {
                    effect.name == *name && effect.module_name.as_deref() == Some(module_name)
                }) else {
                    return UserEffectPathResolution::Missing;
                };
                if imported_effect_is_visible(
                    use_decl,
                    current_module,
                    module_name,
                    effect.visibility,
                    &self.companion_effect_access_targets,
                ) {
                    return UserEffectPathResolution::Found(effect);
                }
                if effect.visibility != Visibility::Public
                    && use_decl.package.is_none()
                    && let Some(access) = current_module
                        .and_then(|module| self.companion_effect_access_targets.get(module))
                    && access.target_module != module_name
                {
                    return UserEffectPathResolution::PrivateCompanionTargetMismatch {
                        effect,
                        access,
                    };
                }
                UserEffectPathResolution::Missing
            }
            _ => UserEffectPathResolution::Missing,
        }
    }

    pub(crate) fn visible_user_effects(
        &self,
        current_module: Option<&str>,
    ) -> Vec<&EffectSignature> {
        self.effects
            .iter()
            .filter(|effect| {
                effect.module_name.as_deref() == current_module
                    || effect.visibility == Visibility::Public
                    || current_module
                        .and_then(|module| self.companion_effect_access_targets.get(module))
                        .is_some_and(|access| {
                            effect.module_name.as_deref() == Some(access.target_module.as_str())
                        })
            })
            .collect()
    }

    pub(crate) fn handler_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> HandlerPathResolution<'_> {
        match segments {
            [name] => self
                .handlers
                .iter()
                .find(|handler| {
                    handler.name == *name && handler.module_name.as_deref() == current_module
                })
                .map_or(HandlerPathResolution::Missing, HandlerPathResolution::Found),
            [_, .., name] => {
                let use_decl = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                );
                let Some(use_decl) = use_decl else {
                    return HandlerPathResolution::Missing;
                };
                let Some(handler) = self.handlers.iter().find(|handler| {
                    handler.name == *name
                        && handler.module_name.as_deref() == Some(use_decl.name.as_str())
                }) else {
                    return HandlerPathResolution::Missing;
                };
                if imported_handler_is_visible(
                    handler,
                    use_decl,
                    current_module,
                    &self.companion_effect_access_targets,
                ) {
                    return HandlerPathResolution::Found(handler);
                }
                if handler.visibility != Visibility::Public
                    && use_decl.package.is_none()
                    && let Some(access) = current_module
                        .and_then(|module| self.companion_effect_access_targets.get(module))
                    && access.target_module != use_decl.name
                {
                    return HandlerPathResolution::PrivateCompanionTargetMismatch {
                        handler,
                        access,
                    };
                }
                HandlerPathResolution::Missing
            }
            _ => HandlerPathResolution::Missing,
        }
    }

    pub(crate) fn function_for(&self, source: &Function) -> Option<&FunctionSignature> {
        let name = source.name.as_deref()?;
        self.functions.iter().find(|function| {
            function.node_id == source.node_id
                && function.name == name
                && function.module_name == source.module_name
                && function.span == source.span
        })
    }

    pub(crate) fn unqualified_function(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> FunctionLookup<'_> {
        if let Some(function) = self.functions.iter().find(|function| {
            function.name == name && function.module_name.as_deref() == current_module
        }) {
            return FunctionLookup::Found(function);
        }

        let mut matches = self.functions.iter().filter(|function| {
            function.name == name
                && function.visibility == Visibility::Public
                && function.module_name.as_deref().is_some_and(|module_name| {
                    self.uses.iter().any(|use_decl| {
                        use_decl.module_name.as_deref() == current_module
                            && use_decl.name.as_str() == module_name
                    })
                })
        });
        let Some(first) = matches.next() else {
            return FunctionLookup::Missing;
        };
        if matches.next().is_some() {
            FunctionLookup::Ambiguous
        } else {
            FunctionLookup::Found(first)
        }
    }

    pub(crate) fn unqualified_function_import_candidates(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Vec<&FunctionSignature> {
        self.functions
            .iter()
            .filter(|function| {
                function.name == name
                    && function.visibility == Visibility::Public
                    && function.module_name.as_deref().is_some_and(|module_name| {
                        self.uses.iter().any(|use_decl| {
                            use_decl.module_name.as_deref() == current_module
                                && use_decl.name.as_str() == module_name
                        })
                    })
            })
            .collect()
    }

    pub(crate) fn function_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_companion_access(segments, current_module, true)
    }

    pub(crate) fn function_path_for_value(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        self.function_path_with_companion_access(segments, current_module, false)
    }

    fn function_path_with_companion_access(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        allow_companion_private_access: bool,
    ) -> Option<&FunctionSignature> {
        match segments {
            [name] => self.function(name),
            [_, .., name] => {
                let use_decl = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                )?;
                let module_name = use_decl.name.as_str();
                self.functions.iter().find(|function| {
                    function.name == *name
                        && function.module_name.as_deref() == Some(module_name)
                        && self.imported_function_is_visible(
                            function,
                            use_decl,
                            current_module,
                            allow_companion_private_access,
                        )
                        && !self.imported_codec_helper_is_hidden(function, use_decl)
                })
            }
            _ => None,
        }
    }

    pub(crate) fn schema_decode_step_signature(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        let schema = self.schema_symbols.schema_target_path(
            schema_path,
            current_module,
            &self.uses,
            true,
            &self.companion_schema_access_targets,
            &mut Vec::new(),
        )?;
        let helper_name = schema_decode_step_function_name(&schema.name);
        self.functions.iter().find(|function| {
            function.name == helper_name
                && function.module_name == schema.module_name
                && self.schema_helper_is_visible(
                    function.visibility,
                    schema.module_name.as_deref(),
                    current_module,
                )
        })
    }

    pub(crate) fn schema_encode_signature(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> Option<&FunctionSignature> {
        let schema = self.schema_symbols.schema_target_path(
            schema_path,
            current_module,
            &self.uses,
            true,
            &self.companion_schema_access_targets,
            &mut Vec::new(),
        )?;
        let helper_name = schema_encode_function_name(&schema.name);
        self.functions.iter().find(|function| {
            function.name == helper_name
                && function.module_name == schema.module_name
                && self.schema_helper_is_visible(
                    function.visibility,
                    schema.module_name.as_deref(),
                    current_module,
                )
        })
    }

    pub(crate) fn unsupported_schema_encode_field(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> Option<UnsupportedSchemaEncodeField> {
        let schema = self.schema_symbols.schema_target_path(
            schema_path,
            current_module,
            &self.uses,
            true,
            &self.companion_schema_access_targets,
            &mut Vec::new(),
        )?;
        let field = schema.unsupported_format_neutral_encode_field.clone()?;
        Some(UnsupportedSchemaEncodeField {
            schema_name: schema.name.clone(),
            schema_span: schema.span.clone(),
            field,
        })
    }

    pub(crate) fn schema_reference_error(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> SchemaReferenceError {
        if self.schema_symbols.private_schema(
            schema_path,
            current_module,
            &self.uses,
            &self.companion_schema_access_targets,
        ) {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::Private,
                resolved_kind: Some("schema"),
            };
        }
        if let Some(alias_target) =
            self.schema_symbols
                .schema_alias_target(schema_path, current_module, &self.uses)
            && let Some(kind) = self.wrong_schema_reference_kind(
                &alias_target.target,
                alias_target.module_name.as_deref(),
            )
        {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::WrongKind,
                resolved_kind: Some(kind),
            };
        }
        if let Some(kind) = self.wrong_schema_reference_kind(schema_path, current_module) {
            return SchemaReferenceError {
                kind: SchemaReferenceErrorKind::WrongKind,
                resolved_kind: Some(kind),
            };
        }
        SchemaReferenceError {
            kind: SchemaReferenceErrorKind::Unresolved,
            resolved_kind: None,
        }
    }

    pub(crate) fn companion_schema_access_target(
        &self,
        current_module: Option<&str>,
    ) -> Option<&str> {
        let current_module = current_module?;
        self.companion_schema_access_targets
            .get(current_module)
            .map(String::as_str)
    }

    fn wrong_schema_reference_kind(
        &self,
        schema_path: &[String],
        current_module: Option<&str>,
    ) -> Option<&'static str> {
        let (name, module_name) = self.resolve_symbol_module(schema_path, current_module)?;
        if self.type_symbols.iter().any(|symbol| {
            symbol.name == name.as_str()
                && symbol.module_name.as_deref() == module_name.as_deref()
                && self.symbol_is_visible(symbol, module_name.as_deref(), current_module)
        }) {
            return Some("type");
        }
        if self.functions.iter().any(|function| {
            function.name == name.as_str()
                && function.module_name.as_deref() == module_name.as_deref()
                && self.symbol_is_visible(function, module_name.as_deref(), current_module)
        }) {
            return Some("function");
        }
        if self.codec_symbols.iter().any(|symbol| {
            symbol.name == name.as_str()
                && symbol.module_name.as_deref() == module_name.as_deref()
                && self.symbol_is_visible(symbol, module_name.as_deref(), current_module)
        }) {
            return Some("codec");
        }
        None
    }

    fn resolve_symbol_module(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<(String, Option<String>)> {
        match segments {
            [name] => Some((name.clone(), current_module.map(str::to_string))),
            [_, .., name] => {
                let use_decl = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                )?;
                Some((name.clone(), Some(use_decl.name.clone())))
            }
            _ => None,
        }
    }

    fn symbol_is_visible(
        &self,
        symbol: &impl SymbolVisibility,
        module_name: Option<&str>,
        current_module: Option<&str>,
    ) -> bool {
        module_name == current_module || symbol.visibility() == Visibility::Public
    }

    fn schema_helper_is_visible(
        &self,
        visibility: Visibility,
        schema_module: Option<&str>,
        current_module: Option<&str>,
    ) -> bool {
        schema_module == current_module
            || visibility == Visibility::Public
            || current_module.is_some_and(|current_module| {
                schema_module.is_some_and(|schema_module| {
                    self.companion_schema_access_targets
                        .get(current_module)
                        .is_some_and(|allowed_target| allowed_target == schema_module)
                })
            })
    }

    fn imported_codec_helper_is_hidden(
        &self,
        function: &FunctionSignature,
        use_decl: &UseDecl,
    ) -> bool {
        function.visibility != Visibility::Public
            && self.codec_calls.iter().any(|codec| {
                codec.module_name.as_deref() == Some(use_decl.name.as_str())
                    && codec.target_name == function.target_name
            })
    }

    fn imported_function_is_visible(
        &self,
        function: &FunctionSignature,
        use_decl: &UseDecl,
        current_module: Option<&str>,
        allow_companion_private_access: bool,
    ) -> bool {
        if function.visibility == Visibility::Public {
            return true;
        }
        if use_decl.package.is_some() {
            return false;
        }
        if current_module.is_some_and(|module| module.starts_with("std::"))
            && function
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))
        {
            return true;
        }
        if !allow_companion_private_access {
            return false;
        }
        current_module.is_some_and(|current_module| {
            function.module_name.as_ref().is_some_and(|target_module| {
                self.companion_function_access_targets
                    .get(current_module)
                    .is_some_and(|allowed_target| allowed_target == target_module)
            })
        })
    }

    pub(crate) fn unqualified_codec_calls(
        &self,
        name: &str,
        current_module: Option<&str>,
    ) -> Vec<&CodecCallSignature> {
        self.codec_calls
            .iter()
            .filter(|codec| codec.name == name && codec.module_name.as_deref() == current_module)
            .collect()
    }

    pub(crate) fn codec_call_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&CodecCallSignature> {
        match segments {
            [name] => self.unqualified_codec_calls(name, current_module),
            [_, .., name] => {
                let Some(use_decl) = imported_use_for_path(
                    &self.uses,
                    &segments[..segments.len() - 1],
                    current_module,
                ) else {
                    return Vec::new();
                };
                let module_name = use_decl.name.as_str();
                self.codec_calls
                    .iter()
                    .filter(|codec| {
                        codec.name == *name
                            && codec.module_name.as_deref() == Some(module_name)
                            && codec.visibility == Visibility::Public
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }
}

pub(crate) fn ordinary_function_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<FunctionSignature> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            let name = function.name.clone()?;
            let (params, variadic) = function_signature_params(function);
            let params = params
                .into_iter()
                .map(|ty| {
                    canonicalize_type_effects(
                        ty,
                        &module.uses,
                        function.module_name.as_deref(),
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect();
            let variadic = variadic.map(|ty| {
                canonicalize_type_effects(
                    ty,
                    &module.uses,
                    function.module_name.as_deref(),
                    effects,
                    adts,
                    companion_effect_access_targets,
                )
            });
            let return_type = canonicalize_type_effects(
                parse_type_or_unknown(function.return_type.as_deref()),
                &module.uses,
                function.module_name.as_deref(),
                effects,
                adts,
                companion_effect_access_targets,
            );
            Some(FunctionSignature {
                target_name: crate::standard_symbols::standard_function_link_name(
                    function.module_name.as_deref(),
                    &name,
                ),
                name,
                module_name: function.module_name.clone(),
                visibility: function.visibility,
                params,
                variadic,
                return_type,
                effects: canonical_declared_effects(
                    function.effects.clone().unwrap_or_default(),
                    &module.uses,
                    function.module_name.as_deref(),
                    effects,
                    companion_effect_access_targets,
                ),
                node_id: function.node_id,
                span: function.span.clone(),
            })
        })
        .collect()
}

fn canonicalize_type_effects(
    ty: Type,
    uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    adts: &AdtRegistry,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Type {
    match ty {
        Type::Named { name, args } => Type::Named {
            name: adts
                .descriptor_for_type_path(&name, args.len(), current_module, uses)
                .map(|descriptor| descriptor.type_name.clone())
                .unwrap_or(name),
            args: args
                .into_iter()
                .map(|arg| {
                    canonicalize_type_effects(
                        arg,
                        uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| {
                    (
                        name,
                        canonicalize_type_effects(
                            ty,
                            uses,
                            current_module,
                            effects,
                            adts,
                            companion_effect_access_targets,
                        ),
                    )
                })
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects: declared,
        } => Type::Function {
            params: params
                .into_iter()
                .map(|param| {
                    canonicalize_type_effects(
                        param,
                        uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .collect(),
            variadic: variadic
                .map(|ty| {
                    canonicalize_type_effects(
                        *ty,
                        uses,
                        current_module,
                        effects,
                        adts,
                        companion_effect_access_targets,
                    )
                })
                .map(Box::new),
            return_type: Box::new(canonicalize_type_effects(
                *return_type,
                uses,
                current_module,
                effects,
                adts,
                companion_effect_access_targets,
            )),
            effects: canonical_declared_effects(
                declared,
                uses,
                current_module,
                effects,
                companion_effect_access_targets,
            ),
        },
        Type::Unknown => Type::Unknown,
    }
}

fn canonical_declared_effects(
    declared: Vec<String>,
    uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<String> {
    let mut canonical = Vec::new();
    for effect in declared {
        if effect.starts_with("...") {
            push_unique_effect(&mut canonical, &effect);
            continue;
        }
        let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
        let label = canonical_user_effect_label(
            &segments,
            uses,
            current_module,
            effects,
            companion_effect_access_targets,
        )
        .unwrap_or(effect);
        push_unique_effect(&mut canonical, &label);
    }
    canonical
}

fn effect_signatures(module: &SurfaceModule) -> Vec<EffectSignature> {
    module
        .effects
        .iter()
        .filter_map(|effect| {
            let name = effect.name.clone()?;
            let qualified_name = if let Some(module_name) = &effect.module_name {
                format!("{module_name}::{name}")
            } else {
                name.clone()
            };
            Some(EffectSignature {
                name,
                qualified_name,
                module_name: effect.module_name.clone(),
                visibility: effect.visibility,
                span: effect.span.clone(),
                operations: effect
                    .operations
                    .iter()
                    .filter_map(|operation| {
                        Some(EffectOperationSignature {
                            name: operation.name.clone()?,
                            params: operation
                                .params
                                .iter()
                                .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                                .collect(),
                            return_type: parse_type_or_unknown(operation.return_type.as_deref()),
                            node_id: operation.node_id,
                            name_span: operation.name_span.clone(),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

fn handler_signatures(
    module: &SurfaceModule,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<HandlerSignature> {
    module
        .handlers
        .iter()
        .filter_map(|handler| {
            let name = handler.name.clone()?;
            let qualified_name = if let Some(module_name) = &handler.module_name {
                format!("{module_name}::{name}")
            } else {
                name.clone()
            };
            let effect = canonical_user_effect_label(
                &handler.effect,
                &module.uses,
                handler.module_name.as_deref(),
                effects,
                companion_effect_access_targets,
            )
            .unwrap_or_else(|| handler.effect.join("::"));
            Some(HandlerSignature {
                name,
                qualified_name,
                module_name: handler.module_name.clone(),
                visibility: handler.visibility,
                params: handler
                    .params
                    .iter()
                    .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                    .collect(),
                effect,
                effects: canonical_declared_effects(
                    handler.effects.clone().unwrap_or_default(),
                    &module.uses,
                    handler.module_name.as_deref(),
                    effects,
                    companion_effect_access_targets,
                ),
                providers: handler
                    .providers
                    .iter()
                    .filter_map(|provider| {
                        Some(HandlerProviderSignature {
                            operation: provider.operation.clone()?,
                            provider: provider.provider.clone(),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum EffectDependencyNode {
    Function(FunctionKey),
    PrivateHandler(String),
}

fn infer_function_and_private_handler_effects(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    user_effects: &[EffectSignature],
    handlers: &mut [HandlerSignature],
) {
    let graph = effect_dependency_graph(module, functions, user_effects, handlers);
    let companion_access_targets = companion_function_access_targets(module);
    let companion_effect_access_targets = companion_access_target_infos(module);
    let provider_companion_access_targets = companion_access_targets_for_signatures(functions);
    let mut effects_by_function = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.effects.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut effects_by_module_path = functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone()?, function.name.clone()),
                (function.effects.clone(), function.visibility),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let handler_index = handlers
        .iter()
        .enumerate()
        .map(|(index, handler)| (handler.qualified_name.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let function_index = functions
        .iter()
        .enumerate()
        .map(|(index, function)| ((function.module_name.clone(), function.name.clone()), index))
        .collect::<BTreeMap<_, _>>();
    let function_ast_by_key = module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut queue = graph.nodes.iter().cloned().collect::<VecDeque<_>>();
    let mut queued = graph.nodes.clone();
    let mut evaluated = BTreeSet::new();

    while let Some(node) = queue.pop_front() {
        queued.remove(&node);
        let is_reevaluation = evaluated.contains(&node);
        let changed = match &node {
            EffectDependencyNode::Function(function_key) => {
                let Some(function) = function_ast_by_key.get(function_key).copied() else {
                    continue;
                };
                if is_reevaluation {
                    #[cfg(test)]
                    effect_inference_counters::record_changed_reevaluation();
                }
                let inferred = collect_function_body_effects(
                    function,
                    module,
                    functions,
                    user_effects,
                    handlers,
                    &effects_by_function,
                    &effects_by_module_path,
                    &companion_access_targets,
                    &companion_effect_access_targets,
                );
                let changed = effects_by_function.get(function_key) != Some(&inferred);
                if changed {
                    effects_by_function.insert(function_key.clone(), inferred.clone());
                    if let Some(module_name) = &function_key.0 {
                        let visibility = function_index
                            .get(function_key)
                            .map(|index| functions[*index].visibility)
                            .unwrap_or(Visibility::Private);
                        effects_by_module_path.insert(
                            (module_name.clone(), function_key.1.clone()),
                            (inferred.clone(), visibility),
                        );
                    }
                    if let Some(index) = function_index.get(function_key).copied() {
                        functions[index].effects = inferred;
                    }
                }
                changed
            }
            EffectDependencyNode::PrivateHandler(qualified_name) => {
                let Some(index) = handler_index.get(qualified_name).copied() else {
                    continue;
                };
                if is_reevaluation {
                    #[cfg(test)]
                    effect_inference_counters::record_changed_reevaluation();
                }
                let inferred = collect_private_handler_effects(
                    &handlers[index],
                    functions,
                    &module.uses,
                    &provider_companion_access_targets,
                );
                let changed = handlers[index].effects != inferred;
                if changed {
                    handlers[index].effects = inferred;
                }
                changed
            }
        };
        evaluated.insert(node.clone());
        if changed && let Some(dependents) = graph.dependents.get(&node) {
            for dependent in dependents {
                if queued.insert(dependent.clone()) {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }
}

struct EffectDependencyGraph {
    nodes: BTreeSet<EffectDependencyNode>,
    dependents: BTreeMap<EffectDependencyNode, BTreeSet<EffectDependencyNode>>,
}

fn effect_dependency_graph(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    user_effects: &[EffectSignature],
    handlers: &[HandlerSignature],
) -> EffectDependencyGraph {
    let companion_access_targets = companion_function_access_targets(module);
    let companion_effect_access_targets = companion_access_target_infos(module);
    let effects_by_function = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.effects.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let effects_by_module_path = functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone()?, function.name.clone()),
                (function.effects.clone(), function.visibility),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut nodes = BTreeSet::new();
    let mut dependents = BTreeMap::<EffectDependencyNode, BTreeSet<EffectDependencyNode>>::new();
    for function in module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
    {
        let Some(name) = &function.name else {
            continue;
        };
        #[cfg(test)]
        effect_inference_counters::record_dependency_discovery_scan();
        let node = EffectDependencyNode::Function((function.module_name.clone(), name.clone()));
        nodes.insert(node.clone());
        for dependency in function_effect_dependencies(
            function,
            module,
            functions,
            handlers,
            user_effects,
            &effects_by_function,
            &effects_by_module_path,
            &companion_access_targets,
            &companion_effect_access_targets,
        ) {
            dependents
                .entry(dependency)
                .or_default()
                .insert(node.clone());
        }
    }
    for handler in handlers
        .iter()
        .filter(|handler| handler.visibility != Visibility::Public)
    {
        #[cfg(test)]
        effect_inference_counters::record_dependency_discovery_scan();
        let node = EffectDependencyNode::PrivateHandler(handler.qualified_name.clone());
        nodes.insert(node.clone());
        for provider in &handler.providers {
            if let Some(function) = function_signature_path(
                &provider.provider,
                &module.uses,
                functions,
                handler.module_name.as_deref(),
                &companion_access_targets,
            ) {
                let dependency = EffectDependencyNode::Function((
                    function.module_name.clone(),
                    function.name.clone(),
                ));
                dependents
                    .entry(dependency)
                    .or_default()
                    .insert(node.clone());
            }
        }
    }
    EffectDependencyGraph { nodes, dependents }
}

fn collect_private_handler_effects(
    handler: &HandlerSignature,
    functions: &[FunctionSignature],
    uses: &[UseDecl],
    companion_access_targets: &BTreeMap<String, String>,
) -> Vec<String> {
    #[cfg(test)]
    effect_inference_counters::record_handler_provider_evaluation();
    let mut inferred = Vec::new();
    for provider in &handler.providers {
        if let Some(function) = function_signature_path(
            &provider.provider,
            uses,
            functions,
            handler.module_name.as_deref(),
            companion_access_targets,
        ) {
            for effect in &function.effects {
                push_unique_effect(&mut inferred, effect);
            }
        }
    }
    inferred
}

fn collect_function_body_effects(
    function: &Function,
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    user_effects: &[EffectSignature],
    handlers: &[HandlerSignature],
    effects_by_function: &BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &BTreeMap<String, String>,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Vec<String> {
    #[cfg(test)]
    effect_inference_counters::record_function_body_collection();
    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let function_key = (
        function.module_name.clone(),
        function.name.clone().unwrap_or_default(),
    );
    let mut inferred = effects_by_function
        .get(&function_key)
        .cloned()
        .unwrap_or_default();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let context = ExprEffectContext {
                    uses: &module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions,
                    effects_by_function,
                    effects_by_module_path,
                    companion_access_targets,
                    companion_effect_access_targets,
                    user_effects,
                    handlers,
                };
                collect_expr_effects(expr, &context, &mut inferred);
                let ty = parse_type_or_unknown(annotation.as_deref());
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                let context = ExprEffectContext {
                    uses: &module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions,
                    effects_by_function,
                    effects_by_module_path,
                    companion_access_targets,
                    companion_effect_access_targets,
                    user_effects,
                    handlers,
                };
                collect_expr_effects(expr, &context, &mut inferred);
            }
        }
    }
    inferred
}

fn function_effect_dependencies(
    function: &Function,
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    handlers: &[HandlerSignature],
    user_effects: &[EffectSignature],
    effects_by_function: &BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &BTreeMap<String, String>,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> BTreeSet<EffectDependencyNode> {
    let mut dependencies = BTreeSet::new();
    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let context = ExprEffectContext {
                    uses: &module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions,
                    effects_by_function,
                    effects_by_module_path,
                    companion_access_targets,
                    companion_effect_access_targets,
                    user_effects,
                    handlers,
                };
                collect_expr_effect_dependencies(expr, &context, &mut dependencies);
                let ty = parse_type_or_unknown(annotation.as_deref());
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                let context = ExprEffectContext {
                    uses: &module.uses,
                    current_module: function.module_name.as_deref(),
                    bindings: &bindings,
                    functions,
                    effects_by_function,
                    effects_by_module_path,
                    companion_access_targets,
                    companion_effect_access_targets,
                    user_effects,
                    handlers,
                };
                collect_expr_effect_dependencies(expr, &context, &mut dependencies);
            }
        }
    }
    dependencies
}

pub(crate) fn canonical_user_effect_label(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    effects: &[EffectSignature],
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> Option<String> {
    match segments {
        [name] => effects
            .iter()
            .find(|effect| effect.name == *name && effect.module_name.as_deref() == current_module)
            .map(|effect| effect.qualified_name.clone()),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            effects
                .iter()
                .find(|effect| {
                    effect.name == *name
                        && effect.module_name.as_deref() == Some(use_decl.name.as_str())
                        && imported_effect_is_visible(
                            use_decl,
                            current_module,
                            use_decl.name.as_str(),
                            effect.visibility,
                            companion_effect_access_targets,
                        )
                })
                .map(|effect| effect.qualified_name.clone())
        }
        _ => None,
    }
}

pub(crate) fn imported_effect_is_visible(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    target_module: &str,
    visibility: Visibility,
    companion_effect_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> bool {
    visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                (current_module.starts_with("std::") && target_module.starts_with("std::"))
                    || companion_effect_access_targets
                        .get(current_module)
                        .is_some_and(|access| access.target_module == target_module)
            }))
}

fn function_signature_params(function: &veln_ast::Function) -> (Vec<Type>, Option<Type>) {
    let mut params = Vec::new();
    let mut variadic = None;
    for param in &function.params {
        let ty = parse_type_or_unknown(param.ty.as_deref());
        if param.is_variadic {
            variadic = Some(ty);
        } else {
            params.push(ty);
        }
    }
    (params, variadic)
}

fn infer_private_function_body_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let mut changed = true;
    while changed {
        changed = false;
        let signatures_by_path = signatures_by_path(functions);
        let omitted_private_returns = omitted_private_returns_that_can_change(module, functions);
        if omitted_private_returns.is_empty() {
            return;
        }
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && private_function_key(function)
                    .is_some_and(|key| omitted_private_returns.contains(&key))
        }) {
            let Some(name) = &function.name else {
                continue;
            };
            let key = (function.module_name.clone(), name.clone());
            let inferred = infer_private_function_tail_type(
                function,
                &module.uses,
                &signatures_by_path,
                &returns_by_path,
                adts,
            );
            if inferred == Type::Unknown {
                continue;
            }
            let Some(signature) = functions
                .iter_mut()
                .find(|signature| signature.module_name == key.0 && signature.name == key.1)
            else {
                continue;
            };
            if signature.return_type == inferred {
                continue;
            }
            if !type_has_unknown(&signature.return_type) {
                continue;
            }
            signature.return_type = inferred;
            changed = true;
        }
    }
}

fn infer_private_function_call_site_signature_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let initial_omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
    if initial_omitted_private_slots.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_slot_omissions(&initial_omitted_private_slots),
        &initial_omitted_private_slots
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>(),
    );
    let contributors = private_call_site_constraint_contributors(
        module,
        &initial_omitted_private_slots,
        &private_references,
    );
    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_slots = omitted_private_slots_that_can_change(module, functions);
        if omitted_private_slots.is_empty() {
            return;
        }
        let signatures_by_path = signatures_by_path_with_aliases(module, functions);
        let returns_by_path = returns_by_path(functions);
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_call_site_constraints(
                function,
                &mut PrivateCallSiteConstraintContext {
                    uses: &module.uses,
                    function_by_path: &function_by_path,
                    omitted_private_slots: &omitted_private_slots,
                    signatures_by_path: &signatures_by_path,
                    returns_by_path: &returns_by_path,
                    functions,
                    adts,
                    changed: &mut changed,
                },
            );
        }
    }
}

fn function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

fn private_function_key(function: &Function) -> Option<FunctionKey> {
    Some((function.module_name.clone(), function.name.clone()?))
}

fn signature_for_key<'a>(
    functions: &'a [FunctionSignature],
    key: &FunctionKey,
) -> Option<&'a FunctionSignature> {
    functions
        .iter()
        .find(|signature| signature.module_name == key.0 && signature.name == key.1)
}

fn omitted_private_returns_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let can_change = signature_for_key(functions, &key)
                .is_some_and(|signature| type_has_unknown(&signature.return_type));
            can_change.then_some(key)
        })
        .collect()
}

fn omitted_private_slots_that_can_change(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> PrivateSlotMap {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.name.is_some()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            let omitted_params = function
                .params
                .iter()
                .enumerate()
                .map(|(index, param)| {
                    if !parameter_annotation_is_omitted(param) {
                        return false;
                    }
                    if param.is_variadic {
                        signature.variadic.as_ref().is_some_and(type_has_unknown)
                    } else {
                        signature.params.get(index).is_some_and(type_has_unknown)
                    }
                })
                .collect::<Vec<_>>();
            let omitted_return =
                function.return_type.is_none() && type_has_unknown(&signature.return_type);
            (omitted_params.iter().any(|omitted| *omitted) || omitted_return)
                .then_some((key, (omitted_params, omitted_return)))
        })
        .collect()
}

fn modules_with_private_slot_omissions(
    omitted_private_slots: &PrivateSlotMap,
) -> BTreeSet<Option<String>> {
    omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect()
}

fn modules_with_private_return_omissions(
    omitted_private_returns: &BTreeSet<FunctionKey>,
) -> BTreeSet<Option<String>> {
    omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect()
}

fn omitted_private_returns_requiring_prelude_pass(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    module
        .functions
        .iter()
        .filter(|function| {
            function.kind == FunctionKind::Function
                && function.visibility == Visibility::Private
                && function.return_type.is_none()
        })
        .filter_map(|function| {
            let key = private_function_key(function)?;
            let signature = signature_for_key(functions, &key)?;
            (type_has_unknown(&signature.return_type)
                || private_tail_can_use_expected(function, &signature.return_type, uses, adts))
            .then_some(key)
        })
        .collect()
}

fn private_reference_map(
    module: &SurfaceModule,
    function_by_path: &FunctionAstMap<'_>,
    modules_with_omitted_slots: &BTreeSet<Option<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> PrivateReferenceMap {
    let candidates_by_module = private_reference_candidates_by_module(omitted_private_keys);
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter(|function| {
            private_function_needs_reference_index(
                function,
                function_by_path,
                &candidates_by_module,
                omitted_private_keys,
            )
        })
        .filter_map(|function| {
            let key = function_key(function)?;
            let mut references = BTreeSet::new();
            #[cfg(test)]
            private_inference_counters::record_private_reference_index_scan();
            collect_private_function_references(function, function_by_path, &mut references);
            Some((key, references))
        })
        .collect()
}

fn private_reference_candidates_by_module(
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> BTreeMap<Option<String>, BTreeSet<String>> {
    let mut candidates: BTreeMap<Option<String>, BTreeSet<String>> = BTreeMap::new();
    for (module_name, name) in omitted_private_keys {
        candidates
            .entry(module_name.clone())
            .or_default()
            .insert(name.clone());
    }
    candidates
}

fn private_function_needs_reference_index(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates_by_module: &BTreeMap<Option<String>, BTreeSet<String>>,
    omitted_private_keys: &BTreeSet<FunctionKey>,
) -> bool {
    let Some(key) = function_key(function) else {
        return false;
    };
    if omitted_private_keys.contains(&key) {
        return true;
    }
    let Some(candidates) = candidates_by_module.get(&function.module_name) else {
        return false;
    };
    #[cfg(test)]
    private_inference_counters::record_private_reference_candidate_scan();
    private_function_mentions_candidate(function, function_by_path, candidates)
}

fn private_function_mentions_candidate(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
) -> bool {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        if private_line_mentions_candidate(
            line,
            current_module,
            function_by_path,
            candidates,
            &mut bindings,
        ) {
            return true;
        }
    }
    false
}

fn private_line_mentions_candidate(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &mut Vec<Binding>,
) -> bool {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            let mentions = private_expr_mentions_candidate(
                expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
            mentions
        }
        BodyLineKind::Expr { expr } => private_expr_mentions_candidate(
            expr,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
    }
}

fn private_expr_mentions_candidate(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    candidates: &BTreeSet<String>,
    bindings: &[Binding],
) -> bool {
    if private_expr_reference_target(expr, current_module, function_by_path, bindings)
        .is_some_and(|key| key.0.as_deref() == current_module && candidates.contains(&key.1))
    {
        return true;
    }
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            private_expr_mentions_candidate(
                item,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            private_expr_mentions_candidate(
                &entry.key,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                &entry.value,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            private_expr_mentions_candidate(
                &field.expr,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Call { callee, args } => {
            private_expr_mentions_candidate(
                callee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::Perform { args, .. } => args.iter().any(|arg| {
            private_expr_mentions_candidate(
                arg,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }),
        ExprKind::Handle { body, args, .. } => {
            private_expr_mentions_candidate(
                body,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || args.iter().any(|arg| {
                private_expr_mentions_candidate(
                    arg,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            })
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_expr_mentions_candidate(
                input,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                base,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => private_expr_mentions_candidate(
            value,
            current_module,
            function_by_path,
            candidates,
            bindings,
        ),
        ExprKind::Match { scrutinee, arms } => {
            private_expr_mentions_candidate(
                scrutinee,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || arms.iter().any(|arm| {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                private_expr_mentions_candidate(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    &arm_bindings,
                )
            })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_expr_mentions_candidate(
                condition,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                then_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || else_if_branches.iter().any(|branch| {
                private_expr_mentions_candidate(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                ) || private_expr_mentions_candidate(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    candidates,
                    bindings,
                )
            }) || private_expr_mentions_candidate(
                else_branch,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::Binary { left, right, .. } => {
            private_expr_mentions_candidate(
                left,
                current_module,
                function_by_path,
                candidates,
                bindings,
            ) || private_expr_mentions_candidate(
                right,
                current_module,
                function_by_path,
                candidates,
                bindings,
            )
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => false,
    }
}

fn private_reference_initial_bindings(function: &Function) -> Vec<Binding> {
    function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect()
}

fn collect_private_reference_pattern_bindings(pattern: &Pattern, bindings: &mut Vec<Binding>) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(Binding::new(name.clone(), Type::Unknown)),
        PatternKind::Record(fields) => {
            for field in fields {
                collect_private_reference_pattern_bindings(&field.pattern, bindings);
            }
        }
        PatternKind::Constructor { args, .. } => {
            for arg in args {
                collect_private_reference_pattern_bindings(arg, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit => {}
    }
}

fn collect_private_function_references(
    function: &Function,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
) {
    let current_module = function.module_name.as_deref();
    let mut bindings = private_reference_initial_bindings(function);
    for line in &function.body {
        collect_private_line_references(
            line,
            current_module,
            function_by_path,
            references,
            &mut bindings,
        );
    }
}

fn collect_private_line_references(
    line: &BodyLine,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &line.kind {
        BodyLineKind::Let { pattern, expr, .. } => {
            collect_private_expr_references(
                expr,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            let initializer_private_function =
                private_expr_reference_target(expr, current_module, function_by_path, bindings);
            collect_let_pattern_bindings(
                pattern,
                &Type::Unknown,
                initializer_private_function,
                bindings,
            );
        }
        BodyLineKind::Expr { expr } => collect_private_expr_references(
            expr,
            current_module,
            function_by_path,
            references,
            bindings,
        ),
    }
}

fn collect_private_expr_references(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    references: &mut BTreeSet<FunctionKey>,
    bindings: &[Binding],
) {
    if let Some(key) =
        private_expr_reference_target(expr, current_module, function_by_path, bindings)
    {
        references.insert(key);
    }
    match &expr.kind {
        ExprKind::List(items) => {
            for item in items {
                collect_private_expr_references(
                    item,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_private_expr_references(
                    &entry.key,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &entry.value,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_private_expr_references(
                    &field.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_expr_references(
                callee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_expr_references(
                body,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arg in args {
                collect_private_expr_references(
                    arg,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_expr_references(
                input,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                base,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            collect_private_expr_references(
                value,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_expr_references(
                scrutinee,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for arm in arms {
                let mut arm_bindings = bindings.to_vec();
                collect_private_reference_pattern_bindings(&arm.pattern, &mut arm_bindings);
                collect_private_expr_references(
                    &arm.expr,
                    current_module,
                    function_by_path,
                    references,
                    &arm_bindings,
                );
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_expr_references(
                condition,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                then_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            for branch in else_if_branches {
                collect_private_expr_references(
                    &branch.condition,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
                collect_private_expr_references(
                    &branch.expr,
                    current_module,
                    function_by_path,
                    references,
                    bindings,
                );
            }
            collect_private_expr_references(
                else_branch,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_expr_references(
                left,
                current_module,
                function_by_path,
                references,
                bindings,
            );
            collect_private_expr_references(
                right,
                current_module,
                function_by_path,
                references,
                bindings,
            );
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn private_expr_reference_target(
    expr: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &expr.kind else {
        return None;
    };
    private_reference_name_path_target(segments, current_module, function_by_path, bindings)
}

fn private_reference_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
    bindings: &[Binding],
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == *name) {
        return binding.private_function_value.clone();
    }
    private_name_path_target(segments, current_module, function_by_path)
}

fn signatures_by_path(functions: &[FunctionSignature]) -> FunctionSignatureMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.clone(),
            )
        })
        .collect()
}

fn private_call_site_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_slots: &PrivateSlotMap,
    private_references: &PrivateReferenceMap,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_slots = omitted_private_slots
        .keys()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_slots.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            #[cfg(test)]
            private_inference_counters::record_call_site_discovery_scan();
            (omitted_private_slots.contains_key(&key)
                || private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_slots.contains_key(reference))
                }))
            .then_some(key)
        })
        .collect()
}

fn signatures_by_path_with_aliases(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> FunctionSignatureMap {
    let mut signatures = signatures_by_path(functions);
    for alias in function_alias_signatures(module, functions) {
        let key = (alias.module_name.clone(), alias.name.clone());
        signatures.entry(key).or_insert(alias);
    }
    signatures
}

fn returns_by_path(functions: &[FunctionSignature]) -> FunctionReturnMap {
    functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect()
}

struct PrivateCallSiteConstraintContext<'a> {
    uses: &'a [UseDecl],
    function_by_path: &'a FunctionAstMap<'a>,
    omitted_private_slots: &'a PrivateSlotMap,
    signatures_by_path: &'a FunctionSignatureMap,
    returns_by_path: &'a FunctionReturnMap,
    functions: &'a mut [FunctionSignature],
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

struct PrivateCallSiteExprContext<'a, 'b> {
    current_module: Option<&'b str>,
    caller_key: Option<&'b FunctionKey>,
    bindings: &'b [Binding],
    constraints: &'b mut PrivateCallSiteConstraintContext<'a>,
}

fn collect_private_call_site_constraints(
    function: &Function,
    context: &mut PrivateCallSiteConstraintContext<'_>,
) {
    #[cfg(test)]
    private_inference_counters::record_call_site_scan();

    let current_module = function.module_name.as_deref();
    let caller_key = function
        .name
        .as_ref()
        .map(|name| (function.module_name.clone(), name.clone()));
    let mut bindings = private_function_body_bindings(function, context.signatures_by_path);
    let declared_return = function.return_type.as_deref().map_or_else(
        || {
            caller_key
                .as_ref()
                .and_then(|key| context.signatures_by_path.get(key))
                .map(|signature| signature.return_type.clone())
                .filter(|ty| !type_has_unknown(ty))
        },
        |return_type| Some(parse_type_or_unknown(Some(return_type))),
    );

    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_call_site_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            current_module,
                            context.function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        current_module,
                        context.uses,
                        &bindings,
                        context.returns_by_path,
                        context.adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_call_site_expr_constraints(
                    expr,
                    expected,
                    &mut PrivateCallSiteExprContext {
                        current_module,
                        caller_key: caller_key.as_ref(),
                        bindings: &bindings,
                        constraints: context,
                    },
                );
            }
        }
    }
}

fn collect_private_call_site_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_call_site_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_call_site_expr_constraints(&entry.key, key_expected, context);
                collect_private_call_site_expr_constraints(&entry.value, value_expected, context);
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_call_site_expr_constraints(&field.expr, field_expected, context);
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_call_site_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_call_site_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_call_site_expr_constraints(body, expected, context);
            for arg in args {
                collect_private_call_site_expr_constraints(arg, None, context);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_call_site_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_call_site_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_call_site_expr_constraints(value, None, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_call_site_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
                arms,
                context.current_module,
                context.constraints.uses,
                context.constraints.adts,
            ) {
                MatchScrutineePatternInference::Inferred(ty) => Some(ty),
                MatchScrutineePatternInference::Uninferred
                | MatchScrutineePatternInference::Ambiguous(_) => None,
            };
            collect_private_call_site_expr_constraints(
                scrutinee,
                scrutinee_expected.as_ref(),
                context,
            );
            for arm in arms {
                collect_private_call_site_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_call_site_expr_constraints(condition, Some(&Type::bool()), context);
            collect_private_call_site_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_call_site_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_call_site_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_call_site_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_call_site_expr_constraints(left, expected, context);
            collect_private_call_site_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            collect_private_parameter_constraints(segments, expected, context);
            collect_private_function_value_constraints(segments, expected, context);
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_parameter_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(expected) = expected.filter(|ty| !type_has_unknown(ty)) else {
        return;
    };
    let [name] = segments else {
        return;
    };
    let Some(caller_key) = context.caller_key else {
        return;
    };
    let Some((omitted_params, _)) = context.constraints.omitted_private_slots.get(caller_key)
    else {
        return;
    };
    let Some(function) = context.constraints.function_by_path.get(caller_key) else {
        return;
    };
    let Some(index) = function
        .params
        .iter()
        .position(|param| param.name == *name && parameter_annotation_is_omitted(param))
    else {
        return;
    };
    if !omitted_params.get(index).copied().unwrap_or(false) {
        return;
    }
    if function.params[index].is_variadic {
        let Some(item_type) = expected.vec_part().filter(|ty| !type_has_unknown(ty)) else {
            return;
        };
        update_private_signature_variadic(
            context.constraints.functions,
            caller_key,
            item_type.clone(),
            context.constraints.changed,
        );
    } else {
        update_private_signature_param(
            context.constraints.functions,
            caller_key,
            index,
            expected.clone(),
            context.constraints.changed,
        );
    }
}

fn collect_private_call_site_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let Some(target_key) = private_same_module_call_target(
        callee,
        context.current_module,
        context.constraints.function_by_path,
    ) else {
        collect_private_call_site_non_target_call_args(callee, args, expected, context);
        return;
    };

    let is_recursive_edge = context.caller_key == Some(&target_key);
    if !is_recursive_edge
        && let Some((omitted_params, omitted_return)) =
            context.constraints.omitted_private_slots.get(&target_key)
    {
        if let Some(target_params) = context
            .constraints
            .signatures_by_path
            .get(&target_key)
            .map(|signature| signature.params.clone())
        {
            for (index, arg) in args.iter().enumerate() {
                if omitted_params.get(index).copied().unwrap_or(false) {
                    let actual = infer_private_signature_expr_type(
                        arg,
                        None,
                        context.current_module,
                        context.constraints.uses,
                        context.bindings,
                        context.constraints.returns_by_path,
                        context.constraints.adts,
                    );
                    if !type_has_unknown(&actual) {
                        update_private_signature_param(
                            context.constraints.functions,
                            &target_key,
                            index,
                            actual,
                            context.constraints.changed,
                        );
                    }
                }
                let arg_expected = target_params
                    .get(index)
                    .filter(|ty| private_expected_can_constrain(ty));
                collect_private_call_site_expr_constraints(arg, arg_expected, context);
            }
        }

        if *omitted_return
            && let Some(expected) = expected
            && !type_has_unknown(expected)
        {
            update_private_signature_return(
                context.constraints.functions,
                &target_key,
                expected.clone(),
                context.constraints.changed,
            );
        }
    }

    if context
        .constraints
        .omitted_private_slots
        .contains_key(&target_key)
    {
        return;
    }
    let Some(target_params) = context
        .constraints
        .signatures_by_path
        .get(&target_key)
        .map(|signature| signature.params.clone())
    else {
        return;
    };
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = target_params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

fn collect_private_call_site_non_target_call_args(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        for arg in args {
            collect_private_call_site_expr_constraints(arg, None, context);
        }
        return;
    };
    let params = private_call_site_non_target_params(segments, args, expected, context);
    for (index, arg) in args.iter().enumerate() {
        let arg_expected = params
            .get(index)
            .filter(|ty| private_expected_can_constrain(ty));
        collect_private_call_site_expr_constraints(arg, arg_expected, context);
    }
}

fn private_expected_can_constrain(ty: &Type) -> bool {
    if !type_has_unknown(ty) {
        return true;
    }
    matches!(
        ty,
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } if !variadic.as_deref().is_some_and(type_has_unknown)
            && (params.iter().any(|param| !type_has_unknown(param))
            || !type_has_unknown(return_type)
            || variadic.as_deref().is_some_and(|ty| !type_has_unknown(ty)))
    )
}

fn private_call_site_non_target_params(
    segments: &[String],
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Vec<Type> {
    if let crate::adt::ConstructorLookup::Found(constructor) = context.constraints.adts.constructor(
        segments,
        context.current_module,
        context.constraints.uses,
    ) {
        return expected
            .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
            .map(|_| {
                constructor
                    .variant
                    .payload_fields
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        expected
                            .and_then(|expected| adt::payload_type(expected, constructor, index))
                            .filter(|ty| !type_has_unknown(ty))
                            .unwrap_or(Type::Unknown)
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
    }

    if let Some(signature) = private_call_site_declared_signature(
        segments,
        context.current_module,
        context.constraints.uses,
        context.constraints.signatures_by_path,
    )
    .filter(|signature| {
        context.current_module == Some("std::prelude")
            || signature.module_name.as_deref() != Some("std::prelude")
    }) {
        return signature.params.clone();
    }

    private_prelude_constraint_name(
        segments,
        context.current_module,
        context.constraints.function_by_path,
    )
    .and_then(|name| {
        let input_type = private_prelude_input_arg(args, name).map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.constraints.uses,
                context.bindings,
                context.constraints.returns_by_path,
                context.constraints.adts,
            )
        });
        let mut params =
            crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
                .map(|(params, _)| params)?;
        if name == "vec_try_map_with" {
            let context_type = args.first().map(|arg| {
                infer_private_signature_expr_type(
                    arg,
                    None,
                    context.current_module,
                    context.constraints.uses,
                    context.bindings,
                    context.constraints.returns_by_path,
                    context.constraints.adts,
                )
            });
            apply_vec_try_map_with_context_param(&mut params, context_type);
        }
        Some(params)
    })
    .unwrap_or_default()
}

fn private_prelude_input_arg<'a>(args: &'a [Expr], helper_name: &str) -> Option<&'a Expr> {
    match helper_name {
        "vec_try_map_with" | "dict_map_with" | "dict_filter_with" | "dict_fold_with"
        | "dict_try_map_with" => args.get(1),
        _ => args.first(),
    }
}

fn collect_private_function_value_constraints(
    segments: &[String],
    expected: Option<&Type>,
    context: &mut PrivateCallSiteExprContext<'_, '_>,
) {
    let expected = expected.filter(|ty| private_expected_can_constrain(ty));
    let Some(Type::Function {
        params,
        variadic,
        return_type,
        ..
    }) = expected
    else {
        return;
    };
    let Some(target_key) = private_function_value_target(segments, context) else {
        return;
    };
    if context.caller_key == Some(&target_key) {
        return;
    }
    let Some(target_function) = context.constraints.function_by_path.get(&target_key) else {
        return;
    };
    let Some((omitted_params, omitted_return)) =
        context.constraints.omitted_private_slots.get(&target_key)
    else {
        return;
    };
    for (index, param) in params.iter().enumerate() {
        if omitted_params.get(index).copied().unwrap_or(false) && !type_has_unknown(param) {
            update_private_signature_param(
                context.constraints.functions,
                &target_key,
                index,
                param.clone(),
                context.constraints.changed,
            );
        }
    }
    if let Some(variadic) = variadic.as_deref().filter(|ty| !type_has_unknown(ty))
        && let Some(index) = target_function
            .params
            .iter()
            .position(|param| param.is_variadic && parameter_annotation_is_omitted(param))
        && omitted_params.get(index).copied().unwrap_or(false)
    {
        update_private_signature_variadic(
            context.constraints.functions,
            &target_key,
            variadic.clone(),
            context.constraints.changed,
        );
    }
    if *omitted_return && !type_has_unknown(return_type) {
        update_private_signature_return(
            context.constraints.functions,
            &target_key,
            return_type.as_ref().clone(),
            context.constraints.changed,
        );
    }
}

fn private_function_value_target(
    segments: &[String],
    context: &PrivateCallSiteExprContext<'_, '_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    if let Some(binding) = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
    {
        return binding.private_function_value.clone();
    }
    Some((context.current_module.map(str::to_string), name.clone()))
}

fn private_call_site_declared_signature<'a>(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    signatures_by_path: &'a FunctionSignatureMap,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => signatures_by_path.get(&(current_module.map(str::to_string), name.clone())),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    signatures_by_path.get(&(Some(use_decl.name.clone()), name.clone()))
                })
                .filter(|signature| signature.visibility == Visibility::Public)
        }
        _ => None,
    }
}

fn parameter_annotation_is_omitted(param: &veln_ast::Param) -> bool {
    param
        .ty
        .as_deref()
        .is_none_or(|annotation| param.is_variadic && annotation.is_empty())
}

fn private_name_path_target(
    segments: &[String],
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let [name] = segments else {
        return None;
    };
    let key = (current_module.map(str::to_string), name.clone());
    let function = function_by_path.get(&key)?;
    (function.kind == FunctionKind::Function && function.visibility == Visibility::Private)
        .then_some(key)
}

fn private_same_module_call_target(
    callee: &Expr,
    current_module: Option<&str>,
    function_by_path: &FunctionAstMap<'_>,
) -> Option<FunctionKey> {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return None;
    };
    private_name_path_target(segments, current_module, function_by_path)
}

fn update_private_signature_param(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    index: usize,
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    let Some(current) = signature.params.get_mut(index) else {
        return;
    };
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}

fn update_private_signature_variadic(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    let Some(current) = signature.variadic.as_mut() else {
        return;
    };
    if type_has_unknown(current) {
        *current = inferred;
        *changed = true;
    }
}

fn update_private_signature_return(
    functions: &mut [FunctionSignature],
    key: &(Option<String>, String),
    inferred: Type,
    changed: &mut bool,
) {
    let Some(signature) = functions
        .iter_mut()
        .find(|function| function.module_name == key.0 && function.name == key.1)
    else {
        return;
    };
    if type_has_unknown(&signature.return_type) {
        signature.return_type = inferred;
        *changed = true;
    }
}

fn infer_private_prelude_callback_return_types(
    module: &SurfaceModule,
    functions: &mut [FunctionSignature],
    adts: &AdtRegistry,
) {
    let function_by_path = module
        .functions
        .iter()
        .filter_map(|function| {
            Some((
                (function.module_name.clone(), function.name.clone()?),
                function,
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let mut returns_by_path = functions
        .iter()
        .map(|function| {
            (
                (function.module_name.clone(), function.name.clone()),
                function.return_type.clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let initial_omitted_private_returns =
        omitted_private_returns_requiring_prelude_pass(module, functions, &module.uses, adts);
    if initial_omitted_private_returns.is_empty() {
        return;
    }
    let private_references = private_reference_map(
        module,
        &function_by_path,
        &modules_with_private_return_omissions(&initial_omitted_private_returns),
        &initial_omitted_private_returns,
    );
    let contributors = private_prelude_callback_constraint_contributors(
        module,
        &initial_omitted_private_returns,
        &returns_by_path,
        &function_by_path,
        &private_references,
        &module.uses,
        adts,
    );
    if contributors.is_empty() {
        return;
    }

    let mut changed = true;
    while changed {
        changed = false;
        let omitted_private_returns = initial_omitted_private_returns.clone();
        for function in module.functions.iter().filter(|function| {
            function_key(function).is_some_and(|key| contributors.contains(&key))
        }) {
            collect_private_prelude_callback_return_constraints(
                function,
                &module.uses,
                &function_by_path,
                &omitted_private_returns,
                &mut returns_by_path,
                adts,
                &mut changed,
            );
        }
        for function in functions.iter_mut() {
            let key = (function.module_name.clone(), function.name.clone());
            if !omitted_private_returns.contains(&key) {
                continue;
            }
            if let Some(inferred) = returns_by_path.get(&key)
                && inferred != &function.return_type
            {
                function.return_type = inferred.clone();
            }
        }
    }
}

fn collect_private_prelude_callback_return_constraints(
    function: &Function,
    uses: &[UseDecl],
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
    omitted_private_returns: &BTreeSet<(Option<String>, String)>,
    returns_by_path: &mut BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
    changed: &mut bool,
) {
    #[cfg(test)]
    private_inference_counters::record_prelude_callback_scan();

    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    annotation_type.as_ref(),
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            function.module_name.as_deref(),
                            function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                collect_private_prelude_callback_expr_constraints(
                    expr,
                    expected,
                    &mut PrivatePreludeCallbackConstraintContext {
                        current_module: function.module_name.as_deref(),
                        uses,
                        bindings: &bindings,
                        function_by_path,
                        omitted_private_returns,
                        returns_by_path,
                        adts,
                        changed,
                    },
                );
            }
        }
    }
}

fn private_prelude_callback_constraint_contributors(
    module: &SurfaceModule,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    private_references: &PrivateReferenceMap,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> BTreeSet<FunctionKey> {
    let modules_with_omitted_returns = omitted_private_returns
        .iter()
        .map(|key| key.0.clone())
        .collect::<BTreeSet<_>>();
    module
        .functions
        .iter()
        .filter(|function| modules_with_omitted_returns.contains(&function.module_name))
        .filter_map(|function| {
            let key = function_key(function)?;
            if !omitted_private_returns.contains(&key)
                && !private_references.get(&key).is_some_and(|references| {
                    references
                        .iter()
                        .any(|reference| omitted_private_returns.contains(reference))
                })
            {
                return None;
            }
            private_prelude_callback_function_can_constrain(
                function,
                &key,
                omitted_private_returns,
                returns_by_path,
                function_by_path,
                uses,
                adts,
            )
            .then_some(key)
        })
        .collect()
}

fn private_prelude_callback_function_can_constrain(
    function: &Function,
    key: &FunctionKey,
    omitted_private_returns: &BTreeSet<FunctionKey>,
    returns_by_path: &FunctionReturnMap,
    function_by_path: &FunctionAstMap<'_>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    if omitted_private_returns.contains(key)
        && returns_by_path.get(key).is_some_and(|return_type| {
            private_tail_can_use_expected(function, return_type, uses, adts)
        })
    {
        return true;
    }

    #[cfg(test)]
    private_inference_counters::record_prelude_callback_discovery_scan();

    let mut bindings = function
        .params
        .iter()
        .map(|param| Binding::new(param.name.clone(), function_body_param_type(param)))
        .collect::<Vec<_>>();
    let declared_return = function
        .return_type
        .as_deref()
        .map(|return_type| parse_type_or_unknown(Some(return_type)));
    for (index, line) in function.body.iter().enumerate() {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(
                    expr,
                    annotation_type.as_ref(),
                    &reference_context,
                ) {
                    return true;
                }
                let initializer_private_function = annotation_type
                    .is_none()
                    .then(|| {
                        private_same_module_call_target(
                            expr,
                            function.module_name.as_deref(),
                            function_by_path,
                        )
                    })
                    .flatten();
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_let_pattern_bindings(
                    pattern,
                    &ty,
                    initializer_private_function,
                    &mut bindings,
                );
            }
            BodyLineKind::Expr { expr } => {
                let expected = (index + 1 == function.body.len())
                    .then_some(declared_return.as_ref())
                    .flatten();
                let reference_context = PrivatePreludeCallbackReferenceContext {
                    current_module: function.module_name.as_deref(),
                    uses,
                    bindings: &bindings,
                    omitted_private_returns,
                    returns_by_path,
                    function_by_path,
                    adts,
                };
                if private_prelude_callback_expr_references_slot(expr, expected, &reference_context)
                {
                    return true;
                }
            }
        }
    }
    false
}

fn private_prelude_callback_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    if let ExprKind::NamePath(segments) = &expr.kind
        && expected.is_some_and(|expected| {
            private_callback_return_constraint_can_update(segments, expected, context)
        })
    {
        return true;
    }
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            let direct_reference =
                private_prelude_callback_call_references_slot(callee, args, expected, context);
            direct_reference
                || !matches!(callee.kind, ExprKind::NamePath(_))
                    && private_prelude_callback_expr_references_slot(callee, None, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::List(_) | ExprKind::Dict(_) | ExprKind::Record(_) => {
            private_prelude_callback_collection_references_slot(expr, expected, context)
        }
        ExprKind::Perform { .. }
        | ExprKind::Handle { .. }
        | ExprKind::SchemaDecode { .. }
        | ExprKind::SchemaEncode { .. }
        | ExprKind::FieldAccess { .. }
        | ExprKind::Try(_)
        | ExprKind::Prefix { .. } => {
            private_prelude_callback_wrapped_expr_references_slot(expr, expected, context)
        }
        ExprKind::Match { .. } | ExprKind::If { .. } | ExprKind::Binary { .. } => {
            private_prelude_callback_control_flow_references_slot(expr, expected, context)
        }
        ExprKind::NamePath(_)
        | ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => false,
    }
}

fn private_prelude_callback_collection_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::List(items) => items.iter().any(|item| {
            let item_expected = expected.and_then(Type::vec_part);
            private_prelude_callback_expr_references_slot(item, item_expected, context)
        }),
        ExprKind::Dict(entries) => entries.iter().any(|entry| {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            private_prelude_callback_expr_references_slot(&entry.key, key_expected, context)
                || private_prelude_callback_expr_references_slot(
                    &entry.value,
                    value_expected,
                    context,
                )
        }),
        ExprKind::Record(fields) => fields.iter().any(|field| {
            let field_expected = expected.and_then(|expected| expected.record_field(&field.name));
            private_prelude_callback_expr_references_slot(&field.expr, field_expected, context)
        }),
        _ => false,
    }
}

fn private_prelude_callback_wrapped_expr_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Perform { args, .. } => args
            .iter()
            .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context)),
        ExprKind::Handle { body, args, .. } => {
            private_prelude_callback_expr_references_slot(body, expected, context)
                || args
                    .iter()
                    .any(|arg| private_prelude_callback_expr_references_slot(arg, None, context))
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            private_prelude_callback_expr_references_slot(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            ) || private_prelude_callback_expr_references_slot(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            )
        }
        ExprKind::SchemaEncode { value, .. }
        | ExprKind::FieldAccess { base: value, .. }
        | ExprKind::Try(value)
        | ExprKind::Prefix { expr: value, .. } => {
            private_prelude_callback_expr_references_slot(value, None, context)
        }
        _ => false,
    }
}

fn private_prelude_callback_control_flow_references_slot(
    expr: &Expr,
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            private_prelude_callback_expr_references_slot(scrutinee, None, context)
                || arms.iter().any(|arm| {
                    private_prelude_callback_expr_references_slot(&arm.expr, expected, context)
                })
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            private_prelude_callback_expr_references_slot(condition, Some(&Type::bool()), context)
                || private_prelude_callback_expr_references_slot(then_branch, expected, context)
                || else_if_branches.iter().any(|branch| {
                    private_prelude_callback_expr_references_slot(
                        &branch.condition,
                        Some(&Type::bool()),
                        context,
                    ) || private_prelude_callback_expr_references_slot(
                        &branch.expr,
                        expected,
                        context,
                    )
                })
                || private_prelude_callback_expr_references_slot(else_branch, expected, context)
        }
        ExprKind::Binary { left, right, .. } => {
            private_prelude_callback_expr_references_slot(left, expected, context)
                || private_prelude_callback_expr_references_slot(right, expected, context)
        }
        _ => false,
    }
}

fn private_prelude_callback_call_references_slot(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return false;
    };
    let Some(name) =
        private_prelude_constraint_name(segments, context.current_module, context.function_by_path)
    else {
        return false;
    };
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    let Some((mut params, _)) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
    else {
        return false;
    };
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    args.iter()
        .zip(params.iter())
        .any(|(arg, param)| private_prelude_callback_arg_references_slot(arg, param, context))
}

struct PrivatePreludeCallbackReferenceContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    omitted_private_returns: &'a BTreeSet<FunctionKey>,
    returns_by_path: &'a FunctionReturnMap,
    function_by_path: &'a FunctionAstMap<'a>,
    adts: &'a AdtRegistry,
}

fn private_prelude_callback_arg_references_slot(
    expr: &Expr,
    expected: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    match &expr.kind {
        ExprKind::NamePath(segments) => {
            private_callback_return_constraint_can_update(segments, expected, context)
        }
        _ => private_prelude_callback_expr_references_slot(expr, Some(expected), context),
    }
}

fn private_callback_return_constraint_can_update(
    segments: &[String],
    expected_callback: &Type,
    context: &PrivatePreludeCallbackReferenceContext<'_>,
) -> bool {
    let Type::Function { return_type, .. } = expected_callback else {
        return false;
    };
    if type_has_unknown(return_type) {
        return false;
    }
    let [name] = segments else {
        return false;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return false;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return false;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return false;
    }
    context.returns_by_path.get(&key) != Some(return_type)
}

struct PrivatePreludeCallbackConstraintContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    function_by_path: &'a BTreeMap<(Option<String>, String), &'a Function>,
    omitted_private_returns: &'a BTreeSet<(Option<String>, String)>,
    returns_by_path: &'a mut BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
    changed: &'a mut bool,
}

fn collect_private_prelude_callback_expr_constraints(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    match &expr.kind {
        ExprKind::List(items) => {
            let item_expected = expected.and_then(Type::vec_part);
            for item in items {
                collect_private_prelude_callback_expr_constraints(item, item_expected, context);
            }
        }
        ExprKind::Dict(entries) => {
            let (key_expected, value_expected) = expected
                .and_then(Type::dict_parts)
                .map_or((None, None), |(key, value)| (Some(key), Some(value)));
            for entry in entries {
                collect_private_prelude_callback_expr_constraints(
                    &entry.key,
                    key_expected,
                    context,
                );
                collect_private_prelude_callback_expr_constraints(
                    &entry.value,
                    value_expected,
                    context,
                );
            }
        }
        ExprKind::Record(fields) => {
            for field in fields {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                collect_private_prelude_callback_expr_constraints(
                    &field.expr,
                    field_expected,
                    context,
                );
            }
        }
        ExprKind::Call { callee, args } => {
            collect_private_prelude_callback_call_constraints(callee, args, expected, context);
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::Handle { body, args, .. } => {
            collect_private_prelude_callback_expr_constraints(body, expected, context);
            for arg in args {
                collect_private_prelude_callback_expr_constraints(arg, None, context);
            }
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            collect_private_prelude_callback_expr_constraints(
                input,
                Some(&Type::named("ByteView", Vec::new())),
                context,
            );
            collect_private_prelude_callback_expr_constraints(
                base,
                Some(&Type::named("ByteOffset", Vec::new())),
                context,
            );
        }
        ExprKind::SchemaEncode { value, .. } => {
            collect_private_prelude_callback_expr_constraints(value, None, context);
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::Prefix { expr: base, .. } => {
            collect_private_prelude_callback_expr_constraints(base, None, context);
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_private_prelude_callback_expr_constraints(scrutinee, None, context);
            for arm in arms {
                collect_private_prelude_callback_expr_constraints(&arm.expr, expected, context);
            }
        }
        ExprKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => {
            collect_private_prelude_callback_expr_constraints(
                condition,
                Some(&Type::bool()),
                context,
            );
            collect_private_prelude_callback_expr_constraints(then_branch, expected, context);
            for branch in else_if_branches {
                collect_private_prelude_callback_expr_constraints(
                    &branch.condition,
                    Some(&Type::bool()),
                    context,
                );
                collect_private_prelude_callback_expr_constraints(&branch.expr, expected, context);
            }
            collect_private_prelude_callback_expr_constraints(else_branch, expected, context);
        }
        ExprKind::Binary { left, right, .. } => {
            collect_private_prelude_callback_expr_constraints(left, expected, context);
            collect_private_prelude_callback_expr_constraints(right, expected, context);
        }
        ExprKind::NamePath(segments) => {
            if let Some(expected) = expected {
                collect_private_callback_return_constraint_for_segments(
                    segments, expected, context,
                );
            }
        }
        ExprKind::Missing
        | ExprKind::Hole { .. }
        | ExprKind::StringLiteral(_)
        | ExprKind::IntLiteral(_)
        | ExprKind::FloatLiteral(_)
        | ExprKind::BoolLiteral(_)
        | ExprKind::Unit
        | ExprKind::TypeApply { .. } => {}
    }
}

fn collect_private_prelude_callback_call_constraints(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let ExprKind::NamePath(segments) = &callee.kind else {
        return;
    };
    let Some(name) =
        private_prelude_constraint_name(segments, context.current_module, context.function_by_path)
    else {
        return;
    };
    let input_type = private_prelude_input_arg(args, name).map(|arg| {
        infer_private_signature_expr_type(
            arg,
            None,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )
    });
    let Some((mut params, _)) =
        crate::prelude::prelude_signature_with_input(name, expected, input_type.as_ref())
    else {
        return;
    };
    if name == "vec_try_map_with" {
        let context_type = args.first().map(|arg| {
            infer_private_signature_expr_type(
                arg,
                None,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            )
        });
        apply_vec_try_map_with_context_param(&mut params, context_type);
    }
    for (arg, param) in args.iter().zip(params.iter()) {
        collect_private_callback_return_constraint(arg, param, context);
        collect_private_prelude_callback_expr_constraints(arg, Some(param), context);
    }
}

fn apply_vec_try_map_with_context_param(params: &mut [Type], context_type: Option<Type>) {
    let Some(context_type) = context_type else {
        return;
    };
    if let Some(param) = params.first_mut() {
        *param = context_type.clone();
    }
    let Some(Type::Function {
        params: callback_params,
        ..
    }) = params.get_mut(2)
    else {
        return;
    };
    if let Some(callback_context) = callback_params.first_mut() {
        *callback_context = context_type;
    }
}

fn private_prelude_constraint_name<'a>(
    segments: &'a [String],
    current_module: Option<&str>,
    function_by_path: &BTreeMap<(Option<String>, String), &Function>,
) -> Option<&'a str> {
    match segments {
        [name]
            if !function_by_path
                .contains_key(&(current_module.map(str::to_string), name.clone())) =>
        {
            Some(name)
        }
        [module, name] if module == "prelude" || module == "prelude_builtin" => Some(name),
        _ => None,
    }
}

fn collect_private_callback_return_constraint(
    arg: &Expr,
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let ExprKind::NamePath(segments) = &arg.kind else {
        return;
    };
    collect_private_callback_return_constraint_for_segments(segments, expected_callback, context);
}

fn collect_private_callback_return_constraint_for_segments(
    segments: &[String],
    expected_callback: &Type,
    context: &mut PrivatePreludeCallbackConstraintContext<'_>,
) {
    let Type::Function { return_type, .. } = expected_callback else {
        return;
    };
    if type_has_unknown(return_type) {
        return;
    }
    let [name] = segments else {
        return;
    };
    let key = context
        .bindings
        .iter()
        .rev()
        .find(|binding| binding.name == *name)
        .and_then(|binding| binding.private_function_value.clone())
        .unwrap_or_else(|| (context.current_module.map(str::to_string), name.clone()));
    if !context.omitted_private_returns.contains(&key) {
        return;
    }
    let Some(function) = context.function_by_path.get(&key) else {
        return;
    };
    if !private_tail_can_use_expected(function, return_type, context.uses, context.adts) {
        return;
    }
    if context.returns_by_path.get(&key) == Some(return_type) {
        return;
    }
    context
        .returns_by_path
        .insert(key, return_type.as_ref().clone());
    *context.changed = true;
}

fn private_tail_can_use_expected(
    function: &Function,
    expected: &Type,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    let Some(BodyLineKind::Expr { expr }) = function.body.last().map(|line| &line.kind) else {
        return false;
    };
    tail_expr_can_use_expected(expr, expected, function.module_name.as_deref(), uses, adts)
}

fn tail_expr_can_use_expected(
    expr: &Expr,
    expected: &Type,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> bool {
    match &expr.kind {
        ExprKind::List(_) => expected.vec_part().is_some(),
        ExprKind::Dict(_) => expected.dict_parts().is_some(),
        ExprKind::Record(fields) => {
            if fields.is_empty() && expected.dict_parts().is_some() {
                return true;
            }
            !fields.is_empty()
                && fields
                    .iter()
                    .all(|field| expected.record_field(&field.name).is_some())
        }
        ExprKind::NamePath(segments) => {
            matches!(
                adts.nullary_constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Call { callee, .. } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return false;
            };
            matches!(
                adts.constructor(segments, current_module, uses),
                crate::adt::ConstructorLookup::Found(constructor)
                    if adt::adt_args(expected, constructor.descriptor).is_some()
            )
        }
        ExprKind::Match { arms, .. } => arms
            .iter()
            .all(|arm| tail_expr_can_use_expected(&arm.expr, expected, current_module, uses, adts)),
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => std::iter::once(then_branch.as_ref())
            .chain(else_if_branches.iter().map(|branch| &branch.expr))
            .chain(std::iter::once(else_branch.as_ref()))
            .all(|branch| tail_expr_can_use_expected(branch, expected, current_module, uses, adts)),
        _ => false,
    }
}

fn infer_private_function_tail_type(
    function: &veln_ast::Function,
    uses: &[UseDecl],
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    #[cfg(test)]
    private_inference_counters::record_body_return_scan();

    let mut bindings = private_function_body_bindings(function, signatures_by_path);
    let mut tail = Type::unit();
    for line in &function.body {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => {
                let annotation_type = annotation
                    .as_deref()
                    .map(|annotation| parse_type_or_unknown(Some(annotation)));
                let ty = annotation_type.unwrap_or_else(|| {
                    infer_private_signature_expr_type(
                        expr,
                        None,
                        function.module_name.as_deref(),
                        uses,
                        &bindings,
                        returns_by_path,
                        adts,
                    )
                });
                collect_pattern_bindings(pattern, &ty, &mut bindings);
            }
            BodyLineKind::Expr { expr } => {
                tail = infer_private_signature_expr_type(
                    expr,
                    None,
                    function.module_name.as_deref(),
                    uses,
                    &bindings,
                    returns_by_path,
                    adts,
                );
            }
        }
    }
    tail
}

fn private_function_body_bindings(
    function: &veln_ast::Function,
    signatures_by_path: &BTreeMap<(Option<String>, String), FunctionSignature>,
) -> Vec<Binding> {
    let signature = function
        .name
        .as_ref()
        .and_then(|name| signatures_by_path.get(&(function.module_name.clone(), name.clone())));
    function
        .params
        .iter()
        .enumerate()
        .map(|(index, param)| {
            let ty = if param.is_variadic {
                signature
                    .and_then(|signature| signature.variadic.clone())
                    .map(|ty| Type::named("List", vec![ty]))
                    .unwrap_or_else(|| function_body_param_type(param))
            } else {
                signature
                    .and_then(|signature| signature.params.get(index).cloned())
                    .unwrap_or_else(|| function_body_param_type(param))
            };
            Binding::new(param.name.clone(), ty)
        })
        .collect()
}

fn infer_private_signature_expr_type(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    let context = PrivateSignatureInferContext {
        current_module,
        uses,
        bindings,
        returns_by_path,
        adts,
    };
    match &expr.kind {
        ExprKind::Missing | ExprKind::Hole { .. } | ExprKind::TypeApply { .. } => Type::Unknown,
        ExprKind::StringLiteral(_) => Type::string(),
        ExprKind::IntLiteral(_) => Type::int(),
        ExprKind::FloatLiteral(_) => Type::float(),
        ExprKind::BoolLiteral(_) => Type::bool(),
        ExprKind::Unit => Type::unit(),
        ExprKind::NamePath(segments) => infer_private_signature_name_type(
            segments,
            expected,
            current_module,
            uses,
            bindings,
            returns_by_path,
            adts,
        ),
        ExprKind::List(items) => infer_private_list_type(items, expected, &context),
        ExprKind::Dict(entries) => infer_private_dict_type(entries, expected, &context),
        ExprKind::Record(fields) => infer_private_record_type(fields, expected, &context),
        ExprKind::Call { callee, args } => {
            infer_private_signature_call_type(callee, args, expected, &context)
        }
        ExprKind::Perform { args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Type::Unknown
        }
        ExprKind::Handle { body, args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            context.infer(body, expected)
        }
        ExprKind::SchemaDecode { input, base, .. } => {
            context.infer(input, Some(&Type::named("ByteView", Vec::new())));
            context.infer(base, Some(&Type::named("ByteOffset", Vec::new())));
            Type::Unknown
        }
        ExprKind::SchemaEncode { value, .. } => {
            context.infer(value, None);
            Type::Unknown
        }
        ExprKind::FieldAccess { base, field, .. } => context
            .infer(base, None)
            .record_field(field)
            .cloned()
            .unwrap_or(Type::Unknown),
        ExprKind::Try(inner) => expected.cloned().unwrap_or_else(|| {
            let inner_type = context.infer(inner, None);
            adt::result_parts(&inner_type).map_or(Type::Unknown, |(value, _)| value.clone())
        }),
        ExprKind::Match { scrutinee, arms } => {
            infer_private_match_type(scrutinee, arms, expected, &context)
        }
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => infer_private_if_result_type(
            then_branch,
            else_if_branches,
            else_branch,
            expected,
            &context,
        ),
        ExprKind::Prefix { expr, .. } => {
            context.infer(expr, expected);
            Type::Unknown
        }
        ExprKind::Binary { op, left, right } => {
            infer_private_binary_type(*op, left, right, expected, &context)
        }
    }
}

fn infer_private_list_type(
    items: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut item_type = expected
        .and_then(Type::vec_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    for item in items {
        let actual = context.infer(item, item_type_unknown_as_none(&item_type));
        if item_type == Type::Unknown {
            item_type = actual;
        }
    }
    Type::vec(item_type)
}

fn infer_private_dict_type(
    entries: &[DictEntry],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let (mut key_type, mut value_type) = expected
        .and_then(Type::dict_parts)
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    for entry in entries {
        let key_actual = context.infer(&entry.key, item_type_unknown_as_none(&key_type));
        if key_type == Type::Unknown {
            key_type = key_actual;
        }
        let value_actual = context.infer(&entry.value, item_type_unknown_as_none(&value_type));
        if value_type == Type::Unknown {
            value_type = value_actual;
        }
    }
    Type::dict(key_type, value_type)
}

fn infer_private_record_type(
    fields: &[RecordField],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if fields.is_empty()
        && let Some(expected) = expected
        && expected.dict_parts().is_some()
    {
        return expected.clone();
    }
    Type::Record(
        fields
            .iter()
            .map(|field| {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                (
                    field.name.clone(),
                    context.infer(&field.expr, field_expected),
                )
            })
            .collect(),
    )
}

fn infer_private_match_type(
    scrutinee: &Expr,
    arms: &[MatchArm],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
        arms,
        context.current_module,
        context.uses,
        context.adts,
    ) {
        MatchScrutineePatternInference::Inferred(ty) => Some(ty),
        MatchScrutineePatternInference::Uninferred
        | MatchScrutineePatternInference::Ambiguous(_) => None,
    };
    context.infer(scrutinee, scrutinee_expected.as_ref());
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for arm in arms {
        let actual = context.infer(&arm.expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

fn infer_private_if_result_type(
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for branch_expr in std::iter::once(then_branch)
        .chain(else_if_branches.iter().map(|branch| &branch.expr))
        .chain(std::iter::once(else_branch))
    {
        let actual = context.infer(branch_expr, item_type_unknown_as_none(&result));
        if result == Type::Unknown {
            result = actual;
        }
    }
    result
}

fn infer_private_binary_type(
    op: veln_ast::BinaryOp,
    left: &Expr,
    right: &Expr,
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    match op {
        veln_ast::BinaryOp::Equal
        | veln_ast::BinaryOp::NotEqual
        | veln_ast::BinaryOp::Less
        | veln_ast::BinaryOp::LessEqual
        | veln_ast::BinaryOp::Greater
        | veln_ast::BinaryOp::GreaterEqual
        | veln_ast::BinaryOp::Or
        | veln_ast::BinaryOp::And => Type::bool(),
        veln_ast::BinaryOp::BitwiseOr
        | veln_ast::BinaryOp::BitwiseXor
        | veln_ast::BinaryOp::BitwiseAnd
        | veln_ast::BinaryOp::ShiftLeft
        | veln_ast::BinaryOp::ShiftRight
        | veln_ast::BinaryOp::ShiftRightLogical => Type::int(),
        veln_ast::BinaryOp::Add
        | veln_ast::BinaryOp::Subtract
        | veln_ast::BinaryOp::Multiply
        | veln_ast::BinaryOp::Divide => {
            let left = context.infer(left, expected);
            let right = context.infer(right, expected);
            if left == Type::float() || right == Type::float() {
                Type::float()
            } else {
                Type::int()
            }
        }
        veln_ast::BinaryOp::PipeGreater => Type::Unknown,
    }
}

fn item_type_unknown_as_none(ty: &Type) -> Option<&Type> {
    (ty != &Type::Unknown).then_some(ty)
}

pub(crate) fn infer_match_scrutinee_type_from_constructor_patterns(
    arms: &[MatchArm],
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> MatchScrutineePatternInference {
    let mut inferred: Option<(crate::adt::AdtConstructor<'_>, Vec<Type>)> = None;

    for arm in arms {
        let PatternKind::Constructor { name, args } = &arm.pattern.kind else {
            continue;
        };
        let candidates = adts.constructor_candidates(name, current_module, uses);
        if candidates.is_empty() {
            continue;
        }
        let descriptor_names = unique_constructor_descriptor_names(&candidates);
        if descriptor_names.len() != 1 {
            return MatchScrutineePatternInference::Ambiguous(descriptor_names);
        }
        let constructor = candidates[0];
        if let Some((previous, _)) = &inferred {
            if !same_constructor_descriptor(previous, &constructor) {
                let mut names = unique_constructor_descriptor_names(&[*previous, constructor]);
                names.sort();
                return MatchScrutineePatternInference::Ambiguous(names);
            }
        } else {
            inferred = Some((
                constructor,
                vec![Type::Unknown; constructor.descriptor.type_parameters.len()],
            ));
        }
        let Some((_, type_args)) = &mut inferred else {
            continue;
        };
        for (index, pattern) in args.iter().enumerate() {
            let Some(pattern_type) =
                infer_pattern_type_from_constructor_patterns(pattern, current_module, uses, adts)
            else {
                continue;
            };
            adt::merge_type_args_from_payload(type_args, constructor, index, &pattern_type);
        }
    }

    match inferred {
        Some((constructor, type_args)) => MatchScrutineePatternInference::Inferred(
            adt::constructed_type_from_args(constructor, &type_args),
        ),
        None => MatchScrutineePatternInference::Uninferred,
    }
}

fn infer_pattern_type_from_constructor_patterns(
    pattern: &Pattern,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> Option<Type> {
    match &pattern.kind {
        PatternKind::StringLiteral(_) => Some(Type::string()),
        PatternKind::IntLiteral(_) => Some(Type::int()),
        PatternKind::FloatLiteral(_) => Some(Type::float()),
        PatternKind::BoolLiteral(_) => Some(Type::bool()),
        PatternKind::Unit => Some(Type::unit()),
        PatternKind::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|field| {
                    (
                        field.name.clone(),
                        infer_pattern_type_from_constructor_patterns(
                            &field.pattern,
                            current_module,
                            uses,
                            adts,
                        )
                        .unwrap_or(Type::Unknown),
                    )
                })
                .collect(),
        )),
        PatternKind::Constructor { name, args } => {
            let candidates = adts.constructor_candidates(name, current_module, uses);
            let [constructor] = candidates.as_slice() else {
                return None;
            };
            let mut type_args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
            for (index, pattern) in args.iter().enumerate() {
                let Some(pattern_type) = infer_pattern_type_from_constructor_patterns(
                    pattern,
                    current_module,
                    uses,
                    adts,
                ) else {
                    continue;
                };
                adt::merge_type_args_from_payload(
                    &mut type_args,
                    *constructor,
                    index,
                    &pattern_type,
                );
            }
            Some(adt::constructed_type_from_args(*constructor, &type_args))
        }
        PatternKind::Wildcard | PatternKind::Binding(_) => None,
    }
}

fn unique_constructor_descriptor_names(
    constructors: &[crate::adt::AdtConstructor<'_>],
) -> Vec<String> {
    let mut names = Vec::new();
    for constructor in constructors {
        let name = constructor.descriptor.diagnostic_name.clone();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn same_constructor_descriptor(
    left: &crate::adt::AdtConstructor<'_>,
    right: &crate::adt::AdtConstructor<'_>,
) -> bool {
    left.descriptor.type_name == right.descriptor.type_name
        && left.descriptor.module_name == right.descriptor.module_name
        && left.descriptor.type_parameters.len() == right.descriptor.type_parameters.len()
}

fn type_has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(type_has_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_has_unknown(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_has_unknown)
                || variadic.as_deref().is_some_and(type_has_unknown)
                || type_has_unknown(return_type)
        }
    }
}

fn infer_private_signature_name_type(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    if let crate::adt::ConstructorLookup::Found(constructor) =
        adts.nullary_constructor(segments, current_module, uses)
    {
        return expected
            .and_then(|expected| {
                adt::adt_args(expected, constructor.descriptor).map(|_| expected.clone())
            })
            .unwrap_or_else(|| adt::constructed_type(constructor, &[]));
    }
    match segments {
        [name] => bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                returns_by_path
                    .get(&(current_module.map(str::to_string), name.clone()))
                    .cloned()
            })
            .unwrap_or(Type::Unknown),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                        .cloned()
                })
                .unwrap_or(Type::Unknown)
        }
        _ => Type::Unknown,
    }
}

struct PrivateSignatureInferContext<'a> {
    current_module: Option<&'a str>,
    uses: &'a [UseDecl],
    bindings: &'a [Binding],
    returns_by_path: &'a BTreeMap<(Option<String>, String), Type>,
    adts: &'a AdtRegistry,
}

impl PrivateSignatureInferContext<'_> {
    fn infer(&self, expr: &Expr, expected: Option<&Type>) -> Type {
        infer_private_signature_expr_type(
            expr,
            expected,
            self.current_module,
            self.uses,
            self.bindings,
            self.returns_by_path,
            self.adts,
        )
    }
}

fn infer_private_signature_call_type(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'_>,
) -> Type {
    if let ExprKind::NamePath(segments) = &callee.kind {
        if let crate::adt::ConstructorLookup::Found(constructor) =
            context
                .adts
                .constructor(segments, context.current_module, context.uses)
        {
            let actual_args = args
                .iter()
                .map(|arg| context.infer(arg, None))
                .collect::<Vec<_>>();
            if expected
                .and_then(|expected| adt::adt_args(expected, constructor.descriptor))
                .is_some()
            {
                return expected.cloned().unwrap_or(Type::Unknown);
            }
            return adt::constructed_type(constructor, &actual_args);
        }
        if let Some(name) = segments.last() {
            if let Some(return_type) = match segments.as_slice() {
                [name] => context
                    .returns_by_path
                    .get(&(context.current_module.map(str::to_string), name.clone())),
                [_, .., name] => imported_use_for_path(
                    context.uses,
                    &segments[..segments.len() - 1],
                    context.current_module,
                )
                .and_then(|use_decl| {
                    context
                        .returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                }),
                _ => None,
            } {
                return return_type.clone();
            }
            if let Some((params, return_type)) = crate::prelude::prelude_signature(name, expected) {
                for (arg, param) in args.iter().zip(params.iter()) {
                    context.infer(arg, Some(param));
                }
                return return_type;
            }
        }
    }
    Type::Unknown
}

pub(crate) fn function_body_param_type(param: &veln_ast::Param) -> Type {
    let ty = parse_type_or_unknown(param.ty.as_deref());
    if param.is_variadic {
        Type::named("List", vec![ty])
    } else {
        ty
    }
}

fn codec_call_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<CodecCallSignature> {
    module
        .codecs
        .iter()
        .flat_map(|codec| {
            let name = codec.name.clone()?;
            Some(
                codec
                    .implementations
                    .iter()
                    .flat_map(move |implementation| {
                        match (&implementation.direction, &implementation.kind) {
                            (
                                CodecDirection::Decode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::HandWrittenDecode,
                            )
                            .into_iter()
                            .collect(),
                            (
                                CodecDirection::Encode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::Direct,
                            )
                            .into_iter()
                            .collect(),
                            (CodecDirection::Decode, CodecImplementationKind::Derive) => {
                                codec_derive_decode_signature(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                                .into_iter()
                                .collect()
                            }
                            (CodecDirection::Encode, CodecImplementationKind::Derive) => {
                                codec_derive_encode_signatures(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                            }
                            (_, CodecImplementationKind::With { function: None }) => Vec::new(),
                        }
                    }),
            )
        })
        .flatten()
        .collect()
}

fn codec_with_signature(
    codec: &CodecDecl,
    functions: &[FunctionSignature],
    name: String,
    function_name: &str,
    boundary: CodecCallBoundary,
) -> Option<CodecCallSignature> {
    let function = functions.iter().find(|function| {
        function.name == function_name && function.module_name == codec.module_name
    })?;
    Some(CodecCallSignature {
        name,
        target_name: function.target_name.clone(),
        boundary,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    })
}

fn codec_derive_decode_signature(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Option<CodecCallSignature> {
    let schema = codec_referenced_schema(module, codec)?;
    let schema_name = schema.name.as_ref()?;
    let step_name = schema_decode_step_function_name(schema_name);
    let function = functions.iter().find(|function| {
        function.name == step_name && function.module_name == schema.module_name
    })?;
    Some(CodecCallSignature {
        name,
        target_name: function.target_name.clone(),
        boundary: CodecCallBoundary::Direct,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    })
}

fn codec_derive_encode_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Vec<CodecCallSignature> {
    let Some(schema) = codec_referenced_schema(module, codec) else {
        return Vec::new();
    };
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    let encode_name = schema_encode_function_name(schema_name);
    let Some(function) = functions.iter().find(|function| {
        function.name == encode_name && function.module_name == schema.module_name
    }) else {
        return Vec::new();
    };
    let unbounded = CodecCallSignature {
        name,
        target_name: format!("{SCHEMA_ENCODE_STEP_TARGET_PREFIX}{schema_name}"),
        boundary: CodecCallBoundary::Direct,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: Type::named("EncodeStep", vec![Type::unit()]),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    };
    let Some(value_type) = function.params.first().cloned() else {
        return vec![unbounded];
    };
    let mut state_fields = match &value_type {
        Type::Record(fields) => fields.clone(),
        _ => Vec::new(),
    };
    state_fields.push((
        "encoded_offset".to_string(),
        Type::named("ByteCount", Vec::new()),
    ));
    let budgeted = CodecCallSignature {
        name: unbounded.name.clone(),
        target_name: unbounded.target_name.clone(),
        boundary: unbounded.boundary,
        module_name: unbounded.module_name.clone(),
        visibility: unbounded.visibility,
        params: vec![value_type, Type::named("ByteCount", Vec::new())],
        return_type: Type::named("EncodeStep", vec![Type::Record(state_fields)]),
        effects: unbounded.effects.clone(),
        node_id: unbounded.node_id,
        span: unbounded.span.clone(),
    };
    vec![unbounded, budgeted]
}

fn codec_referenced_schema<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
) -> Option<&'a SchemaDecl> {
    let schema_name = codec.schema.as_ref()?;
    let segments = schema_name
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    schema_reference(
        module,
        &segments,
        codec.module_name.as_deref(),
        true,
        &mut Vec::new(),
    )
}

fn schema_reference<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    let companion_access_targets = companion_access_targets(module);
    match segments {
        [name] => schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            schema_in_module(
                module,
                Some(&use_decl.name),
                name,
                companion_private_schema_access_allowed(
                    use_decl,
                    current_module,
                    &companion_access_targets,
                ),
                visited_aliases,
            )
        }
        _ => None,
    }
}

pub(crate) fn schema_field_target<'a>(
    module: &'a SurfaceModule,
    containing_schema: &SchemaDecl,
    text: &str,
) -> Option<&'a SchemaDecl> {
    if schema_field_uses_existing_grammar(containing_schema, text) {
        return None;
    }
    let segments = schema_payload_name_path(text)?;
    schema_reference(
        module,
        &segments,
        containing_schema.module_name.as_deref(),
        true,
        &mut Vec::new(),
    )
}

pub(crate) fn schema_field_uses_existing_grammar(schema: &SchemaDecl, text: &str) -> bool {
    match schema.format.as_ref().map(|format| format.name.as_str()) {
        None => matches!(text, "Int" | "Bool" | "Float" | "String"),
        Some("binary") => {
            exact_width_schema_primitive(text).is_some()
                || lowercase_reserved_bits_schema_primitive(text).is_some()
                || lowercase_schema_primitive(text).is_some()
                || !lowercase_schema_primitive_nested_payloads(text).is_empty()
                || byte_view_schema_primitive(text).is_some()
                || repeat_schema_primitive(text).is_some()
                || binary_schema_anonymous_record_decode_type(text).is_some()
                || closed_dispatch_schema_primitive(text).is_some()
                || extension_dispatch_schema_primitive(text).is_some()
                || reserved_bits_schema_primitive(text).is_some()
        }
        Some(_) => false,
    }
}

fn schema_in_module<'a>(
    module: &'a SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return (allow_private_schema || schema.visibility == Visibility::Public).then_some(schema);
    }
    let alias = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    })?;
    let alias_name = alias.name.as_ref()?;
    let key = (alias.module_name.clone(), alias_name.clone());
    if visited_aliases.contains(&key) {
        return None;
    }
    visited_aliases.push(key);
    let schema = schema_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    schema
}

fn schema_decode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .flat_map(|schema| schema_decode_function_signatures_for_schema(module, schema))
        .collect()
}

fn schema_decode_function_signatures_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Vec<FunctionSignature> {
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    if schema.format.is_none() {
        return format_neutral_schema_decode_function_signature_for_schema(module, schema)
            .into_iter()
            .collect();
    }
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return Vec::new();
    }
    let Some(fields) = schema_decode_record_fields(module, schema) else {
        return Vec::new();
    };
    let byte_view = Type::named("ByteView", Vec::new());
    let byte_offset = Type::named("ByteOffset", Vec::new());
    let decoded_type = Type::Record(fields.into_iter().map(|(name, ty, _)| (name, ty)).collect());
    let result = Type::named("Result", vec![decoded_type.clone(), Type::string()]);
    let step = Type::named("DecodeStep", vec![decoded_type]);
    vec![
        FunctionSignature {
            name: schema_decode_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view.clone()],
            variadic: None,
            return_type: result,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
        FunctionSignature {
            name: schema_decode_step_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_STEP_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view, byte_offset],
            variadic: None,
            return_type: step,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
    ]
}

fn format_neutral_schema_decode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    let decoded_type = Type::Record(format_neutral_schema_decode_record_fields(module, schema)?);
    Some(FunctionSignature {
        name: schema_decode_function_name(schema_name),
        target_name: format!("{SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![decoded_type.clone()],
        variadic: None,
        return_type: Type::named("Result", vec![decoded_type, Type::string()]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

pub(crate) fn format_neutral_schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type)>> {
    let adts = AdtRegistry::from_module(module);
    schema
        .fields
        .iter()
        .map(|field| {
            Some((
                field.name.clone(),
                format_neutral_schema_field_type_for_schema(module, schema, &adts, &field.ty)?,
            ))
        })
        .collect()
}

pub(crate) fn format_neutral_schema_field_type_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> Option<Type> {
    let ty = parse_type_annotation(text).ok()?;
    if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
        module,
        schema.module_name.as_deref(),
        adts,
        &ty,
        &mut FormatNeutralSchemaTraversalState::default(),
        FormatNeutralSchemaTraversal::Decode,
    ) {
        return Some(ty);
    }
    if let Some(target) = schema_field_target(module, schema, text)
        && target.format.is_none()
    {
        return format_neutral_schema_composition_value_type(
            module,
            target,
            FormatNeutralSchemaTraversal::Decode,
            &mut Vec::new(),
        );
    }
    None
}

pub(crate) fn binary_schema_anonymous_record_decode_type(text: &str) -> Option<Type> {
    let Type::Record(fields) = parse_type_annotation(text).ok()? else {
        return None;
    };
    binary_schema_anonymous_record_type(fields).map(Type::Record)
}

fn binary_schema_anonymous_record_type(fields: Vec<(String, Type)>) -> Option<Vec<(String, Type)>> {
    fields
        .into_iter()
        .map(|(name, ty)| {
            if binary_schema_anonymous_record_leaf_type(&ty).is_some() {
                return Some((name, Type::int()));
            }
            let Type::Record(fields) = ty else {
                return None;
            };
            Some((
                name,
                Type::Record(binary_schema_anonymous_record_type(fields)?),
            ))
        })
        .collect()
}

fn binary_schema_anonymous_record_leaf_type(ty: &Type) -> Option<()> {
    match ty {
        Type::Named { name, args }
            if args.is_empty() && exact_width_schema_primitive(name).is_some() =>
        {
            Some(())
        }
        _ => None,
    }
}

fn format_neutral_schema_scalar_type_is_supported(name: &str, args: &[Type]) -> bool {
    args.is_empty() && matches!(name, "Int" | "Bool" | "Float" | "String")
}

fn format_neutral_schema_scalar_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args }
            if format_neutral_schema_scalar_type_is_supported(name, args)
    )
}

pub(crate) fn format_neutral_schema_encode_field_type_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> Option<Type> {
    let ty = parse_type_annotation(text).ok()?;
    if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
        module,
        schema.module_name.as_deref(),
        adts,
        &ty,
        &mut FormatNeutralSchemaTraversalState::default(),
        FormatNeutralSchemaTraversal::Encode,
    ) {
        return Some(ty);
    }
    if let Some(target) = schema_field_target(module, schema, text)
        && target.format.is_none()
    {
        return format_neutral_schema_composition_value_type(
            module,
            target,
            FormatNeutralSchemaTraversal::Encode,
            &mut Vec::new(),
        );
    }
    None
}

pub(crate) fn format_neutral_schema_encode_field_is_source_adt_candidate(text: &str) -> bool {
    parse_type_annotation(text)
        .ok()
        .is_some_and(|ty| format_neutral_schema_encode_type_is_source_adt_candidate(&ty))
}

fn format_neutral_schema_encode_type_is_source_adt_candidate(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if !matches!(
        name.as_str(),
        "Int" | "Bool" | "Float" | "String" | "Option" | "List" | "Vec" | "Dict" | "Result"
    ))
}

fn format_neutral_schema_encode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type)>> {
    let adts = AdtRegistry::from_module(module);
    schema
        .fields
        .iter()
        .map(|field| {
            let ty = format_neutral_schema_encode_field_type_for_schema(
                module, schema, &adts, &field.ty,
            )?;
            Some((field.name.clone(), ty))
        })
        .collect()
}

fn format_neutral_schema_first_unsupported_encode_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<SchemaField> {
    if schema.format.is_some() {
        return None;
    }
    let adts = AdtRegistry::from_module(module);
    schema
        .fields
        .iter()
        .find(|field| {
            let declaration_diagnostic_exists =
                format_neutral_schema_field_type_for_schema(module, schema, &adts, &field.ty)
                    .is_none()
                    && format_neutral_schema_encode_field_is_source_adt_candidate(&field.ty);
            !declaration_diagnostic_exists
                && format_neutral_schema_encode_field_type_for_schema(
                    module, schema, &adts, &field.ty,
                )
                .is_none()
        })
        .cloned()
}

#[derive(Clone, PartialEq, Eq)]
struct FormatNeutralSchemaAdtFrame {
    module_name: Option<String>,
    type_name: String,
    type_arguments: Vec<Type>,
}

#[derive(Default)]
struct FormatNeutralSchemaTraversalState {
    stack: Vec<FormatNeutralSchemaAdtFrame>,
    stack_cacheable: Vec<bool>,
    completed: Vec<(FormatNeutralSchemaAdtFrame, bool)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatNeutralSchemaTraversal {
    Decode,
    Encode,
}

fn format_neutral_schema_composition_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    traversal: FormatNeutralSchemaTraversal,
    stack: &mut Vec<(Option<String>, String)>,
) -> Option<Type> {
    let key = (schema.module_name.clone(), schema.name.clone()?);
    if stack.contains(&key) {
        return None;
    }
    stack.push(key);
    let adts = AdtRegistry::from_module(module);
    let mut fields = Vec::new();
    for field in &schema.fields {
        let parsed = parse_type_annotation(&field.ty).ok()?;
        let ty = if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
            module,
            schema.module_name.as_deref(),
            &adts,
            &parsed,
            &mut FormatNeutralSchemaTraversalState::default(),
            traversal,
        ) {
            ty
        } else if let Some(target) = schema_field_target(module, schema, &field.ty)
            && target.format.is_none()
        {
            format_neutral_schema_composition_value_type(module, target, traversal, stack)?
        } else {
            return None;
        };
        fields.push((field.name.clone(), ty));
    }
    stack.pop();
    Some(Type::Record(fields))
}

fn format_neutral_schema_visible_shape_type_for_schema(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    match ty {
        Type::Named { name, args }
            if matches!(name.as_str(), "List" | "Vec") && args.len() == 1 =>
        {
            Some(Type::named(
                name.clone(),
                vec![format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[0],
                    state,
                    traversal,
                )?],
            ))
        }
        Type::Named { name, args } if name == "Option" && args.len() == 1 => Some(Type::named(
            "Option",
            vec![format_neutral_schema_visible_shape_type_for_schema(
                module,
                current_module,
                adts,
                &args[0],
                state,
                traversal,
            )?],
        )),
        Type::Named { name, args } if name == "Dict" && args.len() == 2 => {
            if !matches!(&args[0], Type::Named { name, args } if name == "String" && args.is_empty())
            {
                return None;
            }
            Some(Type::dict(
                Type::string(),
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[1],
                    state,
                    traversal,
                )?,
            ))
        }
        Type::Named { name, args } if name == "Result" && args.len() == 2 => Some(Type::named(
            "Result",
            vec![
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[0],
                    state,
                    traversal,
                )?,
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[1],
                    state,
                    traversal,
                )?,
            ],
        )),
        Type::Named { .. } if format_neutral_schema_scalar_type(ty) => Some(ty.clone()),
        Type::Named { .. } => format_neutral_schema_source_adt_type(
            module,
            current_module,
            adts,
            ty,
            state,
            traversal,
        ),
        Type::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|(name, field_ty)| {
                    Some((
                        name.clone(),
                        format_neutral_schema_visible_shape_type_for_schema(
                            module,
                            current_module,
                            adts,
                            field_ty,
                            state,
                            traversal,
                        )?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

fn format_neutral_schema_source_adt_type(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    let descriptor = format_neutral_schema_source_adt_descriptor(module, current_module, adts, ty)?;
    let descriptor_ty = format_neutral_schema_descriptor_type(ty, descriptor);
    let Type::Named {
        args: type_arguments,
        ..
    } = &descriptor_ty
    else {
        return None;
    };
    let frame = FormatNeutralSchemaAdtFrame {
        module_name: descriptor.module_name.clone(),
        type_name: descriptor.type_name.clone(),
        type_arguments: type_arguments.clone(),
    };
    if let Some((_, supported)) = state.completed.iter().find(|(key, _)| key == &frame) {
        return (*supported).then_some(descriptor_ty);
    }
    if let Some(index) = state.stack.iter().position(|active| active == &frame) {
        state.stack_cacheable[index + 1..].fill(false);
        return Some(descriptor_ty);
    }
    if let Some(index) = state.stack.iter().position(|active| {
        active.module_name == frame.module_name && active.type_name == frame.type_name
    }) {
        state.stack_cacheable[index + 1..].fill(false);
        if traversal == FormatNeutralSchemaTraversal::Decode {
            return Some(descriptor_ty);
        }
        return type_arguments
            .iter()
            .all(|arg| {
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    descriptor.module_name.as_deref(),
                    adts,
                    arg,
                    state,
                    traversal,
                )
                .is_some()
            })
            .then_some(descriptor_ty);
    }
    state.stack.push(frame.clone());
    state.stack_cacheable.push(true);
    let supported = descriptor.variants.iter().all(|variant| {
        variant
            .payload_fields
            .iter()
            .enumerate()
            .all(|(index, _field)| {
                let Some(payload_ty) = adt::payload_type(
                    &descriptor_ty,
                    adt::AdtConstructor {
                        descriptor,
                        variant,
                    },
                    index,
                ) else {
                    return false;
                };
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    descriptor.module_name.as_deref(),
                    adts,
                    &payload_ty,
                    state,
                    traversal,
                )
                .is_some()
            })
    });
    state.stack.pop();
    if state
        .stack_cacheable
        .pop()
        .expect("ADT traversal cacheability should match the active stack")
    {
        state.completed.push((frame, supported));
    }
    supported.then_some(descriptor_ty)
}

fn format_neutral_schema_descriptor_type(ty: &Type, descriptor: &adt::AdtDescriptor) -> Type {
    let Type::Named { args, .. } = ty else {
        return ty.clone();
    };
    Type::named(descriptor.type_name.clone(), args.clone())
}

fn format_neutral_schema_source_adt_descriptor<'a>(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &'a AdtRegistry,
    ty: &Type,
) -> Option<&'a adt::AdtDescriptor> {
    let Type::Named { name, args } = ty else {
        return None;
    };
    let segments = name.split("::").collect::<Vec<_>>();
    match segments.as_slice() {
        [local_name] => adts
            .descriptor_for_type_in_module(&Type::named(*local_name, args.clone()), current_module),
        [_, .., type_name] => {
            let import_path = segments[..segments.len() - 1]
                .iter()
                .map(|segment| (*segment).to_string())
                .collect::<Vec<_>>();
            let use_decl = imported_use_for_path(&module.uses, &import_path, current_module)?;
            adts.descriptors().iter().rev().find(|descriptor| {
                descriptor.type_name == *type_name
                    && descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
                    && descriptor.type_parameters.len() == args.len()
                    && descriptor.visibility == Visibility::Public
            })
        }
        _ => None,
    }
}

pub(crate) fn schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type, u8)>> {
    schema_decode_record_fields_inner(module, schema, &mut Vec::new())
}

fn schema_decode_record_fields_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Vec<(String, Type, u8)>> {
    let schema_name = schema.name.as_ref()?;
    if stack.iter().any(|name| name == schema_name) {
        return None;
    }
    stack.push(schema_name.clone());
    let fields = schema_decode_record_fields_inner_after_push(module, schema, stack);
    stack.pop();
    fields
}

fn schema_decode_record_fields_inner_after_push(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Vec<(String, Type, u8)>> {
    if schema.format.is_none() {
        return format_neutral_schema_decode_record_fields(module, schema)
            .map(|fields| fields.into_iter().map(|(name, ty)| (name, ty, 0)).collect());
    }
    let mut decoded_fields = BTreeMap::<String, Type>::new();
    let mut fields = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        let decoded = schema_decode_binary_record_field(
            module,
            schema,
            &decoded_fields,
            index,
            field,
            stack,
        )?;
        let SchemaDecodedRecordField::Visible { ty, width } = decoded else {
            continue;
        };
        decoded_fields.insert(field.name.clone(), ty.clone());
        fields.push((field.name.clone(), ty, width));
    }
    Some(fields)
}

enum SchemaDecodedRecordField {
    Omitted,
    Visible { ty: Type, width: u8 },
}

fn schema_decode_binary_record_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    index: usize,
    field: &SchemaField,
    stack: &mut Vec<String>,
) -> Option<SchemaDecodedRecordField> {
    if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
        supported_encode_reserved_bits(&schema.fields, index, reserved)?;
        return Some(SchemaDecodedRecordField::Omitted);
    }
    if let Some(width) = exact_width_schema_primitive(&field.ty) {
        return Some(SchemaDecodedRecordField::Visible {
            ty: Type::int(),
            width,
        });
    }
    if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
        return schema_references_are_decoded_ints(decoded_fields, length_expr.references())
            .then_some(SchemaDecodedRecordField::Visible {
                ty: Type::named("ByteView", Vec::new()),
                width: 0,
            });
    }
    if let Some(repeat) = repeat_schema_primitive(&field.ty) {
        return schema_decode_repeat_record_field(module, schema, decoded_fields, &repeat, stack);
    }
    if let Some(nested) = schema_field_target(module, schema, &field.ty)
        && nested.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
    {
        return Some(SchemaDecodedRecordField::Visible {
            ty: schema_decode_value_type_inner(module, nested, stack)?,
            width: 0,
        });
    }
    if let Some(ty) = binary_schema_anonymous_record_decode_type(&field.ty) {
        return Some(SchemaDecodedRecordField::Visible { ty, width: 0 });
    }
    schema_decode_dispatch_record_field(module, schema, decoded_fields, field, stack)
}

fn schema_decode_repeat_record_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    repeat: &SchemaRepeatSpec,
    stack: &mut Vec<String>,
) -> Option<SchemaDecodedRecordField> {
    let count_references = schema_length_expression_references(&repeat.count_field)?;
    if !schema_references_are_decoded_ints(decoded_fields, count_references) {
        return None;
    }
    if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload {
        let length_references = schema_length_expression_references(length_field)?;
        if !schema_references_are_decoded_ints(decoded_fields, length_references) {
            return None;
        }
    }
    if let SchemaRepeatPayload::ReservedBits { .. } = repeat.payload {
        return Some(SchemaDecodedRecordField::Omitted);
    }
    let element_ty = schema_repeat_payload_type(module, schema, repeat, stack)?;
    Some(SchemaDecodedRecordField::Visible {
        ty: Type::named("List", vec![element_ty]),
        width: 0,
    })
}

fn schema_decode_dispatch_record_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    field: &SchemaField,
    stack: &mut Vec<String>,
) -> Option<SchemaDecodedRecordField> {
    let dispatch = closed_dispatch_schema_primitive(&field.ty)
        .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
    let references =
        std::iter::once(dispatch.tag_field.as_str()).chain(dispatch.length_field.as_deref());
    if !schema_references_are_decoded_ints(decoded_fields, references) {
        return None;
    }
    let payload_types = schema_dispatch_case_types(module, schema, &dispatch, stack)?;
    let payload_ty = schema_dispatch_payload_type(module, schema, &dispatch, &payload_types)?;
    let ty = if dispatch.preserves_unknown {
        Type::named("SchemaDispatchPayload", vec![payload_ty])
    } else {
        payload_ty
    };
    Some(SchemaDecodedRecordField::Visible { ty, width: 0 })
}

fn schema_references_are_decoded_ints<'a>(
    decoded_fields: &BTreeMap<String, Type>,
    references: impl IntoIterator<Item = &'a str>,
) -> bool {
    references.into_iter().all(|reference| {
        schema_field_reference_type(decoded_fields, reference) == Some(&Type::int())
    })
}

fn schema_dispatch_case_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    stack: &mut Vec<String>,
) -> Option<Vec<(i64, Type)>> {
    dispatch
        .cases
        .iter()
        .map(|case| {
            let ty = schema_dispatch_case_type(module, schema, case, stack)?;
            Some((case.tag, ty))
        })
        .collect()
}

fn schema_dispatch_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    payload_types: &[(i64, Type)],
) -> Option<Type> {
    let first = payload_types.first()?.1.clone();
    if payload_types.iter().all(|(_, ty)| ty == &first) {
        Some(first)
    } else if dispatch.length_field.is_some()
        && dispatch.cases.iter().any(|case| {
            matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name }
                if recursive_dispatch_decode_only_payload_case_is_eligible(
                    module,
                    schema,
                    dispatch,
                    schema_name,
                )
            )
        })
    {
        schema_recursive_dispatch_helper_payload_type(module, schema, dispatch)
    } else {
        None
    }
}

pub(crate) fn schema_dispatch_case_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: &SchemaDispatchCase,
    stack: &mut Vec<String>,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if schema.name.as_deref() == Some(schema_name.as_str()) {
                return schema_recursive_dispatch_payload_type(module, schema);
            }
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            if schema_payload_has_generalized_reserved_byte_prefix(nested) {
                return None;
            }
            schema_decode_value_type_inner(module, nested, stack)
        }
    }
}

fn schema_encode_dispatch_case_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    case: &SchemaDispatchCase,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if recursive_dispatch_payload_case_is_eligible(
                module,
                schema,
                field,
                dispatch,
                schema_name,
            ) {
                return schema_recursive_dispatch_payload_type(module, schema);
            }
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            if schema_payload_has_generalized_reserved_byte_prefix(nested) {
                return None;
            }
            schema_encode_value_type(module, nested)
        }
    }
}

fn schema_repeat_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    repeat: &SchemaRepeatSpec,
    stack: &mut Vec<String>,
) -> Option<Type> {
    match &repeat.payload {
        SchemaRepeatPayload::Primitive { .. } => Some(Type::int()),
        SchemaRepeatPayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaRepeatPayload::ByteView { .. } => Some(Type::named("ByteView", Vec::new())),
        SchemaRepeatPayload::Schema { schema_name } => {
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            if schema_payload_has_generalized_reserved_byte_prefix(nested) {
                return None;
            }
            schema_decode_value_type_inner(module, nested, stack)
        }
    }
}

fn schema_decode_value_type_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Type> {
    let fields = schema_decode_record_fields_inner(module, schema, stack)?;
    Some(Type::Record(
        fields.into_iter().map(|(name, ty, _)| (name, ty)).collect(),
    ))
}

pub(crate) fn schema_recursive_dispatch_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    let schema_name = schema.name.as_deref()?;
    schema.fields.iter().find_map(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .filter(|dispatch| {
                recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name)
            })
            .and_then(|dispatch| {
                schema_recursive_dispatch_helper_payload_type(module, schema, &dispatch)
            })
    })
}

pub(crate) fn schema_imported_recursive_dispatch_payload_type(
    _module: &SurfaceModule,
    _schema: &SchemaDecl,
    _dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    None
}

pub(crate) fn schema_recursive_dispatch_helper_payload_type(
    _module: &SurfaceModule,
    _schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    dispatch
        .cases
        .iter()
        .any(|case| matches!(case.payload, SchemaDispatchCasePayload::Primitive { .. }))
        .then_some(Type::int())
}

pub(crate) fn recursive_dispatch_payload_is_eligible(
    schema: &SchemaDecl,
    _field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema.name.as_deref() == Some(schema_name)
        && dispatch_has_recursive_schema_payload_case(dispatch, schema_name)
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_primitive_payload_case(dispatch)
}

pub(crate) fn recursive_dispatch_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    if recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name) {
        return true;
    }
    imported_recursive_dispatch_payload_case_is_eligible(module, schema, dispatch, schema_name)
        && schema_imported_recursive_dispatch_payload_type(module, schema, dispatch).is_some()
}

pub(crate) fn imported_recursive_dispatch_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema_name.contains("::")
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_payload_case(module, schema, dispatch)
        && recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
}

pub(crate) fn recursive_dispatch_decode_only_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    imported_recursive_dispatch_decode_only_payload_case_is_eligible(
        module,
        schema,
        dispatch,
        schema_name,
    ) || (!schema_name.contains("::")
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_primitive_payload_case(dispatch)
        && recursive_dispatch_payload_target_is_eligible(module, schema, schema_name))
}

fn imported_recursive_dispatch_decode_only_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema_name.contains("::")
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_primitive_payload_case(dispatch)
        && recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
}

pub(crate) fn schema_has_eligible_recursive_dispatch_payload(schema: &SchemaDecl) -> bool {
    let Some(schema_name) = schema.name.as_deref() else {
        return false;
    };
    schema.fields.iter().any(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .is_some_and(|dispatch| {
                recursive_dispatch_payload_is_eligible(schema, field, &dispatch, schema_name)
            })
    })
}

pub(crate) fn schema_has_recursive_dispatch_payload(schema: &SchemaDecl) -> bool {
    let Some(schema_name) = schema.name.as_deref() else {
        return false;
    };
    schema.fields.iter().any(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .is_some_and(|dispatch| {
                dispatch.cases.iter().any(|case| {
                    matches!(
                        &case.payload,
                        SchemaDispatchCasePayload::Schema { schema_name: payload_name }
                            if payload_name == schema_name
                    )
                })
            })
    })
}

fn recursive_dispatch_payload_target_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> bool {
    schema_dispatch_payload_schema(module, schema, schema_name)
        .is_some_and(schema_has_eligible_recursive_dispatch_payload)
}

fn dispatch_has_non_recursive_payload_case(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    dispatch.cases.iter().any(|case| match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => true,
        SchemaDispatchCasePayload::ReservedBits { .. } => true,
        SchemaDispatchCasePayload::Schema { schema_name } => {
            !recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
        }
    })
}

fn dispatch_has_non_recursive_primitive_payload_case(dispatch: &SchemaDispatchSpec) -> bool {
    dispatch
        .cases
        .iter()
        .any(|case| matches!(case.payload, SchemaDispatchCasePayload::Primitive { .. }))
}

fn dispatch_has_recursive_schema_payload_case(
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name: payload_name }
                if payload_name == schema_name
        )
    })
}

pub(crate) fn same_module_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    if schema_name.contains("::") {
        return None;
    }
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    module
        .schemas
        .iter()
        .enumerate()
        .find_map(|(index, candidate)| {
            (candidate.name.as_deref() == Some(schema_name)
                && candidate.module_name.as_deref() == schema.module_name.as_deref()
                && candidate.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
                && index < current_index)
                .then_some(candidate)
        })
}

pub(crate) fn schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    let segments = schema_payload_name_path(schema_name)?;
    match segments.as_slice() {
        [name] => same_module_schema(module, schema, name),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            )?;
            let target_module = Some(use_decl.name.as_str());
            module.schemas.iter().find(|candidate| {
                candidate.name.as_deref() == Some(name)
                    && candidate.module_name.as_deref() == target_module
                    && candidate.visibility == Visibility::Public
                    && candidate.format.as_ref().map(|format| format.name.as_str())
                        == Some("binary")
            })
        }
        _ => None,
    }
}

pub(crate) fn schema_decode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_decode_value_type_inner(module, schema, &mut Vec::new())
}

pub(crate) fn schema_payload_name_path(text: &str) -> Option<Vec<String>> {
    let segments = text.split("::").map(str::trim).collect::<Vec<_>>();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| !is_schema_identifier(segment))
    {
        return None;
    }
    Some(segments.into_iter().map(str::to_string).collect())
}

pub(crate) fn schema_payload_name_is_path(text: &str) -> bool {
    schema_payload_name_path(text).is_some()
}

pub(crate) fn schema_payload_name_last_segment(text: &str) -> &str {
    text.rsplit("::").next().unwrap_or(text)
}

fn schema_encode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_encode_function_signature_for_schema(module, schema))
        .collect()
}

fn schema_validate_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_validate_function_signature_for_schema(module, schema))
        .collect()
}

fn schema_validate_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let fields = schema_decode_record_fields(module, schema)?
        .into_iter()
        .map(|(name, ty, _)| (name, ty))
        .collect::<Vec<_>>();
    let decoded_type = Type::Record(fields);
    Some(FunctionSignature {
        name: schema_validate_function_name(schema_name),
        target_name: format!("{SCHEMA_VALIDATE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![decoded_type.clone()],
        variadic: None,
        return_type: Type::named("Result", vec![decoded_type, Type::string()]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

fn schema_encode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.is_none() {
        let value_type = Type::Record(format_neutral_schema_encode_record_fields(module, schema)?);
        return Some(FunctionSignature {
            name: schema_encode_function_name(schema_name),
            target_name: format!("{SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![value_type.clone()],
            variadic: None,
            return_type: Type::named("Result", vec![value_type, Type::string()]),
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        });
    }
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let (fields, exact_width_field_names) =
        schema_encode::schema_encode_schema_fields(module, schema)?;
    let value_fields =
        schema_encode_value_fields(module, schema, &fields, &exact_width_field_names)?;
    let byte_chunk = Type::named("ByteChunk", Vec::new());
    let encode_error = Type::named("EncodeError", Vec::new());
    Some(FunctionSignature {
        name: schema_encode_function_name(schema_name),
        target_name: format!("{SCHEMA_ENCODE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![Type::Record(value_fields)],
        variadic: None,
        return_type: Type::named("Result", vec![byte_chunk, encode_error]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

pub(crate) fn schema_encode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_encode_function_signature_for_schema(module, schema)
        .and_then(|signature| signature.params.into_iter().next())
}

fn schema_encode_value_fields(
    _module: &SurfaceModule,
    _schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    _exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    Some(schema_fields.to_vec())
}

pub(crate) fn schema_decode_function_name(schema_name: &str) -> String {
    format!("byte_decode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_decode_step_function_name(schema_name: &str) -> String {
    format!("byte_decode_step_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_encode_function_name(schema_name: &str) -> String {
    format!("byte_encode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_validate_function_name(schema_name: &str) -> String {
    format!("validate_{}", snake_case_identifier(schema_name))
}

pub(crate) fn exact_width_schema_primitive(ty: &str) -> Option<u8> {
    match canonical_schema_primitive_name(ty).as_deref() {
        Some(name) => exact_width_schema_primitive(name),
        None => match ty.trim() {
            "UInt1" | "UInt2" | "UInt3" | "UInt4" | "UInt5" | "UInt6" | "UInt7" => Some(1),
            "UInt8" => Some(1),
            "UInt16be" | "UInt16le" => Some(2),
            "UInt24be" | "UInt24le" => Some(3),
            "UInt31be" | "UInt31le" | "UInt32be" | "UInt32le" => Some(4),
            "UInt40be" | "UInt40le" => Some(5),
            "UInt48be" | "UInt48le" => Some(6),
            "UInt56be" | "UInt56le" => Some(7),
            "UInt64be" | "UInt64le" => Some(8),
            _ => None,
        },
    }
}

pub(crate) fn exact_width_schema_primitive_little_endian(ty: &str) -> bool {
    let canonical = canonical_schema_primitive_name(ty);
    let name = canonical.as_deref().unwrap_or_else(|| ty.trim());
    matches!(
        name,
        "UInt16le"
            | "UInt24le"
            | "UInt31le"
            | "UInt32le"
            | "UInt40le"
            | "UInt48le"
            | "UInt56le"
            | "UInt64le"
    )
}

pub(crate) fn exact_width_schema_primitive_bit_width(ty: &str) -> Option<u8> {
    match canonical_schema_primitive_name(ty).as_deref() {
        Some(name) => exact_width_schema_primitive_bit_width(name),
        None => match ty.trim() {
            "UInt1" => Some(1),
            "UInt2" => Some(2),
            "UInt3" => Some(3),
            "UInt4" => Some(4),
            "UInt5" => Some(5),
            "UInt6" => Some(6),
            "UInt7" => Some(7),
            "UInt8" => Some(8),
            "UInt16be" | "UInt16le" => Some(16),
            "UInt24be" | "UInt24le" => Some(24),
            "UInt31be" | "UInt31le" => Some(31),
            "UInt32be" | "UInt32le" => Some(32),
            "UInt40be" | "UInt40le" => Some(40),
            "UInt48be" | "UInt48le" => Some(48),
            "UInt56be" | "UInt56le" => Some(56),
            "UInt64be" | "UInt64le" => Some(64),
            _ => None,
        },
    }
}

pub(crate) fn exact_width_schema_primitive_max_value(ty: &str) -> Option<i64> {
    match canonical_schema_primitive_name(ty).as_deref() {
        Some(name) => exact_width_schema_primitive_max_value(name),
        None => match ty.trim() {
            "UInt1" => Some(0x1),
            "UInt2" => Some(0x3),
            "UInt3" => Some(0x7),
            "UInt4" => Some(0xf),
            "UInt5" => Some(0x1f),
            "UInt6" => Some(0x3f),
            "UInt7" => Some(0x7f),
            "UInt8" => Some(0xff),
            "UInt16be" | "UInt16le" => Some(0xffff),
            "UInt24be" | "UInt24le" => Some(0xffffff),
            "UInt31be" | "UInt31le" => Some(0x7fffffff),
            "UInt32be" | "UInt32le" => Some(0xffffffff),
            "UInt40be" | "UInt40le" => Some(0xffffffffff),
            "UInt48be" | "UInt48le" => Some(0xffffffffffff),
            "UInt56be" | "UInt56le" => Some(0xffffffffffffff),
            "UInt64be" | "UInt64le" => Some(i64::MAX),
            _ => None,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LowercaseSchemaPrimitive {
    pub(crate) spelling: String,
    pub(crate) family: &'static str,
    pub(crate) width_bits: u16,
    pub(crate) endian: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LowercaseSchemaPrimitiveError {
    MissingWidth,
    UnknownEndian,
    MissingEndian,
    RedundantEndian,
    UnsupportedWidth,
    ReservesValue,
}

impl LowercaseSchemaPrimitive {
    pub(crate) fn canonical_name(&self) -> String {
        let family = match self.family {
            "uint" => "UInt",
            _ => unreachable!("schema primitive family is fixed"),
        };
        match self.endian {
            Some(endian) => format!("{family}{}{endian}", self.width_bits),
            None => format!("{family}{}", self.width_bits),
        }
    }
}

pub(crate) fn lowercase_schema_primitive(
    text: &str,
) -> Option<Result<LowercaseSchemaPrimitive, LowercaseSchemaPrimitiveError>> {
    let spelling = text.trim();
    let rest = spelling.strip_prefix("uint")?;
    let family = "uint";
    if rest.is_empty() {
        return Some(Err(LowercaseSchemaPrimitiveError::MissingWidth));
    }
    if !rest.chars().next().is_some_and(|ch| ch.is_ascii_digit()) {
        return match rest {
            "be" | "le" => Some(Err(LowercaseSchemaPrimitiveError::MissingWidth)),
            _ => None,
        };
    }
    let width_len = rest
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
        .unwrap_or(0);
    if width_len == 0 {
        return Some(Err(LowercaseSchemaPrimitiveError::MissingWidth));
    }
    let width_text = &rest[..width_len];
    let suffix = &rest[width_len..];
    let Ok(width_bits) = width_text.parse::<u16>() else {
        return Some(Err(LowercaseSchemaPrimitiveError::UnsupportedWidth));
    };
    let endian = match suffix {
        "" => None,
        "be" => Some("be"),
        "le" => Some("le"),
        _ => return Some(Err(LowercaseSchemaPrimitiveError::UnknownEndian)),
    };
    let supported_width = matches!(
        width_bits,
        1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 16 | 24 | 31 | 32 | 40 | 48 | 56 | 64
    );
    if !supported_width {
        return Some(Err(LowercaseSchemaPrimitiveError::UnsupportedWidth));
    }
    if width_bits <= 8 && endian.is_some() {
        return Some(Err(LowercaseSchemaPrimitiveError::RedundantEndian));
    }
    if width_bits > 8 && endian.is_none() {
        return Some(Err(LowercaseSchemaPrimitiveError::MissingEndian));
    }
    Some(Ok(LowercaseSchemaPrimitive {
        spelling: spelling.to_string(),
        family,
        width_bits,
        endian,
    }))
}

pub(crate) fn lowercase_reserved_bits_schema_primitive(
    text: &str,
) -> Option<Result<(i64, i64), LowercaseSchemaPrimitiveError>> {
    let spelling = text.trim();
    let mut parts = spelling.split_whitespace();
    let primitive_text = parts.next()?;
    if parts.next()? != "reserves" {
        return None;
    }
    let Some(value_text) = parts.next() else {
        return Some(Err(LowercaseSchemaPrimitiveError::ReservesValue));
    };
    if parts.next().is_some() {
        return Some(Err(LowercaseSchemaPrimitiveError::ReservesValue));
    }
    let primitive = match lowercase_schema_primitive(primitive_text)? {
        Ok(primitive) => primitive,
        Err(reason) => return Some(Err(reason)),
    };
    let Some(value) = parse_reserved_bits_integer(value_text) else {
        return Some(Err(LowercaseSchemaPrimitiveError::ReservesValue));
    };
    Some(Ok((i64::from(primitive.width_bits), value)))
}

pub(crate) fn canonical_schema_primitive_name(text: &str) -> Option<String> {
    match lowercase_schema_primitive(text)? {
        Ok(primitive) => Some(primitive.canonical_name()),
        Err(_) => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ByteViewLengthExpr {
    Field(String),
    Sum { left: String, right: String },
    Difference { left: String, right: String },
    Product { left: String, right: String },
    Quotient { left: String, right: String },
}

impl ByteViewLengthExpr {
    pub(crate) fn references(&self) -> Vec<&str> {
        match self {
            Self::Field(field) => vec![field.as_str()],
            Self::Sum { left, right } => vec![left.as_str(), right.as_str()],
            Self::Difference { left, right } => vec![left.as_str(), right.as_str()],
            Self::Product { left, right } => vec![left.as_str(), right.as_str()],
            Self::Quotient { left, right } => vec![left.as_str(), right.as_str()],
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Field(field) => field.clone(),
            Self::Sum { left, right } => format!("{left} + {right}"),
            Self::Difference { left, right } => format!("{left} - {right}"),
            Self::Product { left, right } => format!("{left} * {right}"),
            Self::Quotient { left, right } => format!("{left} / {right}"),
        }
    }
}

pub(crate) fn schema_length_expression(text: &str) -> Option<ByteViewLengthExpr> {
    schema_length_expression_with_product(text, true)
}

fn schema_length_expression_with_product(
    text: &str,
    allow_product: bool,
) -> Option<ByteViewLengthExpr> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(ByteViewLengthExpr::Field(text.to_string()));
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '+') {
        return Some(ByteViewLengthExpr::Sum {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '-') {
        return Some(ByteViewLengthExpr::Difference {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '/') {
        return Some(ByteViewLengthExpr::Quotient {
            left: left.to_string(),
            right: right.to_string(),
        });
    }
    if !allow_product {
        return None;
    }
    let (left, right) = schema_length_binary_expression_operands(text, '*')?;
    Some(ByteViewLengthExpr::Product {
        left: left.to_string(),
        right: right.to_string(),
    })
}

pub(crate) fn schema_length_expression_references(text: &str) -> Option<Vec<&str>> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(vec![text]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '+') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '-') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '*') {
        return Some(vec![left, right]);
    }
    if let Some((left, right)) = schema_length_binary_expression_operands(text, '/') {
        return Some(vec![left, right]);
    }
    None
}

fn schema_length_binary_expression_operands(text: &str, op: char) -> Option<(&str, &str)> {
    for other_op in ['+', '-', '*', '/'] {
        if other_op != op && text.contains(other_op) {
            return None;
        }
    }
    let (left, right) = text.split_once(op)?;
    if right.contains(op) {
        return None;
    }
    let left = left.trim();
    let right = right.trim();
    if is_simple_schema_field_reference(left) && is_simple_schema_field_reference(right) {
        Some((left, right))
    } else {
        None
    }
}

pub(crate) fn byte_view_schema_primitive(ty: &str) -> Option<ByteViewLengthExpr> {
    let text = ty.trim();
    let inner = text.strip_prefix("ByteView(")?.strip_suffix(')')?.trim();
    schema_length_expression_with_product(inner, true)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ByteViewMultipleConstraint {
    Field(String),
    Literal(i64),
}

impl ByteViewMultipleConstraint {
    pub(crate) fn reference(&self) -> Option<&str> {
        match self {
            Self::Field(field) => Some(field.as_str()),
            Self::Literal(_) => None,
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Field(field) => field.clone(),
            Self::Literal(value) => value.to_string(),
        }
    }
}

pub(crate) fn byte_view_multiple_constraint(predicate: &str) -> Option<ByteViewMultipleConstraint> {
    let divisor = predicate
        .trim()
        .strip_prefix("payload_count multiple of ")?
        .trim();
    if divisor.is_empty() || divisor.contains(char::is_whitespace) {
        return None;
    }
    if let Ok(literal) = parse_integer_literal(divisor) {
        return (literal.value > 0).then_some(ByteViewMultipleConstraint::Literal(literal.value));
    }
    is_simple_schema_field_reference(divisor)
        .then(|| ByteViewMultipleConstraint::Field(divisor.to_string()))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaRepeatSpec {
    pub(crate) count_field: String,
    pub(crate) payload: SchemaRepeatPayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaRepeatPayload {
    Primitive {
        width: u8,
        max_value: i64,
        little_endian: bool,
    },
    ReservedBits {
        bit_width: u8,
        expected_value: i64,
    },
    ByteView {
        length_field: String,
    },
    Schema {
        schema_name: String,
    },
}

pub(crate) fn repeat_schema_primitive(ty: &str) -> Option<SchemaRepeatSpec> {
    if let Some((payload, count_field)) = canonical_repeat_schema_primitive_parts(ty) {
        return repeat_schema_primitive_from_parts(count_field, payload);
    }
    let inner = schema_call_inner(ty, "Repeat")?;
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    let [count_field, primitive] = args.as_slice() else {
        return None;
    };
    repeat_schema_primitive_from_parts(count_field, primitive)
}

fn repeat_schema_primitive_from_parts(
    count_field: &str,
    primitive: &str,
) -> Option<SchemaRepeatSpec> {
    let count_expr = schema_length_expression(count_field)?;
    let payload = if let Some(width) = exact_width_schema_primitive(primitive) {
        if exact_width_schema_primitive_bit_width(primitive)? < 8 {
            return None;
        }
        SchemaRepeatPayload::Primitive {
            width,
            max_value: exact_width_schema_primitive_max_value(primitive)?,
            little_endian: exact_width_schema_primitive_little_endian(primitive),
        }
    } else if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(primitive) {
        SchemaRepeatPayload::ReservedBits {
            bit_width: dispatch_reserved_bits_width(bit_width, expected_value)?,
            expected_value,
        }
    } else if let Some(length_expr) = byte_view_schema_primitive(primitive) {
        match length_expr {
            ByteViewLengthExpr::Field(_)
            | ByteViewLengthExpr::Sum { .. }
            | ByteViewLengthExpr::Difference { .. } => SchemaRepeatPayload::ByteView {
                length_field: length_expr.render(),
            },
            ByteViewLengthExpr::Product { .. } | ByteViewLengthExpr::Quotient { .. } => {
                return None;
            }
        }
    } else if schema_payload_name_path(primitive).is_some() {
        SchemaRepeatPayload::Schema {
            schema_name: (*primitive).to_string(),
        }
    } else {
        return None;
    };
    Some(SchemaRepeatSpec {
        count_field: count_expr.render(),
        payload,
    })
}

pub(crate) fn schema_repeat_payload_accepts_lowercase_primitive(text: &str) -> bool {
    (lowercase_schema_primitive(text).is_some()
        || lowercase_reserved_bits_schema_primitive(text).is_some())
        && repeat_schema_primitive_from_parts("count", text).is_some()
}

fn canonical_repeat_schema_primitive_parts(ty: &str) -> Option<(&str, &str)> {
    let text = ty.trim();
    let inner = text.strip_prefix('[')?.strip_suffix(']')?.trim();
    let (payload, count) = split_top_level_once(inner, ';')?;
    if count.contains(';') {
        return None;
    }
    let payload = payload.trim();
    let count = count.trim();
    if payload.is_empty() || count.is_empty() {
        return None;
    }
    Some((payload, count))
}

fn split_top_level_once(text: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ if ch == delimiter && depth == 0 => {
                return Some((&text[..index], &text[index + ch.len_utf8()..]));
            }
            _ => {}
        }
    }
    None
}

fn is_simple_schema_field_reference(text: &str) -> bool {
    !text.is_empty() && text.split('.').all(is_schema_identifier)
}

pub(crate) fn schema_field_reference_type<'a>(
    fields: &'a BTreeMap<String, Type>,
    reference: &str,
) -> Option<&'a Type> {
    let mut segments = reference.split('.');
    let mut ty = fields.get(segments.next()?)?;
    for segment in segments {
        let Type::Record(record_fields) = ty else {
            return None;
        };
        ty = record_fields
            .iter()
            .find_map(|(name, ty)| (name == segment).then_some(ty))?;
    }
    Some(ty)
}

pub(crate) fn reserved_bits_schema_primitive(ty: &str) -> Option<(i64, i64)> {
    if let Some(reserved) = lowercase_reserved_bits_schema_primitive(ty) {
        return reserved.ok();
    }
    let rest = ty.strip_prefix("ReservedBits")?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    if !rest.starts_with('(') || !rest.ends_with(')') {
        return None;
    }
    let inner = rest[1..rest.len() - 1].trim();
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    let [width, value] = args.as_slice() else {
        return None;
    };
    let width = parse_reserved_bits_integer(width)?;
    let value = parse_reserved_bits_integer(value)?;
    Some((width, value))
}

fn canonical_schema_primitive_is(ty: &str, expected: &str) -> bool {
    canonical_schema_primitive_name(ty)
        .as_deref()
        .unwrap_or_else(|| ty.trim())
        == expected
}

pub(crate) fn supported_encode_reserved_bits(
    fields: &[veln_ast::SchemaField],
    index: usize,
    reserved: (i64, i64),
) -> Option<(u8, i64)> {
    ReservedBitsEncodeContext {
        fields,
        index,
        bit_width: reserved.0,
        expected_value: reserved.1,
    }
    .is_supported()
    .then_some((reserved.0 as u8, reserved.1))
}

struct ReservedBitsEncodeContext<'a> {
    fields: &'a [veln_ast::SchemaField],
    index: usize,
    bit_width: i64,
    expected_value: i64,
}

impl ReservedBitsEncodeContext<'_> {
    fn is_supported(&self) -> bool {
        supported_bit_packed_reserved_group(self.fields, self.index)
            || supported_byte_interleaved_reserved_group(
                self.fields,
                self.index,
                self.bit_width,
                self.expected_value,
            )
            || self.supports_forward_layout()
            || self.supports_backward_layout()
            || self.supports_middle_layout()
            || self.supports_standalone_layout()
    }

    fn supports_forward_layout(&self) -> bool {
        let next = self.next();
        (self.bit_width == 1
            && self.expected_value == 0
            && next.is_some_and(|field| canonical_schema_primitive_is(&field.ty, "UInt31be")))
            || supported_reserved_byte_prefix(self.bit_width, self.expected_value, next)
            || self.supports_packed_prefix(next)
            || next.zip(self.next_next()).is_some_and(|(first, second)| {
                supported_prefix_reserved_group(first, second, self.bit_width, self.expected_value)
            })
    }

    fn supports_packed_prefix(&self, next: Option<&veln_ast::SchemaField>) -> bool {
        packed_reserved_storage_bit_width(self.bit_width).is_some_and(|storage_bit_width| {
            next.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
                .is_some_and(|next_bit_width| {
                    i64::from(next_bit_width) + self.bit_width == storage_bit_width
                })
                && self
                    .maximum_value()
                    .is_some_and(|max_value| self.expected_value <= max_value)
        })
    }

    fn supports_backward_layout(&self) -> bool {
        let previous = self.previous();
        self.supports_packed_suffix(previous)
            || previous.is_some_and(|field| {
                supported_byte_visible_reserved_suffix(field, (self.bit_width, self.expected_value))
            })
            || self
                .previous_previous()
                .zip(previous)
                .is_some_and(|(first, second)| {
                    supported_suffix_reserved_group(
                        first,
                        second,
                        self.bit_width,
                        self.expected_value,
                    )
                })
    }

    fn supports_packed_suffix(&self, previous: Option<&veln_ast::SchemaField>) -> bool {
        suffix_packed_reserved_storage_bit_width(self.bit_width).is_some_and(|storage_bit_width| {
            !self.previous_previous().is_some_and(|field| {
                previous.is_some_and(|visible| supported_packed_reserved_prefix(field, visible))
            }) && previous
                .and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
                .is_some_and(|previous_bit_width| {
                    i64::from(previous_bit_width) + self.bit_width == storage_bit_width
                })
                && self
                    .maximum_value()
                    .is_some_and(|max_value| self.expected_value <= max_value)
        })
    }

    fn supports_middle_layout(&self) -> bool {
        self.previous()
            .zip(self.next())
            .is_some_and(|(previous, next)| {
                supported_middle_reserved_bits(previous, next, self.bit_width, self.expected_value)
            })
    }

    fn supports_standalone_layout(&self) -> bool {
        self.bit_width > 0
            && self.bit_width <= 32
            && self.bit_width % 8 == 0
            && self
                .maximum_value()
                .is_some_and(|max_value| self.expected_value <= max_value)
    }

    fn maximum_value(&self) -> Option<i64> {
        if self.bit_width == 32 {
            Some(0xffff_ffff)
        } else {
            reserved_bits_max_value(self.bit_width)
        }
    }

    fn previous_previous(&self) -> Option<&veln_ast::SchemaField> {
        self.index
            .checked_sub(2)
            .and_then(|index| self.fields.get(index))
    }

    fn previous(&self) -> Option<&veln_ast::SchemaField> {
        self.index
            .checked_sub(1)
            .and_then(|index| self.fields.get(index))
    }

    fn next(&self) -> Option<&veln_ast::SchemaField> {
        self.fields.get(self.index + 1)
    }

    fn next_next(&self) -> Option<&veln_ast::SchemaField> {
        self.fields.get(self.index + 2)
    }
}

fn supported_bit_packed_reserved_group(fields: &[veln_ast::SchemaField], index: usize) -> bool {
    for start in 0..=index {
        let mut total_bit_width = 0_i64;
        let mut has_reserved = false;
        let mut has_visible = false;
        for (offset, field) in fields[start..].iter().enumerate() {
            let Some(bit_width) = bit_packed_group_field_width(field) else {
                break;
            };
            total_bit_width += bit_width;
            has_reserved |= reserved_bits_schema_primitive(&field.ty).is_some();
            has_visible |= reserved_bits_schema_primitive(&field.ty).is_none();
            if matches!(total_bit_width, 8 | 16 | 24 | 32 | 40 | 48 | 56 | 64) {
                let end = start + offset;
                if has_reserved && has_visible && start <= index && index <= end {
                    return true;
                }
                break;
            }
            if total_bit_width > 64 {
                break;
            }
        }
    }
    false
}

fn bit_packed_group_field_width(field: &veln_ast::SchemaField) -> Option<i64> {
    if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&field.ty) {
        if bit_width <= 0 || bit_width >= 64 || bit_width % 8 == 0 {
            return None;
        }
        let max_value = reserved_bits_max_value(bit_width)?;
        return (expected_value <= max_value).then_some(bit_width);
    }
    if exact_width_schema_primitive_little_endian(&field.ty) {
        return None;
    }
    let bit_width = i64::from(exact_width_schema_primitive_bit_width(&field.ty)?);
    (bit_width % 8 != 0).then_some(bit_width)
}

fn reserved_bits_max_value(bit_width: i64) -> Option<i64> {
    if !(1..=63).contains(&bit_width) {
        return None;
    }
    if bit_width == 63 {
        return Some(i64::MAX);
    }
    Some((1_i64 << bit_width) - 1)
}

fn supported_prefix_reserved_group(
    first_visible_field: &veln_ast::SchemaField,
    second_visible_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 57 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&first_visible_field.ty)
        || exact_width_schema_primitive_little_endian(&second_visible_field.ty)
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_visible_field.ty)
    else {
        return false;
    };
    let Some(second_bit_width) = exact_width_schema_primitive_bit_width(&second_visible_field.ty)
    else {
        return false;
    };
    if first_bit_width > 8 || second_bit_width > 8 {
        return false;
    }
    let total_bit_width = bit_width + i64::from(first_bit_width) + i64::from(second_bit_width);
    let supported_one_byte_group = bit_width % 8 != 0
        && (bit_width + i64::from(first_bit_width)) % 8 != 0
        && total_bit_width == 8;
    let supported_two_byte_group = total_bit_width == 16;
    let supported_three_byte_group = (17..=23).contains(&bit_width) && total_bit_width == 24;
    let supported_four_byte_group = (25..=31).contains(&bit_width) && total_bit_width == 32;
    let supported_five_byte_group = bit_width == 33 && total_bit_width == 40;
    let supported_six_byte_group = bit_width == 41 && total_bit_width == 48;
    let supported_seven_byte_group = bit_width == 49 && total_bit_width == 56;
    let supported_eight_byte_group = bit_width == 57 && total_bit_width == 64;
    (supported_one_byte_group
        || supported_two_byte_group
        || supported_three_byte_group
        || supported_four_byte_group
        || supported_five_byte_group
        || supported_six_byte_group
        || supported_seven_byte_group
        || supported_eight_byte_group)
        && expected_value < (1_i64 << bit_width)
}

fn supported_suffix_reserved_group(
    first_visible_field: &veln_ast::SchemaField,
    second_visible_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 7 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&first_visible_field.ty)
        || exact_width_schema_primitive_little_endian(&second_visible_field.ty)
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_visible_field.ty)
    else {
        return false;
    };
    let Some(second_bit_width) = exact_width_schema_primitive_bit_width(&second_visible_field.ty)
    else {
        return false;
    };
    first_bit_width <= 8
        && second_bit_width == 8
        && i64::from(first_bit_width) + i64::from(second_bit_width) + bit_width == 16
        && expected_value < (1_i64 << bit_width)
}

fn supported_byte_visible_reserved_suffix(
    visible_field: &veln_ast::SchemaField,
    reserved: (i64, i64),
) -> bool {
    let (bit_width, expected_value) = reserved;
    if bit_width <= 8 || bit_width >= 56 || bit_width % 8 == 0 {
        return false;
    }
    if !canonical_schema_primitive_is(&visible_field.ty, "UInt8") {
        return false;
    }
    let storage_bit_width = ((8 + bit_width + 7) / 8) * 8;
    storage_bit_width > 16
        && storage_bit_width <= 64
        && reserved_bits_max_value(bit_width).is_some_and(|max_value| expected_value <= max_value)
}

fn supported_packed_reserved_prefix(
    reserved_field: &veln_ast::SchemaField,
    visible_field: &veln_ast::SchemaField,
) -> bool {
    let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&reserved_field.ty)
    else {
        return false;
    };
    packed_reserved_storage_bit_width(bit_width).is_some_and(|storage_bit_width| {
        exact_width_schema_primitive_bit_width(&visible_field.ty).is_some_and(|visible_bit_width| {
            i64::from(visible_bit_width) + bit_width == storage_bit_width
        }) && expected_value < (1_i64 << bit_width)
    })
}

fn supported_reserved_byte_prefix(
    bit_width: i64,
    expected_value: i64,
    visible_field: Option<&veln_ast::SchemaField>,
) -> bool {
    bit_width > 0
        && bit_width <= 56
        && bit_width % 8 != 0
        && reserved_bits_max_value(bit_width)
            .is_some_and(|max_value| (0..=max_value).contains(&expected_value))
        && visible_field.is_some_and(|field| canonical_schema_primitive_is(&field.ty, "UInt8"))
}

pub(crate) fn schema_payload_has_generalized_reserved_byte_prefix(schema: &SchemaDecl) -> bool {
    schema.fields.iter().enumerate().any(|(index, field)| {
        let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&field.ty) else {
            return false;
        };
        schema_field_uses_generalized_reserved_byte_prefix(
            &schema.fields,
            index,
            (bit_width, expected_value),
        )
    })
}

pub(crate) fn schema_field_uses_generalized_reserved_byte_prefix(
    fields: &[veln_ast::SchemaField],
    index: usize,
    reserved: (i64, i64),
) -> bool {
    let (bit_width, expected_value) = reserved;
    supported_reserved_byte_prefix(bit_width, expected_value, fields.get(index + 1))
        && !matches!((bit_width, expected_value), (1, 0) | (2, 0) | (9, 0))
}

fn supported_middle_reserved_bits(
    previous_field: &veln_ast::SchemaField,
    next_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 32 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&previous_field.ty)
        || exact_width_schema_primitive_little_endian(&next_field.ty)
    {
        return false;
    }
    let Some(previous_bit_width) = exact_width_schema_primitive_bit_width(&previous_field.ty)
    else {
        return false;
    };
    let Some(next_bit_width) = exact_width_schema_primitive_bit_width(&next_field.ty) else {
        return false;
    };
    let total_bit_width = i64::from(previous_bit_width) + bit_width + i64::from(next_bit_width);
    previous_bit_width % 8 != 0
        && (i64::from(previous_bit_width) + bit_width) % 8 != 0
        && matches!(total_bit_width, 8 | 16 | 24 | 32)
        && expected_value < (1_i64 << bit_width)
}

fn supported_byte_interleaved_reserved_group(
    fields: &[veln_ast::SchemaField],
    index: usize,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 7 {
        return false;
    }
    let Some(first_field) = index
        .checked_sub(1)
        .and_then(|previous| fields.get(previous))
    else {
        return false;
    };
    let (Some(byte_field), Some(last_field)) = (fields.get(index + 1), fields.get(index + 2))
    else {
        return false;
    };
    if [first_field, byte_field, last_field]
        .iter()
        .any(|field| exact_width_schema_primitive_little_endian(&field.ty))
    {
        return false;
    }
    let Some(first_bit_width) = exact_width_schema_primitive_bit_width(&first_field.ty) else {
        return false;
    };
    let Some(byte_bit_width) = exact_width_schema_primitive_bit_width(&byte_field.ty) else {
        return false;
    };
    let Some(last_bit_width) = exact_width_schema_primitive_bit_width(&last_field.ty) else {
        return false;
    };
    first_bit_width < 8
        && byte_bit_width == 8
        && last_bit_width < 8
        && i64::from(first_bit_width) + bit_width + 8 + i64::from(last_bit_width) == 16
        && (i64::from(first_bit_width) + bit_width) % 8 != 0
        && expected_value < (1_i64 << bit_width)
}

fn packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    }
}

fn suffix_packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    packed_reserved_storage_bit_width(bit_width).or_else(|| {
        if (33..=39).contains(&bit_width) {
            Some(40)
        } else if (41..=47).contains(&bit_width) {
            Some(48)
        } else if (49..=55).contains(&bit_width) {
            Some(56)
        } else if (57..=63).contains(&bit_width) {
            Some(64)
        } else {
            None
        }
    })
}

fn parse_reserved_bits_integer(text: &str) -> Option<i64> {
    parse_integer_literal(text)
        .ok()
        .map(|literal| literal.value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDispatchSpec {
    pub(crate) tag_field: String,
    pub(crate) length_field: Option<String>,
    pub(crate) preserves_unknown: bool,
    pub(crate) cases: Vec<SchemaDispatchCase>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDispatchCase {
    pub(crate) tag: i64,
    pub(crate) payload: SchemaDispatchCasePayload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaDispatchCasePayload {
    Primitive { width: u8, little_endian: bool },
    ReservedBits { bit_width: u8, expected_value: i64 },
    Schema { schema_name: String },
}

pub(crate) fn closed_dispatch_schema_primitive(ty: &str) -> Option<SchemaDispatchSpec> {
    let inner = schema_call_inner(ty, "Dispatch")?;
    let mut args = split_top_level_args(inner).into_iter().peekable();
    let tag_field = args.next()?.to_string();
    if !is_simple_schema_field_reference(&tag_field) {
        return None;
    }
    let length_field = args
        .peek()
        .filter(|arg| !arg.contains("=>"))
        .map(|arg| (*arg).to_string());
    if length_field
        .as_deref()
        .is_some_and(|length_field| !is_simple_schema_field_reference(length_field))
    {
        return None;
    }
    if length_field.is_some() {
        args.next();
    }
    let cases = args
        .map(|arg| {
            let (tag, primitive) = arg.split_once("=>")?;
            let tag = parse_schema_tag(tag.trim())?;
            let payload = schema_dispatch_case_payload(primitive.trim())?;
            Some(SchemaDispatchCase { tag, payload })
        })
        .collect::<Option<Vec<_>>>()?;
    if cases.is_empty() {
        return None;
    }
    Some(SchemaDispatchSpec {
        tag_field,
        length_field,
        preserves_unknown: false,
        cases,
    })
}

pub(crate) fn extension_dispatch_schema_primitive(ty: &str) -> Option<SchemaDispatchSpec> {
    let inner = schema_call_inner(ty, "ExtensionDispatch")?;
    let mut args = split_top_level_args(inner).into_iter();
    let tag_field = args.next()?.to_string();
    let length_field = args.next()?.to_string();
    if !is_simple_schema_field_reference(&tag_field)
        || !is_simple_schema_field_reference(&length_field)
    {
        return None;
    }
    let cases = args
        .map(|arg| {
            let (tag, primitive) = arg.split_once("=>")?;
            let tag = parse_schema_tag(tag.trim())?;
            let payload = schema_dispatch_case_payload(primitive.trim())?;
            Some(SchemaDispatchCase { tag, payload })
        })
        .collect::<Option<Vec<_>>>()?;
    if cases.is_empty() {
        return None;
    }
    Some(SchemaDispatchSpec {
        tag_field,
        length_field: Some(length_field),
        preserves_unknown: true,
        cases,
    })
}

fn schema_dispatch_case_payload(text: &str) -> Option<SchemaDispatchCasePayload> {
    if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(text) {
        let bit_width = dispatch_reserved_bits_width(bit_width, expected_value)?;
        return Some(SchemaDispatchCasePayload::ReservedBits {
            bit_width,
            expected_value,
        });
    }
    if let Some(width) = exact_width_schema_primitive(text) {
        if exact_width_schema_primitive_bit_width(text)? < 8 {
            return None;
        }
        return Some(SchemaDispatchCasePayload::Primitive {
            width,
            little_endian: exact_width_schema_primitive_little_endian(text),
        });
    }
    schema_payload_name_is_path(text).then(|| SchemaDispatchCasePayload::Schema {
        schema_name: text.to_string(),
    })
}

pub(crate) fn schema_dispatch_payload_accepts_lowercase_primitive(text: &str) -> bool {
    (lowercase_schema_primitive(text).is_some()
        || lowercase_reserved_bits_schema_primitive(text).is_some())
        && schema_dispatch_case_payload(text).is_some()
}

fn dispatch_reserved_bits_width(bit_width: i64, expected_value: i64) -> Option<u8> {
    if bit_width <= 0 || bit_width > 32 {
        return None;
    }
    if !(1..=7).contains(&bit_width) && bit_width % 8 != 0 {
        return None;
    }
    let max_value = if bit_width == 32 {
        0xffffffff
    } else {
        (1_i64 << bit_width) - 1
    };
    (expected_value <= max_value).then_some(bit_width as u8)
}

pub(crate) fn lowercase_schema_primitive_nested_payloads(ty: &str) -> Vec<(&str, &'static str)> {
    let mut payloads = Vec::new();
    if let Some(inner) = schema_call_inner(ty, "Repeat") {
        let args = inner
            .split(',')
            .map(str::trim)
            .filter(|arg| !arg.is_empty())
            .collect::<Vec<_>>();
        if let [_, payload] = args.as_slice()
            && (lowercase_schema_primitive(payload).is_some()
                || lowercase_reserved_bits_schema_primitive(payload).is_some())
        {
            payloads.push((*payload, "repeat_payload"));
        }
    }
    for call_name in ["Dispatch", "ExtensionDispatch"] {
        if let Some(inner) = schema_call_inner(ty, call_name) {
            for arg in split_top_level_args(inner) {
                let Some((_, payload)) = arg.split_once("=>") else {
                    continue;
                };
                let payload = payload.trim();
                if lowercase_schema_primitive(payload).is_some()
                    || lowercase_reserved_bits_schema_primitive(payload).is_some()
                {
                    payloads.push((payload, "dispatch_payload"));
                }
            }
        }
    }
    payloads
}

fn split_top_level_args(text: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = text[start..index].trim();
                if !arg.is_empty() {
                    args.push(arg);
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg);
    }
    args
}

fn schema_call_inner<'a>(ty: &'a str, name: &str) -> Option<&'a str> {
    let rest = ty.trim().strip_prefix(name)?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    rest.strip_prefix('(')?.strip_suffix(')')
}

fn parse_schema_tag(text: &str) -> Option<i64> {
    parse_integer_literal(text)
        .ok()
        .map(|literal| literal.value)
}

fn is_schema_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn snake_case_identifier(name: &str) -> String {
    let mut out = String::new();
    let mut previous_was_lower_or_digit = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if previous_was_lower_or_digit && !out.ends_with('_') {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
                previous_was_lower_or_digit = false;
            } else {
                out.push(ch);
                previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            }
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
            previous_was_lower_or_digit = false;
        }
    }
    out.trim_matches('_').to_string()
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

trait SymbolVisibility {
    fn visibility(&self) -> Visibility;
}

impl SymbolVisibility for FunctionSignature {
    fn visibility(&self) -> Visibility {
        self.visibility
    }
}

impl SymbolVisibility for NamedSymbol {
    fn visibility(&self) -> Visibility {
        self.visibility
    }
}

impl SchemaSymbolTable {
    fn from_module(module: &SurfaceModule) -> Self {
        let schemas = module
            .schemas
            .iter()
            .filter_map(|schema| {
                Some(SchemaSymbol {
                    name: schema.name.clone()?,
                    module_name: schema.module_name.clone(),
                    visibility: schema.visibility,
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field:
                        format_neutral_schema_first_unsupported_encode_field(module, schema),
                })
            })
            .collect();
        let aliases = module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Schema)
            .filter_map(|alias| {
                Some(SchemaAliasSymbol {
                    name: alias.name.clone()?,
                    module_name: alias.module_name.clone(),
                    target: alias.target.clone(),
                })
            })
            .collect();
        Self { schemas, aliases }
    }

    fn private_schema(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
    ) -> bool {
        self.schema_path(
            segments,
            current_module,
            uses,
            true,
            companion_access_targets,
            &mut Vec::new(),
        ) == SchemaPathLookup::Private
    }

    fn schema_alias_target(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<SchemaAliasTarget> {
        match segments {
            [name] => self.schema_alias_target_in_module(current_module, name),
            [_, .., name] => {
                let use_decl =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
                self.schema_alias_target_in_module(Some(&use_decl.name), name)
            }
            _ => None,
        }
    }

    fn schema_alias_target_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
    ) -> Option<SchemaAliasTarget> {
        let alias = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)?;
        Some(SchemaAliasTarget {
            target: alias.target.clone(),
            module_name: alias.module_name.clone(),
        })
    }

    fn schema_target_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        match segments {
            [name] => self.schema_target_in_module(
                current_module,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let use_decl =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
                self.schema_target_in_module(
                    Some(&use_decl.name),
                    name,
                    companion_private_schema_access_allowed(
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ),
                    uses,
                    companion_access_targets,
                    visited_aliases,
                )
            }
            _ => None,
        }
    }

    fn schema_target_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        if let Some(schema) = self
            .schemas
            .iter()
            .find(|schema| schema.name == name && schema.module_name.as_deref() == module_name)
        {
            return (allow_private_schema || schema.visibility == Visibility::Public).then(|| {
                ResolvedSchemaSymbol {
                    name: schema.name.clone(),
                    module_name: schema.module_name.clone(),
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field: schema
                        .unsupported_format_neutral_encode_field
                        .clone(),
                }
            });
        }
        let alias = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)?;
        let key = (alias.module_name.clone(), alias.name.clone());
        if visited_aliases.contains(&key) {
            return None;
        }
        visited_aliases.push(key);
        let result = self.schema_target_path(
            &alias.target,
            alias.module_name.as_deref(),
            uses,
            false,
            companion_access_targets,
            visited_aliases,
        );
        visited_aliases.pop();
        result
    }

    fn schema_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> SchemaPathLookup {
        match segments {
            [name] => self.schema_in_module(
                current_module,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let Some(use_decl) =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                else {
                    return SchemaPathLookup::Missing;
                };
                self.schema_in_module(
                    Some(&use_decl.name),
                    name,
                    companion_private_schema_access_allowed(
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ),
                    uses,
                    companion_access_targets,
                    visited_aliases,
                )
            }
            _ => SchemaPathLookup::Missing,
        }
    }

    fn schema_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> SchemaPathLookup {
        if let Some(schema) = self
            .schemas
            .iter()
            .find(|schema| schema.name == name && schema.module_name.as_deref() == module_name)
        {
            return if allow_private_schema || schema.visibility == Visibility::Public {
                SchemaPathLookup::Visible
            } else {
                SchemaPathLookup::Private
            };
        }
        let Some(alias) = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)
        else {
            return SchemaPathLookup::Missing;
        };
        let key = (alias.module_name.clone(), alias.name.clone());
        if visited_aliases.contains(&key) {
            return SchemaPathLookup::Missing;
        }
        visited_aliases.push(key);
        let result = self.schema_path(
            &alias.target,
            alias.module_name.as_deref(),
            uses,
            false,
            companion_access_targets,
            visited_aliases,
        );
        visited_aliases.pop();
        result
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SchemaPathLookup {
    Visible,
    Private,
    Missing,
}

fn named_type_symbols(module: &SurfaceModule) -> Vec<NamedSymbol> {
    let mut symbols = module
        .types
        .iter()
        .filter_map(|ty| {
            Some(NamedSymbol {
                name: ty.name.clone()?,
                module_name: ty.module_name.clone(),
                visibility: ty.visibility,
            })
        })
        .collect::<Vec<_>>();
    symbols.extend(
        module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Type)
            .filter_map(|alias| {
                Some(NamedSymbol {
                    name: alias.name.clone()?,
                    module_name: alias.module_name.clone(),
                    visibility: Visibility::Public,
                })
            }),
    );
    symbols
}

fn named_codec_symbols(module: &SurfaceModule) -> Vec<NamedSymbol> {
    module
        .codecs
        .iter()
        .filter_map(|codec| {
            Some(NamedSymbol {
                name: codec.name.clone()?,
                module_name: codec.module_name.clone(),
                visibility: codec.visibility,
            })
        })
        .collect()
}

pub(crate) fn function_alias_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<FunctionSignature> {
    let companion_access_targets = BTreeMap::new();
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let target = function_signature_path(
                &alias.target,
                &module.uses,
                functions,
                alias.module_name.as_deref(),
                &companion_access_targets,
            )?;
            Some(FunctionSignature {
                name,
                target_name: target.target_name.clone(),
                module_name: alias.module_name.clone(),
                visibility: Visibility::Public,
                params: target.params.clone(),
                variadic: target.variadic.clone(),
                return_type: target.return_type.clone(),
                effects: target.effects.clone(),
                node_id: alias.node_id,
                span: alias.span.clone(),
            })
        })
        .collect()
}

fn function_signature_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    functions: &'a [FunctionSignature],
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> Option<&'a FunctionSignature> {
    match segments {
        [name] => functions.iter().find(|function| function.name == *name),
        [_, .., name] => {
            let use_decl =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
            let module_name = use_decl.name.as_str();
            functions.iter().find(|function| {
                function.name == *name
                    && function.module_name.as_deref() == Some(module_name)
                    && imported_function_is_visible(
                        function,
                        use_decl,
                        current_module,
                        companion_access_targets,
                    )
            })
        }
        _ => None,
    }
}

fn collect_pattern_bindings(pattern: &Pattern, ty: &Type, bindings: &mut Vec<Binding>) {
    collect_let_pattern_bindings(pattern, ty, None, bindings);
}

fn collect_let_pattern_bindings(
    pattern: &Pattern,
    ty: &Type,
    private_function_value: Option<FunctionKey>,
    bindings: &mut Vec<Binding>,
) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(match private_function_value {
            Some(target) => Binding::private_function_value(name.clone(), ty.clone(), target),
            None => Binding::new(name.clone(), ty.clone()),
        }),
        PatternKind::Record(fields) => {
            for field in fields {
                let field_ty = ty.record_field(&field.name).unwrap_or(&Type::Unknown);
                collect_let_pattern_bindings(&field.pattern, field_ty, None, bindings);
            }
        }
        PatternKind::Wildcard
        | PatternKind::StringLiteral(_)
        | PatternKind::IntLiteral(_)
        | PatternKind::FloatLiteral(_)
        | PatternKind::BoolLiteral(_)
        | PatternKind::Unit
        | PatternKind::Constructor { .. } => {}
    }
}

struct ExprEffectContext<'a> {
    uses: &'a [UseDecl],
    current_module: Option<&'a str>,
    bindings: &'a [Binding],
    functions: &'a [FunctionSignature],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &'a BTreeMap<String, String>,
    companion_effect_access_targets: &'a BTreeMap<String, CompanionAccessTarget>,
    user_effects: &'a [EffectSignature],
    handlers: &'a [HandlerSignature],
}

fn handler_for_path<'a>(
    segments: &[String],
    context: &ExprEffectContext<'a>,
) -> Option<&'a HandlerSignature> {
    match segments {
        [name] => context.handlers.iter().find(|handler| {
            handler.name == *name && handler.module_name.as_deref() == context.current_module
        }),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                context.uses,
                &segments[..segments.len() - 1],
                context.current_module,
            )?;
            context.handlers.iter().find(|handler| {
                handler.name == *name
                    && handler.module_name.as_deref() == Some(use_decl.name.as_str())
                    && imported_handler_is_visible(
                        handler,
                        use_decl,
                        context.current_module,
                        context.companion_effect_access_targets,
                    )
            })
        }
        _ => None,
    }
}

fn collect_expr_effect_dependencies(
    expr: &Expr,
    context: &ExprEffectContext<'_>,
    dependencies: &mut BTreeSet<EffectDependencyNode>,
) {
    ExprEffectDependencyCollector {
        context,
        dependencies,
    }
    .collect(expr);
}

struct ExprEffectDependencyCollector<'context, 'data, 'output> {
    context: &'context ExprEffectContext<'data>,
    dependencies: &'output mut BTreeSet<EffectDependencyNode>,
}

impl ExprEffectDependencyCollector<'_, '_, '_> {
    fn collect(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { callee, args } => self.collect_call(callee, args),
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => self.collect_handle(body, handler, args),
            ExprKind::SchemaDecode { input, base, .. } => self.collect_pair(input, base),
            ExprKind::Perform { args, .. } => self.collect_all(args),
            ExprKind::SchemaEncode { value, .. } => self.collect(value),
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::TypeApply { callee: base, .. }
            | ExprKind::Prefix { expr: base, .. } => self.collect(base),
            ExprKind::Record(fields) => {
                for field in fields {
                    self.collect(&field.expr);
                }
            }
            ExprKind::Dict(entries) => {
                for entry in entries {
                    self.collect_pair(&entry.key, &entry.value);
                }
            }
            ExprKind::List(items) => self.collect_all(items),
            ExprKind::Match { scrutinee, arms } => {
                self.collect(scrutinee);
                for arm in arms {
                    self.collect(&arm.expr);
                }
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => {
                self.collect_pair(condition, then_branch);
                for branch in else_if_branches {
                    self.collect_pair(&branch.condition, &branch.expr);
                }
                self.collect(else_branch);
            }
            ExprKind::Binary { left, right, .. } => self.collect_pair(left, right),
            ExprKind::NamePath(segments) => self.collect_name_path(segments),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_call(&mut self, callee: &Expr, args: &[Expr]) {
        if let Some(segments) = callee_name_path(callee) {
            self.collect_name_path(segments);
        } else {
            self.collect(callee);
        }
        self.collect_all(args);
    }

    fn collect_handle(&mut self, body: &Expr, handler: &[String], args: &[Expr]) {
        self.collect_all(args);
        if let Some(handler) = handler_for_path(handler, self.context)
            && handler.visibility != Visibility::Public
        {
            self.dependencies
                .insert(EffectDependencyNode::PrivateHandler(
                    handler.qualified_name.clone(),
                ));
        }
        self.collect(body);
    }

    fn collect_name_path(&mut self, segments: &[String]) {
        if let Some(signature) = function_signature_path(
            segments,
            self.context.uses,
            self.context.functions,
            self.context.current_module,
            self.context.companion_access_targets,
        ) {
            self.dependencies.insert(EffectDependencyNode::Function((
                signature.module_name.clone(),
                signature.name.clone(),
            )));
        }
        if let [name] = segments
            && let Some(target) = self
                .context
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == *name)
                .and_then(|binding| binding.private_function_value.clone())
        {
            self.dependencies
                .insert(EffectDependencyNode::Function(target));
        }
    }

    fn collect_pair(&mut self, first: &Expr, second: &Expr) {
        self.collect(first);
        self.collect(second);
    }

    fn collect_all(&mut self, expressions: &[Expr]) {
        for expression in expressions {
            self.collect(expression);
        }
    }
}

fn collect_expr_effects(expr: &Expr, context: &ExprEffectContext<'_>, inferred: &mut Vec<String>) {
    ExprEffectCollector { context, inferred }.collect(expr);
}

struct ExprEffectCollector<'context, 'data, 'output> {
    context: &'context ExprEffectContext<'data>,
    inferred: &'output mut Vec<String>,
}

impl ExprEffectCollector<'_, '_, '_> {
    fn collect(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Call { callee, args } => self.collect_call(callee, args),
            ExprKind::SchemaDecode { input, base, .. } => self.collect_pair(input, base),
            ExprKind::Perform { effect, args, .. } => self.collect_perform(effect, args),
            ExprKind::Handle {
                body,
                handler,
                args,
                ..
            } => self.collect_handle(body, handler, args),
            ExprKind::SchemaEncode { value, .. } => self.collect(value),
            ExprKind::FieldAccess { base, .. }
            | ExprKind::Try(base)
            | ExprKind::TypeApply { callee: base, .. }
            | ExprKind::Prefix { expr: base, .. } => self.collect(base),
            ExprKind::Record(fields) => self.collect_record_fields(fields),
            ExprKind::Dict(entries) => self.collect_dict_entries(entries),
            ExprKind::List(items) => self.collect_all(items),
            ExprKind::Match { scrutinee, arms } => self.collect_match(scrutinee, arms),
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.collect_if(condition, then_branch, else_if_branches, else_branch),
            ExprKind::Binary { left, right, .. } => self.collect_pair(left, right),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath(_)
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit => {}
        }
    }

    fn collect_call(&mut self, callee: &Expr, args: &[Expr]) {
        let Some(segments) = callee_name_path(callee) else {
            self.collect(callee);
            self.collect_all(args);
            return;
        };
        if is_stdio_call(segments) {
            push_unique_effect(self.inferred, "stdio");
        } else if let Some(effects) = concurrency_effects_for_call(segments, args, self.context) {
            self.push_all(&effects);
        } else if let Some(effects) = standard_library_effects(segments) {
            for effect in effects {
                push_unique_effect(self.inferred, effect);
            }
        } else if let Some(effects) = prelude_effects(segments) {
            for effect in effects {
                push_unique_effect(self.inferred, effect);
            }
        } else if let Some(signature) = function_signature_path(
            segments,
            self.context.uses,
            self.context.functions,
            self.context.current_module,
            self.context.companion_access_targets,
        ) {
            self.push_all(&instantiate_call_effect_rows(signature, args, self.context));
        } else {
            for effect in effects_for_callee_path(
                segments,
                self.context.uses,
                self.context.current_module,
                self.context.bindings,
                self.context.effects_by_function,
                self.context.effects_by_module_path,
                self.context.companion_access_targets,
            ) {
                push_unique_effect(self.inferred, effect);
            }
        }
        self.collect_all(args);
    }

    fn collect_perform(&mut self, effect: &[String], args: &[Expr]) {
        if let Some(label) = canonical_user_effect_label(
            effect,
            self.context.uses,
            self.context.current_module,
            self.context.user_effects,
            self.context.companion_effect_access_targets,
        ) {
            push_unique_effect(self.inferred, &label);
        }
        self.collect_all(args);
    }

    fn collect_handle(&mut self, body: &Expr, handler: &[String], args: &[Expr]) {
        self.collect_all(args);
        let Some((handled_effect, handler_effects)) = handler_for_path(handler, self.context)
            .map(|handler| (handler.effect.clone(), handler.effects.clone()))
        else {
            self.collect(body);
            return;
        };
        let before_body = self.inferred.len();
        self.collect(body);
        let retained_body_effects = self
            .inferred
            .drain(before_body..)
            .filter(|effect| effect != &handled_effect)
            .collect::<Vec<_>>();
        self.inferred.extend(retained_body_effects);
        self.push_all(&handler_effects);
    }

    fn collect_pair(&mut self, first: &Expr, second: &Expr) {
        self.collect(first);
        self.collect(second);
    }

    fn collect_all(&mut self, expressions: &[Expr]) {
        for expression in expressions {
            self.collect(expression);
        }
    }

    fn collect_record_fields(&mut self, fields: &[RecordField]) {
        for field in fields {
            self.collect(&field.expr);
        }
    }

    fn collect_dict_entries(&mut self, entries: &[DictEntry]) {
        for entry in entries {
            self.collect_pair(&entry.key, &entry.value);
        }
    }

    fn collect_match(&mut self, scrutinee: &Expr, arms: &[MatchArm]) {
        self.collect(scrutinee);
        for arm in arms {
            self.collect(&arm.expr);
        }
    }

    fn collect_if(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_if_branches: &[IfBranch],
        else_branch: &Expr,
    ) {
        self.collect_pair(condition, then_branch);
        for branch in else_if_branches {
            self.collect_pair(&branch.condition, &branch.expr);
        }
        self.collect(else_branch);
    }

    fn push_all(&mut self, effects: &[String]) {
        for effect in effects {
            push_unique_effect(self.inferred, effect);
        }
    }
}

fn instantiate_call_effect_rows(
    signature: &FunctionSignature,
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Vec<String> {
    let mut row_substitutions = Vec::<(String, Vec<String>)>::new();
    for (param, arg) in signature.params.iter().zip(args) {
        let Some(actual) = function_type_for_expr(arg, context) else {
            continue;
        };
        collect_effect_row_substitution_from_types(param, &actual, &mut row_substitutions);
    }
    instantiate_effect_row_entries(&signature.effects, &row_substitutions)
}

fn function_type_for_expr(expr: &Expr, context: &ExprEffectContext<'_>) -> Option<Type> {
    let segments = callee_name_path(expr)?;
    match segments.as_slice() {
        [name] => context
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                function_signature_path(
                    segments,
                    context.uses,
                    context.functions,
                    context.current_module,
                    context.companion_access_targets,
                )
                .map(FunctionSignature::ty)
            }),
        _ => {
            let public_or_same_module_access = BTreeMap::new();
            function_signature_path(
                segments,
                context.uses,
                context.functions,
                context.current_module,
                &public_or_same_module_access,
            )
            .map(FunctionSignature::ty)
        }
    }
}

fn collect_effect_row_substitution_from_types(
    expected: &Type,
    actual: &Type,
    row_substitutions: &mut Vec<(String, Vec<String>)>,
) {
    let (
        Type::Function {
            params: expected_params,
            variadic: expected_variadic,
            return_type: expected_return,
            effects: expected_effects,
        },
        Type::Function {
            params: actual_params,
            variadic: actual_variadic,
            return_type: actual_return,
            effects: actual_effects,
        },
    ) = (expected, actual)
    else {
        return;
    };

    for effect in expected_effects {
        let Some(row) = effect.strip_prefix("...") else {
            continue;
        };
        let concrete = actual_effects
            .iter()
            .filter(|actual_effect| {
                !expected_effects
                    .iter()
                    .any(|expected_effect| expected_effect == *actual_effect)
            })
            .cloned()
            .collect::<Vec<_>>();
        merge_effect_row_substitution(row_substitutions, row, concrete);
    }

    for (expected_param, actual_param) in expected_params.iter().zip(actual_params) {
        collect_effect_row_substitution_from_types(expected_param, actual_param, row_substitutions);
    }
    if let (Some(expected), Some(actual)) =
        (expected_variadic.as_deref(), actual_variadic.as_deref())
    {
        collect_effect_row_substitution_from_types(expected, actual, row_substitutions);
    }
    collect_effect_row_substitution_from_types(expected_return, actual_return, row_substitutions);
}

fn merge_effect_row_substitution(
    row_substitutions: &mut Vec<(String, Vec<String>)>,
    row: &str,
    effects: Vec<String>,
) {
    if let Some((_, existing)) = row_substitutions
        .iter_mut()
        .find(|(existing_row, _)| existing_row == row)
    {
        for effect in effects {
            push_unique_effect(existing, &effect);
        }
        return;
    }
    let mut unique = Vec::new();
    for effect in effects {
        push_unique_effect(&mut unique, &effect);
    }
    row_substitutions.push((row.to_string(), unique));
}

fn instantiate_effect_row_entries(
    effects: &[String],
    row_substitutions: &[(String, Vec<String>)],
) -> Vec<String> {
    let mut instantiated = Vec::new();
    for effect in effects {
        if let Some(row) = effect.strip_prefix("...") {
            if let Some((_, substitution)) = row_substitutions
                .iter()
                .find(|(candidate, _)| candidate == row)
            {
                for substituted in substitution {
                    push_unique_effect(&mut instantiated, substituted);
                }
            } else {
                push_unique_effect(&mut instantiated, effect);
            }
        } else {
            push_unique_effect(&mut instantiated, effect);
        }
    }
    instantiated
}

fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
}

fn concurrency_effects_for_call(
    segments: &[String],
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Vec<String>> {
    let mut effects = concurrency_effects(segments)?
        .iter()
        .map(|effect| (*effect).to_string())
        .collect::<Vec<_>>();
    if matches!(segments, [module, name] if module == "task" && matches!(name.as_str(), "spawn" | "spawn_with"))
        && let Some(job_effects) = args.first().and_then(callee_name_path).map(|segments| {
            effects_for_callee_path(
                segments,
                context.uses,
                context.current_module,
                context.bindings,
                context.effects_by_function,
                context.effects_by_module_path,
                context.companion_access_targets,
            )
        })
    {
        for effect in job_effects {
            push_unique_effect(&mut effects, effect);
        }
    }
    Some(effects)
}

fn effects_for_callee_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
    companion_access_targets: &'a BTreeMap<String, String>,
) -> &'a [String] {
    match segments {
        [name] => effects_for_bare_callee(name, current_module, bindings, effects_by_function),
        [_, .., name] => {
            let Some(use_decl) =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            else {
                return &[];
            };
            effects_by_module_path
                .get(&(use_decl.name.clone(), name.clone()))
                .filter(|(_, visibility)| {
                    imported_effects_are_visible(
                        use_decl,
                        current_module,
                        use_decl.name.as_str(),
                        *visibility,
                        companion_access_targets,
                    )
                })
                .map_or(&[], |(effects, _)| effects.as_slice())
        }
        _ => &[],
    }
}

pub(crate) fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn imported_function_is_visible(
    function: &FunctionSignature,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    if function.visibility == Visibility::Public {
        return true;
    }
    if use_decl.package.is_none()
        && current_module.is_some_and(|module| module.starts_with("std::"))
        && function
            .module_name
            .as_deref()
            .is_some_and(|module| module.starts_with("std::"))
    {
        return true;
    }
    use_decl.package.is_none()
        && current_module.is_some_and(|current_module| {
            function.module_name.as_ref().is_some_and(|target_module| {
                companion_access_targets
                    .get(current_module)
                    .is_some_and(|allowed| allowed == target_module)
            })
        })
}

fn imported_effects_are_visible(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    target_module: &str,
    visibility: Visibility,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                (current_module.starts_with("std::") && target_module.starts_with("std::"))
                    || companion_access_targets
                        .get(current_module)
                        .is_some_and(|allowed| allowed == target_module)
            }))
}

fn imported_handler_is_visible(
    handler: &HandlerSignature,
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, CompanionAccessTarget>,
) -> bool {
    handler.visibility == Visibility::Public
        || (use_decl.package.is_none()
            && current_module.is_some_and(|current_module| {
                handler.module_name.as_deref().is_some_and(|target_module| {
                    (current_module.starts_with("std::") && target_module.starts_with("std::"))
                        || companion_access_targets
                            .get(current_module)
                            .is_some_and(|access| access.target_module == target_module)
                })
            }))
}

fn companion_private_schema_access_allowed(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    use_decl.package.is_none()
        && current_module.is_some_and(|current_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed| allowed == use_decl.name.as_str())
        })
}

fn companion_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target(function.span.file.as_str(), function.module_name.as_deref())
        })
        .chain(module.schemas.iter().filter_map(|schema| {
            companion_access_target(schema.span.file.as_str(), schema.module_name.as_deref())
        }))
        .collect()
}

fn companion_access_target(path: &str, module_name: Option<&str>) -> Option<(String, String)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((companion_module, target_module))
}

fn companion_access_target_infos(
    module: &SurfaceModule,
) -> BTreeMap<String, CompanionAccessTarget> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target_info(
                function.span.file.as_str(),
                function.module_name.as_deref(),
            )
        })
        .chain(module.handlers.iter().filter_map(|handler| {
            companion_access_target_info(handler.span.file.as_str(), handler.module_name.as_deref())
        }))
        .chain(module.effects.iter().filter_map(|effect| {
            companion_access_target_info(effect.span.file.as_str(), effect.module_name.as_deref())
        }))
        .collect()
}

fn companion_access_target_info(
    path: &str,
    module_name: Option<&str>,
) -> Option<(String, CompanionAccessTarget)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((
        companion_module,
        CompanionAccessTarget {
            companion_path: companion.companion_path,
            target_module,
        },
    ))
}

fn companion_function_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            let companion = classify_companion_source(function.span.file.as_str())?;
            let companion_module = function.module_name.clone()?;
            let target_module = companion
                .target_path
                .strip_suffix(".veln")?
                .replace('/', "::");
            Some((companion_module, target_module))
        })
        .collect()
}

fn companion_access_targets_for_signatures(
    functions: &[FunctionSignature],
) -> BTreeMap<String, String> {
    functions
        .iter()
        .filter_map(|function| {
            let companion = classify_companion_source(function.span.file.as_str())?;
            let companion_module = function.module_name.clone()?;
            let target_module = companion
                .target_path
                .strip_suffix(".veln")?
                .replace('/', "::");
            Some((companion_module, target_module))
        })
        .collect()
}

fn effects_for_bare_callee<'a>(
    name: &str,
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_function: &'a BTreeMap<(Option<String>, String), Vec<String>>,
) -> &'a [String] {
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == name)
        && let Some(effects) = binding.ty.function_effects()
    {
        return effects;
    }
    if let Some(current_module) = current_module {
        return effects_by_function
            .get(&(Some(current_module.to_string()), name.to_string()))
            .map_or(&[], Vec::as_slice);
    }
    effects_by_function
        .get(&(None, name.to_string()))
        .map_or(&[], Vec::as_slice)
}

fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}

#[cfg(test)]
#[path = "types/tests.rs"]
mod tests;
