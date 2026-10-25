use veln_source::SourceSpan;
use veln_syntax::{
    BinaryOp as SyntaxBinaryOp, BodyLine as SyntaxBodyLine, ContractClause as SyntaxContract,
    ContractKind as SyntaxContractKind, DictEntry as SyntaxDictEntry,
    EffectDecl as SyntaxEffectDecl, EffectOperationDecl as SyntaxEffectOperationDecl,
    Expr as SyntaxExpr, ExprKind as SyntaxExprKind, FunctionDecl as SyntaxFunction,
    HandlerDecl as SyntaxHandlerDecl,
    HandlerOperationClauseDecl as SyntaxHandlerOperationClauseDecl, IfBranch as SyntaxIfBranch,
    MatchArm as SyntaxMatchArm, ModuleDecl as SyntaxModule, Param as SyntaxParam,
    Pattern as SyntaxPattern, PatternKind as SyntaxPatternKind, PrefixOp as SyntaxPrefixOp,
    PublicAliasDecl as SyntaxPublicAlias, PublicAliasKind as SyntaxPublicAliasKind,
    RecordField as SyntaxRecordField, SchemaDecl as SyntaxSchemaDecl, SyntaxItem, SyntaxTree,
    TypeDecl as SyntaxTypeDecl, UseDecl as SyntaxUse, Visibility as SyntaxVisibility,
};

use crate::{
    BinaryOp, BodyLine, BodyLineKind, Contract, ContractKind, DictEntry, EffectDecl,
    EffectOperationDecl, Expr, ExprKind, Function, FunctionKind, HandlerDecl,
    HandlerOperationClauseDecl, InvalidName, MatchArm, ModuleHeader, NameClass, NameOccurrence,
    NodeId, Param, Pattern, PatternField, PatternKind, PredicateRemainder, PrefixOp, PublicAlias,
    PublicAliasKind, RecordField, ResultBinding, SchemaDecl, SchemaField, SchemaFieldWhereClause,
    SchemaFormatClause, SchemaValidationClause, SurfaceModule, TypeAnnotationStructure, TypeDecl,
    TypeVariantDecl, TypeVariantField, UseDecl, UseOrigin, Visibility,
};

mod expressions;
mod invalid_names;
mod items;

use invalid_names::collect_invalid_names;

pub fn lower_surface_ast(tree: &SyntaxTree) -> SurfaceModule {
    let mut builder = AstBuilder { next_node_id: 1 };
    let module = tree
        .module
        .as_ref()
        .map(|module| builder.lower_module_header(module));
    let module_name = module
        .as_ref()
        .filter(|module| module_name_has_valid_initial(&module.name))
        .map(|module| module.name.clone());
    builder.lower_surface_ast_with_module(tree, module, module_name, None)
}

pub fn lower_surface_ast_with_module_identity(
    tree: &SyntaxTree,
    name: String,
    span: SourceSpan,
) -> SurfaceModule {
    let mut builder = AstBuilder { next_node_id: 1 };
    let mut module = builder.lower_surface_ast_with_module(tree, None, Some(name.clone()), None);
    module.module = Some(ModuleHeader {
        node_id: builder.alloc(),
        name,
        span,
    });
    module
}

pub fn lower_surface_ast_with_package_module_identity(
    tree: &SyntaxTree,
    package_name: String,
    module_name: String,
    span: SourceSpan,
) -> SurfaceModule {
    let internal_name = format!("{package_name}::{module_name}");
    let mut builder = AstBuilder { next_node_id: 1 };
    let mut module = builder.lower_surface_ast_with_module(
        tree,
        None,
        Some(internal_name.clone()),
        Some(package_name),
    );
    module.module = Some(ModuleHeader {
        node_id: builder.alloc(),
        name: internal_name,
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
        package_name: Option<String>,
    ) -> SurfaceModule {
        let uses = tree
            .uses
            .iter()
            .map(|use_decl| self.lower_use_decl(use_decl, module_name.clone()))
            .collect();
        let items = self.lower_items(&tree.items, &module_name, &package_name);

        SurfaceModule {
            module,
            uses,
            aliases: items.aliases,
            effects: items.effects,
            handlers: items.handlers,
            types: items.types,
            schemas: items.schemas,
            functions: items.functions,
            invalid_names: collect_invalid_names(tree),
        }
    }
}

fn module_name_has_valid_initial(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
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
        SyntaxBinaryOp::Remainder => BinaryOp::Remainder,
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
            written_name: use_decl.name.clone(),
            alias: use_decl
                .alias
                .as_ref()
                .map_or_else(|| import_alias(&use_decl.name), |alias| alias.name.clone()),
            alias_span: use_decl.alias.as_ref().map(|alias| alias.span.clone()),
            explicit_alias: use_decl.alias.is_some(),
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
            syntax_recovered: use_decl.recovered,
            resolution_quarantined: false,
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
                            ty_span: field.ty_span.clone(),
                            ty_paths: field.ty_paths.clone(),
                            ty_refinements: field.ty_refinements.clone(),
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
            return_type_paths: operation.return_type_paths.clone(),
            return_type_refinements: operation.return_type_refinements.clone(),
            span: operation.span.clone(),
        }
    }

    fn lower_handler_decl(
        &mut self,
        handler: &SyntaxHandlerDecl,
        module_name: Option<String>,
        package_name: Option<String>,
    ) -> HandlerDecl {
        HandlerDecl {
            node_id: self.alloc(),
            module_name,
            package_name,
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
                    name_span: param.name_span.clone(),
                    ty: None,
                    ty_span: None,
                    ty_paths: Vec::new(),
                    ty_refinements: Vec::new(),
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
                    ty_paths: field.ty_paths.clone(),
                    ty_refinements: field.ty_refinements.clone(),
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

    fn lower_function(
        &mut self,
        function: &SyntaxFunction,
        module_name: Option<String>,
        package_name: Option<String>,
    ) -> Function {
        Function {
            node_id: self.alloc(),
            module_name,
            package_name,
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
            return_type_paths: function.return_type_paths.clone(),
            return_type_refinements: function.return_type_refinements.clone(),
            effects: function.effects.clone(),
            effect_spans: function.effect_spans.clone(),
            callsite: function.callsite.clone(),
            contracts: function
                .contracts
                .iter()
                .map(|contract| self.lower_contract(contract))
                .collect(),
            body: function
                .body
                .iter()
                .map(|line| self.lower_body_line(line))
                .collect(),
            span: function.span.clone(),
        }
    }

    fn lower_contract(&mut self, contract: &SyntaxContract) -> Contract {
        Contract {
            node_id: self.alloc(),
            kind: match contract.kind {
                SyntaxContractKind::Require => ContractKind::Require,
                SyntaxContractKind::Ensure => ContractKind::Ensure,
                SyntaxContractKind::Invariant => ContractKind::Invariant,
            },
            text: contract.text.clone(),
            predicate_tokens: contract
                .predicate_tokens
                .iter()
                .map(|token| crate::PredicateTokenSpan {
                    text_range: token.text_range.clone(),
                    source_span: token.source_span.clone(),
                })
                .collect(),
            remainder_spans: contract
                .remainder_spans
                .iter()
                .map(lower_predicate_remainder)
                .collect(),
            callsite_reference_spans: contract.callsite_reference_spans.clone(),
            call_callee_spans: contract.call_callee_spans.clone(),
            span: contract.span.clone(),
        }
    }

    fn lower_body_line(&mut self, line: &SyntaxBodyLine) -> BodyLine {
        let node_id = self.alloc();
        match line {
            SyntaxBodyLine::Let {
                pattern,
                annotation,
                annotation_paths,
                annotation_refinements,
                expr,
                span,
            } => BodyLine {
                node_id,
                kind: BodyLineKind::Let {
                    pattern: self.lower_pattern(pattern),
                    annotation: annotation.clone(),
                    annotation_structure: Box::new(TypeAnnotationStructure {
                        paths: annotation_paths.to_vec(),
                        variant_refinements: annotation_refinements.to_vec(),
                    }),
                    expr: self.lower_expr(expr),
                },
                span: span.clone(),
            },
            SyntaxBodyLine::Expr { expr, span } => BodyLine {
                node_id,
                kind: BodyLineKind::Expr {
                    expr: self.lower_expr(expr),
                },
                span: span.clone(),
            },
            SyntaxBodyLine::Defer {
                body,
                keyword_span,
                block_span,
                span,
            } => BodyLine {
                node_id,
                kind: BodyLineKind::Defer {
                    body: body.iter().map(|line| self.lower_body_line(line)).collect(),
                    keyword_span: keyword_span.clone(),
                    block_span: block_span.clone(),
                },
                span: span.clone(),
            },
        }
    }

    fn lower_params(&mut self, params: &[SyntaxParam]) -> Vec<Param> {
        params
            .iter()
            .map(|param| Param {
                node_id: self.alloc(),
                name: param.name.clone(),
                name_span: param.name_span.clone(),
                ty: param.ty.clone(),
                ty_span: param.ty_span.clone(),
                ty_paths: param.ty_paths.clone(),
                ty_refinements: param.ty_refinements.clone(),
                is_variadic: param.is_variadic,
                span: param.span.clone(),
            })
            .collect()
    }
}

fn lower_predicate_remainder(remainder: &veln_syntax::PredicateRemainder) -> PredicateRemainder {
    PredicateRemainder {
        left_range: remainder.left_range.clone(),
        right_range: remainder.right_range.clone(),
        operation_range: remainder.operation_range.clone(),
        left_remainder: remainder.left_remainder,
        operator_span: remainder.operator_span.clone(),
        left_span: remainder.left_span.clone(),
        right_span: remainder.right_span.clone(),
        operation_span: remainder.operation_span.clone(),
    }
}
