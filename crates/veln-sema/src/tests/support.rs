pub(super) use crate::*;
pub(super) use veln_ast::{SurfaceModule, lower_surface_ast};
pub(super) use veln_core::{
    ContractObligationStatus, CoreBlocker, CoreCallTarget, CoreExprKind, CorePatternKind,
    CoreReadiness, CoreStmtKind, CoreType,
};
pub(super) use veln_diagnostics::{Diagnostic, DiagnosticKind};
pub(super) use veln_ir::{IrCallTarget, IrExprKind, IrPatternKind, IrStmtKind};
pub(super) use veln_source::SourceFile;
pub(super) use veln_syntax::parse;

pub(super) fn exhaustive_case_split_predicate(subject: &str, fields: &[&str]) -> String {
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

pub(super) fn bool_record_type(fields: &[&str]) -> String {
    fields
        .iter()
        .map(|field| format!("{field}: Bool"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn assert_diagnostic_span(
    diagnostic: &Diagnostic,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
) {
    let span = diagnostic
        .span
        .as_ref()
        .expect("diagnostic should have a span");
    assert_eq!(
        (
            span.start.line,
            span.start.column,
            span.end.line,
            span.end.column
        ),
        (start_line, start_column, end_line, end_column)
    );
}

pub(super) fn partial_case_split_chain_predicate(subject: &str, fields: &[&str]) -> String {
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
