use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{
    BinaryOp, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationKind, Expr, ExprKind,
    FunctionKind, NodeId, Pattern, PatternKind, PublicAliasKind, SchemaDecl, SchemaField,
    SchemaMappingClause, SurfaceModule, TypeDecl, TypeVariantDecl, UseDecl, Visibility,
};
use veln_core::CoreType;
use veln_source::SourceSpan;

use crate::adt::{self, AdtConstructor, AdtRegistry, ConstructorLookup};
use crate::effects::{is_concurrency_call, is_stdio_call, standard_library_effects};

pub(crate) struct TypeEnvironment {
    functions: Vec<FunctionSignature>,
    codec_calls: Vec<CodecCallSignature>,
    pub(crate) uses: Vec<UseDecl>,
    pub(crate) adts: AdtRegistry,
}

#[derive(Clone)]
pub(crate) struct FunctionSignature {
    pub(crate) name: String,
    pub(crate) target_name: String,
    pub(crate) module_name: Option<String>,
    pub(crate) visibility: Visibility,
    pub(crate) params: Vec<Type>,
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
pub(crate) const SCHEMA_ENCODE_TARGET_PREFIX: &str = "schema-encode:";
pub(crate) const SCHEMA_ENCODE_STEP_TARGET_PREFIX: &str = "schema-encode-step:";
pub(crate) const SCHEMA_VALIDATE_TARGET_PREFIX: &str = "schema-validate:";

pub(crate) enum FunctionLookup<'a> {
    Found(&'a FunctionSignature),
    Ambiguous,
    Missing,
}

impl<'a> FunctionLookup<'a> {
    pub(crate) fn found(self) -> Option<&'a FunctionSignature> {
        match self {
            Self::Found(function) => Some(function),
            Self::Ambiguous | Self::Missing => None,
        }
    }
}

pub(crate) struct CallOrigin {
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) symbol: String,
    pub(crate) effects: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct EffectUse {
    pub(crate) effect: String,
    pub(crate) node_id: NodeId,
    pub(crate) span: SourceSpan,
    pub(crate) kind: &'static str,
    pub(crate) symbol: String,
}

#[derive(Clone)]
pub(crate) struct Binding {
    pub(crate) name: String,
    pub(crate) ty: Type,
}

#[derive(Clone)]
pub(crate) struct ExpectedType {
    pub(crate) ty: Type,
    pub(crate) source: ExpectedTypeSource,
    pub(crate) origin_node_id: NodeId,
    pub(crate) origin_span: Option<SourceSpan>,
    pub(crate) origin_message: &'static str,
}

#[derive(Clone, Copy)]
pub(crate) enum ExpectedTypeSource {
    DeclaredReturn,
    DeclaredParameter,
    LocalAnnotation,
    Inferred,
    Unknown,
}

impl ExpectedTypeSource {
    pub(crate) fn as_type_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn => "declared_return",
            Self::DeclaredParameter => "declared_parameter",
            Self::LocalAnnotation => "local_annotation",
            Self::Inferred => "inferred_expression",
            Self::Unknown => "unknown",
        }
    }

    pub(crate) fn as_hole_source(self) -> &'static str {
        match self {
            Self::DeclaredReturn | Self::DeclaredParameter | Self::LocalAnnotation => "declared",
            Self::Inferred => "inferred",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    Unknown,
    Named {
        name: String,
        args: Vec<Type>,
    },
    Record(Vec<(String, Type)>),
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
        effects: Vec<String>,
    },
}

impl Type {
    pub(crate) fn named(name: impl Into<String>, args: Vec<Type>) -> Self {
        Self::Named {
            name: name.into(),
            args,
        }
    }

    pub(crate) fn bool() -> Self {
        Self::named("Bool", Vec::new())
    }

    pub(crate) fn int() -> Self {
        Self::named("Int", Vec::new())
    }

    pub(crate) fn float() -> Self {
        Self::named("Float", Vec::new())
    }

    pub(crate) fn string() -> Self {
        Self::named("String", Vec::new())
    }

    pub(crate) fn unit() -> Self {
        Self::named("Unit", Vec::new())
    }

    #[cfg(test)]
    pub(crate) fn result(value: Type, error: Type) -> Self {
        Self::named("Result", vec![value, error])
    }

    pub(crate) fn vec(item: Type) -> Self {
        Self::named("Vec", vec![item])
    }

    pub(crate) fn dict(key: Type, value: Type) -> Self {
        Self::named("Dict", vec![key, value])
    }

    pub(crate) fn function(params: Vec<Type>, return_type: Type, effects: Vec<String>) -> Self {
        Self::Function {
            params,
            return_type: Box::new(return_type),
            effects,
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Unknown => "unknown".to_string(),
            Self::Named { name, args } if name == "Unit" && args.is_empty() => "()".to_string(),
            Self::Named { name, args } if args.is_empty() => name.clone(),
            Self::Named { name, args } => {
                let args = args.iter().map(Type::render).collect::<Vec<_>>().join(", ");
                format!("{name}<{args}>")
            }
            Self::Record(fields) => {
                let fields = fields
                    .iter()
                    .map(|(name, ty)| format!("{name}: {}", ty.render()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{fields}}}")
            }
            Self::Function {
                params,
                return_type,
                effects,
            } => {
                let params = params
                    .iter()
                    .map(Type::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                let effects = if effects.is_empty() {
                    String::new()
                } else {
                    format!(" effects [{}]", effects.join(", "))
                };
                format!("fn({params}) -> {}{effects}", return_type.render())
            }
        }
    }

    pub(crate) fn vec_part(&self) -> Option<&Type> {
        match self {
            Self::Named { name, args } if name == "Vec" && args.len() == 1 => Some(&args[0]),
            _ => None,
        }
    }

    pub(crate) fn dict_parts(&self) -> Option<(&Type, &Type)> {
        match self {
            Self::Named { name, args } if name == "Dict" && args.len() == 2 => {
                Some((&args[0], &args[1]))
            }
            _ => None,
        }
    }

    pub(crate) fn record_field(&self, field_name: &str) -> Option<&Type> {
        match self {
            Self::Record(fields) => fields
                .iter()
                .find_map(|(name, ty)| (name == field_name).then_some(ty)),
            _ => None,
        }
    }

    pub(crate) fn function_parts(&self) -> Option<(&[Type], &Type)> {
        match self {
            Self::Function {
                params,
                return_type,
                ..
            } => Some((params, return_type)),
            _ => None,
        }
    }

    pub(crate) fn function_effects(&self) -> Option<&[String]> {
        match self {
            Self::Function { effects, .. } => Some(effects),
            _ => None,
        }
    }
}

impl TypeEnvironment {
    pub(crate) fn from_module(module: &SurfaceModule) -> Self {
        let mut functions = ordinary_function_signatures(module);
        functions.extend(schema_decode_function_signatures(module));
        functions.extend(schema_encode_function_signatures(module));
        functions.extend(schema_validate_function_signatures(module));
        infer_function_body_effects(module, &mut functions);
        let codec_calls = codec_call_signatures(module, &functions);
        let aliases = function_alias_signatures(module, &functions);
        functions.extend(aliases);
        Self {
            functions,
            codec_calls,
            uses: module.uses.clone(),
            adts: AdtRegistry::from_module(module),
        }
    }

    pub(crate) fn function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.iter().find(|function| function.name == name)
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
                        && imported_function_is_visible(function, use_decl)
                })
            }
            _ => None,
        }
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

fn ordinary_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .functions
        .iter()
        .filter(|function| function.kind == FunctionKind::Function)
        .filter_map(|function| {
            let name = function.name.clone()?;
            let params = function
                .params
                .iter()
                .map(|param| parse_type_or_unknown(param.ty.as_deref()))
                .collect();
            let return_type = parse_type_or_unknown(function.return_type.as_deref());
            Some(FunctionSignature {
                target_name: name.clone(),
                name,
                module_name: function.module_name.clone(),
                visibility: function.visibility,
                params,
                return_type,
                effects: function.effects.clone().unwrap_or_default(),
                node_id: function.node_id,
                span: function.span.clone(),
            })
        })
        .collect()
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
            schema_in_module(module, Some(&use_decl.name), name, false, visited_aliases)
        }
        _ => None,
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
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return Vec::new();
    }
    let Some(fields) = schema_decode_record_fields(module, schema) else {
        return Vec::new();
    };
    let byte_view = Type::named("ByteView", Vec::new());
    let byte_offset = Type::named("ByteOffset", Vec::new());
    let mapped_fields = schema_decode_mapping_record_fields(module, schema, &fields)
        .unwrap_or_else(|| fields.into_iter().map(|(name, ty, _)| (name, ty)).collect());
    let decoded_type = Type::Record(mapped_fields);
    let result = Type::named("Result", vec![decoded_type.clone(), Type::string()]);
    let step = Type::named("DecodeStep", vec![decoded_type]);
    vec![
        FunctionSignature {
            name: schema_decode_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view.clone()],
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
            return_type: step,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
    ]
}

fn schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type, u8)>> {
    schema_decode_record_fields_inner(module, schema, &mut Vec::new())
}

pub(crate) fn schema_decode_record_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    if schema.format.as_ref()?.name != "binary" {
        return None;
    }
    Some(Type::Record(
        schema_decode_record_fields(module, schema)?
            .into_iter()
            .map(|(name, ty, _)| (name, ty))
            .collect(),
    ))
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
    let mut decoded_fields = BTreeMap::<String, Type>::new();
    let mut fields = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            supported_encode_reserved_bits(&schema.fields, index, reserved)?;
            continue;
        }
        let (width, ty) = if let Some(width) = exact_width_schema_primitive(&field.ty) {
            let ty = if let Some(flag_type) = flag_schema_primitive(&field.ty) {
                Type::named(flag_type, Vec::new())
            } else {
                Type::int()
            };
            (width, ty)
        } else if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if length_expr
                .references()
                .into_iter()
                .any(|reference| decoded_fields.get(reference) != Some(&Type::int()))
            {
                return None;
            }
            (0, Type::named("ByteView", Vec::new()))
        } else if let Some(repeat) = repeat_schema_primitive(&field.ty) {
            if schema_length_expression_references(&repeat.count_field)?
                .into_iter()
                .any(|reference| decoded_fields.get(reference) != Some(&Type::int()))
            {
                return None;
            }
            if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload
                && decoded_fields.get(length_field) != Some(&Type::int())
            {
                return None;
            }
            let element_ty = schema_repeat_payload_type(module, schema, &repeat, stack)?;
            (0, Type::named("List", vec![element_ty]))
        } else {
            let dispatch = closed_dispatch_schema_primitive(&field.ty)
                .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
            if decoded_fields.get(&dispatch.tag_field) != Some(&Type::int())
                || dispatch.length_field.as_ref().is_some_and(|length_field| {
                    decoded_fields.get(length_field) != Some(&Type::int())
                })
            {
                return None;
            }
            let payload_types = schema_dispatch_case_types(module, schema, &dispatch, stack)?;
            let payload_ty = schema_dispatch_payload_type(schema, &dispatch, &payload_types)?;
            let field_ty = if dispatch.preserves_unknown {
                Type::named("SchemaDispatchPayload", vec![payload_ty])
            } else {
                payload_ty
            };
            (0, field_ty)
        };
        decoded_fields.insert(field.name.clone(), ty.clone());
        fields.push((field.name.clone(), ty, width));
    }
    Some(fields)
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
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    payload_types: &[(i64, Type)],
) -> Option<Type> {
    let first = payload_types.first()?.1.clone();
    if payload_types.iter().all(|(_, ty)| ty == &first)
        || selected_mappings_cover_closed_dispatch(schema, dispatch)
    {
        Some(first)
    } else {
        None
    }
}

fn schema_dispatch_case_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: &SchemaDispatchCase,
    stack: &mut Vec<String>,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if schema.name.as_deref() == Some(schema_name.as_str()) {
                return schema_recursive_dispatch_payload_type(module, schema);
            }
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            schema_decode_value_type_inner(module, nested, stack)
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
        SchemaRepeatPayload::ByteView { .. } => Some(Type::named("ByteView", Vec::new())),
        SchemaRepeatPayload::Schema { schema_name } => {
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
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
    let mapped_fields = schema_decode_mapping_record_fields(module, schema, &fields)
        .unwrap_or_else(|| fields.into_iter().map(|(name, ty, _)| (name, ty)).collect());
    Some(Type::Record(mapped_fields))
}

pub(crate) fn schema_recursive_dispatch_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    let [first_mapping, rest @ ..] = schema.mappings.as_slice() else {
        return None;
    };
    let target_fields = schema_mapping_target_record_fields(module, schema, first_mapping)?;
    for mapping in rest {
        mapping.selector.as_ref()?;
        if schema_mapping_target_record_fields(module, schema, mapping)? != target_fields {
            return None;
        }
    }
    Some(Type::Record(target_fields))
}

pub(crate) fn recursive_closed_dispatch_payload_is_eligible(
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema.name.as_deref() == Some(schema_name)
        && !dispatch.preserves_unknown
        && dispatch.length_field.is_some()
        && schema.mappings.len() == dispatch.cases.len()
        && selected_mappings_cover_closed_dispatch(schema, dispatch)
        && schema
            .fields
            .iter()
            .position(|candidate| candidate.node_id == field.node_id)
            .is_some_and(|index| index > 0)
        && dispatch.cases.iter().any(|case| {
            !matches!(
                &case.payload,
                SchemaDispatchCasePayload::Schema { schema_name }
                    if schema.name.as_deref() == Some(schema_name.as_str())
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

pub(crate) fn selected_mappings_cover_closed_dispatch(
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    if dispatch.preserves_unknown
        || schema.mappings.len() != dispatch.cases.len()
        || schema.mappings.is_empty()
    {
        return false;
    }
    let case_tags = dispatch
        .cases
        .iter()
        .map(|case| case.tag)
        .collect::<BTreeSet<_>>();
    let mut selector_tags = BTreeSet::<i64>::new();
    for mapping in &schema.mappings {
        let Some(selector) = &mapping.selector else {
            return false;
        };
        if selector.field != dispatch.tag_field
            || !case_tags.contains(&selector.value)
            || !selector_tags.insert(selector.value)
        {
            return false;
        }
    }
    selector_tags == case_tags
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
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let mut fields = Vec::new();
    let mut exact_width_field_names = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            supported_encode_reserved_bits(&schema.fields, index, reserved)?;
            continue;
        }
        if exact_width_schema_primitive(&field.ty).is_some() {
            exact_width_field_names.push(field.name.clone());
            let ty = if let Some(flag_type) = flag_schema_primitive(&field.ty) {
                Type::named(flag_type, Vec::new())
            } else {
                Type::int()
            };
            fields.push((field.name.clone(), ty));
            continue;
        }
        if let Some(repeat) = repeat_schema_primitive(&field.ty) {
            if schema_length_expression_references(&repeat.count_field)?
                .into_iter()
                .any(|reference| {
                    !exact_width_field_names
                        .iter()
                        .any(|field| field == reference)
                })
            {
                return None;
            }
            if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload
                && !exact_width_field_names
                    .iter()
                    .any(|field| field == length_field)
            {
                return None;
            }
            let element_ty = schema_repeat_payload_type(module, schema, &repeat, &mut Vec::new())?;
            fields.push((field.name.clone(), Type::named("List", vec![element_ty])));
            continue;
        }
        if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if length_expr.references().into_iter().any(|reference| {
                !exact_width_field_names
                    .iter()
                    .any(|field| field == reference)
            }) {
                return None;
            }
            fields.push((field.name.clone(), Type::named("ByteView", Vec::new())));
            continue;
        }
        let dispatch = closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
        let recursive_closed_dispatch_payload =
            recursive_closed_dispatch_encode_payload_field(schema, field, &dispatch);
        if !exact_width_field_names.contains(&dispatch.tag_field)
            || dispatch
                .length_field
                .as_ref()
                .is_some_and(|length_field| !exact_width_field_names.contains(length_field))
            || (dispatch.length_field.is_some()
                && !dispatch.preserves_unknown
                && !recursive_closed_dispatch_payload)
        {
            return None;
        }
        let mut payload_types = dispatch
            .cases
            .iter()
            .map(|case| schema_dispatch_case_type(module, schema, case, &mut Vec::new()))
            .collect::<Option<Vec<_>>>()?;
        let payload_ty = if recursive_closed_dispatch_payload
            && selected_mappings_cover_closed_dispatch(schema, &dispatch)
        {
            schema_recursive_dispatch_payload_type(module, schema)?
        } else {
            let payload_ty = payload_types.pop()?;
            if payload_types.iter().any(|ty| ty != &payload_ty) {
                return None;
            }
            payload_ty
        };
        if !recursive_closed_dispatch_payload && payload_types.iter().any(|ty| ty != &payload_ty) {
            return None;
        }
        if dispatch.preserves_unknown {
            fields.push((
                field.name.clone(),
                Type::named("SchemaDispatchPayload", vec![payload_ty]),
            ));
        } else {
            fields.push((field.name.clone(), payload_ty));
        }
    }
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
        return_type: Type::named("Result", vec![byte_chunk, encode_error]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

fn recursive_closed_dispatch_encode_payload_field(
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    schema.name.as_deref().is_some_and(|schema_name| {
        recursive_closed_dispatch_payload_is_eligible(schema, field, dispatch, schema_name)
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
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    let [] = schema.mappings.as_slice() else {
        return schema_encode_mapping_value_fields(
            module,
            schema,
            schema_fields,
            exact_width_field_names,
        );
    };
    Some(schema_fields.to_vec())
}

fn schema_encode_mapping_value_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    let [mapping] = schema.mappings.as_slice() else {
        return schema_encode_selected_mapping_value_fields(
            module,
            schema,
            schema_fields,
            exact_width_field_names,
        );
    };
    let target_fields = schema_mapping_target_record_fields(module, schema, mapping)?;
    let schema_field_types = schema_fields.iter().cloned().collect::<BTreeMap<_, _>>();
    let source_context = SchemaEncodeMappingSourceContext {
        module,
        schema,
        schema_field_types: &schema_field_types,
        exact_width_field_names,
        allow_single_payload_variant: false,
    };
    if mapping.selector.is_some()
        || schema_encode_mapping_source_targets(
            schema_fields,
            &source_context,
            mapping,
            &target_fields,
        )
        .is_none()
    {
        return None;
    }
    Some(target_fields)
}

fn schema_encode_selected_mapping_value_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    let [first, rest @ ..] = schema.mappings.as_slice() else {
        return None;
    };
    first.selector.as_ref()?;
    let target_fields = schema_mapping_target_record_fields(module, schema, first)?;
    let (schema_field_types, supported_int_field_names) = schema_encode_mapping_field_types(
        module,
        schema,
        schema_fields,
        exact_width_field_names,
        first,
    )?;
    let source_context = SchemaEncodeMappingSourceContext {
        module,
        schema,
        schema_field_types: &schema_field_types,
        exact_width_field_names: &supported_int_field_names,
        allow_single_payload_variant: true,
    };
    schema_encode_mapping_source_targets(schema_fields, &source_context, first, &target_fields)?;
    for mapping in rest {
        mapping.selector.as_ref()?;
        let candidate_target_fields = schema_mapping_target_record_fields(module, schema, mapping)?;
        if candidate_target_fields != target_fields {
            return None;
        }
        let (schema_field_types, supported_int_field_names) = schema_encode_mapping_field_types(
            module,
            schema,
            schema_fields,
            exact_width_field_names,
            mapping,
        )?;
        let source_context = SchemaEncodeMappingSourceContext {
            module,
            schema,
            schema_field_types: &schema_field_types,
            exact_width_field_names: &supported_int_field_names,
            allow_single_payload_variant: true,
        };
        schema_encode_mapping_source_targets(
            schema_fields,
            &source_context,
            mapping,
            &target_fields,
        )?;
    }
    Some(target_fields)
}

fn schema_encode_mapping_field_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    exact_width_field_names: &[String],
    mapping: &veln_ast::SchemaMappingClause,
) -> Option<(BTreeMap<String, Type>, Vec<String>)> {
    let mut schema_field_types = schema_fields.iter().cloned().collect::<BTreeMap<_, _>>();
    let mut supported_int_field_names = exact_width_field_names.to_vec();
    let selector = mapping.selector.as_ref()?;
    for field in &schema.fields {
        let Some(dispatch) = closed_dispatch_schema_primitive(&field.ty) else {
            continue;
        };
        if dispatch.preserves_unknown || selector.field != dispatch.tag_field {
            continue;
        }
        let case = dispatch
            .cases
            .iter()
            .find(|case| case.tag == selector.value)?;
        let case_ty = schema_dispatch_case_type(module, schema, case, &mut Vec::new())?;
        if case_ty == Type::int() {
            supported_int_field_names.push(field.name.clone());
        }
        schema_field_types.insert(field.name.clone(), case_ty);
    }
    Some((schema_field_types, supported_int_field_names))
}

struct SchemaEncodeMappingSourceContext<'a> {
    module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    schema_field_types: &'a BTreeMap<String, Type>,
    exact_width_field_names: &'a [String],
    allow_single_payload_variant: bool,
}

fn schema_encode_mapping_source_targets(
    schema_fields: &[(String, Type)],
    context: &SchemaEncodeMappingSourceContext<'_>,
    mapping: &veln_ast::SchemaMappingClause,
    target_fields: &[(String, Type)],
) -> Option<BTreeMap<String, String>> {
    let target_field_types = target_fields.iter().cloned().collect::<BTreeMap<_, _>>();
    let mut source_to_target = BTreeMap::<String, String>::new();
    for assignment in &mapping.assignments {
        let target_ty = target_field_types.get(&assignment.target)?;
        let sources = schema_encode_mapping_assignment_sources(
            context.module,
            context.schema,
            context.schema_field_types,
            context.exact_width_field_names,
            assignment,
            target_ty,
            context.allow_single_payload_variant,
        )?;
        for source in sources {
            if source_to_target
                .insert(source.clone(), assignment.target.clone())
                .is_some()
            {
                return None;
            }
        }
    }
    if schema_fields
        .iter()
        .any(|(source, _)| !source_to_target.contains_key(source))
    {
        return None;
    }
    Some(source_to_target)
}

fn schema_encode_mapping_assignment_sources(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
    assignment: &veln_ast::SchemaMappingAssignment,
    target_ty: &Type,
    allow_single_payload_variant: bool,
) -> Option<Vec<String>> {
    if let ExprKind::NamePath(segments) = &assignment.expr.kind {
        let [source] = segments.as_slice() else {
            return None;
        };
        let source_ty = schema_field_types.get(source)?;
        return is_assignable(target_ty, source_ty).then(|| vec![source.clone()]);
    }

    let typed = schema_mapping_expr_typed(
        module,
        schema,
        schema_field_types,
        &assignment.expr,
        target_ty,
    )
    .ok()?;
    match typed.expr {
        SchemaDecodeMappingExpr::Constructor { name, args } => {
            let registry = AdtRegistry::from_module(module);
            let descriptor = registry.descriptor_for_type(target_ty)?;
            if name.len() != 2 || name[0] != descriptor.type_name {
                return None;
            }
            if !descriptor.variants.iter().any(|variant| {
                variant.name == name[1] && variant.payload_fields.len() == args.len()
            }) {
                return None;
            }
            if !allow_single_payload_variant
                && args.len() == 1
                && descriptor.variants.len() != 1
                && !matches!(args.first(), Some(SchemaDecodeMappingExpr::Record(_)))
            {
                return None;
            }
            let mut sources = Vec::with_capacity(args.len());
            for arg in args {
                let arg_sources = schema_encode_mapping_expr_sources(
                    &arg,
                    schema_field_types,
                    exact_width_field_names,
                )?;
                if arg_sources.is_empty() {
                    return None;
                }
                sources.extend(arg_sources);
            }
            Some(sources)
        }
        expr => {
            let sources = schema_encode_mapping_expr_sources(
                &expr,
                schema_field_types,
                exact_width_field_names,
            )?;
            (!sources.is_empty()).then_some(sources)
        }
    }
}

fn schema_encode_mapping_expr_sources(
    expr: &SchemaDecodeMappingExpr,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
) -> Option<Vec<String>> {
    match expr {
        SchemaDecodeMappingExpr::Field(source) => {
            let source_ty = schema_field_types.get(source)?;
            schema_encode_mapping_source_supported(source, source_ty, exact_width_field_names)
                .then(|| vec![source.clone()])
        }
        SchemaDecodeMappingExpr::Record(fields) => {
            let mut sources = Vec::with_capacity(fields.len());
            for field in fields {
                let SchemaDecodeMappingExpr::Field(source) = &field.expr else {
                    return None;
                };
                let source_ty = schema_field_types.get(source)?;
                if !schema_encode_mapping_source_supported(
                    source,
                    source_ty,
                    exact_width_field_names,
                ) {
                    return None;
                }
                sources.push(source.clone());
            }
            Some(sources)
        }
        SchemaDecodeMappingExpr::FieldAccess { base, field } => {
            schema_encode_mapping_selected_record_source(
                base,
                field,
                schema_field_types,
                exact_width_field_names,
            )
            .map(|source| vec![source])
        }
        SchemaDecodeMappingExpr::Constructor { .. }
        | SchemaDecodeMappingExpr::Converter { .. }
        | SchemaDecodeMappingExpr::Binary { .. } => None,
    }
}

fn schema_encode_mapping_selected_record_source(
    base: &SchemaDecodeMappingExpr,
    field: &str,
    schema_field_types: &BTreeMap<String, Type>,
    exact_width_field_names: &[String],
) -> Option<String> {
    let SchemaDecodeMappingExpr::Record(fields) = base else {
        return None;
    };
    let selected = fields.iter().find(|candidate| candidate.name == field)?;
    let SchemaDecodeMappingExpr::Field(source) = &selected.expr else {
        return None;
    };
    let source_ty = schema_field_types.get(source)?;
    schema_encode_mapping_source_supported(source, source_ty, exact_width_field_names)
        .then(|| source.clone())
}

fn schema_encode_mapping_source_supported(
    source: &str,
    ty: &Type,
    exact_width_field_names: &[String],
) -> bool {
    is_type_flag_bitset(ty)
        || ty != &Type::int()
        || exact_width_field_names.iter().any(|field| field == source)
}

fn is_type_flag_bitset(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args }
            if matches!(
                name.as_str(),
                "Flag8" | "Flag16be" | "Flag16le" | "Flag32be" | "Flag32le"
                    | "Flag64be" | "Flag64le"
            )
                && args.is_empty()
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMappingField {
    pub(crate) target: String,
    pub(crate) source: String,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMapping {
    pub(crate) selector: Option<SchemaDecodeMappingSelector>,
    pub(crate) fields: Vec<SchemaDecodeMappingField>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMappingSelector {
    pub(crate) field: String,
    pub(crate) value: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaDecodeMappingExpr {
    Field(String),
    FieldAccess {
        base: Box<SchemaDecodeMappingExpr>,
        field: String,
    },
    Record(Vec<SchemaDecodeMappingRecordField>),
    Constructor {
        name: Vec<String>,
        args: Vec<SchemaDecodeMappingExpr>,
    },
    Converter {
        function: String,
        arg: Box<SchemaDecodeMappingExpr>,
    },
    Binary {
        op: BinaryOp,
        left: Box<SchemaDecodeMappingExpr>,
        right: Box<SchemaDecodeMappingExpr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaDecodeMappingRecordField {
    pub(crate) name: String,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SchemaMappingTypedExpr {
    pub(crate) ty: Type,
    pub(crate) expr: SchemaDecodeMappingExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaMappingConverterInput {
    SourceField(String),
    Expression(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SchemaMappingExprError {
    Unsupported {
        text: String,
        span: SourceSpan,
    },
    UnknownSchemaField {
        name: String,
        span: SourceSpan,
    },
    UnresolvedConstructor {
        name: String,
        span: SourceSpan,
    },
    UnresolvedConverter {
        name: String,
        span: SourceSpan,
    },
    PrivateConverter {
        name: String,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ConstructorArity {
        name: String,
        expected: usize,
        actual: usize,
        span: SourceSpan,
    },
    ConverterArity {
        name: String,
        expected: usize,
        actual: usize,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ConverterInputType {
        name: String,
        expected: Box<Type>,
        actual: Box<Type>,
        input: SchemaMappingConverterInput,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ConverterReturnType {
        name: String,
        expected: Box<Type>,
        actual: Box<Type>,
        input: SchemaMappingConverterInput,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    ImpureConverter {
        name: String,
        effects: Vec<String>,
        span: SourceSpan,
        function_span: SourceSpan,
    },
    RecordField {
        name: String,
        span: SourceSpan,
    },
    MissingRecordField {
        name: String,
        span: SourceSpan,
    },
    TypeMismatch {
        expected: Box<Type>,
        actual: Box<Type>,
        text: String,
        span: SourceSpan,
    },
}

type SchemaMappingExprResult = Result<SchemaMappingTypedExpr, Box<SchemaMappingExprError>>;

struct SchemaMappingExprContext<'a> {
    module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    registry: &'a AdtRegistry,
    converter_functions: &'a [FunctionSignature],
    schema_fields: &'a BTreeMap<String, Type>,
}

pub(crate) fn schema_decode_mapping_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<SchemaDecodeMappingField>> {
    let decoded_fields = schema_decode_record_fields(module, schema)?;
    schema_decode_mapping_fields_from_decoded_fields(module, schema, &decoded_fields)
}

pub(crate) fn schema_decode_mappings(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<SchemaDecodeMapping>> {
    let decoded_fields = schema_decode_record_fields(module, schema)?;
    schema_decode_mappings_from_decoded_fields(module, schema, &decoded_fields)
}

fn schema_decode_mapping_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
) -> Option<Vec<(String, Type)>> {
    let [first_mapping, rest @ ..] = schema.mappings.as_slice() else {
        return None;
    };
    let target_fields = schema_mapping_target_record_fields(module, schema, first_mapping)?;
    let source_field_types = decoded_fields
        .iter()
        .map(|(name, ty, _)| (name.clone(), ty.clone()))
        .collect::<BTreeMap<_, _>>();
    let mapping_source_field_types =
        schema_mapping_source_field_types(module, schema, &source_field_types, first_mapping)?;
    validate_schema_decode_mapping_fields(
        module,
        schema,
        &mapping_source_field_types,
        first_mapping,
        &target_fields,
    )?;
    for mapping in rest {
        mapping.selector.as_ref()?;
        if schema_mapping_target_record_fields(module, schema, mapping)? != target_fields {
            return None;
        }
        let mapping_source_field_types =
            schema_mapping_source_field_types(module, schema, &source_field_types, mapping)?;
        validate_schema_decode_mapping_fields(
            module,
            schema,
            &mapping_source_field_types,
            mapping,
            &target_fields,
        )?;
    }
    Some(target_fields)
}

fn schema_decode_mapping_fields_from_decoded_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
) -> Option<Vec<SchemaDecodeMappingField>> {
    let [mapping] = schema.mappings.as_slice() else {
        return None;
    };
    schema_decode_mapping_fields_for_mapping(module, schema, decoded_fields, mapping)
}

fn schema_decode_mappings_from_decoded_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
) -> Option<Vec<SchemaDecodeMapping>> {
    if schema.mappings.is_empty() {
        return None;
    }
    schema
        .mappings
        .iter()
        .map(|mapping| {
            let fields =
                schema_decode_mapping_fields_for_mapping(module, schema, decoded_fields, mapping)?;
            let selector = mapping
                .selector
                .as_ref()
                .map(|selector| SchemaDecodeMappingSelector {
                    field: selector.field.clone(),
                    value: selector.value,
                });
            Some(SchemaDecodeMapping { selector, fields })
        })
        .collect()
}

fn schema_decode_mapping_fields_for_mapping(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &[(String, Type, u8)],
    mapping: &SchemaMappingClause,
) -> Option<Vec<SchemaDecodeMappingField>> {
    let target_fields = schema_mapping_target_record_fields(module, schema, mapping)?;
    let source_field_types = decoded_fields
        .iter()
        .map(|(name, ty, _)| (name.clone(), ty.clone()))
        .collect::<BTreeMap<_, _>>();
    let source_field_types =
        schema_mapping_source_field_types(module, schema, &source_field_types, mapping)?;
    let mut fields = Vec::new();
    for (target, target_ty) in target_fields {
        let assignment = mapping
            .assignments
            .iter()
            .find(|assignment| assignment.target == target)?;
        let typed = schema_mapping_expr_typed(
            module,
            schema,
            &source_field_types,
            &assignment.expr,
            &target_ty,
        )
        .ok()?;
        fields.push(SchemaDecodeMappingField {
            target,
            source: assignment.source.clone(),
            expr: typed.expr,
        });
    }
    Some(fields)
}

pub(crate) fn schema_mapping_source_field_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
    mapping: &SchemaMappingClause,
) -> Option<BTreeMap<String, Type>> {
    let mut fields = schema_fields.clone();
    let Some(selector) = &mapping.selector else {
        return Some(fields);
    };
    for field in &schema.fields {
        let Some(dispatch) = closed_dispatch_schema_primitive(&field.ty) else {
            continue;
        };
        if dispatch.tag_field != selector.field
            || !selected_mappings_cover_closed_dispatch(schema, &dispatch)
        {
            continue;
        }
        let case = dispatch
            .cases
            .iter()
            .find(|case| case.tag == selector.value)?;
        let ty = schema_dispatch_case_type(module, schema, case, &mut Vec::new())?;
        fields.insert(field.name.clone(), ty);
    }
    Some(fields)
}

fn validate_schema_decode_mapping_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    source_field_types: &BTreeMap<String, Type>,
    mapping: &SchemaMappingClause,
    target_fields: &[(String, Type)],
) -> Option<()> {
    for (target, target_ty) in target_fields {
        let assignment = mapping
            .assignments
            .iter()
            .find(|assignment| assignment.target == *target)?;
        schema_mapping_expr_typed(
            module,
            schema,
            source_field_types,
            &assignment.expr,
            target_ty,
        )
        .ok()?;
    }
    Some(())
}

pub(crate) fn schema_mapping_expr_typed(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_fields: &BTreeMap<String, Type>,
    expr: &Expr,
    expected: &Type,
) -> SchemaMappingExprResult {
    let registry = AdtRegistry::from_module(module);
    let mut converter_functions = ordinary_function_signatures(module);
    infer_function_body_effects(module, &mut converter_functions);
    let context = SchemaMappingExprContext {
        module,
        schema,
        registry: &registry,
        converter_functions: &converter_functions,
        schema_fields,
    };
    let typed = schema_mapping_expr_typed_unchecked(&context, expr, expected, true)?;
    if !is_assignable(expected, &typed.ty) {
        return Err(Box::new(SchemaMappingExprError::TypeMismatch {
            expected: Box::new(expected.clone()),
            actual: Box::new(typed.ty),
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        }));
    }
    Ok(typed)
}

fn schema_mapping_expr_typed_unchecked(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    expected: &Type,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    match &expr.kind {
        ExprKind::NamePath(segments) => schema_mapping_name_expr(context, expr, segments, expected),
        ExprKind::FieldAccess { base, field, .. } => {
            let typed_base = schema_mapping_expr_inferred(context, base, allow_converter_calls)?;
            let Some(field_ty) = typed_base.ty.record_field(field) else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            Ok(SchemaMappingTypedExpr {
                ty: field_ty.clone(),
                expr: SchemaDecodeMappingExpr::FieldAccess {
                    base: Box::new(typed_base.expr),
                    field: field.clone(),
                },
            })
        }
        ExprKind::Record(fields) => {
            let Type::Record(expected_fields) = expected else {
                return Err(Box::new(SchemaMappingExprError::TypeMismatch {
                    expected: Box::new(expected.clone()),
                    actual: Box::new(schema_mapping_record_actual_type(context, fields)),
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            let mut seen = BTreeMap::<String, SourceSpan>::new();
            let mut record_fields = Vec::new();
            for field in fields {
                if seen
                    .insert(field.name.clone(), field.span.clone())
                    .is_some()
                {
                    return Err(Box::new(SchemaMappingExprError::RecordField {
                        name: field.name.clone(),
                        span: field.span.clone(),
                    }));
                }
                let Some((_, field_ty)) =
                    expected_fields.iter().find(|(name, _)| name == &field.name)
                else {
                    return Err(Box::new(SchemaMappingExprError::RecordField {
                        name: field.name.clone(),
                        span: field.span.clone(),
                    }));
                };
                let typed = schema_mapping_expr_typed_inner(
                    context,
                    &field.expr,
                    field_ty,
                    allow_converter_calls,
                )?;
                record_fields.push(SchemaDecodeMappingRecordField {
                    name: field.name.clone(),
                    expr: typed.expr,
                });
            }
            for (name, _) in expected_fields {
                if !seen.contains_key(name) {
                    return Err(Box::new(SchemaMappingExprError::MissingRecordField {
                        name: name.clone(),
                        span: expr.span.clone(),
                    }));
                }
            }
            Ok(SchemaMappingTypedExpr {
                ty: expected.clone(),
                expr: SchemaDecodeMappingExpr::Record(record_fields),
            })
        }
        ExprKind::Call { callee, args } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            if !allow_converter_calls && schema_mapping_name_can_be_converter(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            if allow_converter_calls {
                match schema_mapping_converter_function(context, segments) {
                    SchemaMappingConverterLookup::Found(function) => {
                        return schema_mapping_converter_expr(
                            context, expr, callee, args, function, expected,
                        );
                    }
                    SchemaMappingConverterLookup::Private(function) => {
                        return Err(Box::new(SchemaMappingExprError::PrivateConverter {
                            name: segments.join("::"),
                            span: callee.span.clone(),
                            function_span: function.span.clone(),
                        }));
                    }
                    SchemaMappingConverterLookup::Missing => {}
                }
            }
            if let [name] = segments.as_slice()
                && !schema_mapping_name_can_be_constructor(segments)
            {
                return Err(Box::new(SchemaMappingExprError::UnresolvedConverter {
                    name: name.clone(),
                    span: callee.span.clone(),
                }));
            }
            if segments.len() > 1 && schema_mapping_name_can_be_converter(segments) {
                return Err(Box::new(SchemaMappingExprError::UnresolvedConverter {
                    name: segments.join("::"),
                    span: callee.span.clone(),
                }));
            }
            if !schema_mapping_name_can_be_constructor(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            let constructor = schema_mapping_constructor(
                context.registry,
                context.module,
                context.schema,
                segments,
                expected,
            )
            .ok_or_else(|| {
                Box::new(SchemaMappingExprError::UnresolvedConstructor {
                    name: segments.join("::"),
                    span: callee.span.clone(),
                })
            })?;
            schema_mapping_constructor_expr(
                context,
                expr,
                args,
                expected,
                constructor,
                allow_converter_calls,
            )
        }
        ExprKind::Binary { op, left, right } => {
            schema_mapping_binary_expr(context, expr, *op, left, right, allow_converter_calls)
        }
        _ => Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        })),
    }
}

fn schema_mapping_expr_typed_inner(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    expected: &Type,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    let typed =
        schema_mapping_expr_typed_unchecked(context, expr, expected, allow_converter_calls)?;
    if !is_assignable(expected, &typed.ty) {
        return Err(Box::new(SchemaMappingExprError::TypeMismatch {
            expected: Box::new(expected.clone()),
            actual: Box::new(typed.ty),
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        }));
    }
    Ok(typed)
}

fn schema_mapping_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    match &expr.kind {
        ExprKind::NamePath(segments) => schema_mapping_name_expr_inferred(context, expr, segments),
        ExprKind::FieldAccess { base, field, .. } => {
            let typed_base = schema_mapping_expr_inferred(context, base, allow_converter_calls)?;
            let Some(field_ty) = typed_base.ty.record_field(field) else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            Ok(SchemaMappingTypedExpr {
                ty: field_ty.clone(),
                expr: SchemaDecodeMappingExpr::FieldAccess {
                    base: Box::new(typed_base.expr),
                    field: field.clone(),
                },
            })
        }
        ExprKind::Record(fields) => {
            let mut seen = BTreeMap::<String, SourceSpan>::new();
            let mut record_fields = Vec::new();
            let mut field_types = Vec::new();
            for field in fields {
                if seen
                    .insert(field.name.clone(), field.span.clone())
                    .is_some()
                {
                    return Err(Box::new(SchemaMappingExprError::RecordField {
                        name: field.name.clone(),
                        span: field.span.clone(),
                    }));
                }
                let typed =
                    schema_mapping_expr_inferred(context, &field.expr, allow_converter_calls)?;
                field_types.push((field.name.clone(), typed.ty));
                record_fields.push(SchemaDecodeMappingRecordField {
                    name: field.name.clone(),
                    expr: typed.expr,
                });
            }
            Ok(SchemaMappingTypedExpr {
                ty: Type::Record(field_types),
                expr: SchemaDecodeMappingExpr::Record(record_fields),
            })
        }
        ExprKind::Call { callee, args } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            };
            if !allow_converter_calls && schema_mapping_name_can_be_converter(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            if allow_converter_calls {
                match schema_mapping_converter_function(context, segments) {
                    SchemaMappingConverterLookup::Found(function) => {
                        return schema_mapping_converter_expr_inferred(
                            context, expr, callee, args, function,
                        );
                    }
                    SchemaMappingConverterLookup::Private(function) => {
                        return Err(Box::new(SchemaMappingExprError::PrivateConverter {
                            name: segments.join("::"),
                            span: callee.span.clone(),
                            function_span: function.span.clone(),
                        }));
                    }
                    SchemaMappingConverterLookup::Missing => {}
                }
            }
            if !schema_mapping_name_can_be_constructor(segments) {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(expr),
                    span: expr.span.clone(),
                }));
            }
            let constructor = schema_mapping_constructor(
                context.registry,
                context.module,
                context.schema,
                segments,
                &Type::Unknown,
            )
            .ok_or_else(|| {
                Box::new(SchemaMappingExprError::UnresolvedConstructor {
                    name: segments.join("::"),
                    span: callee.span.clone(),
                })
            })?;
            schema_mapping_constructor_expr_inferred(
                context,
                expr,
                args,
                constructor,
                allow_converter_calls,
            )
        }
        ExprKind::Binary { op, left, right } => {
            schema_mapping_binary_expr(context, expr, *op, left, right, allow_converter_calls)
        }
        _ => Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        })),
    }
}

fn schema_mapping_binary_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    op: BinaryOp,
    left: &Expr,
    right: &Expr,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    if !matches!(op, BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply) {
        return Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(expr),
            span: expr.span.clone(),
        }));
    }
    let expected = Type::int();
    let left =
        schema_mapping_arithmetic_operand(context, expr, left, &expected, allow_converter_calls)?;
    let right =
        schema_mapping_arithmetic_operand(context, expr, right, &expected, allow_converter_calls)?;
    Ok(SchemaMappingTypedExpr {
        ty: expected,
        expr: SchemaDecodeMappingExpr::Binary {
            op,
            left: Box::new(left.expr),
            right: Box::new(right.expr),
        },
    })
}

fn schema_mapping_arithmetic_operand(
    context: &SchemaMappingExprContext<'_>,
    whole_expr: &Expr,
    operand: &Expr,
    expected: &Type,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    match &operand.kind {
        ExprKind::NamePath(segments) => {
            let [name] = segments.as_slice() else {
                return Err(Box::new(SchemaMappingExprError::Unsupported {
                    text: schema_mapping_expr_render(whole_expr),
                    span: whole_expr.span.clone(),
                }));
            };
            let Some(ty) = context.schema_fields.get(name) else {
                return Err(Box::new(SchemaMappingExprError::UnknownSchemaField {
                    name: name.clone(),
                    span: operand.span.clone(),
                }));
            };
            if !is_assignable(expected, ty) {
                return Err(Box::new(SchemaMappingExprError::TypeMismatch {
                    expected: Box::new(expected.clone()),
                    actual: Box::new(ty.clone()),
                    text: schema_mapping_expr_render(operand),
                    span: operand.span.clone(),
                }));
            }
            Ok(SchemaMappingTypedExpr {
                ty: ty.clone(),
                expr: SchemaDecodeMappingExpr::Field(name.clone()),
            })
        }
        ExprKind::Binary { op, left, right } => {
            schema_mapping_binary_expr(context, operand, *op, left, right, allow_converter_calls)
                .map_err(|error| match *error {
                    SchemaMappingExprError::Unsupported { .. } => {
                        Box::new(SchemaMappingExprError::Unsupported {
                            text: schema_mapping_expr_render(whole_expr),
                            span: whole_expr.span.clone(),
                        })
                    }
                    other => Box::new(other),
                })
        }
        _ => Err(Box::new(SchemaMappingExprError::Unsupported {
            text: schema_mapping_expr_render(whole_expr),
            span: whole_expr.span.clone(),
        })),
    }
}

enum SchemaMappingConverterLookup<'a> {
    Found(&'a FunctionSignature),
    Private(&'a FunctionSignature),
    Missing,
}

fn schema_mapping_converter_function<'a>(
    context: &SchemaMappingExprContext<'a>,
    segments: &[String],
) -> SchemaMappingConverterLookup<'a> {
    match segments {
        [name] => context
            .converter_functions
            .iter()
            .find(|function| {
                function.name == *name && schema_mapping_same_module(function, context.schema)
            })
            .map_or(
                SchemaMappingConverterLookup::Missing,
                SchemaMappingConverterLookup::Found,
            ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &context.module.uses,
                &segments[..segments.len() - 1],
                context.schema.module_name.as_deref(),
            ) else {
                return SchemaMappingConverterLookup::Missing;
            };
            let module_name = use_decl.name.as_str();
            let Some(function) = context.converter_functions.iter().find(|function| {
                function.name == *name && function.module_name.as_deref() == Some(module_name)
            }) else {
                return SchemaMappingConverterLookup::Missing;
            };
            if function.visibility == Visibility::Public {
                SchemaMappingConverterLookup::Found(function)
            } else {
                SchemaMappingConverterLookup::Private(function)
            }
        }
        _ => SchemaMappingConverterLookup::Missing,
    }
}

fn schema_mapping_same_module(function: &FunctionSignature, schema: &SchemaDecl) -> bool {
    match (
        function.module_name.as_deref(),
        schema.module_name.as_deref(),
    ) {
        (Some(function_module), Some(schema_module)) => function_module == schema_module,
        (None, None) => function.span.file == schema.span.file,
        _ => false,
    }
}

fn schema_mapping_converter_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    function: &FunctionSignature,
    expected: &Type,
) -> SchemaMappingExprResult {
    let (typed_arg, input) =
        schema_mapping_converter_arg_expr(context, expr, callee, args, function)?;
    if !is_assignable(expected, &function.return_type) {
        return Err(Box::new(SchemaMappingExprError::ConverterReturnType {
            name: function.name.clone(),
            expected: Box::new(expected.clone()),
            actual: Box::new(function.return_type.clone()),
            input,
            span: expr.span.clone(),
            function_span: function.span.clone(),
        }));
    }

    Ok(SchemaMappingTypedExpr {
        ty: function.return_type.clone(),
        expr: SchemaDecodeMappingExpr::Converter {
            function: function.target_name.clone(),
            arg: Box::new(typed_arg.expr),
        },
    })
}

fn schema_mapping_converter_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    function: &FunctionSignature,
) -> SchemaMappingExprResult {
    let (typed_arg, _) = schema_mapping_converter_arg_expr(context, expr, callee, args, function)?;
    Ok(SchemaMappingTypedExpr {
        ty: function.return_type.clone(),
        expr: SchemaDecodeMappingExpr::Converter {
            function: function.target_name.clone(),
            arg: Box::new(typed_arg.expr),
        },
    })
}

fn schema_mapping_converter_arg_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    callee: &Expr,
    args: &[Expr],
    function: &FunctionSignature,
) -> Result<(SchemaMappingTypedExpr, SchemaMappingConverterInput), Box<SchemaMappingExprError>> {
    if args.len() != 1 {
        return Err(Box::new(SchemaMappingExprError::ConverterArity {
            name: function.name.clone(),
            expected: 1,
            actual: args.len(),
            span: expr.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    if function.params.len() != 1 {
        return Err(Box::new(SchemaMappingExprError::ConverterArity {
            name: function.name.clone(),
            expected: 1,
            actual: function.params.len(),
            span: callee.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    if !function.effects.is_empty() {
        return Err(Box::new(SchemaMappingExprError::ImpureConverter {
            name: function.name.clone(),
            effects: function.effects.clone(),
            span: callee.span.clone(),
            function_span: function.span.clone(),
        }));
    }

    let arg = &args[0];
    let input = schema_mapping_converter_input(arg);
    let param_ty = &function.params[0];
    let typed_arg = match schema_mapping_expr_typed_unchecked(context, arg, param_ty, false) {
        Ok(typed) => typed,
        Err(error) => match *error {
            SchemaMappingExprError::TypeMismatch {
                actual, span, text, ..
            } if text == schema_mapping_expr_render(arg) => {
                return Err(Box::new(SchemaMappingExprError::ConverterInputType {
                    name: function.name.clone(),
                    expected: Box::new(param_ty.clone()),
                    actual,
                    input,
                    span,
                    function_span: function.span.clone(),
                }));
            }
            other => return Err(Box::new(other)),
        },
    };
    if !is_assignable(param_ty, &typed_arg.ty) {
        return Err(Box::new(SchemaMappingExprError::ConverterInputType {
            name: function.name.clone(),
            expected: Box::new(param_ty.clone()),
            actual: Box::new(typed_arg.ty),
            input,
            span: arg.span.clone(),
            function_span: function.span.clone(),
        }));
    }
    Ok((typed_arg, input))
}

fn schema_mapping_converter_input(arg: &Expr) -> SchemaMappingConverterInput {
    if let ExprKind::NamePath(segments) = &arg.kind
        && let [source] = segments.as_slice()
    {
        return SchemaMappingConverterInput::SourceField(source.clone());
    }
    SchemaMappingConverterInput::Expression(schema_mapping_expr_render(arg))
}

fn schema_mapping_record_actual_type(
    context: &SchemaMappingExprContext<'_>,
    fields: &[veln_ast::RecordField],
) -> Type {
    Type::Record(
        fields
            .iter()
            .map(|field| {
                (
                    field.name.clone(),
                    schema_mapping_expr_actual_type(context, &field.expr),
                )
            })
            .collect(),
    )
}

fn schema_mapping_expr_actual_type(context: &SchemaMappingExprContext<'_>, expr: &Expr) -> Type {
    match &expr.kind {
        ExprKind::NamePath(segments) => {
            if let [name] = segments.as_slice()
                && let Some(ty) = context.schema_fields.get(name)
            {
                return ty.clone();
            }
            Type::Unknown
        }
        ExprKind::Record(fields) => schema_mapping_record_actual_type(context, fields),
        ExprKind::FieldAccess { base, field, .. } => schema_mapping_expr_actual_type(context, base)
            .record_field(field)
            .cloned()
            .unwrap_or(Type::Unknown),
        ExprKind::Binary { op, left, right }
            if matches!(op, BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply)
                && schema_mapping_expr_actual_type(context, left) == Type::int()
                && schema_mapping_expr_actual_type(context, right) == Type::int() =>
        {
            Type::int()
        }
        _ => Type::Unknown,
    }
}

fn schema_mapping_name_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    segments: &[String],
    expected: &Type,
) -> SchemaMappingExprResult {
    if let [name] = segments {
        if let Some(ty) = context.schema_fields.get(name) {
            return Ok(SchemaMappingTypedExpr {
                ty: ty.clone(),
                expr: SchemaDecodeMappingExpr::Field(name.clone()),
            });
        }
        if let Some(constructor) = schema_mapping_constructor(
            context.registry,
            context.module,
            context.schema,
            segments,
            expected,
        ) && constructor.variant.payload_fields.is_empty()
        {
            return Ok(SchemaMappingTypedExpr {
                ty: expected.clone(),
                expr: SchemaDecodeMappingExpr::Constructor {
                    name: schema_mapping_constructor_name(constructor),
                    args: Vec::new(),
                },
            });
        }
        return Err(Box::new(SchemaMappingExprError::UnknownSchemaField {
            name: name.clone(),
            span: expr.span.clone(),
        }));
    }
    let constructor = schema_mapping_constructor(
        context.registry,
        context.module,
        context.schema,
        segments,
        expected,
    )
    .ok_or_else(|| {
        Box::new(SchemaMappingExprError::UnresolvedConstructor {
            name: segments.join("::"),
            span: expr.span.clone(),
        })
    })?;
    if !constructor.variant.payload_fields.is_empty() {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: segments.join("::"),
            expected: constructor.variant.payload_fields.len(),
            actual: 0,
            span: expr.span.clone(),
        }));
    }
    Ok(SchemaMappingTypedExpr {
        ty: expected.clone(),
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: Vec::new(),
        },
    })
}

fn schema_mapping_name_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    segments: &[String],
) -> SchemaMappingExprResult {
    if let [name] = segments
        && let Some(ty) = context.schema_fields.get(name)
    {
        return Ok(SchemaMappingTypedExpr {
            ty: ty.clone(),
            expr: SchemaDecodeMappingExpr::Field(name.clone()),
        });
    }
    let constructor = schema_mapping_constructor(
        context.registry,
        context.module,
        context.schema,
        segments,
        &Type::Unknown,
    )
    .ok_or_else(|| {
        Box::new(if segments.len() == 1 {
            SchemaMappingExprError::UnknownSchemaField {
                name: segments[0].clone(),
                span: expr.span.clone(),
            }
        } else {
            SchemaMappingExprError::UnresolvedConstructor {
                name: segments.join("::"),
                span: expr.span.clone(),
            }
        })
    })?;
    if !constructor.variant.payload_fields.is_empty() {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: segments.join("::"),
            expected: constructor.variant.payload_fields.len(),
            actual: 0,
            span: expr.span.clone(),
        }));
    }
    Ok(SchemaMappingTypedExpr {
        ty: adt::constructed_type(constructor, &[]),
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: Vec::new(),
        },
    })
}

fn schema_mapping_constructor_expr(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    args: &[Expr],
    expected: &Type,
    constructor: AdtConstructor<'_>,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    let expected_count = constructor.variant.payload_fields.len();
    if args.len() != expected_count {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: schema_mapping_constructor_name(constructor).join("::"),
            expected: expected_count,
            actual: args.len(),
            span: expr.span.clone(),
        }));
    }
    let mut payload_exprs = Vec::new();
    let mut payload_types = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        let payload_ty = adt::payload_type(expected, constructor, index).unwrap_or(Type::Unknown);
        let typed =
            schema_mapping_expr_typed_inner(context, arg, &payload_ty, allow_converter_calls)?;
        payload_types.push(typed.ty);
        payload_exprs.push(typed.expr);
    }
    let ty = if adt::adt_args(expected, constructor.descriptor).is_some() {
        expected.clone()
    } else {
        adt::constructed_type(constructor, &payload_types)
    };
    Ok(SchemaMappingTypedExpr {
        ty,
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: payload_exprs,
        },
    })
}

fn schema_mapping_constructor_expr_inferred(
    context: &SchemaMappingExprContext<'_>,
    expr: &Expr,
    args: &[Expr],
    constructor: AdtConstructor<'_>,
    allow_converter_calls: bool,
) -> SchemaMappingExprResult {
    let expected_count = constructor.variant.payload_fields.len();
    if args.len() != expected_count {
        return Err(Box::new(SchemaMappingExprError::ConstructorArity {
            name: schema_mapping_constructor_name(constructor).join("::"),
            expected: expected_count,
            actual: args.len(),
            span: expr.span.clone(),
        }));
    }
    let expected_ty = adt::constructed_type(constructor, &vec![Type::Unknown; expected_count]);
    let mut payload_exprs = Vec::new();
    let mut payload_types = Vec::new();
    for (index, arg) in args.iter().enumerate() {
        let payload_ty =
            adt::payload_type(&expected_ty, constructor, index).unwrap_or(Type::Unknown);
        let typed =
            schema_mapping_expr_typed_inner(context, arg, &payload_ty, allow_converter_calls)?;
        payload_types.push(typed.ty);
        payload_exprs.push(typed.expr);
    }
    Ok(SchemaMappingTypedExpr {
        ty: adt::constructed_type(constructor, &payload_types),
        expr: SchemaDecodeMappingExpr::Constructor {
            name: schema_mapping_constructor_name(constructor),
            args: payload_exprs,
        },
    })
}

fn schema_mapping_constructor<'a>(
    registry: &'a AdtRegistry,
    module: &SurfaceModule,
    schema: &SchemaDecl,
    segments: &[String],
    expected: &Type,
) -> Option<AdtConstructor<'a>> {
    match registry.constructor(segments, schema.module_name.as_deref(), &module.uses) {
        ConstructorLookup::Found(constructor) => Some(constructor),
        ConstructorLookup::Ambiguous => {
            registry
                .descriptor_for_type(expected)
                .and_then(|descriptor| {
                    registry.constructor_for_descriptor(
                        segments,
                        descriptor,
                        schema.module_name.as_deref(),
                        &module.uses,
                    )
                })
        }
        ConstructorLookup::Missing => None,
    }
}

fn schema_mapping_constructor_name(constructor: AdtConstructor<'_>) -> Vec<String> {
    vec![
        constructor.descriptor.type_name.clone(),
        constructor.variant.name.clone(),
    ]
}

fn schema_mapping_name_can_be_constructor(segments: &[String]) -> bool {
    segments.len() > 1
        || segments
            .last()
            .and_then(|name| name.chars().next())
            .is_some_and(char::is_uppercase)
}

fn schema_mapping_name_can_be_converter(segments: &[String]) -> bool {
    segments
        .last()
        .and_then(|name| name.chars().next())
        .is_some_and(char::is_lowercase)
}

fn schema_mapping_expr_render(expr: &Expr) -> String {
    match &expr.kind {
        ExprKind::Missing => "<missing>".to_string(),
        ExprKind::Hole { name, .. } => format!("_{}", name.as_deref().unwrap_or("")),
        ExprKind::NamePath(segments) => segments.join("::"),
        ExprKind::StringLiteral(value)
        | ExprKind::IntLiteral(value)
        | ExprKind::FloatLiteral(value) => value.clone(),
        ExprKind::BoolLiteral(true) => "true".to_string(),
        ExprKind::BoolLiteral(false) => "false".to_string(),
        ExprKind::Unit => "()".to_string(),
        ExprKind::TypeApply { callee, type_args } => {
            format!(
                "{}<{}>",
                schema_mapping_expr_render(callee),
                type_args.join(", ")
            )
        }
        ExprKind::Call { callee, args } => {
            let args = args
                .iter()
                .map(schema_mapping_expr_render)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}({args})", schema_mapping_expr_render(callee))
        }
        ExprKind::FieldAccess { base, field, .. } => {
            format!("{}.{field}", schema_mapping_expr_render(base))
        }
        ExprKind::Try(inner) => format!("{}?", schema_mapping_expr_render(inner)),
        ExprKind::Record(fields) => {
            let fields = fields
                .iter()
                .map(|field| {
                    format!(
                        "{}: {}",
                        field.name,
                        schema_mapping_expr_render(&field.expr)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {fields} }}")
        }
        ExprKind::Dict(entries) => {
            let entries = entries
                .iter()
                .map(|entry| {
                    format!(
                        "{}: {}",
                        schema_mapping_expr_render(&entry.key),
                        schema_mapping_expr_render(&entry.value)
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {entries} }}")
        }
        ExprKind::List(items) => {
            let items = items
                .iter()
                .map(schema_mapping_expr_render)
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{items}]")
        }
        ExprKind::Match { .. } => "match".to_string(),
        ExprKind::Prefix { op, expr } => match op {
            veln_ast::PrefixOp::Not => format!("not {}", schema_mapping_expr_render(expr)),
            veln_ast::PrefixOp::Negate => format!("-{}", schema_mapping_expr_render(expr)),
        },
        ExprKind::Binary { op, left, right } => {
            format!(
                "{} {} {}",
                schema_mapping_expr_render(left),
                schema_mapping_binary_op_text(*op),
                schema_mapping_expr_render(right)
            )
        }
    }
}

fn schema_mapping_binary_op_text(op: veln_ast::BinaryOp) -> &'static str {
    match op {
        veln_ast::BinaryOp::PipeGreater => "|>",
        veln_ast::BinaryOp::Or => "or",
        veln_ast::BinaryOp::And => "and",
        veln_ast::BinaryOp::Equal => "==",
        veln_ast::BinaryOp::NotEqual => "!=",
        veln_ast::BinaryOp::Less => "<",
        veln_ast::BinaryOp::LessEqual => "<=",
        veln_ast::BinaryOp::Greater => ">",
        veln_ast::BinaryOp::GreaterEqual => ">=",
        veln_ast::BinaryOp::Add => "+",
        veln_ast::BinaryOp::Subtract => "-",
        veln_ast::BinaryOp::Multiply => "*",
        veln_ast::BinaryOp::Divide => "/",
    }
}

pub(crate) fn schema_mapping_target_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    mapping: &SchemaMappingClause,
) -> Option<Vec<(String, Type)>> {
    let target = mapping.target.as_ref()?;
    let target_decl = schema_mapping_target_type(module, schema, target)?;
    if target_decl.params.is_empty() && target_decl.variants.len() == 1 {
        return Some(type_variant_record_fields(&target_decl.variants[0]));
    }
    None
}

fn type_variant_record_fields(variant: &TypeVariantDecl) -> Vec<(String, Type)> {
    variant
        .fields
        .iter()
        .map(|field| (field.name.clone(), parse_type_or_unknown(Some(&field.ty))))
        .collect()
}

fn schema_mapping_target_type<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    target: &str,
) -> Option<&'a TypeDecl> {
    let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => module.types.iter().find(|type_decl| {
            type_decl.name.as_deref() == Some(name.as_str())
                && type_decl.module_name.as_deref() == schema.module_name.as_deref()
        }),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            )?;
            module.types.iter().find(|type_decl| {
                type_decl.name.as_deref() == Some(name.as_str())
                    && type_decl.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
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
    match ty.trim() {
        "UInt1" | "UInt2" | "UInt3" | "UInt4" | "UInt5" | "UInt6" | "UInt7" => Some(1),
        "UInt8" | "Flag8" => Some(1),
        "Flag16be" | "Flag16le" => Some(2),
        "Flag32be" | "Flag32le" => Some(4),
        "Flag64be" | "Flag64le" => Some(8),
        "UInt16be" | "UInt16le" => Some(2),
        "UInt24be" | "UInt24le" => Some(3),
        "UInt31be" | "UInt31le" | "UInt32be" | "UInt32le" => Some(4),
        "UInt64be" | "UInt64le" => Some(8),
        _ => None,
    }
}

pub(crate) fn exact_width_schema_primitive_little_endian(ty: &str) -> bool {
    matches!(
        ty.trim(),
        "UInt16le"
            | "Flag16le"
            | "UInt24le"
            | "UInt31le"
            | "UInt32le"
            | "Flag32le"
            | "UInt64le"
            | "Flag64le"
    )
}

pub(crate) fn flag_schema_primitive(ty: &str) -> Option<&'static str> {
    match ty.trim() {
        "Flag8" => Some("Flag8"),
        "Flag16be" => Some("Flag16be"),
        "Flag16le" => Some("Flag16le"),
        "Flag32be" => Some("Flag32be"),
        "Flag32le" => Some("Flag32le"),
        "Flag64be" => Some("Flag64be"),
        "Flag64le" => Some("Flag64le"),
        _ => None,
    }
}

pub(crate) fn exact_width_schema_primitive_bit_width(ty: &str) -> Option<u8> {
    match ty.trim() {
        "UInt1" => Some(1),
        "UInt2" => Some(2),
        "UInt3" => Some(3),
        "UInt4" => Some(4),
        "UInt5" => Some(5),
        "UInt6" => Some(6),
        "UInt7" => Some(7),
        "UInt8" | "Flag8" => Some(8),
        "Flag16be" | "Flag16le" => Some(16),
        "Flag32be" | "Flag32le" => Some(32),
        "Flag64be" | "Flag64le" => Some(64),
        "UInt16be" | "UInt16le" => Some(16),
        "UInt24be" | "UInt24le" => Some(24),
        "UInt31be" | "UInt31le" => Some(31),
        "UInt32be" | "UInt32le" => Some(32),
        "UInt64be" | "UInt64le" => Some(64),
        _ => None,
    }
}

pub(crate) fn exact_width_schema_primitive_max_value(ty: &str) -> Option<i64> {
    match ty.trim() {
        "UInt1" => Some(0x1),
        "UInt2" => Some(0x3),
        "UInt3" => Some(0x7),
        "UInt4" => Some(0xf),
        "UInt5" => Some(0x1f),
        "UInt6" => Some(0x3f),
        "UInt7" => Some(0x7f),
        "UInt8" | "Flag8" => Some(0xff),
        "Flag16be" | "Flag16le" => Some(0xffff),
        "Flag32be" | "Flag32le" => Some(0xffffffff),
        "Flag64be" | "Flag64le" => Some(i64::MAX),
        "UInt16be" | "UInt16le" => Some(0xffff),
        "UInt24be" | "UInt24le" => Some(0xffffff),
        "UInt31be" | "UInt31le" => Some(0x7fffffff),
        "UInt32be" | "UInt32le" => Some(0xffffffff),
        "UInt64be" | "UInt64le" => Some(i64::MAX),
        _ => None,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ByteViewLengthExpr {
    Field(String),
    Difference { left: String, right: String },
}

impl ByteViewLengthExpr {
    pub(crate) fn references(&self) -> Vec<&str> {
        match self {
            Self::Field(field) => vec![field.as_str()],
            Self::Difference { left, right } => vec![left.as_str(), right.as_str()],
        }
    }

    pub(crate) fn render(&self) -> String {
        match self {
            Self::Field(field) => field.clone(),
            Self::Difference { left, right } => format!("{left} - {right}"),
        }
    }
}

pub(crate) fn schema_length_expression(text: &str) -> Option<ByteViewLengthExpr> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(ByteViewLengthExpr::Field(text.to_string()));
    }
    let (left, right) = text.split_once('-')?;
    let left = left.trim();
    let right = right.trim();
    if is_simple_schema_field_reference(left) && is_simple_schema_field_reference(right) {
        Some(ByteViewLengthExpr::Difference {
            left: left.to_string(),
            right: right.to_string(),
        })
    } else {
        None
    }
}

pub(crate) fn schema_length_expression_references(text: &str) -> Option<Vec<&str>> {
    let text = text.trim();
    if is_simple_schema_field_reference(text) {
        return Some(vec![text]);
    }
    let (left, right) = text.split_once('-')?;
    if right.contains('-') {
        return None;
    }
    let left = left.trim();
    let right = right.trim();
    if is_simple_schema_field_reference(left) && is_simple_schema_field_reference(right) {
        Some(vec![left, right])
    } else {
        None
    }
}

pub(crate) fn byte_view_schema_primitive(ty: &str) -> Option<ByteViewLengthExpr> {
    let text = ty.trim();
    let inner = text.strip_prefix("ByteView(")?.strip_suffix(')')?.trim();
    schema_length_expression(inner)
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
    ByteView {
        length_field: String,
    },
    Schema {
        schema_name: String,
    },
}

pub(crate) fn repeat_schema_primitive(ty: &str) -> Option<SchemaRepeatSpec> {
    let inner = schema_call_inner(ty, "Repeat")?;
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    let [count_field, primitive] = args.as_slice() else {
        return None;
    };
    let count_expr = schema_length_expression(count_field)?;
    let payload = if let Some(width) = exact_width_schema_primitive(primitive) {
        if exact_width_schema_primitive_bit_width(primitive)? < 8
            || flag_schema_primitive(primitive).is_some()
        {
            return None;
        }
        SchemaRepeatPayload::Primitive {
            width,
            max_value: exact_width_schema_primitive_max_value(primitive)?,
            little_endian: exact_width_schema_primitive_little_endian(primitive),
        }
    } else if let Some(length_expr) = byte_view_schema_primitive(primitive) {
        match length_expr {
            ByteViewLengthExpr::Field(length_field) => {
                SchemaRepeatPayload::ByteView { length_field }
            }
            ByteViewLengthExpr::Difference { .. } => return None,
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

fn is_simple_schema_field_reference(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

pub(crate) fn reserved_bits_schema_primitive(ty: &str) -> Option<(i64, i64)> {
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

pub(crate) fn supported_encode_reserved_bits(
    fields: &[veln_ast::SchemaField],
    index: usize,
    reserved: (i64, i64),
) -> Option<(u8, i64)> {
    let (bit_width, expected_value) = reserved;
    if supported_bit_packed_reserved_group(fields, index) {
        return Some((bit_width as u8, expected_value));
    }
    let previous_previous_field = index
        .checked_sub(2)
        .and_then(|previous| fields.get(previous));
    let previous_field = index
        .checked_sub(1)
        .and_then(|previous| fields.get(previous));
    let next_field = fields.get(index + 1);
    let next_next_field = fields.get(index + 2);
    if bit_width == 1
        && expected_value == 0
        && next_field.is_some_and(|field| field.ty.trim() == "UInt31be")
    {
        return Some((1, 0));
    }
    let packed_storage_bit_width = if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    };
    if packed_storage_bit_width.is_some_and(|storage_bit_width| {
        next_field
            .and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
            .is_some_and(|next_bit_width| {
                i64::from(next_bit_width) + bit_width == storage_bit_width
            })
    }) {
        let max_value = (1_i64 << bit_width) - 1;
        if expected_value <= max_value {
            return Some((bit_width as u8, expected_value));
        }
    }
    if let (Some(next_field), Some(next_next_field)) = (next_field, next_next_field)
        && supported_prefix_reserved_group(next_field, next_next_field, bit_width, expected_value)
    {
        return Some((bit_width as u8, expected_value));
    }
    if let Some(packed_storage_bit_width) = packed_reserved_storage_bit_width(bit_width)
        && !previous_previous_field.is_some_and(|field| {
            previous_field.is_some_and(|visible| supported_packed_reserved_prefix(field, visible))
        })
        && previous_field
            .and_then(|field| exact_width_schema_primitive_bit_width(&field.ty))
            .is_some_and(|previous_bit_width| {
                i64::from(previous_bit_width) + bit_width == packed_storage_bit_width
            })
    {
        let max_value = (1_i64 << bit_width) - 1;
        if expected_value <= max_value {
            return Some((bit_width as u8, expected_value));
        }
    }
    if let (Some(previous_field), Some(next_field)) = (previous_field, next_field)
        && supported_middle_reserved_bits(previous_field, next_field, bit_width, expected_value)
    {
        return Some((bit_width as u8, expected_value));
    }
    if bit_width <= 0 || bit_width > 32 || bit_width % 8 != 0 {
        return None;
    }
    let max_value = if bit_width == 32 {
        0xffffffff
    } else {
        (1_i64 << bit_width) - 1
    };
    if expected_value <= max_value {
        return Some((bit_width as u8, expected_value));
    }
    None
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
            if matches!(total_bit_width, 8 | 16 | 24 | 32) {
                let end = start + offset;
                if has_reserved && has_visible && start <= index && index <= end {
                    return true;
                }
                break;
            }
            if total_bit_width > 32 {
                break;
            }
        }
    }
    false
}

fn bit_packed_group_field_width(field: &veln_ast::SchemaField) -> Option<i64> {
    if let Some((bit_width, expected_value)) = reserved_bits_schema_primitive(&field.ty) {
        if bit_width <= 0 || bit_width > 32 || bit_width % 8 == 0 {
            return None;
        }
        let max_value = if bit_width == 32 {
            0xffffffff
        } else {
            (1_i64 << bit_width) - 1
        };
        return (expected_value <= max_value).then_some(bit_width);
    }
    if exact_width_schema_primitive_little_endian(&field.ty)
        || flag_schema_primitive(&field.ty).is_some()
    {
        return None;
    }
    let bit_width = i64::from(exact_width_schema_primitive_bit_width(&field.ty)?);
    (bit_width % 8 != 0).then_some(bit_width)
}

fn supported_prefix_reserved_group(
    first_visible_field: &veln_ast::SchemaField,
    second_visible_field: &veln_ast::SchemaField,
    bit_width: i64,
    expected_value: i64,
) -> bool {
    if bit_width <= 0 || bit_width > 32 {
        return false;
    }
    if exact_width_schema_primitive_little_endian(&first_visible_field.ty)
        || exact_width_schema_primitive_little_endian(&second_visible_field.ty)
        || flag_schema_primitive(&first_visible_field.ty).is_some()
        || flag_schema_primitive(&second_visible_field.ty).is_some()
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
    let total_bit_width = bit_width + i64::from(first_bit_width) + i64::from(second_bit_width);
    bit_width % 8 != 0
        && (bit_width + i64::from(first_bit_width)) % 8 != 0
        && total_bit_width == 8
        && expected_value < (1_i64 << bit_width)
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
        || flag_schema_primitive(&previous_field.ty).is_some()
        || flag_schema_primitive(&next_field.ty).is_some()
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

fn parse_reserved_bits_integer(text: &str) -> Option<i64> {
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
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
    Schema { schema_name: String },
}

pub(crate) fn closed_dispatch_schema_primitive(ty: &str) -> Option<SchemaDispatchSpec> {
    let inner = schema_call_inner(ty, "Dispatch")?;
    let mut args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .peekable();
    let tag_field = args.next()?.to_string();
    if !is_schema_identifier(&tag_field) {
        return None;
    }
    let length_field = args
        .peek()
        .filter(|arg| !arg.contains("=>"))
        .map(|arg| (*arg).to_string());
    if length_field
        .as_deref()
        .is_some_and(|length_field| !is_schema_identifier(length_field))
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
    let mut args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty());
    let tag_field = args.next()?.to_string();
    let length_field = args.next()?.to_string();
    if !is_schema_identifier(&tag_field) || !is_schema_identifier(&length_field) {
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
    if text.is_empty() || !text.chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
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
        Type::function(
            self.params.clone(),
            self.return_type.clone(),
            self.effects.clone(),
        )
    }
}

fn function_alias_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<FunctionSignature> {
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
            )?;
            Some(FunctionSignature {
                name,
                target_name: target.target_name.clone(),
                module_name: alias.module_name.clone(),
                visibility: Visibility::Public,
                params: target.params.clone(),
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
                    && imported_function_is_visible(function, use_decl)
            })
        }
        _ => None,
    }
}

fn infer_function_body_effects(module: &SurfaceModule, functions: &mut [FunctionSignature]) {
    let mut effects_by_name = functions
        .iter()
        .map(|function| (function.name.clone(), function.effects.clone()))
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

    let mut changed = true;
    while changed {
        changed = false;
        for function in module
            .functions
            .iter()
            .filter(|function| function.kind == FunctionKind::Function)
        {
            let Some(name) = &function.name else {
                continue;
            };
            let mut bindings = function
                .params
                .iter()
                .map(|param| Binding {
                    name: param.name.clone(),
                    ty: parse_type_or_unknown(param.ty.as_deref()),
                })
                .collect::<Vec<_>>();
            let mut inferred = effects_by_name.get(name).cloned().unwrap_or_default();
            for line in &function.body {
                match &line.kind {
                    BodyLineKind::Let {
                        pattern,
                        annotation,
                        expr,
                    } => {
                        collect_expr_effects(
                            expr,
                            &module.uses,
                            function.module_name.as_deref(),
                            &bindings,
                            &effects_by_name,
                            &effects_by_module_path,
                            &mut inferred,
                        );
                        let ty = parse_type_or_unknown(annotation.as_deref());
                        collect_pattern_bindings(pattern, &ty, &mut bindings);
                    }
                    BodyLineKind::Expr { expr } => {
                        collect_expr_effects(
                            expr,
                            &module.uses,
                            function.module_name.as_deref(),
                            &bindings,
                            &effects_by_name,
                            &effects_by_module_path,
                            &mut inferred,
                        );
                    }
                }
            }
            if effects_by_name.get(name) != Some(&inferred) {
                effects_by_name.insert(name.clone(), inferred);
                if let Some(module_name) = &function.module_name {
                    effects_by_module_path.insert(
                        (module_name.clone(), name.clone()),
                        (effects_by_name[name].clone(), function.visibility),
                    );
                }
                changed = true;
            }
        }
    }

    for function in functions {
        if let Some(effects) = effects_by_name.remove(&function.name) {
            function.effects = effects;
        }
    }
}

fn collect_pattern_bindings(pattern: &Pattern, ty: &Type, bindings: &mut Vec<Binding>) {
    match &pattern.kind {
        PatternKind::Binding(name) => bindings.push(Binding {
            name: name.clone(),
            ty: ty.clone(),
        }),
        PatternKind::Record(fields) => {
            for field in fields {
                let field_ty = ty.record_field(&field.name).unwrap_or(&Type::Unknown);
                collect_pattern_bindings(&field.pattern, field_ty, bindings);
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

fn collect_expr_effects(
    expr: &Expr,
    uses: &[UseDecl],
    current_module: Option<&str>,
    bindings: &[Binding],
    effects_by_name: &BTreeMap<String, Vec<String>>,
    effects_by_module_path: &BTreeMap<(String, String), (Vec<String>, Visibility)>,
    inferred: &mut Vec<String>,
) {
    match &expr.kind {
        ExprKind::Call { callee, args } => {
            if let Some(segments) = callee_name_path(callee) {
                if is_stdio_call(segments) {
                    push_unique_effect(inferred, "stdio");
                } else if is_concurrency_call(segments) {
                    push_unique_effect(inferred, "concurrency");
                } else if let Some(effects) = standard_library_effects(segments) {
                    for effect in effects {
                        push_unique_effect(inferred, effect);
                    }
                } else {
                    for effect in effects_for_callee_path(
                        segments,
                        uses,
                        current_module,
                        bindings,
                        effects_by_name,
                        effects_by_module_path,
                    ) {
                        push_unique_effect(inferred, effect);
                    }
                }
            } else {
                collect_expr_effects(
                    callee,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
            for arg in args {
                collect_expr_effects(
                    arg,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::FieldAccess { base, .. }
        | ExprKind::Try(base)
        | ExprKind::TypeApply { callee: base, .. }
        | ExprKind::Prefix { expr: base, .. } => {
            collect_expr_effects(
                base,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
        }
        ExprKind::Record(fields) => {
            for field in fields {
                collect_expr_effects(
                    &field.expr,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::Dict(entries) => {
            for entry in entries {
                collect_expr_effects(
                    &entry.key,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
                collect_expr_effects(
                    &entry.value,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::List(items) => {
            for item in items {
                collect_expr_effects(
                    item,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_effects(
                scrutinee,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            for arm in arms {
                collect_expr_effects(
                    &arm.expr,
                    uses,
                    current_module,
                    bindings,
                    effects_by_name,
                    effects_by_module_path,
                    inferred,
                );
            }
        }
        ExprKind::Binary { left, right, .. } => {
            collect_expr_effects(
                left,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
            collect_expr_effects(
                right,
                uses,
                current_module,
                bindings,
                effects_by_name,
                effects_by_module_path,
                inferred,
            );
        }
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

fn callee_name_path(callee: &Expr) -> Option<&Vec<String>> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments),
        ExprKind::TypeApply { callee, .. } => callee_name_path(callee),
        _ => None,
    }
}

fn effects_for_callee_path<'a>(
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
    bindings: &'a [Binding],
    effects_by_name: &'a BTreeMap<String, Vec<String>>,
    effects_by_module_path: &'a BTreeMap<(String, String), (Vec<String>, Visibility)>,
) -> &'a [String] {
    match segments {
        [name] => effects_for_bare_callee(name, bindings, effects_by_name),
        [_, .., name] => {
            let Some(use_decl) =
                imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            else {
                return &[];
            };
            effects_by_module_path
                .get(&(use_decl.name.clone(), name.clone()))
                .filter(|(_, visibility)| {
                    use_decl.package.is_none() || *visibility == Visibility::Public
                })
                .map_or(&[], |(effects, _)| effects.as_slice())
        }
        _ => &[],
    }
}

fn imported_use_for_path<'a>(
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

fn imported_function_is_visible(function: &FunctionSignature, use_decl: &UseDecl) -> bool {
    use_decl.package.is_none() || function.visibility == Visibility::Public
}

fn imported_module_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a str> {
    imported_use_for_path(uses, segments, current_module).map(|use_decl| use_decl.name.as_str())
}

fn effects_for_bare_callee<'a>(
    name: &str,
    bindings: &'a [Binding],
    effects_by_name: &'a BTreeMap<String, Vec<String>>,
) -> &'a [String] {
    if let Some(binding) = bindings.iter().rev().find(|binding| binding.name == name)
        && let Some(effects) = binding.ty.function_effects()
    {
        return effects;
    }
    effects_by_name.get(name).map_or(&[], Vec::as_slice)
}

fn push_unique_effect(effects: &mut Vec<String>, effect: &str) {
    if !effects.iter().any(|existing| existing == effect) {
        effects.push(effect.to_string());
    }
}

pub(crate) fn is_assignable(expected: &Type, actual: &Type) -> bool {
    if expected == &Type::Unknown || actual == &Type::Unknown || expected == actual {
        return true;
    }
    match (expected, actual) {
        (Type::Record(expected_fields), Type::Record(actual_fields)) => {
            expected_fields.iter().all(|(expected_name, expected_ty)| {
                actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == expected_name)
                    .is_some_and(|(_, actual_ty)| is_assignable(expected_ty, actual_ty))
            })
        }
        (
            Type::Named {
                name: expected_name,
                args: expected_args,
            },
            Type::Named {
                name: actual_name,
                args: actual_args,
            },
        ) => {
            expected_name == actual_name
                && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| is_assignable(expected, actual))
        }
        (
            Type::Function {
                params: expected_params,
                return_type: expected_return,
                effects: expected_effects,
            },
            Type::Function {
                params: actual_params,
                return_type: actual_return,
                effects: actual_effects,
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| is_assignable(expected, actual))
                && is_assignable(expected_return, actual_return)
                && effects_are_assignable(expected_effects, actual_effects)
        }
        _ => false,
    }
}

fn effects_are_assignable(expected: &[String], actual: &[String]) -> bool {
    actual
        .iter()
        .all(|effect| expected.iter().any(|expected| expected == effect))
}

pub(crate) fn parse_type_or_unknown(text: Option<&str>) -> Type {
    text.and_then(|text| parse_type_annotation(text).ok())
        .unwrap_or(Type::Unknown)
}

pub(crate) fn core_type(ty: &Type) -> CoreType {
    match ty {
        Type::Unknown => CoreType::Unknown,
        Type::Named { name, args } => {
            CoreType::named(name.clone(), args.iter().map(core_type).collect())
        }
        Type::Record(fields) => CoreType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), core_type(ty)))
                .collect(),
        ),
        Type::Function {
            params,
            return_type,
            effects,
        } => CoreType::Function {
            params: params.iter().map(core_type).collect(),
            return_type: Box::new(core_type(return_type)),
            effects: effects.clone(),
        },
    }
}

pub(crate) fn parse_type_annotation(text: &str) -> Result<Type, String> {
    let mut parser = TypeParser::new(text);
    let ty = parser.parse_type()?;
    parser.skip_ws();
    if parser.at_end() {
        Ok(ty)
    } else {
        Err(format!("unexpected `{}`", &parser.text[parser.cursor..]))
    }
}

struct TypeParser<'a> {
    text: &'a str,
    cursor: usize,
}

impl<'a> TypeParser<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, cursor: 0 }
    }

    fn parse_type(&mut self) -> Result<Type, String> {
        self.skip_ws();
        if self.eat('{') {
            return self.parse_record_type();
        }
        if self.eat('(') {
            self.skip_ws();
            if self.eat(')') {
                return Ok(Type::unit());
            }
            return Err("expected `)` for unit type `()`".to_string());
        }
        if self.eat_keyword("fn") {
            return self.parse_function_type();
        }

        let Some(name) = self.parse_ident() else {
            return Err("expected type".to_string());
        };
        self.skip_ws();
        let args = if self.eat('<') {
            let args = self.parse_type_list('>')?;
            self.expect('>')?;
            args
        } else if self.at('(') {
            return Err(format!("unexpected `{}`", &self.text[self.cursor..]));
        } else {
            Vec::new()
        };
        self.validate_named_type(name, args)
    }

    fn parse_record_type(&mut self) -> Result<Type, String> {
        let mut fields = Vec::new();
        while !self.at_end() && !self.at('}') {
            let Some(name) = self.parse_ident() else {
                return Err("expected record field name".to_string());
            };
            if fields
                .iter()
                .any(|(field_name, _): &(String, Type)| field_name == &name)
            {
                return Err(format!("duplicate record field `{name}`"));
            }
            self.expect(':')?;
            let ty = self.parse_type()?;
            fields.push((name, ty));
            self.skip_ws();
            if !self.eat(',') {
                break;
            }
            self.skip_ws();
            if self.at('}') {
                break;
            }
        }
        self.expect('}')?;
        Ok(Type::Record(fields))
    }

    fn parse_function_type(&mut self) -> Result<Type, String> {
        self.expect('(')?;
        let params = self.parse_type_list(')')?;
        self.expect(')')?;
        self.skip_ws();
        if !self.eat_str("->") {
            return Err("expected `->` in function type".to_string());
        }
        let return_type = self.parse_type()?;
        let effects = if self.eat_keyword("effects") {
            self.expect('[')?;
            let mut effects = Vec::new();
            while !self.at_end() && !self.at(']') {
                let Some(effect) = self.parse_ident() else {
                    return Err("expected effect name".to_string());
                };
                effects.push(effect);
                self.skip_ws();
                if !self.eat(',') {
                    break;
                }
            }
            self.expect(']')?;
            effects
        } else {
            Vec::new()
        };
        Ok(Type::Function {
            params,
            return_type: Box::new(return_type),
            effects,
        })
    }

    fn parse_type_list(&mut self, end: char) -> Result<Vec<Type>, String> {
        let mut args = Vec::new();
        self.skip_ws();
        while !self.at_end() && !self.at(end) {
            args.push(self.parse_type()?);
            self.skip_ws();
            if !self.eat(',') {
                break;
            }
            self.skip_ws();
            if self.at(end) {
                break;
            }
        }
        Ok(args)
    }

    fn validate_named_type(&self, name: String, args: Vec<Type>) -> Result<Type, String> {
        let expected_arity = match name.as_str() {
            "Bool" | "Int" | "Float" | "String" | "Unit" => Some(0),
            "Option" | "Vec" => Some(1),
            "Result" | "Dict" => Some(2),
            _ => None,
        };
        if let Some(expected) = expected_arity
            && args.len() != expected
        {
            return Err(format!(
                "`{name}` expects {expected} type argument(s), found {}",
                args.len()
            ));
        }
        if name == "Dict" && args.len() == 2 {
            Ok(Type::dict(args[0].clone(), args[1].clone()))
        } else {
            Ok(Type::named(name, args))
        }
    }

    fn parse_ident(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.cursor;
        while let Some(ch) = self.current() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.cursor += ch.len_utf8();
            } else {
                break;
            }
        }
        while self.text[self.cursor..].starts_with("::") {
            self.cursor += 2;
            let segment_start = self.cursor;
            while let Some(ch) = self.current() {
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    self.cursor += ch.len_utf8();
                } else {
                    break;
                }
            }
            if self.cursor == segment_start {
                self.cursor = start;
                return None;
            }
        }
        (self.cursor > start).then(|| self.text[start..self.cursor].to_string())
    }

    fn skip_ws(&mut self) {
        while self.current().is_some_and(char::is_whitespace) {
            self.cursor += 1;
        }
    }

    fn eat(&mut self, expected: char) -> bool {
        self.skip_ws();
        if self.at(expected) {
            self.cursor += expected.len_utf8();
            true
        } else {
            false
        }
    }

    fn at(&self, expected: char) -> bool {
        self.current() == Some(expected)
    }

    fn expect(&mut self, expected: char) -> Result<(), String> {
        if self.eat(expected) {
            Ok(())
        } else {
            Err(format!("expected `{expected}`"))
        }
    }

    fn eat_keyword(&mut self, keyword: &str) -> bool {
        self.skip_ws();
        if self.text[self.cursor..].starts_with(keyword)
            && self.text[self.cursor + keyword.len()..]
                .chars()
                .next()
                .is_none_or(|ch| !ch.is_ascii_alphanumeric() && ch != '_')
        {
            self.cursor += keyword.len();
            true
        } else {
            false
        }
    }

    fn eat_str(&mut self, expected: &str) -> bool {
        self.skip_ws();
        if self.text[self.cursor..].starts_with(expected) {
            self.cursor += expected.len();
            true
        } else {
            false
        }
    }

    fn at_end(&self) -> bool {
        self.cursor >= self.text.len()
    }

    fn current(&self) -> Option<char> {
        self.text[self.cursor..].chars().next()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_tuple_spelling_as_unit_type() {
        assert_eq!(parse_type_annotation("()"), Ok(Type::unit()));
        assert_eq!(
            parse_type_annotation("Result<(), AppError>"),
            Ok(Type::result(
                Type::unit(),
                Type::named("AppError", Vec::new())
            ))
        );
    }

    #[test]
    fn renders_unit_type_with_empty_tuple_spelling() {
        assert_eq!(Type::unit().render(), "()");
        assert_eq!(
            Type::result(Type::unit(), Type::named("AppError", Vec::new())).render(),
            "Result<(), AppError>"
        );
    }

    #[test]
    fn keeps_unit_name_as_compatibility_alias() {
        assert_eq!(parse_type_annotation("Unit"), Ok(Type::unit()));
    }

    #[test]
    fn renders_record_and_function_types() {
        let record = Type::Record(vec![
            ("name".to_string(), Type::string()),
            ("scores".to_string(), Type::vec(Type::int())),
        ]);
        let pure_function = Type::Function {
            params: vec![Type::int(), Type::float()],
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let effectful_function = Type::Function {
            params: vec![record.clone()],
            return_type: Box::new(Type::result(
                Type::unit(),
                Type::named("AppError", Vec::new()),
            )),
            effects: vec!["stdio".to_string(), "net".to_string()],
        };

        assert_eq!(record.render(), "{name: String, scores: Vec<Int>}");
        assert_eq!(pure_function.render(), "fn(Int, Float) -> Bool");
        assert_eq!(
            effectful_function.render(),
            "fn({name: String, scores: Vec<Int>}) -> Result<(), AppError> effects [stdio, net]"
        );
    }

    #[test]
    fn exposes_type_parts_and_core_type_shape() {
        let function = Type::Function {
            params: vec![Type::vec(Type::int())],
            return_type: Box::new(Type::Record(vec![("ok".to_string(), Type::bool())])),
            effects: vec!["stdio".to_string()],
        };

        let (params, return_type) = function
            .function_parts()
            .expect("function type should expose parts");
        assert_eq!(params, &[Type::vec(Type::int())]);
        assert_eq!(
            return_type,
            &Type::Record(vec![("ok".to_string(), Type::bool())])
        );
        assert!(Type::string().function_parts().is_none());
        assert_eq!(
            core_type(&function),
            CoreType::Function {
                params: vec![CoreType::vec(CoreType::int())],
                return_type: Box::new(CoreType::Record(vec![("ok".to_string(), CoreType::bool())])),
                effects: vec!["stdio".to_string()],
            }
        );
    }

    #[test]
    fn assignability_allows_unknowns_record_width_and_function_shapes() {
        let expected_record = Type::Record(vec![
            ("name".to_string(), Type::string()),
            (
                "meta".to_string(),
                Type::Record(vec![("count".to_string(), Type::int())]),
            ),
        ]);
        let actual_record = Type::Record(vec![
            ("name".to_string(), Type::string()),
            ("extra".to_string(), Type::bool()),
            (
                "meta".to_string(),
                Type::Record(vec![("count".to_string(), Type::int())]),
            ),
        ]);
        let wrong_record = Type::Record(vec![("name".to_string(), Type::int())]);
        let expected_pure_function = Type::Function {
            params: vec![Type::int()],
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let actual_effectful_function = Type::Function {
            params: vec![Type::int()],
            return_type: Box::new(Type::bool()),
            effects: vec!["stdio".to_string()],
        };
        let expected_effectful_function = Type::Function {
            params: vec![Type::int()],
            return_type: Box::new(Type::bool()),
            effects: vec!["stdio".to_string()],
        };
        let actual_pure_function = Type::Function {
            params: vec![Type::int()],
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };
        let wrong_function = Type::Function {
            params: vec![Type::int(), Type::int()],
            return_type: Box::new(Type::bool()),
            effects: Vec::new(),
        };

        assert!(is_assignable(&Type::Unknown, &Type::string()));
        assert!(is_assignable(&Type::string(), &Type::Unknown));
        assert!(is_assignable(&expected_record, &actual_record));
        assert!(!is_assignable(&expected_record, &wrong_record));
        assert!(!is_assignable(
            &Type::named("Path", Vec::new()),
            &Type::string()
        ));
        assert!(!is_assignable(
            &Type::string(),
            &Type::named("Path", Vec::new())
        ));
        assert!(is_assignable(
            &expected_effectful_function,
            &actual_pure_function
        ));
        assert!(!is_assignable(
            &expected_pure_function,
            &actual_effectful_function
        ));
        assert!(!is_assignable(&expected_pure_function, &wrong_function));
        assert!(!is_assignable(&Type::int(), &Type::float()));
    }

    #[test]
    fn parses_nested_type_annotations_with_whitespace() {
        assert_eq!(
            parse_type_annotation(
                " fn ( Vec< Int > , platform::Request ) -> Result < Dict < String , Int > , AppError > effects [ stdio , net ] "
            ),
            Ok(Type::Function {
                params: vec![
                    Type::vec(Type::int()),
                    Type::named("platform::Request", Vec::new()),
                ],
                return_type: Box::new(Type::result(
                    Type::dict(Type::string(), Type::int()),
                    Type::named("AppError", Vec::new())
                )),
                effects: vec!["stdio".to_string(), "net".to_string()],
            })
        );
        assert_eq!(
            parse_type_annotation("{ name: String, scores: Vec<Int> }"),
            Ok(Type::Record(vec![
                ("name".to_string(), Type::string()),
                ("scores".to_string(), Type::vec(Type::int())),
            ]))
        );
        assert_eq!(
            parse_type_annotation("{ name: String, scores: Vec<Int>, }"),
            Ok(Type::Record(vec![
                ("name".to_string(), Type::string()),
                ("scores".to_string(), Type::vec(Type::int())),
            ]))
        );
    }

    #[test]
    fn parses_angle_bracket_type_annotations() {
        assert_eq!(
            parse_type_annotation(
                "fn(Vec<Int>, domain::Envelope<String, Result<(), AppError>>) -> Dict<String, Int>"
            ),
            Ok(Type::Function {
                params: vec![
                    Type::vec(Type::int()),
                    Type::named(
                        "domain::Envelope",
                        vec![
                            Type::string(),
                            Type::result(Type::unit(), Type::named("AppError", Vec::new())),
                        ],
                    ),
                ],
                return_type: Box::new(Type::dict(Type::string(), Type::int())),
                effects: Vec::new(),
            })
        );
    }

    #[test]
    fn rejects_malformed_type_annotations_with_specific_errors() {
        let cases = [
            ("", "expected type"),
            ("(Int)", "expected `)` for unit type `()`"),
            ("Int trailing", "unexpected `trailing`"),
            ("{ : Int }", "expected record field name"),
            ("{ name: String,, }", "expected record field name"),
            (
                "{ value: Int, value: String }",
                "duplicate record field `value`",
            ),
            ("{ value Int }", "expected `:`"),
            ("fn(Int) Int", "expected `->` in function type"),
            ("fn(Int -> Int", "expected `)`"),
            ("fn() -> () effects [,]", "expected effect name"),
            ("fn() -> () effects [stdio", "expected `]`"),
            ("Result(Int, String)", "unexpected `(Int, String)`"),
            ("Vec", "`Vec` expects 1 type argument(s), found 0"),
            ("Dict<String>", "`Dict` expects 2 type argument(s), found 1"),
            ("std::", "expected type"),
        ];

        for (text, message) in cases {
            assert_eq!(parse_type_annotation(text), Err(message.to_string()));
        }
        assert_eq!(parse_type_or_unknown(Some("Vec")), Type::Unknown);
        assert_eq!(parse_type_or_unknown(None), Type::Unknown);
    }

    #[test]
    fn expected_type_sources_render_for_diagnostics_and_holes() {
        let cases = [
            (
                ExpectedTypeSource::DeclaredReturn,
                "declared_return",
                "declared",
            ),
            (
                ExpectedTypeSource::DeclaredParameter,
                "declared_parameter",
                "declared",
            ),
            (
                ExpectedTypeSource::LocalAnnotation,
                "local_annotation",
                "declared",
            ),
            (
                ExpectedTypeSource::Inferred,
                "inferred_expression",
                "inferred",
            ),
            (ExpectedTypeSource::Unknown, "unknown", "unknown"),
        ];

        for (source, type_source, hole_source) in cases {
            assert_eq!(source.as_type_source(), type_source);
            assert_eq!(source.as_hole_source(), hole_source);
        }
    }
}
