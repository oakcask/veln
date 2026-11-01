use super::*;

pub(super) fn module_value_not_executable_diagnostic(
    value: &veln_ast::ModuleValueDecl,
) -> Diagnostic {
    let name = value.name.as_deref().unwrap_or("<missing>");
    let mut diagnostic = Diagnostic::new(
        "module.value_not_executable",
        Severity::Error,
        DiagnosticKind::Module,
        format!("module value declaration `{name}` is not executable"),
        Some(value.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_value")),
            ("name", JsonValue::string(name)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(
            "The declaration is preserved in checked core and typed IR, but runtime materialization and code generation do not support module values yet.",
        ),
    )]));
    diagnostic
}

#[derive(Clone, Copy)]
struct DiagnosticSpan<'a>(&'a veln_source::SourceSpan);

impl PartialEq for DiagnosticSpan<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for DiagnosticSpan<'_> {}

impl Hash for DiagnosticSpan<'_> {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        self.0.file.hash(state);
        self.0.start.line.hash(state);
        self.0.start.column.hash(state);
        self.0.start.offset.hash(state);
        self.0.end.line.hash(state);
        self.0.end.column.hash(state);
        self.0.end.offset.hash(state);
        self.0.original_path().hash(state);
        self.0
            .original_start()
            .map(|position| (position.line, position.column, position.offset))
            .hash(state);
        self.0
            .original_end()
            .map(|position| (position.line, position.column, position.offset))
            .hash(state);
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleValueGateWork {
    pub(crate) indexed_values: usize,
    pub(crate) scanned_diagnostics: usize,
    pub(crate) span_index_lookups: usize,
    pub(crate) emitted_canonical_diagnostics: usize,
}

#[cfg(test)]
impl ModuleValueGateWork {
    pub(crate) fn total(self) -> usize {
        self.indexed_values
            + self.scanned_diagnostics
            + self.span_index_lookups
            + self.emitted_canonical_diagnostics
    }
}

#[cfg(test)]
thread_local! {
    static MODULE_VALUE_GATE_WORK: std::cell::Cell<ModuleValueGateWork> =
        const { std::cell::Cell::new(ModuleValueGateWork {
            indexed_values: 0,
            scanned_diagnostics: 0,
            span_index_lookups: 0,
            emitted_canonical_diagnostics: 0,
        }) };
}

#[cfg(test)]
fn record_module_value_gate_work(update: impl FnOnce(&mut ModuleValueGateWork)) {
    MODULE_VALUE_GATE_WORK.with(|work| {
        let mut current = work.get();
        update(&mut current);
        work.set(current);
    });
}

#[cfg(test)]
pub(crate) fn reset_module_value_gate_work() {
    MODULE_VALUE_GATE_WORK.set(ModuleValueGateWork::default());
}

#[cfg(test)]
pub(crate) fn take_module_value_gate_work() -> ModuleValueGateWork {
    MODULE_VALUE_GATE_WORK.replace(ModuleValueGateWork::default())
}

pub(super) fn enforce_module_value_lowering_gate(
    module: &SurfaceModule,
    diagnostics: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    let mut value_by_span = HashMap::with_capacity(module.module_values.len());
    for (index, value) in module.module_values.iter().enumerate() {
        value_by_span.insert(DiagnosticSpan(&value.span), index);
        #[cfg(test)]
        record_module_value_gate_work(|work| work.indexed_values += 1);
    }

    let mut emitted = vec![false; module.module_values.len()];
    let mut repaired = Vec::with_capacity(diagnostics.len() + module.module_values.len());
    for diagnostic in diagnostics {
        #[cfg(test)]
        record_module_value_gate_work(|work| work.scanned_diagnostics += 1);
        let value_index = if diagnostic.id == "module.value_not_executable" {
            diagnostic.span.as_ref().and_then(|span| {
                #[cfg(test)]
                record_module_value_gate_work(|work| work.span_index_lookups += 1);
                value_by_span.get(&DiagnosticSpan(span)).copied()
            })
        } else {
            None
        };
        let Some(value_index) = value_index else {
            repaired.push(diagnostic);
            continue;
        };
        if emitted[value_index] {
            continue;
        }
        repaired.push(module_value_not_executable_diagnostic(
            &module.module_values[value_index],
        ));
        emitted[value_index] = true;
        #[cfg(test)]
        record_module_value_gate_work(|work| work.emitted_canonical_diagnostics += 1);
    }

    for (value_index, value) in module.module_values.iter().enumerate() {
        if emitted[value_index] {
            continue;
        }
        repaired.push(module_value_not_executable_diagnostic(value));
        #[cfg(test)]
        record_module_value_gate_work(|work| work.emitted_canonical_diagnostics += 1);
    }
    repaired
}
