use super::*;
use crate::types::ModuleValueLookup;

pub(super) fn lower_module_values(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<CoreModuleValueDecl> {
    module
        .module_values
        .iter()
        .filter_map(|declaration| {
            let name = declaration.name.as_ref()?;
            let ty = environment.canonicalize_type_annotation_in_owner(
                parse_type_or_unknown(Some(&declaration.annotation)),
                declaration.module_name.as_deref(),
                declaration.package_name.as_deref(),
            );
            let expected = core_type(&ty);
            let uses = environment
                .uses
                .iter()
                .filter(|use_decl| {
                    use_decl.module_name.as_deref() == declaration.module_name.as_deref()
                        && use_decl.importing_package.as_deref()
                            == declaration.package_name.as_deref()
                })
                .cloned()
                .collect::<Vec<_>>();
            Some(CoreModuleValueDecl {
                node_id: declaration.node_id,
                identity: CoreModuleValueIdentity {
                    package: declaration.package_name.clone(),
                    module: declaration.module_name.clone(),
                    name: name.clone(),
                },
                annotation: declaration.annotation.clone(),
                ty: expected.clone(),
                visibility: declaration.visibility,
                initializer: ModuleValueLowerer {
                    declaration,
                    environment,
                    uses,
                }
                .lower_expr(&declaration.initializer, Some(&expected)),
                span: declaration.span.clone(),
            })
        })
        .collect()
}

struct ModuleValueLowerer<'a> {
    declaration: &'a veln_ast::ModuleValueDecl,
    environment: &'a TypeEnvironment,
    uses: Vec<veln_ast::UseDecl>,
}

impl ModuleValueLowerer<'_> {
    fn lower_expr(&self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        let (ty, kind) = match &expr.kind {
            ExprKind::StringLiteral(value) => (
                CoreType::string(),
                CoreExprKind::StringLiteral(value.clone()),
            ),
            ExprKind::IntLiteral(value) => {
                let value = parse_integer_literal(value)
                    .map(|literal| literal.value.to_string())
                    .unwrap_or_else(|_| value.clone());
                (CoreType::int(), CoreExprKind::IntLiteral(value))
            }
            ExprKind::BoolLiteral(value) => (CoreType::bool(), CoreExprKind::BoolLiteral(*value)),
            ExprKind::Unit => (CoreType::unit(), CoreExprKind::Unit),
            ExprKind::Prefix {
                op: veln_ast::PrefixOp::Negate,
                expr: inner,
            } => (
                CoreType::int(),
                CoreExprKind::Prefix {
                    op: veln_ast::PrefixOp::Negate,
                    expr: Box::new(self.lower_expr(inner, Some(&CoreType::int()))),
                },
            ),
            ExprKind::Record(fields) => {
                let lowered = fields
                    .iter()
                    .map(|field| CoreRecordField {
                        node_id: field.node_id,
                        name: field.name.clone(),
                        expr: self.lower_expr(
                            &field.expr,
                            expected.and_then(|ty| ty.record_field(&field.name)),
                        ),
                        span: field.span.clone(),
                    })
                    .collect();
                (
                    expected
                        .cloned()
                        .unwrap_or_else(|| CoreType::Record(Vec::new())),
                    CoreExprKind::Record(lowered),
                )
            }
            ExprKind::NamePath { segments, .. } => {
                return self.lower_name(expr, segments, expected);
            }
            ExprKind::Call { callee, args } => {
                return self.lower_constructor(expr, callee, args, expected);
            }
            _ => (
                expected.cloned().unwrap_or(CoreType::Unknown),
                CoreExprKind::Missing,
            ),
        };
        CoreExpr {
            node_id: expr.node_id,
            ty,
            kind,
            span: expr.span.clone(),
        }
    }

    fn lower_name(
        &self,
        expr: &Expr,
        segments: &[String],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if let ModuleValueLookup::Available(value) = self.environment.module_value_path_in_owner(
            segments,
            self.declaration.module_name.as_deref(),
            self.declaration.package_name.as_deref(),
        ) {
            return CoreExpr {
                node_id: expr.node_id,
                ty: core_type(&value.ty),
                kind: CoreExprKind::ModuleValue(module_value_identity(&value.declaration)),
                span: expr.span.clone(),
            };
        }

        match self.environment.adts.constructor_for_expected_type(
            segments,
            expected,
            self.declaration.module_name.as_deref(),
            &self.uses,
            ConstructorShape::Nullary,
        ) {
            ConstructorLookup::Found(constructor) => CoreExpr {
                node_id: expr.node_id,
                ty: expected
                    .cloned()
                    .unwrap_or_else(|| adt::core_constructed_type(constructor, &[])),
                kind: core_nullary_constructor_kind(constructor),
                span: expr.span.clone(),
            },
            ConstructorLookup::Ambiguous | ConstructorLookup::Missing => CoreExpr {
                node_id: expr.node_id,
                ty: expected.cloned().unwrap_or(CoreType::Unknown),
                kind: CoreExprKind::Missing,
                span: expr.span.clone(),
            },
        }
    }

    fn lower_constructor(
        &self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let Some((segments, _)) = callee.callee_name_path_and_type_args() else {
            return self.missing(expr, expected);
        };
        let ConstructorLookup::Found(constructor) =
            self.environment.adts.constructor_for_expected_type(
                segments,
                expected,
                self.declaration.module_name.as_deref(),
                &self.uses,
                ConstructorShape::Payload,
            )
        else {
            return self.missing(expr, expected);
        };
        let payloads = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let payload_type = expected
                    .and_then(|ty| adt::core_payload_type(ty, constructor, index))
                    .unwrap_or(CoreType::Unknown);
                self.lower_expr(arg, Some(&payload_type))
            })
            .collect();
        CoreExpr {
            node_id: expr.node_id,
            ty: expected
                .cloned()
                .unwrap_or_else(|| adt::core_constructed_type(constructor, &[])),
            kind: core_payload_constructor_kind(constructor, payloads),
            span: expr.span.clone(),
        }
    }

    fn missing(&self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        CoreExpr {
            node_id: expr.node_id,
            ty: expected.cloned().unwrap_or(CoreType::Unknown),
            kind: CoreExprKind::Missing,
            span: expr.span.clone(),
        }
    }
}

pub(super) fn module_value_identity(
    declaration: &veln_ast::ModuleValueDecl,
) -> CoreModuleValueIdentity {
    CoreModuleValueIdentity {
        package: declaration.package_name.clone(),
        module: declaration.module_name.clone(),
        name: declaration
            .name
            .clone()
            .expect("resolved module value has a name"),
    }
}
