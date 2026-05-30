use crate::*;
use veln_ast::{SurfaceModule, lower_surface_ast};
use veln_core::{
    ContractObligationStatus, CoreBlocker, CoreCallTarget, CoreExprKind, CorePatternKind,
    CoreReadiness, CoreStmtKind, CoreType,
};
use veln_diagnostics::DiagnosticKind;
use veln_ir::{IrCallTarget, IrExprKind, IrPatternKind, IrStmtKind};
use veln_source::SourceFile;
use veln_syntax::parse;

fn exhaustive_case_split_predicate(subject: &str, fields: &[&str]) -> String {
    let assignment_count = 1usize << fields.len();
    (0..assignment_count)
        .map(|assignment| {
            let conjuncts = fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let bit = 1usize << (fields.len() - index - 1);
                    if assignment & bit != 0 {
                        format!("{subject}.{field}")
                    } else {
                        format!("not {subject}.{field}")
                    }
                })
                .collect::<Vec<_>>()
                .join(" and ");
            format!("({conjuncts})")
        })
        .collect::<Vec<_>>()
        .join(" or ")
}

fn bool_record_type(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| format!("{field}: Bool"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn partial_case_split_chain_predicate(subject: &str, fields: &[&str]) -> String {
    let mut disjuncts = Vec::new();
    for (index, field) in fields.iter().enumerate() {
        let mut conjuncts = fields[..index]
            .iter()
            .map(|field| format!("not {subject}.{field}"))
            .collect::<Vec<_>>();
        conjuncts.push(format!("{subject}.{field}"));
        disjuncts.push(format!("({})", conjuncts.join(" and ")));
    }
    disjuncts.push(format!(
        "({})",
        fields
            .iter()
            .map(|field| format!("not {subject}.{field}"))
            .collect::<Vec<_>>()
            .join(" and ")
    ));
    disjuncts.join(" or ")
}

mod calls_pipeline_and_float_types;
mod channel_effects_and_contract_predicates;
mod contract_case_split_proofs;
mod contract_order_path_proofs;
mod contract_order_split_proofs;
mod contract_rejections_and_result_scope;
mod contract_static_boolean_proofs;
mod declarations_and_names;
mod lowering_and_pattern_semantics;
mod prelude_and_callable_values;
mod satisfy_alias_and_order_repairs;
mod satisfy_boolean_and_bound_repairs;
mod satisfy_case_split_repairs;
mod satisfy_direct_repairs;
mod satisfy_disjunctive_repairs;
mod satisfy_literal_bound_repairs;
mod satisfy_negated_require_repairs;
mod satisfy_require_implication_repairs;
mod satisfy_tautology_repairs;
mod standard_library_effects;
mod typechecking_and_match_exhaustiveness;
