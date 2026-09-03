use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::normalize_source_text;

pub trait GrammarSource {
    fn complete_grammar(&self, repo_root: &Path) -> Result<String, String>;
}

pub struct SwiplGrammarSource;

impl GrammarSource for SwiplGrammarSource {
    fn complete_grammar(&self, repo_root: &Path) -> Result<String, String> {
        let spec = repo_root.join("docs/specification/source-surface-executable.pl");
        let output = Command::new("swipl")
            .current_dir(repo_root)
            .args(["-q", "-s"])
            .arg(spec)
            .args(["--", "--grammar"])
            .output()
            .map_err(|error| {
                format!(
                    "install SWI-Prolog before regenerating the language-reference catalog: {error}"
                )
            })?;
        if !output.status.success() {
            return Err(format!(
                "update the executable grammar before regenerating the language-reference catalog:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        String::from_utf8(output.stdout)
            .map_err(|error| format!("the executable grammar printed non-UTF-8 output: {error}"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct GrammarProduction {
    pub(crate) name: String,
    pub(crate) lines: Vec<String>,
}

pub(crate) fn parse_grammar(text: &str) -> Result<Vec<GrammarProduction>, String> {
    let mut productions = Vec::new();
    for line in normalize_source_text(text).lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(' ') {
            let Some(last): Option<&mut GrammarProduction> = productions.last_mut() else {
                return Err("the executable grammar starts with a continuation line".to_string());
            };
            last.lines.push(line.to_string());
            continue;
        }
        let Some((name, _)) = line.split_once("::=") else {
            return Err(format!(
                "the executable grammar line does not contain a production separator: {line}"
            ));
        };
        productions.push(GrammarProduction {
            name: name.trim().to_string(),
            lines: vec![line.to_string()],
        });
    }
    let mut names = BTreeSet::new();
    for production in &productions {
        if !names.insert(production.name.clone()) {
            return Err(format!(
                "the executable grammar repeats production `{}`",
                production.name
            ));
        }
    }
    Ok(productions)
}
