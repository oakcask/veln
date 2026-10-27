use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::Hash;

use veln_ast::{
    Function, FunctionKind, PublicAlias, PublicAliasKind, SurfaceModule, TypeDecl, Visibility,
};
use veln_diagnostics::Diagnostic;

use crate::types::TypeEnvironment;

use super::{FunctionPublicationIdentity, PublicationDiagnosticCause, UnpublishedDeclaration};

type AdmittedPublicationCauses<Key> = HashMap<Key, HashSet<PublicationDiagnosticCause>>;
type PublicationCauseWorklist<Key> = VecDeque<(Key, PublicationDiagnosticCause)>;

pub(super) fn unpublished_public_declarations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
) -> Vec<UnpublishedDeclaration> {
    let type_causes = type_publication_diagnostic_causes(module, environment, diagnostics);
    let function_causes =
        function_publication_diagnostic_causes(module, environment, diagnostics, &type_causes);
    let mut declarations = module
        .types
        .iter()
        .filter_map(|declaration| {
            unpublished_type_declaration(declaration, environment, &type_causes)
        })
        .collect::<Vec<_>>();
    declarations.extend(module.functions.iter().filter_map(|function| {
        unpublished_function_declaration(function, environment, &function_causes)
    }));
    declarations.extend(module.aliases.iter().filter_map(|alias| {
        unpublished_alias_declaration(alias, environment, &type_causes, &function_causes)
    }));
    declarations
}

fn unpublished_type_declaration(
    declaration: &TypeDecl,
    environment: &TypeEnvironment,
    type_causes: &HashMap<String, Vec<PublicationDiagnosticCause>>,
) -> Option<UnpublishedDeclaration> {
    if declaration.visibility != Visibility::Public {
        return None;
    }
    let name = declaration.name.as_ref()?;
    let descriptor = environment
        .adts
        .descriptor_for_type_path_including_quarantined(
            name,
            declaration.params.len(),
            declaration.module_name.as_deref(),
            &environment.uses,
        )?;
    environment
        .adts
        .descriptor_is_quarantined(descriptor)
        .then(|| UnpublishedDeclaration {
            kind: "type",
            module_name: declaration.module_name.clone(),
            name: name.clone(),
            span: declaration.span.clone(),
            causal_diagnostics: type_causes
                .get(&descriptor.identity())
                .cloned()
                .unwrap_or_default(),
        })
}

fn unpublished_function_declaration(
    function: &Function,
    environment: &TypeEnvironment,
    function_causes: &HashMap<FunctionPublicationIdentity, Vec<PublicationDiagnosticCause>>,
) -> Option<UnpublishedDeclaration> {
    if function.kind != FunctionKind::Function
        || function.visibility != Visibility::Public
        || environment.function_is_published(function)
    {
        return None;
    }
    let name = function.name.as_ref()?;
    Some(UnpublishedDeclaration {
        kind: "function",
        module_name: function.module_name.clone(),
        name: name.clone(),
        span: function.span.clone(),
        causal_diagnostics: function_causes
            .get(&(function.module_name.clone(), name.clone()))
            .cloned()
            .unwrap_or_default(),
    })
}

fn unpublished_alias_declaration(
    alias: &PublicAlias,
    environment: &TypeEnvironment,
    type_causes: &HashMap<String, Vec<PublicationDiagnosticCause>>,
    function_causes: &HashMap<FunctionPublicationIdentity, Vec<PublicationDiagnosticCause>>,
) -> Option<UnpublishedDeclaration> {
    match alias.kind {
        PublicAliasKind::Type => unpublished_type_alias(alias, environment, type_causes),
        PublicAliasKind::Function => {
            unpublished_function_alias(alias, environment, function_causes)
        }
        _ => None,
    }
}

fn unpublished_type_alias(
    alias: &PublicAlias,
    environment: &TypeEnvironment,
    type_causes: &HashMap<String, Vec<PublicationDiagnosticCause>>,
) -> Option<UnpublishedDeclaration> {
    let name = alias.name.as_ref()?;
    let descriptor = environment.adts.unique_descriptor_for_type_path_any_arity(
        name,
        alias.module_name.as_deref(),
        &environment.uses,
    )?;
    if !environment.adts.descriptor_is_quarantined(descriptor) {
        return None;
    }
    let terminal = environment
        .adts
        .terminal_descriptor_for_alias(descriptor)
        .unwrap_or_else(|| environment.adts.canonical_descriptor_for(descriptor));
    Some(UnpublishedDeclaration {
        kind: "type_alias",
        module_name: alias.module_name.clone(),
        name: name.clone(),
        span: alias.span.clone(),
        causal_diagnostics: type_causes
            .get(&terminal.identity())
            .cloned()
            .unwrap_or_default(),
    })
}

fn unpublished_function_alias(
    alias: &PublicAlias,
    environment: &TypeEnvironment,
    function_causes: &HashMap<FunctionPublicationIdentity, Vec<PublicationDiagnosticCause>>,
) -> Option<UnpublishedDeclaration> {
    let name = alias.name.as_ref()?;
    if environment.function(name).is_some() {
        return None;
    }
    Some(UnpublishedDeclaration {
        kind: "function_alias",
        module_name: alias.module_name.clone(),
        name: name.clone(),
        span: alias.span.clone(),
        causal_diagnostics: function_causes
            .get(&(alias.module_name.clone(), name.clone()))
            .cloned()
            .unwrap_or_default(),
    })
}

fn type_publication_diagnostic_causes(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
) -> HashMap<String, Vec<PublicationDiagnosticCause>> {
    let mut causes = HashMap::new();
    let mut entries = Vec::new();
    for (entry, declaration_causes) in module
        .types
        .iter()
        .filter_map(|declaration| public_type_cause_entry(declaration, environment, diagnostics))
    {
        causes.insert(entry.identity.clone(), declaration_causes);
        entries.push(entry);
    }

    let (declaration_order, reverse_dependencies) = type_publication_dependencies(entries);
    propagate_publication_causes(&declaration_order, &reverse_dependencies, &mut causes);
    causes
}

struct TypeCauseEntry {
    identity: String,
    dependencies: Vec<String>,
}

fn public_type_cause_entry(
    declaration: &TypeDecl,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
) -> Option<(TypeCauseEntry, Vec<PublicationDiagnosticCause>)> {
    if declaration.visibility != Visibility::Public {
        return None;
    }
    let name = declaration.name.as_ref()?;
    let descriptor = environment
        .adts
        .descriptor_for_type_path_including_quarantined(
            name,
            declaration.params.len(),
            declaration.module_name.as_deref(),
            &environment.uses,
        )?;
    let identity = descriptor.identity();
    let dependencies =
        crate::analysis::adt_publication_dependency_identities(declaration, environment)
            .into_iter()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
    Some((
        TypeCauseEntry {
            identity,
            dependencies,
        },
        diagnostic_causes_within(&declaration.span, diagnostics),
    ))
}

fn type_publication_dependencies(
    entries: Vec<TypeCauseEntry>,
) -> (Vec<String>, HashMap<String, Vec<String>>) {
    let mut declaration_order = Vec::with_capacity(entries.len());
    let mut reverse_dependencies = HashMap::<String, Vec<String>>::new();
    for entry in entries {
        declaration_order.push(entry.identity.clone());
        for dependency in entry.dependencies {
            reverse_dependencies
                .entry(dependency)
                .or_default()
                .push(entry.identity.clone());
            #[cfg(test)]
            super::record_publication_propagation_work(|work| work.reverse_edges += 1);
        }
    }
    (declaration_order, reverse_dependencies)
}

fn function_publication_diagnostic_causes(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
    type_causes: &HashMap<String, Vec<PublicationDiagnosticCause>>,
) -> HashMap<FunctionPublicationIdentity, Vec<PublicationDiagnosticCause>> {
    let import_index = module
        .aliases
        .iter()
        .any(|alias| alias.kind == PublicAliasKind::Function && alias.target.len() > 1)
        .then(|| crate::name_recovery::NormalImportSelectionIndex::new(module));
    let mut causes = HashMap::new();
    let mut declaration_order = Vec::new();
    for (key, declaration_causes) in module.functions.iter().filter_map(|function| {
        public_function_diagnostic_causes(function, environment, diagnostics, type_causes)
    }) {
        declaration_order.push(key.clone());
        causes.insert(key, declaration_causes);
    }
    let mut reverse_dependencies: HashMap<
        FunctionPublicationIdentity,
        Vec<FunctionPublicationIdentity>,
    > = HashMap::new();
    for (key, target) in module
        .aliases
        .iter()
        .filter_map(|alias| function_alias_publication_dependency(import_index.as_ref(), alias))
    {
        declaration_order.push(key.clone());
        causes.entry(key.clone()).or_default();
        let Some(target) = target else {
            continue;
        };
        reverse_dependencies.entry(target).or_default().push(key);
        #[cfg(test)]
        super::record_publication_propagation_work(|work| work.reverse_edges += 1);
    }
    propagate_publication_causes(&declaration_order, &reverse_dependencies, &mut causes);
    causes
}

fn public_function_diagnostic_causes(
    function: &Function,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
    type_causes: &HashMap<String, Vec<PublicationDiagnosticCause>>,
) -> Option<(FunctionPublicationIdentity, Vec<PublicationDiagnosticCause>)> {
    if function.kind != FunctionKind::Function || function.visibility != Visibility::Public {
        return None;
    }
    let name = function.name.as_ref()?;
    let mut declaration_causes = diagnostic_causes_within(&function.span, diagnostics);
    let dependencies =
        crate::analysis::function_publication_dependency_identities(function, environment)
            .into_iter()
            .collect::<BTreeSet<_>>();
    let mut admitted_causes = declaration_causes.iter().cloned().collect::<HashSet<_>>();
    for cause in dependencies
        .iter()
        .flat_map(|dependency| type_causes.get(dependency).cloned().unwrap_or_default())
    {
        if admitted_causes.insert(cause.clone()) {
            declaration_causes.push(cause);
        }
    }
    Some((
        (function.module_name.clone(), name.clone()),
        declaration_causes,
    ))
}

fn function_alias_publication_dependency(
    import_index: Option<&crate::name_recovery::NormalImportSelectionIndex<'_>>,
    alias: &PublicAlias,
) -> Option<(
    FunctionPublicationIdentity,
    Option<FunctionPublicationIdentity>,
)> {
    if alias.kind != PublicAliasKind::Function {
        return None;
    }
    let name = alias.name.as_ref()?;
    Some((
        (alias.module_name.clone(), name.clone()),
        function_alias_target_key(import_index, alias),
    ))
}

fn propagate_publication_causes<Key>(
    declaration_order: &[Key],
    reverse_dependencies: &HashMap<Key, Vec<Key>>,
    causes: &mut HashMap<Key, Vec<PublicationDiagnosticCause>>,
) where
    Key: Clone + Eq + Hash,
{
    let (mut admitted, mut worklist) = seed_publication_cause_worklist(declaration_order, causes);
    while let Some((dependency, cause)) = worklist.pop_front() {
        propagate_publication_cause(
            &dependency,
            &cause,
            reverse_dependencies,
            &mut admitted,
            causes,
            &mut worklist,
        );
    }
}

fn seed_publication_cause_worklist<Key>(
    declaration_order: &[Key],
    causes: &HashMap<Key, Vec<PublicationDiagnosticCause>>,
) -> (
    AdmittedPublicationCauses<Key>,
    PublicationCauseWorklist<Key>,
)
where
    Key: Clone + Eq + Hash,
{
    let mut admitted = HashMap::<Key, HashSet<PublicationDiagnosticCause>>::new();
    let mut worklist = VecDeque::new();
    for declaration in declaration_order {
        let declaration_causes = causes.get(declaration).cloned().unwrap_or_default();
        let declaration_admitted = admitted.entry(declaration.clone()).or_default();
        for cause in declaration_causes {
            if declaration_admitted.insert(cause.clone()) {
                worklist.push_back((declaration.clone(), cause));
                #[cfg(test)]
                super::record_publication_propagation_work(|work| work.admitted_cause_pairs += 1);
            }
        }
    }
    (admitted, worklist)
}

fn propagate_publication_cause<Key>(
    dependency: &Key,
    cause: &PublicationDiagnosticCause,
    reverse_dependencies: &HashMap<Key, Vec<Key>>,
    admitted: &mut AdmittedPublicationCauses<Key>,
    causes: &mut HashMap<Key, Vec<PublicationDiagnosticCause>>,
    worklist: &mut PublicationCauseWorklist<Key>,
) where
    Key: Clone + Eq + Hash,
{
    let Some(dependents) = reverse_dependencies.get(dependency) else {
        return;
    };
    for dependent in dependents {
        #[cfg(test)]
        super::record_publication_propagation_work(|work| work.propagation_attempts += 1);
        if admitted
            .entry(dependent.clone())
            .or_default()
            .insert(cause.clone())
        {
            causes
                .entry(dependent.clone())
                .or_default()
                .push(cause.clone());
            worklist.push_back((dependent.clone(), cause.clone()));
            #[cfg(test)]
            super::record_publication_propagation_work(|work| work.admitted_cause_pairs += 1);
        }
    }
}

fn function_alias_target_key(
    import_index: Option<&crate::name_recovery::NormalImportSelectionIndex<'_>>,
    alias: &veln_ast::PublicAlias,
) -> Option<(Option<String>, String)> {
    let name = alias.target.last()?.clone();
    if alias.target.len() == 1 {
        return Some((alias.module_name.clone(), name));
    }
    let use_decl = import_index?.imported_use_for_path(
        &alias.target[..alias.target.len() - 1],
        alias.module_name.as_deref(),
        alias.package_name.as_deref(),
    )?;
    Some((
        Some(crate::name_recovery::resolved_import_module_name(
            use_decl,
            alias.module_name.as_deref(),
        )),
        name,
    ))
}

fn diagnostic_causes_within(
    declaration_span: &veln_source::SourceSpan,
    diagnostics: &[Diagnostic],
) -> Vec<PublicationDiagnosticCause> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .filter_map(|diagnostic| {
            let span = diagnostic.span.as_ref()?;
            (span.file == declaration_span.file
                && span.start.offset >= declaration_span.start.offset
                && span.end.offset <= declaration_span.end.offset)
                .then(|| PublicationDiagnosticCause {
                    diagnostic_id: diagnostic.id.clone(),
                    span: span.clone(),
                })
        })
        .collect()
}
