use super::*;
use crate::types::ModuleValueLookup;
use std::collections::HashMap;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleValueImportPreparationWork {
    pub(crate) scanned_imports: usize,
    pub(crate) retained_import_clones: usize,
    pub(crate) owner_lookups: usize,
}

#[cfg(test)]
impl ModuleValueImportPreparationWork {
    pub(crate) fn total(self) -> usize {
        self.scanned_imports + self.retained_import_clones + self.owner_lookups
    }
}

#[cfg(test)]
thread_local! {
    static MODULE_VALUE_IMPORT_PREPARATION_WORK: std::cell::Cell<ModuleValueImportPreparationWork> =
        const { std::cell::Cell::new(ModuleValueImportPreparationWork {
            scanned_imports: 0,
            retained_import_clones: 0,
            owner_lookups: 0,
        }) };
}

#[cfg(test)]
pub(crate) fn reset_module_value_import_preparation_work() {
    MODULE_VALUE_IMPORT_PREPARATION_WORK.set(ModuleValueImportPreparationWork::default());
}

#[cfg(test)]
pub(crate) fn take_module_value_import_preparation_work() -> ModuleValueImportPreparationWork {
    MODULE_VALUE_IMPORT_PREPARATION_WORK.replace(ModuleValueImportPreparationWork::default())
}

fn record_prepared_import() {
    #[cfg(test)]
    MODULE_VALUE_IMPORT_PREPARATION_WORK.set({
        let mut work = MODULE_VALUE_IMPORT_PREPARATION_WORK.get();
        work.scanned_imports += 1;
        work.retained_import_clones += 1;
        work
    });
}

fn record_owner_lookup() {
    #[cfg(test)]
    MODULE_VALUE_IMPORT_PREPARATION_WORK.set({
        let mut work = MODULE_VALUE_IMPORT_PREPARATION_WORK.get();
        work.owner_lookups += 1;
        work
    });
}

pub(super) fn lower_module_values(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<CoreModuleValueDecl> {
    let uses_by_owner = prepare_uses_by_owner(environment);

    module
        .module_values
        .iter()
        .filter_map(|declaration| lower_module_value(declaration, environment, &uses_by_owner))
        .collect()
}

fn prepare_uses_by_owner(
    environment: &TypeEnvironment,
) -> HashMap<veln_ast::ResolvedImportOwner, Vec<veln_ast::UseDecl>> {
    let mut uses_by_owner = HashMap::<veln_ast::ResolvedImportOwner, Vec<veln_ast::UseDecl>>::new();
    for use_decl in &environment.uses {
        record_prepared_import();
        uses_by_owner
            .entry(use_decl.resolved_owner())
            .or_default()
            .push(use_decl.clone());
    }
    uses_by_owner
}

fn lower_module_value(
    declaration: &veln_ast::ModuleValueDecl,
    environment: &TypeEnvironment,
    uses_by_owner: &HashMap<veln_ast::ResolvedImportOwner, Vec<veln_ast::UseDecl>>,
) -> Option<CoreModuleValueDecl> {
    let name = declaration.name.as_ref()?;
    record_owner_lookup();
    let owner = veln_ast::ResolvedImportOwner {
        package: declaration.package_name.clone(),
        module: declaration.module_name.clone(),
    };
    let uses = uses_by_owner.get(&owner).map(Vec::as_slice).unwrap_or(&[]);
    let ty = environment.canonicalize_type_annotation_in_owner(
        parse_type_or_unknown(Some(&declaration.annotation)),
        declaration.module_name.as_deref(),
        declaration.package_name.as_deref(),
    );
    let expected = core_type(&ty);
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
}

struct ModuleValueLowerer<'a> {
    declaration: &'a veln_ast::ModuleValueDecl,
    environment: &'a TypeEnvironment,
    uses: &'a [veln_ast::UseDecl],
}

impl ModuleValueLowerer<'_> {
    fn lower_expr(&self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::StringLiteral(value) => self.lowered(
                expr,
                CoreType::string(),
                CoreExprKind::StringLiteral(value.clone()),
            ),
            ExprKind::IntLiteral(value) => self.lower_int_literal(expr, value),
            ExprKind::BoolLiteral(value) => {
                self.lowered(expr, CoreType::bool(), CoreExprKind::BoolLiteral(*value))
            }
            ExprKind::Unit => self.lowered(expr, CoreType::unit(), CoreExprKind::Unit),
            ExprKind::Prefix {
                op: veln_ast::PrefixOp::Negate,
                expr: inner,
            } => self.lower_negation(expr, inner),
            ExprKind::Record(fields) => self.lower_record(expr, fields, expected),
            ExprKind::NamePath { segments, .. } => self.lower_name(expr, segments, expected),
            ExprKind::Call { callee, args } => self.lower_constructor(expr, callee, args, expected),
            _ => self.missing(expr, expected),
        }
    }

    fn lowered(&self, expr: &Expr, ty: CoreType, kind: CoreExprKind) -> CoreExpr {
        CoreExpr {
            node_id: expr.node_id,
            ty,
            kind,
            span: expr.span.clone(),
        }
    }

    fn lower_int_literal(&self, expr: &Expr, value: &str) -> CoreExpr {
        let value = parse_integer_literal(value)
            .map(|literal| literal.value.to_string())
            .unwrap_or_else(|_| value.to_string());
        self.lowered(expr, CoreType::int(), CoreExprKind::IntLiteral(value))
    }

    fn lower_negation(&self, expr: &Expr, inner: &Expr) -> CoreExpr {
        let inner = self.lower_expr(inner, Some(&CoreType::int()));
        self.lowered(
            expr,
            CoreType::int(),
            CoreExprKind::Prefix {
                op: veln_ast::PrefixOp::Negate,
                expr: Box::new(inner),
            },
        )
    }

    fn lower_record(
        &self,
        expr: &Expr,
        fields: &[veln_ast::RecordField],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
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
        let ty = expected
            .cloned()
            .unwrap_or_else(|| CoreType::Record(Vec::new()));
        self.lowered(expr, ty, CoreExprKind::Record(lowered))
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
            self.uses,
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
                self.uses,
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
