use std::collections::{BTreeMap, BTreeSet};

use veln_ir::{CoreModuleValueIdentity, IrExpr, IrExprKind, TypedProgram};

fn collect_dependencies(expr: &IrExpr, dependencies: &mut BTreeSet<CoreModuleValueIdentity>) {
    let mut pending = vec![expr];
    while let Some(expr) = pending.pop() {
        match &expr.kind {
            IrExprKind::ModuleValue(identity) => {
                dependencies.insert(identity.clone());
            }
            IrExprKind::Record(fields) => {
                pending.extend(fields.iter().rev().map(|field| &field.value));
            }
            IrExprKind::AdtVariant { payloads, .. } => {
                pending.extend(payloads.iter().rev());
            }
            IrExprKind::OptionSome(value)
            | IrExprKind::ResultOk(value)
            | IrExprKind::ResultErr(value)
            | IrExprKind::Prefix { expr: value, .. } => pending.push(value),
            IrExprKind::ListCons { head, tail } => {
                pending.push(tail);
                pending.push(head);
            }
            IrExprKind::BoolLiteral(_)
            | IrExprKind::StringLiteral(_)
            | IrExprKind::IntLiteral(_)
            | IrExprKind::OptionNone
            | IrExprKind::ListNil
            | IrExprKind::Unit => {}
            kind => {
                unreachable!("unsupported module value initializer reached JVM emission: {kind:?}")
            }
        }
    }
}

struct DependencyGraph {
    dependents: Vec<Vec<usize>>,
    remaining_dependencies: Vec<usize>,
}

impl DependencyGraph {
    fn new(program: &TypedProgram) -> Self {
        let indexes = program
            .module_values
            .iter()
            .enumerate()
            .map(|(index, declaration)| (declaration.identity.clone(), index))
            .collect::<BTreeMap<_, _>>();
        let mut dependents = vec![Vec::new(); program.module_values.len()];
        let mut remaining_dependencies = Vec::with_capacity(program.module_values.len());

        for (index, declaration) in program.module_values.iter().enumerate() {
            let mut identities = BTreeSet::new();
            collect_dependencies(&declaration.initializer, &mut identities);
            let dependencies = identities
                .into_iter()
                .map(|identity| {
                    indexes.get(&identity).copied().unwrap_or_else(|| {
                        panic!("missing module value declaration for {identity:?}")
                    })
                })
                .collect::<BTreeSet<_>>();
            for dependency in &dependencies {
                dependents[*dependency].push(index);
            }
            remaining_dependencies.push(dependencies.len());
        }

        Self {
            dependents,
            remaining_dependencies,
        }
    }

    fn into_dependency_order(mut self) -> Vec<usize> {
        let mut ready = self
            .remaining_dependencies
            .iter()
            .enumerate()
            .filter_map(|(index, count)| (*count == 0).then_some(index))
            .collect::<BTreeSet<_>>();
        let mut ordered = Vec::with_capacity(self.remaining_dependencies.len());
        while let Some(index) = ready.pop_first() {
            ordered.push(index);
            for dependent in &self.dependents[index] {
                self.remaining_dependencies[*dependent] -= 1;
                if self.remaining_dependencies[*dependent] == 0 {
                    ready.insert(*dependent);
                }
            }
        }
        assert_eq!(
            ordered.len(),
            self.remaining_dependencies.len(),
            "module value initializers must be acyclic before JVM emission"
        );
        ordered
    }
}

pub(super) fn module_values_in_dependency_order(program: &TypedProgram) -> Vec<usize> {
    DependencyGraph::new(program).into_dependency_order()
}
