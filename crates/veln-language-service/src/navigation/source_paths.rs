fn identifier_token_at(tokens: &[Token], offset: usize) -> Option<(usize, &Token)> {
    tokens.iter().enumerate().find(|(_, token)| {
        token.kind.is_contextual_identifier()
            && offset >= token.range.start
            && offset < token.range.end
            && is_identifier(&token.text)
    })
}

fn qualifier_for_token(tokens: &[Token], name_index: usize) -> Option<String> {
    let separator_index = previous_non_layout_index(tokens, name_index)?;
    if tokens[separator_index].kind != TokenKind::DoubleColon {
        return None;
    }
    let segment_index = previous_non_layout_index(tokens, separator_index)?;
    let mut segments = vec![tokens[segment_index].text.as_str()];
    let mut cursor = segment_index;
    while let Some(previous_separator) = previous_non_layout_index(tokens, cursor) {
        if tokens[previous_separator].kind != TokenKind::DoubleColon {
            break;
        }
        let Some(previous_segment) = previous_non_layout_index(tokens, previous_separator) else {
            break;
        };
        segments.push(tokens[previous_segment].text.as_str());
        cursor = previous_segment;
    }
    segments.reverse();
    Some(segments.join("::"))
}

fn variant_refinement_qualifier_for_token(tokens: &[Token], name_index: usize) -> Option<String> {
    let separator_index = previous_non_layout_index(tokens, name_index)?;
    if tokens[separator_index].kind != TokenKind::DoubleColon {
        return None;
    }
    let mut segment_index = previous_non_layout_index(tokens, separator_index)?;
    if tokens[segment_index].kind == TokenKind::Greater {
        let mut depth = 1usize;
        while depth > 0 {
            segment_index = previous_non_layout_index(tokens, segment_index)?;
            match tokens[segment_index].kind {
                TokenKind::Greater => depth += 1,
                TokenKind::Less => depth -= 1,
                _ => {}
            }
        }
        segment_index = previous_non_layout_index(tokens, segment_index)?;
    }
    if !tokens[segment_index].kind.is_contextual_identifier()
        || !is_identifier(&tokens[segment_index].text)
    {
        return None;
    }
    let mut segments = vec![tokens[segment_index].text.as_str()];
    let mut cursor = segment_index;
    while let Some(previous_separator) = previous_non_layout_index(tokens, cursor) {
        if tokens[previous_separator].kind != TokenKind::DoubleColon {
            break;
        }
        let Some(previous_segment) = previous_non_layout_index(tokens, previous_separator) else {
            break;
        };
        if !tokens[previous_segment].kind.is_contextual_identifier()
            || !is_identifier(&tokens[previous_segment].text)
        {
            break;
        }
        segments.push(tokens[previous_segment].text.as_str());
        cursor = previous_segment;
    }
    segments.reverse();
    Some(segments.join("::"))
}

fn variant_refinement_variant_index(tokens: &[Token], base_index: usize) -> Option<usize> {
    let mut cursor = next_non_layout_index(tokens, base_index)?;
    if tokens[cursor].kind == TokenKind::Less {
        let mut depth = 1usize;
        while depth > 0 {
            cursor = next_non_layout_index(tokens, cursor)?;
            match tokens[cursor].kind {
                TokenKind::Less => depth += 1,
                TokenKind::Greater => depth -= 1,
                _ => {}
            }
        }
        cursor = next_non_layout_index(tokens, cursor)?;
    }
    if tokens[cursor].kind != TokenKind::DoubleColon {
        return None;
    }
    let variant_index = next_non_layout_index(tokens, cursor)?;
    (tokens[variant_index].kind.is_contextual_identifier()
        && is_identifier(&tokens[variant_index].text))
    .then_some(variant_index)
}

fn qualified_reference_matches(
    tokens: &[Token],
    name_index: usize,
    module_segments: &[&str],
) -> bool {
    let mut expected_index = name_index;
    for expected_segment in module_segments.iter().rev() {
        let Some(separator_index) = previous_non_layout_index(tokens, expected_index) else {
            return false;
        };
        if tokens[separator_index].kind != TokenKind::DoubleColon {
            return false;
        }
        let Some(segment_index) = previous_non_layout_index(tokens, separator_index) else {
            return false;
        };
        if tokens[segment_index].text != *expected_segment {
            return false;
        }
        expected_index = segment_index;
    }
    previous_non_layout_token(tokens, expected_index)
        .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
}

fn next_path_segment_index(tokens: &[Token], index: usize) -> Option<usize> {
    let separator_index = next_non_layout_index(tokens, index)?;
    if tokens[separator_index].kind != TokenKind::DoubleColon {
        return None;
    }
    let segment_index = next_non_layout_index(tokens, separator_index)?;
    (tokens[segment_index].kind.is_contextual_identifier()
        && is_identifier(&tokens[segment_index].text))
    .then_some(segment_index)
}

fn previous_path_segment_index(tokens: &[Token], index: usize) -> Option<usize> {
    let separator_index = previous_non_layout_index(tokens, index)?;
    (tokens[separator_index].kind == TokenKind::DoubleColon)
        .then(|| previous_non_layout_index(tokens, separator_index))
        .flatten()
}

fn next_non_layout_token(tokens: &[Token], index: usize) -> Option<&Token> {
    next_non_layout_index(tokens, index).map(|index| &tokens[index])
}

fn next_non_layout_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[index + 1..]
        .iter()
        .position(|token| !is_layout_token(token))
        .map(|relative_index| index + 1 + relative_index)
}

fn previous_non_layout_token(tokens: &[Token], index: usize) -> Option<&Token> {
    let previous = previous_non_layout_index(tokens, index)?;
    Some(&tokens[previous])
}

fn previous_non_layout_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, token)| !is_layout_token(token))
        .map(|(index, _)| index)
}

fn is_layout_token(token: &Token) -> bool {
    is_layout_token_kind(token.kind)
}

fn is_layout_token_kind(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Whitespace | TokenKind::Newline)
}

fn explicit_module_name(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("mod ")?;
        leading_module_path(rest).map(str::to_string)
    })
}

fn module_name_from_path(path: &str) -> Option<String> {
    Some(path.strip_suffix(".veln")?.replace('/', "::"))
}

struct SourceIdentity {
    module: String,
    navigation_isolated: bool,
}

impl SourceIdentity {
    fn new(path: &str, text: &str) -> Self {
        let path_module = module_name_from_path(path);
        let navigation_isolated = path_module_invalid_for_navigation(path_module.as_deref());
        let module = explicit_module_name(text).or(path_module).unwrap_or_default();
        Self {
            module,
            navigation_isolated,
        }
    }
}

struct UseModuleIndex {
    local_modules: BTreeSet<String>,
    external_modules: BTreeSet<(String, String)>,
    local_aliases: BTreeMap<String, String>,
    external_aliases: BTreeMap<String, (String, String)>,
}

impl UseModuleIndex {
    fn new(text: &str) -> Self {
        let mut index = Self {
            local_modules: BTreeSet::new(),
            external_modules: BTreeSet::new(),
            local_aliases: BTreeMap::new(),
            external_aliases: BTreeMap::new(),
        };
        for import in text.lines().filter_map(parse_use_module) {
            index.insert(import);
        }
        index
    }

    fn insert(&mut self, import: UseModule) {
        if let Some(package) = import.package {
            self.external_modules
                .insert((import.module.clone(), package.clone()));
            self.external_aliases
                .insert(import.alias, (import.module, package));
        } else {
            self.local_modules.insert(import.module.clone());
            self.local_aliases.insert(import.alias, import.module);
        }
    }
}

struct UseModule {
    module: String,
    alias: String,
    package: Option<String>,
}

fn parse_use_module(line: &str) -> Option<UseModule> {
    let rest = line.trim_start().strip_prefix("use ")?;
    let module = leading_module_path(rest)?;
    let alias = module.rsplit("::").next().unwrap_or(module).to_string();
    let package = rest[module.len()..]
        .trim()
        .strip_prefix("from ")
        .and_then(|value| value.strip_prefix('"'))
        .and_then(|value| {
            value
                .split_once('"')
                .map(|(package, _)| package.to_string())
        });
    Some(UseModule {
        module: module.to_string(),
        alias,
        package,
    })
}

struct UseDeclarationDiagnosticIndex {
    spanless: bool,
    ranges_by_file: BTreeMap<SourcePath, UseDiagnosticRanges>,
}

struct UseDiagnosticRanges {
    ranges: Vec<(usize, usize)>,
    prefix_max_ends: Vec<usize>,
}

impl UseDeclarationDiagnosticIndex {
    fn new(parsed: &ParseOutput) -> Self {
        let mut spanless = false;
        let mut ranges_by_file = BTreeMap::<SourcePath, Vec<(usize, usize)>>::new();
        for diagnostic in &parsed.diagnostics {
            #[cfg(test)]
            record_use_diagnostic_index_visit();
            if diagnostic.parser_context != "use_declaration" {
                continue;
            }
            if let Some(span) = &diagnostic.span {
                ranges_by_file
                    .entry(span.file.clone())
                    .or_default()
                    .push((span.start.offset, span.end.offset));
            } else {
                spanless = true;
            }
        }
        let ranges_by_file = ranges_by_file
            .into_iter()
            .map(|(file, mut ranges)| {
                ranges.sort_unstable();
                let mut greatest_end = 0;
                let prefix_max_ends = ranges
                    .iter()
                    .map(|range| {
                        greatest_end = greatest_end.max(range.1);
                        greatest_end
                    })
                    .collect();
                (
                    file,
                    UseDiagnosticRanges {
                        ranges,
                        prefix_max_ends,
                    },
                )
            })
            .collect();
        Self {
            spanless,
            ranges_by_file,
        }
    }

    fn overlaps(&self, span: &SourceSpan) -> bool {
        #[cfg(test)]
        record_use_diagnostic_overlap_query();
        if self.spanless {
            return true;
        }
        let Some(index) = self.ranges_by_file.get(&span.file) else {
            return false;
        };
        let candidate_count = index
            .ranges
            .partition_point(|range| range.0 <= span.end.offset);
        candidate_count > 0 && index.prefix_max_ends[candidate_count - 1] >= span.start.offset
    }
}

fn schema_alias_external_imports(
    parsed: &ParseOutput,
    diagnostics: &UseDeclarationDiagnosticIndex,
) -> Vec<ExternalImport> {
    parsed
        .tree
        .uses
        .iter()
        .filter_map(|use_decl| {
            let package = use_decl.package.as_ref()?;
            let alias = use_decl
                .name
                .rsplit("::")
                .next()
                .unwrap_or(use_decl.name.as_str())
                .to_string();
            let syntax_valid = !module_identity_has_invalid_casing(&use_decl.name)
                && !diagnostics.overlaps(&use_decl.span);
            Some(ExternalImport {
                module: use_decl.name.clone(),
                package: package.name.clone(),
                alias,
                syntax_valid,
            })
        })
        .collect()
}

fn workspace_imports(
    parsed: &ParseOutput,
    diagnostics: &UseDeclarationDiagnosticIndex,
) -> Vec<WorkspaceImport> {
    parsed
        .tree
        .uses
        .iter()
        .filter(|use_decl| use_decl.package.is_none())
        .map(|use_decl| {
            let alias = use_decl
                .name
                .rsplit("::")
                .next()
                .unwrap_or(use_decl.name.as_str())
                .to_string();
            let syntax_valid = !module_identity_has_invalid_casing(&use_decl.name)
                && !diagnostics.overlaps(&use_decl.span);
            WorkspaceImport {
                module: use_decl.name.clone(),
                alias,
                syntax_valid,
            }
        })
        .collect()
}

fn workspace_location(span: SourceSpan) -> NavigationLocation {
    NavigationLocation {
        source: NavigationSource::Workspace,
        span,
    }
}

fn leading_module_path(input: &str) -> Option<&str> {
    let end = input
        .char_indices()
        .take_while(|(_, ch)| is_identifier_char(*ch) || *ch == ':')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&input[..end])
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(is_identifier_start) && chars.all(is_identifier_char)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_char(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

fn offset_for_position(text: &str, position: &SourcePosition) -> Option<usize> {
    let line_start = line_start_offset(text, position.line.checked_sub(1)?)?;
    let line = text[line_start..]
        .split_once('\n')
        .map_or(&text[line_start..], |(line, _)| line);
    let offset = line
        .char_indices()
        .nth(position.column.checked_sub(1)?)
        .map(|(index, _)| line_start + index)
        .unwrap_or(line_start + line.len());
    Some(offset)
}

fn line_start_offset(text: &str, zero_based_line: usize) -> Option<usize> {
    if zero_based_line == 0 {
        return Some(0);
    }
    let mut line = 0;
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            line += 1;
            if line == zero_based_line {
                return Some(index + 1);
            }
        }
    }
    None
}
