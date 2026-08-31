use veln_source::SourceSpan;
use veln_syntax::{
    BinaryOp as SyntaxBinaryOp, BodyLine as SyntaxBodyLine, CodecDecl as SyntaxCodecDecl,
    CodecDirection as SyntaxCodecDirection,
    CodecImplementationKind as SyntaxCodecImplementationKind, ContractKind as SyntaxContractKind,
    DictEntry as SyntaxDictEntry, EffectDecl as SyntaxEffectDecl,
    EffectOperationDecl as SyntaxEffectOperationDecl, Expr as SyntaxExpr,
    ExprKind as SyntaxExprKind, FunctionDecl as SyntaxFunction, HandlerDecl as SyntaxHandlerDecl,
    HandlerOperationClauseDecl as SyntaxHandlerOperationClauseDecl, IfBranch as SyntaxIfBranch,
    MatchArm as SyntaxMatchArm, ModuleDecl as SyntaxModule, Param as SyntaxParam,
    Pattern as SyntaxPattern, PatternKind as SyntaxPatternKind, PrefixOp as SyntaxPrefixOp,
    PublicAliasDecl as SyntaxPublicAlias, PublicAliasKind as SyntaxPublicAliasKind,
    RecordField as SyntaxRecordField, SchemaDecl as SyntaxSchemaDecl, SyntaxItem, SyntaxTree,
    TypeDecl as SyntaxTypeDecl, UseDecl as SyntaxUse, Visibility as SyntaxVisibility,
};

use crate::{
    BinaryOp, BodyLine, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationClause,
    CodecImplementationKind, Contract, ContractKind, DictEntry, EffectDecl, EffectOperationDecl,
    Expr, ExprKind, Function, FunctionKind, HandlerDecl, HandlerOperationClauseDecl, InvalidName,
    MatchArm, ModuleHeader, NameClass, NameOccurrence, NodeId, Param, Pattern, PatternField,
    PatternKind, PrefixOp, PublicAlias, PublicAliasKind, RecordField, ResultBinding, SchemaDecl,
    SchemaField, SchemaFieldWhereClause, SchemaFormatClause, SchemaValidationClause, SurfaceModule,
    TypeDecl, TypePathSegments, TypeVariantDecl, TypeVariantField, UseDecl, UseOrigin, Visibility,
};

mod expressions;
mod invalid_names;

use invalid_names::{
    collect_invalid_alias_name, collect_invalid_effect_names, collect_invalid_function_names,
    collect_invalid_handler_names, collect_invalid_schema_names, collect_invalid_type_names,
    collect_invalid_use_name,
};

pub fn lower_surface_ast(tree: &SyntaxTree) -> SurfaceModule {
    let mut builder = AstBuilder { next_node_id: 1 };
    let module = tree
        .module
        .as_ref()
        .map(|module| builder.lower_module_header(module));
    let module_name = module.as_ref().map(|module| module.name.clone());
    builder.lower_surface_ast_with_module(tree, module, module_name)
}

pub fn lower_surface_ast_with_module_identity(
    tree: &SyntaxTree,
    name: String,
    span: SourceSpan,
) -> SurfaceModule {
    let mut builder = AstBuilder { next_node_id: 1 };
    let mut module = builder.lower_surface_ast_with_module(tree, None, Some(name.clone()));
    module.module = Some(ModuleHeader {
        node_id: builder.alloc(),
        name,
        span,
    });
    module
}

impl AstBuilder {
    fn lower_surface_ast_with_module(
        &mut self,
        tree: &SyntaxTree,
        module: Option<ModuleHeader>,
        module_name: Option<String>,
    ) -> SurfaceModule {
        let uses = tree
            .uses
            .iter()
            .map(|use_decl| self.lower_use_decl(use_decl, module_name.clone()))
            .collect();
        let mut types = Vec::new();
        let mut effects = Vec::new();
        let mut handlers = Vec::new();
        let mut schemas = Vec::new();
        let mut codecs = Vec::new();
        let mut functions = Vec::new();
        let mut aliases = Vec::new();
        let mut invalid_names = Vec::new();

        for item in &tree.items {
            match item {
                SyntaxItem::Function(function) => {
                    collect_invalid_function_names(function, &mut invalid_names);
                    functions.push(self.lower_function(function, module_name.clone()));
                }
                SyntaxItem::Effect(effect) => {
                    collect_invalid_effect_names(effect, &mut invalid_names);
                    effects.push(self.lower_effect_decl(effect, module_name.clone()));
                }
                SyntaxItem::Handler(handler) => {
                    collect_invalid_handler_names(handler, &mut invalid_names);
                    handlers.push(self.lower_handler_decl(handler, module_name.clone()));
                }
                SyntaxItem::Type(type_decl) => {
                    collect_invalid_type_names(type_decl, &mut invalid_names);
                    types.push(self.lower_type_decl(type_decl, module_name.clone()));
                }
                SyntaxItem::Schema(schema) => {
                    collect_invalid_schema_names(schema, &mut invalid_names);
                    schemas.push(self.lower_schema_decl(schema, module_name.clone()));
                }
                SyntaxItem::Codec(codec) => {
                    codecs.push(self.lower_codec_decl(codec, module_name.clone()));
                }
                SyntaxItem::PublicAlias(alias) => {
                    collect_invalid_alias_name(alias, &mut invalid_names);
                    aliases.push(self.lower_public_alias(alias, module_name.clone()));
                }
            }
        }
        for use_decl in &tree.uses {
            collect_invalid_use_name(use_decl, &mut invalid_names);
        }

        SurfaceModule {
            module,
            uses,
            aliases,
            effects,
            handlers,
            types,
            schemas,
            codecs,
            functions,
            invalid_names,
        }
    }
}

fn lower_prefix_op(op: SyntaxPrefixOp) -> PrefixOp {
    match op {
        SyntaxPrefixOp::Not => PrefixOp::Not,
        SyntaxPrefixOp::Negate => PrefixOp::Negate,
        SyntaxPrefixOp::BitwiseNot => PrefixOp::BitwiseNot,
    }
}

fn import_alias(name: &str) -> String {
    name.rsplit("::")
        .next()
        .unwrap_or(name)
        .rsplit('.')
        .next()
        .unwrap_or(name)
        .to_string()
}

fn lower_binary_op(op: SyntaxBinaryOp) -> BinaryOp {
    match op {
        SyntaxBinaryOp::PipeGreater => BinaryOp::PipeGreater,
        SyntaxBinaryOp::Or => BinaryOp::Or,
        SyntaxBinaryOp::And => BinaryOp::And,
        SyntaxBinaryOp::BitwiseOr => BinaryOp::BitwiseOr,
        SyntaxBinaryOp::BitwiseXor => BinaryOp::BitwiseXor,
        SyntaxBinaryOp::BitwiseAnd => BinaryOp::BitwiseAnd,
        SyntaxBinaryOp::Equal => BinaryOp::Equal,
        SyntaxBinaryOp::NotEqual => BinaryOp::NotEqual,
        SyntaxBinaryOp::Less => BinaryOp::Less,
        SyntaxBinaryOp::LessEqual => BinaryOp::LessEqual,
        SyntaxBinaryOp::Greater => BinaryOp::Greater,
        SyntaxBinaryOp::GreaterEqual => BinaryOp::GreaterEqual,
        SyntaxBinaryOp::ShiftLeft => BinaryOp::ShiftLeft,
        SyntaxBinaryOp::ShiftRight => BinaryOp::ShiftRight,
        SyntaxBinaryOp::ShiftRightLogical => BinaryOp::ShiftRightLogical,
        SyntaxBinaryOp::Add => BinaryOp::Add,
        SyntaxBinaryOp::Subtract => BinaryOp::Subtract,
        SyntaxBinaryOp::Multiply => BinaryOp::Multiply,
        SyntaxBinaryOp::Divide => BinaryOp::Divide,
    }
}

fn lower_codec_direction(direction: SyntaxCodecDirection) -> CodecDirection {
    match direction {
        SyntaxCodecDirection::Decode => CodecDirection::Decode,
        SyntaxCodecDirection::Encode => CodecDirection::Encode,
    }
}

pub(super) struct AstBuilder {
    next_node_id: u32,
}

impl AstBuilder {
    fn alloc(&mut self) -> NodeId {
        let node_id = NodeId::new(self.next_node_id);
        self.next_node_id += 1;
        node_id
    }

    fn lower_module_header(&mut self, module: &SyntaxModule) -> ModuleHeader {
        ModuleHeader {
            node_id: self.alloc(),
            name: module.name.clone(),
            span: module.span.clone(),
        }
    }

    fn lower_use_decl(&mut self, use_decl: &SyntaxUse, module_name: Option<String>) -> UseDecl {
        UseDecl {
            node_id: self.alloc(),
            module_name,
            name: use_decl.name.clone(),
            alias: import_alias(&use_decl.name),
            name_spans: use_decl.name_spans.clone(),
            package: use_decl
                .package
                .as_ref()
                .map(|package| package.name.clone()),
            package_span: use_decl
                .package
                .as_ref()
                .map(|package| package.span.clone()),
            span: use_decl.span.clone(),
            origin: UseOrigin::Source,
        }
    }

    fn lower_public_alias(
        &mut self,
        alias: &SyntaxPublicAlias,
        module_name: Option<String>,
    ) -> PublicAlias {
        PublicAlias {
            node_id: self.alloc(),
            module_name,
            kind: match alias.kind {
                SyntaxPublicAliasKind::Function => PublicAliasKind::Function,
                SyntaxPublicAliasKind::Type => PublicAliasKind::Type,
                SyntaxPublicAliasKind::Schema => PublicAliasKind::Schema,
            },
            name: alias.name.clone(),
            target: alias.target.clone(),
            target_spans: alias.target_spans.clone(),
            span: alias.span.clone(),
        }
    }

    fn lower_type_decl(
        &mut self,
        type_decl: &SyntaxTypeDecl,
        module_name: Option<String>,
    ) -> TypeDecl {
        TypeDecl {
            node_id: self.alloc(),
            module_name,
            visibility: match type_decl.visibility {
                SyntaxVisibility::Public => Visibility::Public,
                SyntaxVisibility::Private => Visibility::Private,
            },
            name: type_decl.name.clone(),
            params: type_decl.params.clone(),
            variants: type_decl
                .variants
                .iter()
                .map(|variant| TypeVariantDecl {
                    node_id: self.alloc(),
                    visibility: match variant.visibility {
                        SyntaxVisibility::Public => Visibility::Public,
                        SyntaxVisibility::Private => Visibility::Private,
                    },
                    name: variant.name.clone(),
                    fields: variant
                        .fields
                        .iter()
                        .map(|field| TypeVariantField {
                            node_id: self.alloc(),
                            name: field.name.clone(),
                            ty: field.ty.clone(),
                            ty_paths: self.lower_type_paths(&field.ty_paths),
                            span: field.span.clone(),
                        })
                        .collect(),
                    span: variant.span.clone(),
                })
                .collect(),
            span: type_decl.span.clone(),
        }
    }

    fn lower_effect_decl(
        &mut self,
        effect: &SyntaxEffectDecl,
        module_name: Option<String>,
    ) -> EffectDecl {
        EffectDecl {
            node_id: self.alloc(),
            module_name,
            visibility: match effect.visibility {
                SyntaxVisibility::Public => Visibility::Public,
                SyntaxVisibility::Private => Visibility::Private,
            },
            name: effect.name.clone(),
            operations: effect
                .operations
                .iter()
                .map(|operation| self.lower_effect_operation_decl(operation))
                .collect(),
            span: effect.span.clone(),
        }
    }

    fn lower_effect_operation_decl(
        &mut self,
        operation: &SyntaxEffectOperationDecl,
    ) -> EffectOperationDecl {
        EffectOperationDecl {
            node_id: self.alloc(),
            name: operation.name.clone(),
            name_span: operation.name_span.clone(),
            params: self.lower_params(&operation.params),
            return_type: operation.return_type.clone(),
            return_type_paths: self.lower_type_paths(&operation.return_type_paths),
            span: operation.span.clone(),
        }
    }

    fn lower_handler_decl(
        &mut self,
        handler: &SyntaxHandlerDecl,
        module_name: Option<String>,
    ) -> HandlerDecl {
        HandlerDecl {
            node_id: self.alloc(),
            module_name,
            visibility: match handler.visibility {
                SyntaxVisibility::Public => Visibility::Public,
                SyntaxVisibility::Private => Visibility::Private,
            },
            name: handler.name.clone(),
            params: self.lower_params(&handler.params),
            effect: handler.effect.clone(),
            effect_span: handler.effect_span.clone(),
            effects: handler.effects.clone(),
            effect_spans: handler.effect_spans.clone(),
            operation_clauses: handler
                .operation_clauses
                .iter()
                .map(|clause| self.lower_handler_operation_clause_decl(clause))
                .collect(),
            span: handler.span.clone(),
        }
    }

    fn lower_handler_operation_clause_decl(
        &mut self,
        clause: &SyntaxHandlerOperationClauseDecl,
    ) -> HandlerOperationClauseDecl {
        HandlerOperationClauseDecl {
            node_id: self.alloc(),
            operation: clause.operation.clone(),
            operation_span: clause.operation_span.clone(),
            params: clause
                .params
                .iter()
                .map(|param| Param {
                    node_id: self.alloc(),
                    name: param.name.clone(),
                    ty: None,
                    ty_span: None,
                    ty_paths: Vec::new(),
                    is_variadic: false,
                    span: param.span.clone(),
                })
                .collect(),
            body: self.lower_expr(&clause.body),
            span: clause.span.clone(),
        }
    }

    fn lower_schema_decl(
        &mut self,
        schema: &SyntaxSchemaDecl,
        module_name: Option<String>,
    ) -> SchemaDecl {
        SchemaDecl {
            node_id: self.alloc(),
            module_name,
            visibility: match schema.visibility {
                SyntaxVisibility::Public => Visibility::Public,
                SyntaxVisibility::Private => Visibility::Private,
            },
            name: schema.name.clone(),
            format: schema.format.as_ref().map(|format| SchemaFormatClause {
                node_id: self.alloc(),
                name: format.name.clone(),
                span: format.span.clone(),
            }),
            fields: schema
                .fields
                .iter()
                .map(|field| SchemaField {
                    node_id: self.alloc(),
                    name: field.name.clone(),
                    ty: field.ty.clone(),
                    ty_paths: self.lower_type_paths(&field.ty_paths),
                    where_clause: field.where_clause.as_ref().map(|where_clause| {
                        SchemaFieldWhereClause {
                            node_id: self.alloc(),
                            predicate: where_clause.predicate.clone(),
                            span: where_clause.span.clone(),
                        }
                    }),
                    span: field.span.clone(),
                })
                .collect(),
            validations: schema
                .validations
                .iter()
                .map(|validation| SchemaValidationClause {
                    node_id: self.alloc(),
                    predicate: validation.predicate.clone(),
                    span: validation.span.clone(),
                })
                .collect(),
            span: schema.span.clone(),
        }
    }

    fn lower_codec_decl(
        &mut self,
        codec: &SyntaxCodecDecl,
        module_name: Option<String>,
    ) -> CodecDecl {
        CodecDecl {
            node_id: self.alloc(),
            module_name,
            visibility: match codec.visibility {
                SyntaxVisibility::Public => Visibility::Public,
                SyntaxVisibility::Private => Visibility::Private,
            },
            name: codec.name.clone(),
            schema: codec.schema.clone(),
            directions: codec
                .directions
                .iter()
                .copied()
                .map(lower_codec_direction)
                .collect(),
            implementations: codec
                .implementations
                .iter()
                .map(|implementation| CodecImplementationClause {
                    node_id: self.alloc(),
                    direction: lower_codec_direction(implementation.direction),
                    kind: match &implementation.kind {
                        SyntaxCodecImplementationKind::Derive => CodecImplementationKind::Derive,
                        SyntaxCodecImplementationKind::With { function } => {
                            CodecImplementationKind::With {
                                function: function.clone(),
                            }
                        }
                    },
                    span: implementation.span.clone(),
                })
                .collect(),
            span: codec.span.clone(),
        }
    }

    fn lower_function(
        &mut self,
        function: &SyntaxFunction,
        module_name: Option<String>,
    ) -> Function {
        Function {
            node_id: self.alloc(),
            module_name,
            kind: match function.kind {
                veln_syntax::FunctionKind::Function => FunctionKind::Function,
                veln_syntax::FunctionKind::Test => FunctionKind::Test,
            },
            visibility: match function.visibility {
                SyntaxVisibility::Public => Visibility::Public,
                SyntaxVisibility::Private => Visibility::Private,
            },
            name: function.name.clone(),
            effect_binder: function
                .effect_binder
                .as_ref()
                .map(|binder| crate::EffectBinder {
                    name: binder.name.clone(),
                    span: binder.span.clone(),
                }),
            params: self.lower_params(&function.params),
            return_binding: function
                .return_binding
                .as_ref()
                .map(|binding| ResultBinding {
                    node_id: self.alloc(),
                    name: binding.name.clone(),
                    span: binding.span.clone(),
                }),
            return_type: function.return_type.clone(),
            return_type_span: function.return_type_span.clone(),
            return_type_paths: self.lower_type_paths(&function.return_type_paths),
            effects: function.effects.clone(),
            effect_spans: function.effect_spans.clone(),
            contracts: function
                .contracts
                .iter()
                .map(|contract| Contract {
                    node_id: self.alloc(),
                    kind: match contract.kind {
                        SyntaxContractKind::Require => ContractKind::Require,
                        SyntaxContractKind::Ensure => ContractKind::Ensure,
                        SyntaxContractKind::Invariant => ContractKind::Invariant,
                    },
                    text: contract.text.clone(),
                    span: contract.span.clone(),
                })
                .collect(),
            body: function
                .body
                .iter()
                .map(|line| match line {
                    SyntaxBodyLine::Let {
                        pattern,
                        annotation,
                        annotation_paths,
                        expr,
                        span,
                        ..
                    } => BodyLine {
                        node_id: self.alloc(),
                        kind: BodyLineKind::Let {
                            pattern: self.lower_pattern(pattern),
                            annotation: annotation.clone(),
                            annotation_paths: self.lower_type_paths(annotation_paths),
                            expr: self.lower_expr(expr),
                        },
                        span: span.clone(),
                    },
                    SyntaxBodyLine::Expr { expr, span } => BodyLine {
                        node_id: self.alloc(),
                        kind: BodyLineKind::Expr {
                            expr: self.lower_expr(expr),
                        },
                        span: span.clone(),
                    },
                })
                .collect(),
            span: function.span.clone(),
        }
    }

    fn lower_params(&mut self, params: &[SyntaxParam]) -> Vec<Param> {
        params
            .iter()
            .map(|param| Param {
                node_id: self.alloc(),
                name: param.name.clone(),
                ty: param.ty.clone(),
                ty_span: param.ty_span.clone(),
                ty_paths: self.lower_type_paths(&param.ty_paths),
                is_variadic: param.is_variadic,
                span: param.span.clone(),
            })
            .collect()
    }

    fn lower_type_paths(
        &mut self,
        paths: &[veln_syntax::TypePathSegments],
    ) -> Vec<TypePathSegments> {
        paths
            .iter()
            .map(|path| TypePathSegments {
                segments: path.segments.clone(),
                segment_spans: path.segment_spans.clone(),
            })
            .collect()
    }
}
