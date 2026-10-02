use super::generic_type_syntax::{
    GenericSyntax, TypeArgumentRange, scan_generic_syntax, type_argument_text_range,
};
use super::type_paths::is_type_path_segment;
use super::*;

pub(super) fn build_variant_refinements(
    source: &SourceFile,
    tokens: &[Token],
) -> (Vec<VariantRefinementType>, Vec<bool>) {
    let (generic_syntax, _) = scan_generic_syntax(tokens);
    let raw_candidates = refinement_alternatives(tokens, &generic_syntax);
    let depths = refinement_candidate_depths(tokens, &raw_candidates);
    let hierarchy = refinement_hierarchy(tokens, &generic_syntax, &raw_candidates, &depths);
    materialize_refinements(source, tokens, &generic_syntax, &raw_candidates, hierarchy)
}

struct RefinementHierarchy {
    children: Vec<Vec<Vec<usize>>>,
    roots: Vec<usize>,
    retained: Vec<bool>,
}

fn refinement_hierarchy(
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
    candidates: &[RawRefinementCandidate],
    depths: &[usize],
) -> RefinementHierarchy {
    let mut hierarchy = RefinementHierarchy {
        children: candidates
            .iter()
            .map(|candidate| {
                vec![
                    Vec::new();
                    candidate
                        .shape
                        .generic_open
                        .map_or(0, |open| generic_syntax[open].arguments.len())
                ]
            })
            .collect(),
        roots: Vec::new(),
        retained: vec![false; candidates.len()],
    };
    let mut ancestors = Vec::new();
    let mut next_arguments = vec![0usize; candidates.len()];
    for (index, candidate) in candidates.iter().enumerate() {
        discard_finished_ancestors(tokens, candidates, candidate, &mut ancestors);
        let parent = candidate_parent(
            tokens,
            generic_syntax,
            candidates,
            candidate,
            &ancestors,
            &mut next_arguments,
        );
        hierarchy.retained[index] = depths[index] <= crate::MAX_VARIANT_REFINEMENT_NESTING;
        retain_candidate(&mut hierarchy, index, parent);
        ancestors.push(index);
    }
    hierarchy
}

fn discard_finished_ancestors(
    tokens: &[Token],
    candidates: &[RawRefinementCandidate],
    candidate: &RawRefinementCandidate,
    ancestors: &mut Vec<usize>,
) {
    let candidate_offsets = raw_candidate_offsets(tokens, candidate).unwrap_or((0, usize::MAX));
    while ancestors.last().is_some_and(|ancestor| {
        raw_candidate_offsets(tokens, &candidates[*ancestor])
            .is_none_or(|(start, end)| start >= candidate_offsets.0 || end < candidate_offsets.1)
    }) {
        ancestors.pop();
    }
}

fn candidate_parent(
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
    candidates: &[RawRefinementCandidate],
    candidate: &RawRefinementCandidate,
    ancestors: &[usize],
    next_arguments: &mut [usize],
) -> Option<(usize, usize)> {
    let parent_index = *ancestors.last()?;
    let parent = &candidates[parent_index];
    let arguments = parent
        .shape
        .generic_open
        .map(|open| generic_syntax[open].arguments.as_slice())?;
    let (candidate_start, candidate_end) = raw_candidate_offsets(tokens, candidate)?;
    let argument = &mut next_arguments[parent_index];
    while arguments.get(*argument).is_some_and(|range| {
        type_argument_text_range(tokens, range).is_some_and(|range| range.end <= candidate_start)
    }) {
        *argument += 1;
    }
    let range = type_argument_text_range(tokens, arguments.get(*argument)?)?;
    (candidate_start >= range.start && candidate_end <= range.end)
        .then_some((parent_index, *argument))
}

fn retain_candidate(
    hierarchy: &mut RefinementHierarchy,
    index: usize,
    parent: Option<(usize, usize)>,
) {
    if !hierarchy.retained[index] {
        return;
    }
    if let Some((parent, argument)) = parent {
        if hierarchy.retained[parent] {
            hierarchy.children[parent][argument].push(index);
        }
    } else {
        hierarchy.roots.push(index);
    }
}

fn materialize_refinements(
    source: &SourceFile,
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
    raw_candidates: &[RawRefinementCandidate],
    hierarchy: RefinementHierarchy,
) -> (Vec<VariantRefinementType>, Vec<bool>) {
    let mut candidates = (0..raw_candidates.len()).map(|_| None).collect::<Vec<_>>();
    let mut consumed_pipes = vec![false; tokens.len()];
    for index in (0..candidates.len()).rev() {
        if !hierarchy.retained[index] {
            continue;
        }
        let children = materialize_children(
            source,
            tokens,
            &hierarchy.children[index],
            raw_candidates,
            &mut candidates,
            &mut consumed_pipes,
        );
        candidates[index] = Some(materialize_refinement_candidate(
            source,
            tokens,
            generic_syntax,
            &raw_candidates[index],
            children.refinements,
            children.ranges,
        ));
    }
    let root_candidates = hierarchy
        .roots
        .into_iter()
        .map(|root| candidates[root].take().expect("root refinement candidate"))
        .collect();
    let refinements =
        group_variant_refinements(source, tokens, root_candidates, &mut consumed_pipes);
    (refinements, consumed_pipes)
}

struct MaterializedChildren {
    refinements: Vec<Vec<VariantRefinementType>>,
    ranges: Vec<Vec<(usize, usize)>>,
}

fn materialize_children(
    source: &SourceFile,
    tokens: &[Token],
    child_indexes_by_argument: &[Vec<usize>],
    raw_candidates: &[RawRefinementCandidate],
    candidates: &mut [Option<RefinementCandidate>],
    consumed_pipes: &mut [bool],
) -> MaterializedChildren {
    let mut refinements_by_argument = Vec::with_capacity(child_indexes_by_argument.len());
    let mut ranges_by_argument = Vec::with_capacity(child_indexes_by_argument.len());
    for child_indexes in child_indexes_by_argument {
        ranges_by_argument.push(
            child_indexes
                .iter()
                .map(|child| {
                    (
                        raw_candidates[*child].start_index,
                        raw_candidates[*child].end_index + 1,
                    )
                })
                .collect(),
        );
        let child_candidates = child_indexes
            .iter()
            .map(|child| {
                candidates[*child]
                    .take()
                    .expect("child refinement candidate")
            })
            .collect();
        refinements_by_argument.push(group_variant_refinements(
            source,
            tokens,
            child_candidates,
            consumed_pipes,
        ));
    }
    MaterializedChildren {
        refinements: refinements_by_argument,
        ranges: ranges_by_argument,
    }
}

pub(super) fn refinement_candidate_depths(
    tokens: &[Token],
    candidates: &[RawRefinementCandidate],
) -> Vec<usize> {
    let mut depths = vec![0usize; candidates.len()];
    let mut ancestors: Vec<usize> = Vec::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let (candidate_start, candidate_end) =
            raw_candidate_offsets(tokens, candidate).unwrap_or((0, usize::MAX));
        while ancestors.last().is_some_and(|ancestor| {
            raw_candidate_offsets(tokens, &candidates[*ancestor])
                .is_none_or(|(start, end)| start >= candidate_start || end < candidate_end)
        }) {
            ancestors.pop();
        }
        depths[index] = ancestors.len();
        ancestors.push(index);
    }
    depths
}

fn type_argument_fragments(
    source: &SourceFile,
    argument_span: &SourceSpan,
    child_refinements: &[VariantRefinementType],
) -> Vec<String> {
    let mut fragments = Vec::with_capacity(child_refinements.len() + 1);
    let mut cursor = argument_span.start.offset;
    for child in child_refinements {
        fragments.push(source.text()[cursor..child.span.start.offset].to_string());
        cursor = child.span.end.offset;
    }
    fragments.push(source.text()[cursor..argument_span.end.offset].to_string());
    fragments
}

struct RefinementGroup {
    alternatives: Vec<usize>,
    pipes: Vec<usize>,
}

fn group_variant_refinements(
    source: &SourceFile,
    tokens: &[Token],
    candidates: Vec<RefinementCandidate>,
    consumed_pipes: &mut [bool],
) -> Vec<VariantRefinementType> {
    let groups = refinement_groups(tokens, &candidates);
    let mut candidates = candidates.into_iter().map(Some).collect::<Vec<_>>();
    let mut refinements = groups
        .into_iter()
        .map(|group| {
            mark_group_pipes(&group, consumed_pipes);
            materialize_refinement_group(source, tokens, group, &mut candidates)
        })
        .collect::<Vec<_>>();
    refinements.sort_by_key(|refinement| refinement.span.start.offset);
    refinements
}

fn refinement_groups(tokens: &[Token], candidates: &[RefinementCandidate]) -> Vec<RefinementGroup> {
    let candidate_at_start = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| (candidate.start_index, index))
        .collect::<std::collections::HashMap<_, _>>();
    let mut grouped = vec![false; candidates.len()];
    let mut groups = Vec::new();
    for start in 0..candidates.len() {
        if grouped[start] {
            continue;
        }
        let group = connected_refinement_group(tokens, candidates, &candidate_at_start, start);
        for index in &group.alternatives {
            grouped[*index] = true;
        }
        groups.push(group);
    }
    groups
}

fn connected_refinement_group(
    tokens: &[Token],
    candidates: &[RefinementCandidate],
    candidate_at_start: &std::collections::HashMap<usize, usize>,
    start: usize,
) -> RefinementGroup {
    let mut group = RefinementGroup {
        alternatives: vec![start],
        pipes: Vec::new(),
    };
    let mut current = start;
    loop {
        let pipe_index = candidates[current].end_index + 1;
        if tokens.get(pipe_index).map(|token| token.kind) != Some(TokenKind::Pipe) {
            break;
        }
        #[cfg(test)]
        super::record_refinement_grouping_candidate_lookup();
        let Some(next) = candidate_at_start.get(&(pipe_index + 1)).copied() else {
            break;
        };
        group.pipes.push(pipe_index);
        group.alternatives.push(next);
        current = next;
    }
    group
}

fn mark_group_pipes(group: &RefinementGroup, consumed_pipes: &mut [bool]) {
    for index in &group.pipes {
        consumed_pipes[*index] = true;
    }
}

fn materialize_refinement_group(
    source: &SourceFile,
    tokens: &[Token],
    group: RefinementGroup,
    candidates: &mut [Option<RefinementCandidate>],
) -> VariantRefinementType {
    let alternatives = group
        .alternatives
        .into_iter()
        .map(|index| candidates[index].take().expect("grouped refinement").value)
        .collect::<Vec<_>>();
    let first_span = &alternatives
        .first()
        .expect("refinement union has alternatives")
        .span;
    let span = SourceSpan {
        file: first_span.file.clone(),
        start: first_span.start,
        end: alternatives
            .last()
            .expect("refinement union has alternatives")
            .span
            .end,
    };
    VariantRefinementType {
        alternatives,
        pipe_spans: group
            .pipes
            .iter()
            .map(|index| source.span(tokens[*index].range))
            .collect(),
        span,
    }
}

struct RefinementCandidate {
    start_index: usize,
    end_index: usize,
    value: VariantRefinementAlternative,
}

pub(super) struct RawRefinementCandidate {
    pub(super) start_index: usize,
    pub(super) end_index: usize,
    pub(super) shape: RefinementAlternativeShape,
}

#[derive(Clone, Copy)]
pub(super) struct RefinementAlternativeShape {
    base_end: usize,
    pub(super) variant_index: usize,
    generic_open: Option<usize>,
}

pub(super) fn refinement_alternatives(
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
) -> Vec<RawRefinementCandidate> {
    let mut candidates = Vec::new();
    for start in 0..tokens.len() {
        if !is_type_path_segment(&tokens[start])
            || (start > 0 && tokens[start - 1].kind == TokenKind::DoubleColon)
        {
            continue;
        }
        if let Some(shape) = refinement_alternative_shape(tokens, generic_syntax, start) {
            candidates.push(RawRefinementCandidate {
                start_index: start,
                end_index: shape.variant_index,
                shape,
            });
        }
    }
    candidates
}

fn materialize_refinement_candidate(
    source: &SourceFile,
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
    candidate: &RawRefinementCandidate,
    child_refinements: Vec<Vec<VariantRefinementType>>,
    child_token_ranges: Vec<Vec<(usize, usize)>>,
) -> RefinementCandidate {
    let shape = candidate.shape;
    let type_arguments = shape.generic_open.map_or_else(Vec::new, |open| {
        refinement_type_arguments(
            source,
            tokens,
            &generic_syntax[open].arguments,
            child_refinements,
            child_token_ranges,
        )
    });
    let variant = &tokens[shape.variant_index];
    let (segments, segment_spans) =
        refinement_base_path(source, tokens, candidate.start_index, shape.base_end);
    let span = source.span(tokens[candidate.start_index].range.cover(variant.range));
    RefinementCandidate {
        start_index: candidate.start_index,
        end_index: shape.variant_index,
        value: VariantRefinementAlternative {
            base: TypePathSegments {
                segments,
                segment_spans,
            },
            type_arguments,
            variant: variant.text.clone(),
            variant_span: source.span(variant.range),
            span,
        },
    }
}

fn refinement_base_path(
    source: &SourceFile,
    tokens: &[Token],
    start: usize,
    end: usize,
) -> (Vec<String>, Vec<SourceSpan>) {
    let mut segments = Vec::new();
    let mut spans = Vec::new();
    let mut cursor = start;
    loop {
        segments.push(tokens[cursor].text.clone());
        spans.push(source.span(tokens[cursor].range));
        if cursor == end {
            break;
        }
        cursor += 2;
    }
    (segments, spans)
}

fn raw_candidate_offsets(
    tokens: &[Token],
    candidate: &RawRefinementCandidate,
) -> Option<(usize, usize)> {
    Some((
        tokens.get(candidate.start_index)?.range.start,
        tokens.get(candidate.end_index)?.range.end,
    ))
}

fn refinement_alternative_shape(
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
    start: usize,
) -> Option<RefinementAlternativeShape> {
    let (mut cursor, mut base_end) = refinement_base_end(tokens, start);
    let (base_end, generic_open, variant_index) =
        if tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::Less) {
            generic_refinement_shape(tokens, generic_syntax, cursor)?
        } else {
            let segment_count = (base_end - start) / 2 + 1;
            if segment_count < 2 {
                return None;
            }
            let variant_index = base_end;
            base_end = base_end.saturating_sub(2);
            (base_end, None, variant_index)
        };
    cursor = base_end;
    let base_leaf = tokens.get(cursor)?;
    let variant = tokens.get(variant_index)?;
    if !starts_uppercase(&base_leaf.text) || !starts_uppercase(&variant.text) {
        return None;
    }
    Some(RefinementAlternativeShape {
        base_end,
        variant_index,
        generic_open,
    })
}

fn refinement_base_end(tokens: &[Token], start: usize) -> (usize, usize) {
    let mut cursor = start;
    let mut base_end = start;
    while tokens.get(cursor + 1).map(|token| token.kind) == Some(TokenKind::DoubleColon)
        && tokens.get(cursor + 2).is_some_and(is_type_path_segment)
    {
        cursor += 2;
        base_end = cursor;
    }
    (cursor, base_end)
}

fn generic_refinement_shape(
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
    base_end: usize,
) -> Option<(usize, Option<usize>, usize)> {
    let open = base_end + 1;
    let matched = generic_syntax[open].matched?;
    let close = matched.token_index;
    if matched.close_offset + 1 != closing_angle_count(tokens[close].kind) {
        return None;
    }
    if tokens.get(close + 1).map(|token| token.kind) != Some(TokenKind::DoubleColon)
        || !tokens.get(close + 2).is_some_and(is_type_path_segment)
    {
        return None;
    }
    Some((base_end, Some(open), close + 2))
}

fn refinement_type_arguments(
    source: &SourceFile,
    tokens: &[Token],
    ranges: &[TypeArgumentRange],
    child_refinements: Vec<Vec<VariantRefinementType>>,
    child_token_ranges: Vec<Vec<(usize, usize)>>,
) -> Vec<VariantRefinementTypeArgument> {
    let mut arguments = Vec::with_capacity(ranges.len());
    for ((range, child_refinements), child_token_ranges) in
        ranges.iter().zip(child_refinements).zip(child_token_ranges)
    {
        push_refinement_type_argument(
            source,
            tokens,
            range,
            child_refinements,
            &child_token_ranges,
            &mut arguments,
        );
    }
    arguments
}

fn push_refinement_type_argument(
    source: &SourceFile,
    tokens: &[Token],
    argument_range: &TypeArgumentRange,
    child_refinements: Vec<VariantRefinementType>,
    child_token_ranges: &[(usize, usize)],
    arguments: &mut Vec<VariantRefinementTypeArgument>,
) {
    let Some(range) = type_argument_text_range(tokens, argument_range) else {
        return;
    };
    let text = &source.text()[range.start..range.end];
    let leading = text.len() - text.trim_start().len();
    let trailing = text.len() - text.trim_end().len();
    let range = TextRange::new(range.start + leading, range.end.saturating_sub(trailing));
    let mut structure_tokens = owned_type_argument_tokens(
        tokens,
        argument_range.start,
        argument_range.end,
        child_token_ranges,
    );
    append_explicit_angle_closers(tokens, argument_range, &mut structure_tokens);
    #[cfg(test)]
    super::record_refinement_argument_token_copies(structure_tokens.len());
    arguments.push(VariantRefinementTypeArgument {
        ty_fragments: type_argument_fragments(source, &source.span(range), &child_refinements),
        ty_paths: type_paths_from_tokens(source, &structure_tokens),
        ty_refinements: child_refinements,
        span: source.span(range),
    });
}

fn append_explicit_angle_closers(
    tokens: &[Token],
    argument: &TypeArgumentRange,
    structure_tokens: &mut Vec<Token>,
) {
    let Some(explicit_end) = argument.explicit_end else {
        return;
    };
    let Some(close) = tokens.get(argument.end) else {
        return;
    };
    let included_closers = explicit_end.saturating_sub(close.range.start);
    if included_closers == 0 {
        return;
    }
    structure_tokens.push(Token {
        kind: match included_closers {
            1 => TokenKind::Greater,
            2 => TokenKind::ShiftRight,
            _ => TokenKind::ShiftRightLogical,
        },
        text: ">".repeat(included_closers),
        range: TextRange::new(close.range.start, explicit_end),
    });
}

fn owned_type_argument_tokens(
    tokens: &[Token],
    start: usize,
    end: usize,
    child_token_ranges: &[(usize, usize)],
) -> Vec<Token> {
    let mut owned = Vec::new();
    let mut cursor = start;
    for &(child_start, child_end) in child_token_ranges {
        let child_start = child_start.clamp(cursor, end);
        owned.extend_from_slice(&tokens[cursor..child_start]);
        cursor = child_end.clamp(child_start, end);
    }
    owned.extend_from_slice(&tokens[cursor..end]);
    owned
}

pub(super) fn starts_uppercase(text: &str) -> bool {
    text.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}
