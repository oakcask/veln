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

mod part01;
mod part02;
mod part03;
mod part04;
mod part05;
mod part06;
mod part07;
mod part08;
mod part09;
mod part10;
mod part11;
mod part12;
mod part13;
mod part14;
mod part15;
mod part16;
mod part17;
mod part18;
mod part19;
mod part20;
mod part21;
