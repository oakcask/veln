use super::generic_type_syntax::{GenericSyntax, TypeArgumentRange, scan_generic_syntax};
use super::type_paths::is_type_path_segment;
use super::variant_refinements::{
    RawRefinementCandidate, refinement_alternatives, refinement_candidate_depths, starts_uppercase,
};
use super::*;

pub(super) fn malformed_refinement_syntax(tokens: &[Token]) -> Vec<(usize, &'static str)> {
    let mut errors = Vec::new();
    let (generic_syntax, surplus_closers) = scan_generic_syntax(tokens);
    let candidates = refinement_alternatives(tokens, &generic_syntax);
    for (index, token) in tokens.iter().enumerate() {
        diagnose_token(tokens, &surplus_closers, index, token, &mut errors);
    }
    diagnose_excessive_nesting(tokens, &candidates, &mut errors);
    diagnose_generic_arguments(tokens, &generic_syntax, &mut errors);
    diagnose_candidate_suffixes(tokens, &candidates, &mut errors);
    errors.sort_unstable_by_key(|(index, _)| *index);
    errors.dedup();
    errors
}

fn diagnose_token(
    tokens: &[Token],
    surplus_closers: &[bool],
    index: usize,
    token: &Token,
    errors: &mut Vec<(usize, &'static str)>,
) {
    if token.kind == TokenKind::PipeGreater {
        errors.push((
            index,
            "`|>` cannot separate ADT variant refinement alternatives",
        ));
    }
    if token.kind == TokenKind::DoubleColon {
        diagnose_variant_separator(tokens, index, errors);
    }
    diagnose_generic_closer(tokens, surplus_closers, index, token, errors);
}

fn diagnose_variant_separator(
    tokens: &[Token],
    index: usize,
    errors: &mut Vec<(usize, &'static str)>,
) {
    let left_can_be_base = index > 0
        && (is_type_path_segment(&tokens[index - 1])
            || closing_angle_count(tokens[index - 1].kind) > 0);
    let right_is_variant = tokens
        .get(index + 1)
        .is_some_and(|token| is_type_path_segment(token) && starts_uppercase(&token.text));
    if !left_can_be_base && right_is_variant {
        errors.push((index, "variant refinement is missing its ADT base type"));
    } else if left_can_be_base && !tokens.get(index + 1).is_some_and(is_type_path_segment) {
        errors.push((
            index,
            "variant refinement is missing its final variant name",
        ));
    }
    if lowercase_final_after_uppercase_base(tokens, index) {
        errors.push((
            index + 1,
            "variant refinement final segment must start with an ASCII uppercase letter",
        ));
    }
}

fn lowercase_final_after_uppercase_base(tokens: &[Token], index: usize) -> bool {
    index > 0
        && tokens
            .get(index - 1)
            .is_some_and(|token| is_type_path_segment(token) && starts_uppercase(&token.text))
        && tokens
            .get(index + 1)
            .is_some_and(|token| is_type_path_segment(token) && !starts_uppercase(&token.text))
        && tokens.get(index + 2).map(|token| token.kind) != Some(TokenKind::DoubleColon)
}

fn diagnose_generic_closer(
    tokens: &[Token],
    surplus_closers: &[bool],
    index: usize,
    token: &Token,
    errors: &mut Vec<(usize, &'static str)>,
) {
    let followed_by_separator =
        tokens.get(index + 1).map(|next| next.kind) == Some(TokenKind::DoubleColon);
    let followed_by_variant = tokens
        .get(index + 2)
        .is_some_and(|next| is_type_path_segment(next) && starts_uppercase(&next.text));
    if surplus_closers[index] && followed_by_separator && followed_by_variant {
        errors.push((
            index,
            "variant refinement generic base has too many closing angle brackets",
        ));
    }
    if closing_angle_count(token.kind) > 0
        && tokens
            .get(index + 1)
            .is_some_and(|next| is_type_path_segment(next) && starts_uppercase(&next.text))
    {
        errors.push((
            index + 1,
            "variant refinement generic base must be followed by `::Variant`",
        ));
    }
}

fn diagnose_excessive_nesting(
    tokens: &[Token],
    candidates: &[RawRefinementCandidate],
    errors: &mut Vec<(usize, &'static str)>,
) {
    let depths = refinement_candidate_depths(tokens, candidates);
    if let Some((candidate, _)) = candidates
        .iter()
        .zip(depths)
        .find(|(_, depth)| *depth > crate::MAX_VARIANT_REFINEMENT_NESTING)
    {
        errors.push((
            candidate.start_index,
            "variant refinement types are nested too deeply",
        ));
    }
}

fn diagnose_generic_arguments(
    tokens: &[Token],
    generic_syntax: &[GenericSyntax],
    errors: &mut Vec<(usize, &'static str)>,
) {
    for (open, syntax) in generic_syntax.iter().enumerate() {
        let Some(matched) = syntax.matched else {
            continue;
        };
        diagnose_lowercase_generic_variant(tokens, open, matched, errors);
        if generic_closes_before_variant(tokens, matched) {
            for range in &syntax.arguments {
                diagnose_empty_type_argument(tokens, range, errors);
            }
        }
    }
}

fn diagnose_lowercase_generic_variant(
    tokens: &[Token],
    open: usize,
    matched: super::generic_type_syntax::AngleMatch,
    errors: &mut Vec<(usize, &'static str)>,
) {
    let exact_close =
        matched.close_offset + 1 == closing_angle_count(tokens[matched.token_index].kind);
    let variant_index = matched.token_index + 2;
    let uppercase_base = open > 0
        && tokens
            .get(open - 1)
            .is_some_and(|token| is_type_path_segment(token) && starts_uppercase(&token.text));
    let has_separator =
        tokens.get(matched.token_index + 1).map(|token| token.kind) == Some(TokenKind::DoubleColon);
    let lowercase_variant = tokens
        .get(variant_index)
        .is_some_and(|token| is_type_path_segment(token) && !starts_uppercase(&token.text));
    if exact_close && uppercase_base && has_separator && lowercase_variant {
        errors.push((
            variant_index,
            "variant refinement final segment must start with an ASCII uppercase letter",
        ));
    }
}

fn generic_closes_before_variant(
    tokens: &[Token],
    matched: super::generic_type_syntax::AngleMatch,
) -> bool {
    matched.close_offset + 1 == closing_angle_count(tokens[matched.token_index].kind)
        && tokens.get(matched.token_index + 1).map(|token| token.kind)
            == Some(TokenKind::DoubleColon)
        && tokens
            .get(matched.token_index + 2)
            .is_some_and(|token| is_type_path_segment(token) && starts_uppercase(&token.text))
}

fn diagnose_empty_type_argument(
    tokens: &[Token],
    range: &TypeArgumentRange,
    errors: &mut Vec<(usize, &'static str)>,
) {
    let explicit_end = range.explicit_end.unwrap_or_else(|| {
        tokens
            .get(range.end.saturating_sub(1))
            .map_or(0, |token| token.range.end)
    });
    let start_offset = tokens
        .get(range.start)
        .map_or(explicit_end, |token| token.range.start);
    if range.start >= range.end && start_offset >= explicit_end {
        errors.push((
            range.start.min(tokens.len().saturating_sub(1)),
            "variant refinement generic base has an empty type argument",
        ));
    }
}

fn diagnose_candidate_suffixes(
    tokens: &[Token],
    candidates: &[RawRefinementCandidate],
    errors: &mut Vec<(usize, &'static str)>,
) {
    for candidate in candidates {
        let suffix = candidate.shape.variant_index + 1;
        match tokens.get(suffix).map(|token| token.kind) {
            Some(TokenKind::Less) => errors.push((
                suffix,
                "variant refinement type arguments must precede the final `::Variant` segment",
            )),
            Some(TokenKind::DoubleColon) => errors.push((
                suffix,
                "variant refinement must end after its final variant name",
            )),
            _ => {}
        }
    }
}
