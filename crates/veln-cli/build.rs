use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
#[path = "tests/toolchain_harness/manifest_syntax.rs"]
mod manifest_syntax;
#[allow(dead_code)]
#[path = "toolchain_case_inventory.rs"]
mod toolchain_case_inventory;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir set"));
    for root in toolchain_case_inventory::DISCOVERY_ROOTS {
        println!(
            "cargo:rerun-if-changed={}",
            manifest_dir.join(root.relative).display()
        );
    }
    let preflight = toolchain_case_inventory::run_preflight(&manifest_dir)
        .unwrap_or_else(|error| panic!("{error}"));
    let cases = preflight
        .cases
        .iter()
        .map(|case| case.manifest_relative.clone())
        .collect::<Vec<_>>();
    let generated = generated_toolchain_tests(&cases);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir set"));
    fs::write(out_dir.join("toolchain_cases.rs"), generated)
        .expect("generated toolchain cases should be written");
}

fn generated_toolchain_tests(cases: &[PathBuf]) -> String {
    let mut names = BTreeSet::new();
    let mut out = String::from(
        "mod toolchain_semantic_baseline {\n    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/tests/toolchain_semantic_baseline/mod.rs\"));\n}\n\nconst GENERATED_TOOLCHAIN_CASES: &[&str] = &[\n",
    );
    for case in cases {
        let case = case.to_string_lossy().replace('\\', "/");
        out.push_str(&format!("    {case:?},\n"));
    }
    out.push_str("];\n\nmod generated_toolchain_cases {\n    use super::*;\n\n");
    for case in cases {
        let name = unique_test_name(case, &mut names);
        let case = case.to_string_lossy().replace('\\', "/");
        out.push_str("    #[test]\n");
        out.push_str(&format!("    fn {name}() {{\n"));
        out.push_str(&format!(
            "        run_case(&toolchain_case_path({case:?}));\n"
        ));
        out.push_str("    }\n\n");
    }
    out.push_str("}\n");
    out
}

fn unique_test_name(case: &Path, names: &mut BTreeSet<String>) -> String {
    let raw = case.to_string_lossy();
    let mut name = String::from("toolchain_case_");
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
        } else if !name.ends_with('_') {
            name.push('_');
        }
    }
    while name.ends_with('_') {
        name.pop();
    }

    let hash = fnv1a(raw.as_bytes());
    let max_prefix_len = 96usize.saturating_sub(17);
    if name.len() > max_prefix_len {
        name.truncate(max_prefix_len);
        while name.ends_with('_') {
            name.pop();
        }
    }
    name.push_str(&format!("_{hash:016x}"));

    assert!(
        names.insert(name.clone()),
        "duplicate generated test `{name}`"
    );
    name
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn manifest_error(path: &Path, line_number: usize, message: impl std::fmt::Display) -> ! {
    if line_number == 0 {
        panic!("{}: {message}", path.display());
    }
    panic!("{}:{line_number}: {message}", path.display());
}
