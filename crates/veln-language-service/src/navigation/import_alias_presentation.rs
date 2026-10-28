use crate::presentation::{
    CompletionCandidate, CompletionCandidateKind, ImportAliasPresentation,
    ImportAliasPresentationSelection, ImportAliasProvenance,
};

pub(crate) fn explicit_import_alias_presentation_at(
    snapshot: &EffectiveProjectSnapshot,
    position: &SourcePosition,
) -> Option<ImportAliasPresentation> {
    let index = snapshot.navigation_index();
    let file = index
        .files
        .iter()
        .find(|file| file.source.path() == &position.source)?;
    if file.navigation_isolated {
        return None;
    }
    let offset = offset_for_position(file.source.text(), position)?;
    let (token_index, _) = identifier_token_at(&file.tokens, offset)?;
    let request = Arc::clone(&index).symbol_at_position(position.source.as_str(), position)?;

    let (alias, selection) = match request.symbol {
        Symbol::ImportAlias(alias) => {
            let alias = unique_presentation_alias(&index, &alias.module, &alias.name)?;
            (alias, ImportAliasPresentationSelection::Alias)
        }
        Symbol::Recovery(_) => return None,
        symbol => {
            let root_index = qualified_path_root_index(&file.tokens, token_index)?;
            if root_index == token_index {
                return None;
            }
            let root_range = file.tokens[root_index].range;
            let alias_declaration = file
                .explicit_import_alias_occurrences
                .get(&(root_range.start, root_range.end))?;
            let qualifier = &file.tokens[root_index].text;
            let alias = unique_presentation_alias(&index, &file.module, qualifier)?;
            if alias.declaration.span != *alias_declaration {
                return None;
            }
            (
                alias,
                ImportAliasPresentationSelection::Member {
                    name: symbol.name().to_string(),
                    kind: completion_kind_for_symbol(&symbol)?,
                },
            )
        }
    };
    Some(ImportAliasPresentation {
        provenance: alias_provenance(alias),
        selection,
    })
}

pub(crate) fn explicit_import_alias_completion(
    snapshot: &EffectiveProjectSnapshot,
    position: &SourcePosition,
    tokens: &[Token],
    offset: usize,
) -> Option<Vec<CompletionCandidate>> {
    let separator_index = tokens
        .iter()
        .enumerate()
        .take_while(|(_, token)| token.range.end <= offset)
        .filter(|(_, token)| !is_layout_token(token))
        .map(|(index, _)| index)
        .last()?;
    if tokens[separator_index].kind != TokenKind::DoubleColon {
        return None;
    }
    let qualifier_index = previous_non_layout_index(tokens, separator_index)?;
    if !tokens[qualifier_index].kind.is_contextual_identifier() {
        return None;
    }
    if previous_path_segment_index(tokens, qualifier_index).is_some() {
        return None;
    }
    let index = snapshot.navigation_index();
    let file = index
        .files
        .iter()
        .find(|file| file.source.path() == &position.source)?;
    if file.navigation_isolated {
        return Some(Vec::new());
    }
    let alias = unique_presentation_alias(&index, &file.module, &tokens[qualifier_index].text)?;
    Some(alias_completion_candidates(&index, alias))
}

fn qualified_path_root_index(tokens: &[Token], mut index: usize) -> Option<usize> {
    let mut qualified = false;
    while let Some(previous) = previous_path_segment_index(tokens, index) {
        qualified = true;
        index = previous;
    }
    qualified.then_some(index)
}

fn unique_presentation_alias<'a>(
    index: &'a SymbolIndex,
    module: &str,
    qualifier: &str,
) -> Option<&'a ImportAliasSymbol> {
    let mut aliases = index
        .import_aliases
        .iter()
        .filter(|alias| alias.module == module && alias.name == qualifier);
    let alias = aliases.next()?;
    aliases.next().is_none().then_some(alias)
}

fn alias_provenance(alias: &ImportAliasSymbol) -> ImportAliasProvenance {
    ImportAliasProvenance {
        qualifier: alias.name.clone(),
        package: alias.target_package.clone(),
        target_module: alias.target_module.clone(),
    }
}

fn alias_completion_candidates(
    index: &SymbolIndex,
    alias: &ImportAliasSymbol,
) -> Vec<CompletionCandidate> {
    let provenance = alias_provenance(alias);
    let mut candidates = Vec::new();
    candidates.extend(index.functions.iter().filter_map(|symbol| {
        (symbol.public
            && !symbol.invalid_declaration_name
            && presentation_target_matches(
                &symbol.module,
                symbol.package.as_deref(),
                alias,
            ))
        .then(|| completion_candidate(&symbol.name, CompletionCandidateKind::Function, &provenance))
    }));
    candidates.extend(index.types.iter().filter_map(|symbol| {
        (symbol.public
            && presentation_target_matches(
                &symbol.module,
                symbol.package.as_deref(),
                alias,
            ))
        .then(|| completion_candidate(&symbol.name, CompletionCandidateKind::Type, &provenance))
    }));
    candidates.extend(index.type_aliases.iter().filter_map(|symbol| {
        presentation_target_matches(&symbol.module, symbol.package.as_deref(), alias)
            .then(|| completion_candidate(&symbol.name, CompletionCandidateKind::Type, &provenance))
    }));
    candidates.extend(index.constructors.iter().filter_map(|symbol| {
        (symbol.public
            && presentation_target_matches(
                &symbol.module,
                symbol.package.as_deref(),
                alias,
            ))
        .then(|| completion_candidate(&symbol.name, CompletionCandidateKind::Constructor, &provenance))
    }));
    candidates.sort_by(|left, right| {
        left.label
            .cmp(&right.label)
            .then(completion_kind_rank(left.kind).cmp(&completion_kind_rank(right.kind)))
    });
    candidates.dedup_by(|left, right| left.label == right.label && left.kind == right.kind);
    candidates
}

fn presentation_target_matches(
    module: &str,
    package: Option<&str>,
    alias: &ImportAliasSymbol,
) -> bool {
    module == alias.target_module && package == alias.target_package.as_deref()
}

fn completion_candidate(
    name: &str,
    kind: CompletionCandidateKind,
    provenance: &ImportAliasProvenance,
) -> CompletionCandidate {
    let package = provenance.package.as_deref().unwrap_or("workspace");
    CompletionCandidate {
        label: name.to_string(),
        detail: format!(
            "{} from package {package}, module {} via {}",
            completion_kind_name(kind),
            provenance.target_module,
            provenance.qualifier,
        ),
        kind,
        import_alias: Some(provenance.clone()),
    }
}

fn completion_kind_for_symbol(symbol: &Symbol) -> Option<CompletionCandidateKind> {
    match symbol {
        Symbol::Type(_) | Symbol::TypeAlias(_) => Some(CompletionCandidateKind::Type),
        Symbol::Function(_) => Some(CompletionCandidateKind::Function),
        Symbol::Constructor(_) => Some(CompletionCandidateKind::Constructor),
        Symbol::ImportAlias(_)
        | Symbol::Schema(_)
        | Symbol::SchemaAlias(_)
        | Symbol::Effect(_)
        | Symbol::Handler(_)
        | Symbol::EffectOperation(_)
        | Symbol::Local(_)
        | Symbol::Recovery(_) => None,
    }
}

pub(crate) fn completion_kind_name(kind: CompletionCandidateKind) -> &'static str {
    match kind {
        CompletionCandidateKind::DeclarationModifier => "modifier",
        CompletionCandidateKind::BuiltinLocal => "built-in local",
        CompletionCandidateKind::Function => "function",
        CompletionCandidateKind::Type => "type",
        CompletionCandidateKind::Constructor => "constructor",
    }
}

fn completion_kind_rank(kind: CompletionCandidateKind) -> u8 {
    match kind {
        CompletionCandidateKind::Function => 0,
        CompletionCandidateKind::Type => 1,
        CompletionCandidateKind::Constructor => 2,
        CompletionCandidateKind::DeclarationModifier => 3,
        CompletionCandidateKind::BuiltinLocal => 4,
    }
}
