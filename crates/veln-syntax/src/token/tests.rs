use super::{TOKEN_LABELS, TokenKind};

#[test]
fn token_labels_cover_all_token_kinds() {
    assert_eq!(TOKEN_LABELS.len(), TokenKind::ALL.len());
    for (index, kind) in TokenKind::ALL.iter().enumerate() {
        assert_eq!(*kind as usize, index);
        assert!(!kind.label().is_empty());
    }
}
