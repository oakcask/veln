use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use super::*;

const SCHEMA: &str = "veln-toolchain-case-semantics/v0";
const ROOTS: [(&str, &str); 2] = [
    (
        "crates/veln-cli/tests/toolchain_cases",
        "tests/toolchain_cases",
    ),
    ("examples/specification", "../../examples/specification"),
];
const BASELINE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/toolchain-case-semantics.baseline"
));
const LARGE_TEXT_BYTES: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Inventory {
    schema: String,
    roots: Vec<String>,
    source_git_tree: String,
    cases: BTreeMap<String, BTreeMap<String, String>>,
}

impl Inventory {
    fn current(source_git_tree: &str) -> Self {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut cases = BTreeMap::new();
        let inventory = toolchain_case_inventory::run_preflight(&manifest_dir)
            .unwrap_or_else(|error| panic!("{error}"));
        for case in inventory.cases {
            let path = manifest_dir.join(&case.manifest_relative).join("case.toml");
            let manifest = CaseManifest::read(&path);
            assert!(
                cases.insert(case.id.clone(), describe(&manifest)).is_none(),
                "duplicate semantic case identifier `{}`",
                case.id
            );
        }
        Self {
            schema: SCHEMA.to_string(),
            roots: ROOTS.iter().map(|(root, _)| (*root).to_string()).collect(),
            source_git_tree: source_git_tree.to_string(),
            cases,
        }
    }

    fn render(&self) -> String {
        let mut output = String::new();
        line(&mut output, "schema", &json_string(&self.schema));
        line(
            &mut output,
            "source_git_tree",
            &json_string(&self.source_git_tree),
        );
        for root in &self.roots {
            line(&mut output, "root", &json_string(root));
        }
        line(&mut output, "case_count", &self.cases.len().to_string());
        for (id, fields) in &self.cases {
            line(&mut output, "case", &json_string(id));
            for (path, value) in fields {
                output.push_str("field\t");
                output.push_str(path);
                output.push('\t');
                output.push_str(value);
                output.push('\n');
            }
            line(
                &mut output,
                "case_digest",
                &json_string(&fields_digest(fields)),
            );
        }
        line(
            &mut output,
            "aggregate_digest",
            &json_string(&aggregate_digest(&self.cases)),
        );
        output
    }

    fn parse(text: &str) -> Result<Self, String> {
        BaselineParser::default().parse(text)
    }
}

mod compare;
mod describe;
mod digest;
mod parser;

use compare::compare;
use describe::describe;
use digest::{
    aggregate_digest, fields_digest, json_string, line, parse_json_string, sha256,
};
use parser::BaselineParser;

#[cfg(test)]
mod tests;
