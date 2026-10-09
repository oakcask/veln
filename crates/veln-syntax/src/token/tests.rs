use super::{
    PRESENTATION_PARSE_STRUCTURE_LIMIT, PresentationParseStructure, TOKEN_LABELS, TokenKind,
};
use crate::lex;
use veln_source::SourceFile;

#[test]
fn token_labels_cover_all_token_kinds() {
    assert_eq!(TOKEN_LABELS.len(), TokenKind::ALL.len());
    for (index, kind) in TokenKind::ALL.iter().enumerate() {
        assert_eq!(*kind as usize, index);
        assert!(!kind.label().is_empty());
    }
}

#[test]
fn presentation_structure_restores_top_level_after_else_if() {
    assert_top_level_after_ends(
        concat!(
            "if first\n",
            "  1\n",
            "else \t if second\n",
            "  2\n",
            "else\n",
            "  3\n",
            "end\n",
        ),
        &[true],
    );
}

#[test]
fn presentation_structure_restores_top_level_after_multiple_else_if_branches() {
    assert_top_level_after_ends(
        concat!(
            "if first\n",
            "  1\n",
            "else if second\n",
            "  2\n",
            "else   if third\n",
            "  3\n",
            "else\n",
            "  4\n",
            "end\n",
        ),
        &[true],
    );
}

#[test]
fn presentation_structure_keeps_genuinely_nested_if_depth() {
    assert_top_level_after_ends(
        concat!(
            "if outer\n",
            "  if inner\n",
            "    1\n",
            "  else\n",
            "    2\n",
            "  end\n",
            "else\n",
            "  3\n",
            "end\n",
        ),
        &[false, true],
    );
}

#[test]
fn presentation_structure_counts_else_if_work_without_adding_block_depth() {
    assert_else_if_work_is_bounded(PRESENTATION_PARSE_STRUCTURE_LIMIT - 1, true);
    assert_else_if_work_is_bounded(PRESENTATION_PARSE_STRUCTURE_LIMIT, false);
}

fn assert_else_if_work_is_bounded(branch_count: usize, expected_bounded: bool) {
    let mut text = "if first\n  0\n".to_string();
    for _ in 0..branch_count {
        text.push_str("else if next\n  0\n");
    }
    text.push_str("else\n  0\nend\n");
    let source = SourceFile::new("main.veln", text);
    let mut structure = PresentationParseStructure::default();
    let mut exceeded_limit = false;

    for token in lex(&source).tokens {
        exceeded_limit |= !structure.observe(token.kind);
    }
    assert_eq!(
        !exceeded_limit, expected_bounded,
        "else-if continuations must count toward presentation work at the exact limit boundary"
    );
    assert!(structure.is_top_level());
}

fn assert_top_level_after_ends(source: &str, expected: &[bool]) {
    let source = SourceFile::new("main.veln", source);
    let mut structure = PresentationParseStructure::default();
    let mut after_ends = Vec::new();
    for token in lex(&source).tokens {
        structure.observe(token.kind);
        if token.kind == TokenKind::End {
            after_ends.push(structure.is_top_level());
        }
    }
    assert_eq!(after_ends, expected);
}
