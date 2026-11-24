use super::*;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ConstructorPatternScanWork {
    pub(crate) traversed: usize,
    pub(crate) retained: usize,
    pub(crate) state_import_identities_indexed: usize,
    pub(crate) state_import_alias_lookups: usize,
}

#[cfg(test)]
thread_local! {
    static CONSTRUCTOR_PATTERN_SCAN_WORK: std::cell::Cell<ConstructorPatternScanWork> =
        const { std::cell::Cell::new(ConstructorPatternScanWork {
            traversed: 0,
            retained: 0,
            state_import_identities_indexed: 0,
            state_import_alias_lookups: 0,
        }) };
}

#[cfg(test)]
pub(crate) fn reset_constructor_pattern_scan_work() {
    CONSTRUCTOR_PATTERN_SCAN_WORK.set(ConstructorPatternScanWork::default());
}

#[cfg(test)]
pub(crate) fn take_constructor_pattern_scan_work() -> ConstructorPatternScanWork {
    CONSTRUCTOR_PATTERN_SCAN_WORK.replace(ConstructorPatternScanWork::default())
}

#[cfg(test)]
fn record_constructor_pattern_scan(retained: bool) {
    CONSTRUCTOR_PATTERN_SCAN_WORK.set({
        let mut work = CONSTRUCTOR_PATTERN_SCAN_WORK.get();
        work.traversed += 1;
        work.retained += usize::from(retained);
        work
    });
}

#[cfg(not(test))]
fn record_constructor_pattern_scan(_retained: bool) {}

#[cfg(test)]
pub(super) fn record_state_import_identity_indexed() {
    CONSTRUCTOR_PATTERN_SCAN_WORK.set({
        let mut work = CONSTRUCTOR_PATTERN_SCAN_WORK.get();
        work.state_import_identities_indexed += 1;
        work
    });
}

#[cfg(not(test))]
pub(super) fn record_state_import_identity_indexed() {}

#[cfg(test)]
pub(super) fn record_state_import_alias_lookup() {
    CONSTRUCTOR_PATTERN_SCAN_WORK.set({
        let mut work = CONSTRUCTOR_PATTERN_SCAN_WORK.get();
        work.state_import_alias_lookups += 1;
        work
    });
}

#[cfg(not(test))]
pub(super) fn record_state_import_alias_lookup() {}

pub(super) fn collect_constructor_patterns_from_body<'a>(
    body: &'a [BodyLine],
    output: &mut Vec<&'a Pattern>,
) {
    for line in body {
        match &line.kind {
            veln_ast::BodyLineKind::Let { pattern, expr, .. } => {
                collect_constructor_patterns(pattern, output);
                collect_constructor_patterns_from_expr(expr, output);
            }
            veln_ast::BodyLineKind::Expr { expr } => {
                collect_constructor_patterns_from_expr(expr, output);
            }
            veln_ast::BodyLineKind::Defer { body, .. } => {
                collect_constructor_patterns_from_body(body, output);
            }
        }
    }
}

pub(super) fn collect_constructor_patterns_from_expr<'a>(
    expr: &'a Expr,
    output: &mut Vec<&'a Pattern>,
) {
    if let ExprKind::Match { arms, .. } = &expr.kind {
        for arm in arms {
            collect_constructor_patterns(&arm.pattern, output);
        }
    }
    expr.for_each_child(&mut |child| {
        collect_constructor_patterns_from_expr(child, output);
    });
}

fn collect_constructor_patterns<'a>(pattern: &'a Pattern, output: &mut Vec<&'a Pattern>) {
    let mut pending = vec![pattern];
    while let Some(pattern) = pending.pop() {
        let retained = matches!(&pattern.kind, PatternKind::Constructor { .. });
        record_constructor_pattern_scan(retained);
        match &pattern.kind {
            PatternKind::Constructor { args, .. } => {
                output.push(pattern);
                pending.extend(args.iter().rev());
            }
            PatternKind::Record(fields) => {
                pending.extend(fields.iter().rev().map(|field| &field.pattern));
            }
            _ => {}
        }
    }
}
