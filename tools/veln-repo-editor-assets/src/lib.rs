//! Project compiler-owned lexical facts into editor-neutral build inputs.

use std::{collections::BTreeSet, fs, path::Path};

use serde_json::{Value, json};
use veln_source::SourceFile;
use veln_syntax::{
    DELIMITER_PAIRS, FALSE_LITERAL, LINE_COMMENT_START, PUBLIC_KEYWORDS, PUBLIC_PUNCTUATION,
    SATISFY_MARKER, STRING_DELIMITER, STRING_ESCAPE, TRUE_LITERAL, TokenKind, VALIDATE_MARKER, lex,
};

pub const OUTPUT: &str = "editors/vscode/toolchain-metadata.json";

pub fn generate(repo: &Path) -> Result<String, String> {
    let specification = read(&repo.join("docs/specification/source-surface-executable.pl"))?;
    verify_keywords(&specification)?;
    let corpus = read(&repo.join("editors/vscode/fixtures/lexical.veln"))?;
    let keywords: Vec<_> = PUBLIC_KEYWORDS.iter().map(|token| token.spelling).collect();
    let punctuation: Vec<_> = PUBLIC_PUNCTUATION
        .iter()
        .map(|token| token.spelling)
        .collect();
    let spelling = |kind| {
        PUBLIC_PUNCTUATION
            .iter()
            .find(|token| token.kind == kind)
            .map(|token| token.spelling)
            .ok_or_else(|| {
                format!("Add {kind:?} to PUBLIC_PUNCTUATION before generating editor delimiters.")
            })
    };
    let brackets = DELIMITER_PAIRS
        .iter()
        .map(|&(open, close)| Ok([spelling(open)?, spelling(close)?]))
        .collect::<Result<Vec<_>, String>>()?;
    // Include every fixed spelling so a newly added token is exercised by TextMate.
    let sources = keywords
        .iter()
        .chain(punctuation.iter())
        .copied()
        .chain([TRUE_LITERAL, FALSE_LITERAL, SATISFY_MARKER, VALIDATE_MARKER])
        .chain(corpus.lines().filter(|line| !line.trim().is_empty()));
    let fixtures = sources
        .map(lexical_fixture)
        .collect::<Result<Vec<_>, _>>()?;
    let metadata = json!({
        "keywords": keywords,
        "punctuation": punctuation,
        "contextualKeywords": [SATISFY_MARKER, VALIDATE_MARKER],
        "booleanLiterals": [TRUE_LITERAL, FALSE_LITERAL],
        "lineComment": LINE_COMMENT_START.to_string(),
        "stringDelimiter": STRING_DELIMITER.to_string(),
        "stringEscape": STRING_ESCAPE.to_string(),
        "brackets": brackets,
        "customSemanticTokenModifiers": veln_editor::custom_semantic_token_modifiers(),
        "customSemanticTokenTypes": veln_editor::custom_semantic_token_types(),
        "lexicalFixtures": fixtures,
    });
    serde_json::to_string_pretty(&metadata)
        .map(|text| text + "\n")
        .map_err(|error| error.to_string())
}

fn lexical_fixture(text: &str) -> Result<Value, String> {
    let source = SourceFile::new("lexical.veln", text);
    let mut tokens = Vec::new();
    for token in lex(&source).tokens {
        let kind = match token.kind {
            TokenKind::Whitespace | TokenKind::Newline | TokenKind::Eof => continue,
            TokenKind::Comment => "comment",
            TokenKind::String => "string",
            TokenKind::Int | TokenKind::Float => "number",
            TokenKind::Hole | TokenKind::Underscore => "hole",
            TokenKind::Ident
                if [TRUE_LITERAL, FALSE_LITERAL, SATISFY_MARKER, VALIDATE_MARKER]
                    .contains(&token.text.as_str()) =>
            {
                "keyword"
            }
            TokenKind::Ident if token.text.starts_with(|ch: char| ch.is_ascii_uppercase()) => {
                "type"
            }
            TokenKind::Ident => "identifier",
            kind if PUBLIC_KEYWORDS.iter().any(|token| token.kind == kind) => "keyword",
            kind if PUBLIC_PUNCTUATION.iter().any(|token| token.kind == kind) => "operator",
            kind => {
                return Err(format!(
                    "Replace invalid lexical fixture token {kind:?} in `{text}`; editor comparison fixtures must have defined lexer boundaries."
                ));
            }
        };
        tokens.push(json!({
            "start": text[..token.range.start].encode_utf16().count(),
            "end": text[..token.range.end].encode_utf16().count(),
            "kind": kind,
        }));
    }
    Ok(json!({ "source": text, "tokens": tokens }))
}

fn verify_keywords(specification: &str) -> Result<(), String> {
    let specified: BTreeSet<_> = specification
        .lines()
        .filter_map(|line| {
            line.strip_prefix("keyword_kind(\"")?
                .split_once('"')
                .map(|(word, _)| word)
        })
        .collect();
    let implemented: BTreeSet<_> = PUBLIC_KEYWORDS.iter().map(|token| token.spelling).collect();
    if specified != implemented {
        return Err(format!(
            "Align PUBLIC_KEYWORDS and executable source-surface keyword_kind facts before regenerating editor assets; specification-only={:?}, lexer-only={:?}. Editor highlighting must follow implemented source syntax.",
            specified.difference(&implemented).collect::<Vec<_>>(),
            implemented.difference(&specified).collect::<Vec<_>>(),
        ));
    }
    Ok(())
}

pub fn synchronize(repo: &Path, write: bool) -> Result<(), String> {
    let expected = generate(repo)?;
    let path = repo.join(OUTPUT);
    if write {
        fs::write(&path, expected).map_err(|error| format!("Cannot write {OUTPUT}: {error}"))
    } else {
        let actual = read(&path).map_err(|error| format!(
            "{error}. Restore generated editor metadata with `pnpm --filter veln-language generate:syntax` so editor assets can be checked against the toolchain."
        ))?;
        verify_fresh(&actual, &expected)
    }
}

fn verify_fresh(actual: &str, expected: &str) -> Result<(), String> {
    if actual != expected {
        return Err("Regenerate editor assets with `pnpm --filter veln-language generate:syntax`; toolchain metadata is stale, so highlighting may differ from the lexer.".into());
    }
    Ok(())
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("Cannot read editor generation input: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_and_extra_specification_keywords() {
        let specification = PUBLIC_KEYWORDS
            .iter()
            .map(|token| format!("keyword_kind(\"{}\", kind).\n", token.spelling))
            .collect::<String>();
        assert!(verify_keywords(&specification).is_ok());
        assert!(
            verify_keywords(&specification.replace("keyword_kind(\"fn\", kind).\n", "")).is_err()
        );
        assert!(verify_keywords(&(specification + "keyword_kind(\"extra\", kind).\n")).is_err());
    }

    #[test]
    fn check_preserves_stale_file_and_write_repairs_projection() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let repo = std::env::temp_dir().join(format!(
            "veln-editor-assets-{}-{suffix}",
            std::process::id()
        ));
        fs::create_dir_all(repo.join("docs/specification")).unwrap();
        fs::create_dir_all(repo.join("editors/vscode/fixtures")).unwrap();
        let specification = PUBLIC_KEYWORDS
            .iter()
            .map(|token| format!("keyword_kind(\"{}\", kind).\n", token.spelling))
            .collect::<String>();
        fs::write(
            repo.join("docs/specification/source-surface-executable.pl"),
            specification,
        )
        .unwrap();
        fs::write(repo.join("editors/vscode/fixtures/lexical.veln"), "42\n").unwrap();
        fs::write(repo.join(OUTPUT), "stale\n").unwrap();
        assert!(
            synchronize(&repo, false)
                .unwrap_err()
                .contains("generate:syntax")
        );
        assert_eq!(read(&repo.join(OUTPUT)).unwrap(), "stale\n");
        synchronize(&repo, true).unwrap();
        synchronize(&repo, false).unwrap();
        fs::remove_dir_all(repo).unwrap();
    }

    #[test]
    fn lexer_fixture_preserves_utf16_and_longest_operator_boundaries() {
        let fixture = lexical_fixture("\"😀\" >>> _hole").unwrap();
        assert_eq!(
            fixture["tokens"],
            json!([
                {"start": 0, "end": 4, "kind": "string"},
                {"start": 5, "end": 8, "kind": "operator"},
                {"start": 9, "end": 14, "kind": "hole"},
            ])
        );
        assert!(lexical_fixture("0b102").is_err());
    }
}
