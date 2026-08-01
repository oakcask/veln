use veln_source::SourceSpan;
use veln_syntax::{
    BinaryOp as SyntaxBinaryOp, BodyLine as SyntaxBodyLine, CodecDecl as SyntaxCodecDecl,
    CodecDirection as SyntaxCodecDirection,
    CodecImplementationKind as SyntaxCodecImplementationKind, ContractKind as SyntaxContractKind,
    DictEntry as SyntaxDictEntry, EffectDecl as SyntaxEffectDecl,
    EffectOperationDecl as SyntaxEffectOperationDecl, Expr as SyntaxExpr,
    ExprKind as SyntaxExprKind, FunctionDecl as SyntaxFunction, HandlerDecl as SyntaxHandlerDecl,
    HandlerProviderDecl as SyntaxHandlerProviderDecl, ModuleDecl as SyntaxModule,
    Pattern as SyntaxPattern, PatternKind as SyntaxPatternKind, PrefixOp as SyntaxPrefixOp,
    PublicAliasDecl as SyntaxPublicAlias, PublicAliasKind as SyntaxPublicAliasKind,
    RecordField as SyntaxRecordField, SchemaDecl as SyntaxSchemaDecl, SyntaxItem, SyntaxTree,
    TypeDecl as SyntaxTypeDecl, UseDecl as SyntaxUse, Visibility as SyntaxVisibility,
};

use crate::{
    BinaryOp, BodyLine, BodyLineKind, CodecDecl, CodecDirection, CodecImplementationClause,
    CodecImplementationKind, Contract, ContractKind, DictEntry, EffectDecl, EffectOperationDecl,
    Expr, ExprKind, Function, FunctionKind, HandlerDecl, HandlerProviderDecl, MatchArm,
    ModuleHeader, NodeId, Param, Pattern, PatternField, PatternKind, PrefixOp, PublicAlias,
    PublicAliasKind, RecordField, ResultBinding, SchemaDecl, SchemaField, SchemaFieldWhereClause,
    SchemaFormatClause, SchemaValidationClause, SurfaceModule, TypeDecl, TypeVariantDecl,
    TypeVariantField, UseDecl, UseOrigin, Visibility,
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

        for item in &tree.items {
            match item {
                SyntaxItem::Function(function) => {
                    functions.push(self.lower_function(function, module_name.clone()));
                }
                SyntaxItem::Effect(effect) => {
                    effects.push(self.lower_effect_decl(effect, module_name.clone()));
                }
                SyntaxItem::Handler(handler) => {
                    handlers.push(self.lower_handler_decl(handler, module_name.clone()));
                }
                SyntaxItem::Type(type_decl) => {
                    types.push(self.lower_type_decl(type_decl, module_name.clone()));
                }
                SyntaxItem::Schema(schema) => {
                    schemas.push(self.lower_schema_decl(schema, module_name.clone()));
                }
                SyntaxItem::Codec(codec) => {
                    codecs.push(self.lower_codec_decl(codec, module_name.clone()));
                }
                SyntaxItem::PublicAlias(alias) => {
                    aliases.push(self.lower_public_alias(alias, module_name.clone()));
                }
            }
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
    if name.contains("::") {
        name.to_string()
    } else {
        name.split('.').next_back().unwrap_or(name).to_string()
    }
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

struct AstBuilder {
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
            params: operation
                .params
                .iter()
                .map(|param| Param {
                    node_id: self.alloc(),
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    ty_span: param.ty_span.clone(),
                    is_variadic: param.is_variadic,
                    span: param.span.clone(),
                })
                .collect(),
            return_type: operation.return_type.clone(),
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
            params: handler
                .params
                .iter()
                .map(|param| Param {
                    node_id: self.alloc(),
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    ty_span: param.ty_span.clone(),
                    is_variadic: param.is_variadic,
                    span: param.span.clone(),
                })
                .collect(),
            effect: handler.effect.clone(),
            effect_span: handler.effect_span.clone(),
            effects: handler.effects.clone(),
            effect_spans: handler.effect_spans.clone(),
            providers: handler
                .providers
                .iter()
                .map(|provider| self.lower_handler_provider_decl(provider))
                .collect(),
            span: handler.span.clone(),
        }
    }

    fn lower_handler_provider_decl(
        &mut self,
        provider: &SyntaxHandlerProviderDecl,
    ) -> HandlerProviderDecl {
        HandlerProviderDecl {
            node_id: self.alloc(),
            operation: provider.operation.clone(),
            operation_span: provider.operation_span.clone(),
            provider: provider.provider.clone(),
            provider_span: provider.provider_span.clone(),
            span: provider.span.clone(),
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
            params: function
                .params
                .iter()
                .map(|param| Param {
                    node_id: self.alloc(),
                    name: param.name.clone(),
                    ty: param.ty.clone(),
                    ty_span: param.ty_span.clone(),
                    is_variadic: param.is_variadic,
                    span: param.span.clone(),
                })
                .collect(),
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
                        expr,
                        span,
                    } => BodyLine {
                        node_id: self.alloc(),
                        kind: BodyLineKind::Let {
                            pattern: self.lower_pattern(pattern),
                            annotation: annotation.clone(),
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

    fn lower_expr(&mut self, expr: &SyntaxExpr) -> Expr {
        Expr {
            node_id: self.alloc(),
            kind: self.lower_expr_kind(expr),
            span: expr.span.clone(),
        }
    }

    fn lower_expr_kind(&mut self, expr: &SyntaxExpr) -> ExprKind {
        if let Some(kind) = self.lower_scalar_expr_kind(expr) {
            return kind;
        }
        if let Some(kind) = self.lower_call_like_expr_kind(expr) {
            return kind;
        }
        if let Some(kind) = self.lower_collection_expr_kind(expr) {
            return kind;
        }
        self.lower_operator_expr_kind(expr)
    }

    fn lower_scalar_expr_kind(&mut self, expr: &SyntaxExpr) -> Option<ExprKind> {
        match &expr.kind {
            SyntaxExprKind::Missing => Some(ExprKind::Missing),
            SyntaxExprKind::Hole { name, satisfy } => Some(ExprKind::Hole {
                name: name.clone(),
                satisfy: satisfy.as_ref().map(crate::satisfy::lower_satisfy_clause),
            }),
            SyntaxExprKind::NamePath(segments) => Some(ExprKind::NamePath(segments.clone())),
            SyntaxExprKind::StringLiteral(value) => Some(ExprKind::StringLiteral(value.clone())),
            SyntaxExprKind::IntLiteral(value) => Some(ExprKind::IntLiteral(value.clone())),
            SyntaxExprKind::FloatLiteral(value) => Some(ExprKind::FloatLiteral(value.clone())),
            SyntaxExprKind::BoolLiteral(value) => Some(ExprKind::BoolLiteral(*value)),
            SyntaxExprKind::Unit => Some(ExprKind::Unit),
            _ => None,
        }
    }

    fn lower_call_like_expr_kind(&mut self, expr: &SyntaxExpr) -> Option<ExprKind> {
        match &expr.kind {
            SyntaxExprKind::TypeApply { callee, type_args } => Some(ExprKind::TypeApply {
                callee: Box::new(self.lower_expr(callee)),
                type_args: type_args.clone(),
            }),
            SyntaxExprKind::Call { callee, args } => Some(ExprKind::Call {
                callee: Box::new(self.lower_expr(callee)),
                args: self.lower_exprs(args),
            }),
            SyntaxExprKind::Perform {
                effect,
                effect_span,
                operation,
                operation_span,
                args,
            } => Some(ExprKind::Perform {
                effect: effect.clone(),
                effect_span: effect_span.clone(),
                operation: operation.clone(),
                operation_span: operation_span.clone(),
                args: self.lower_exprs(args),
            }),
            SyntaxExprKind::Handle {
                body,
                handler,
                handler_span,
                args,
            } => Some(ExprKind::Handle {
                body: Box::new(self.lower_expr(body)),
                handler: handler.clone(),
                handler_span: handler_span.clone(),
                args: self.lower_exprs(args),
            }),
            SyntaxExprKind::SchemaDecode {
                schema,
                input,
                base,
            } => Some(ExprKind::SchemaDecode {
                schema: schema.clone(),
                input: Box::new(self.lower_expr(input)),
                base: Box::new(self.lower_expr(base)),
            }),
            SyntaxExprKind::SchemaEncode { schema, value } => Some(ExprKind::SchemaEncode {
                schema: schema.clone(),
                value: Box::new(self.lower_expr(value)),
            }),
            SyntaxExprKind::FieldAccess {
                base,
                field,
                field_span,
            } => Some(ExprKind::FieldAccess {
                base: Box::new(self.lower_expr(base)),
                field: field.clone(),
                field_span: field_span.clone(),
            }),
            SyntaxExprKind::Try(expr) => Some(ExprKind::Try(Box::new(self.lower_expr(expr)))),
            _ => None,
        }
    }

    fn lower_collection_expr_kind(&mut self, expr: &SyntaxExpr) -> Option<ExprKind> {
        match &expr.kind {
            SyntaxExprKind::Record(fields) => Some(ExprKind::Record(
                fields
                    .iter()
                    .map(|field| self.lower_record_field(field))
                    .collect(),
            )),
            SyntaxExprKind::Dict(entries) => Some(ExprKind::Dict(
                entries
                    .iter()
                    .map(|entry| self.lower_dict_entry(entry))
                    .collect(),
            )),
            SyntaxExprKind::List(items) => Some(ExprKind::List(self.lower_exprs(items))),
            SyntaxExprKind::Match { scrutinee, arms } => {
                Some(self.lower_match_expr(scrutinee, arms))
            }
            SyntaxExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => Some(ExprKind::If {
                condition: Box::new(self.lower_expr(condition)),
                then_branch: Box::new(self.lower_expr(then_branch)),
                else_if_branches: self.lower_if_branches(else_if_branches),
                else_branch: Box::new(self.lower_expr(else_branch)),
            }),
            _ => None,
        }
    }

    fn lower_operator_expr_kind(&mut self, expr: &SyntaxExpr) -> ExprKind {
        match &expr.kind {
            SyntaxExprKind::Prefix { op, expr } => ExprKind::Prefix {
                op: lower_prefix_op(*op),
                expr: Box::new(self.lower_expr(expr)),
            },
            SyntaxExprKind::Binary { op, left, right } => ExprKind::Binary {
                op: lower_binary_op(*op),
                left: Box::new(self.lower_expr(left)),
                right: Box::new(self.lower_expr(right)),
            },
            _ => {
                unreachable!("syntax expression variant should be handled before operator lowering")
            }
        }
    }

    fn lower_exprs(&mut self, exprs: &[SyntaxExpr]) -> Vec<Expr> {
        exprs.iter().map(|expr| self.lower_expr(expr)).collect()
    }

    fn lower_match_expr(
        &mut self,
        scrutinee: &SyntaxExpr,
        arms: &[veln_syntax::MatchArm],
    ) -> ExprKind {
        ExprKind::Match {
            scrutinee: Box::new(self.lower_expr(scrutinee)),
            arms: arms
                .iter()
                .map(|arm| MatchArm {
                    node_id: self.alloc(),
                    pattern: self.lower_pattern(&arm.pattern),
                    expr: self.lower_expr(&arm.expr),
                    span: arm.span.clone(),
                })
                .collect(),
        }
    }

    fn lower_if_branches(&mut self, branches: &[veln_syntax::IfBranch]) -> Vec<crate::IfBranch> {
        branches
            .iter()
            .map(|branch| crate::IfBranch {
                node_id: self.alloc(),
                condition: self.lower_expr(&branch.condition),
                expr: self.lower_expr(&branch.expr),
                span: branch.span.clone(),
            })
            .collect()
    }

    fn lower_pattern(&mut self, pattern: &SyntaxPattern) -> Pattern {
        Pattern {
            node_id: self.alloc(),
            kind: match &pattern.kind {
                SyntaxPatternKind::Wildcard => PatternKind::Wildcard,
                SyntaxPatternKind::Binding(name) => PatternKind::Binding(name.clone()),
                SyntaxPatternKind::StringLiteral(value) => {
                    PatternKind::StringLiteral(value.clone())
                }
                SyntaxPatternKind::IntLiteral(value) => PatternKind::IntLiteral(value.clone()),
                SyntaxPatternKind::FloatLiteral(value) => PatternKind::FloatLiteral(value.clone()),
                SyntaxPatternKind::BoolLiteral(value) => PatternKind::BoolLiteral(*value),
                SyntaxPatternKind::Unit => PatternKind::Unit,
                SyntaxPatternKind::Record(fields) => PatternKind::Record(
                    fields
                        .iter()
                        .map(|field| PatternField {
                            node_id: self.alloc(),
                            name: field.name.clone(),
                            pattern: self.lower_pattern(&field.pattern),
                            span: field.span.clone(),
                        })
                        .collect(),
                ),
                SyntaxPatternKind::Constructor { name, args } => PatternKind::Constructor {
                    name: name.clone(),
                    args: args.iter().map(|arg| self.lower_pattern(arg)).collect(),
                },
            },
            span: pattern.span.clone(),
        }
    }

    fn lower_record_field(&mut self, field: &SyntaxRecordField) -> RecordField {
        RecordField {
            node_id: self.alloc(),
            name: field.name.clone(),
            expr: self.lower_expr(&field.expr),
            span: field.span.clone(),
        }
    }

    fn lower_dict_entry(&mut self, entry: &SyntaxDictEntry) -> DictEntry {
        DictEntry {
            node_id: self.alloc(),
            key: self.lower_expr(&entry.key),
            value: self.lower_expr(&entry.value),
            span: entry.span.clone(),
        }
    }
}
