use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir set"));
    let roots = ["tests/toolchain_cases", "../../examples/specification"];
    let mut cases = Vec::new();

    for root in roots {
        collect_cases(&manifest_dir.join(root), Path::new(root), &mut cases)
            .unwrap_or_else(|error| panic!("failed to collect `{root}` cases: {error}"));
    }

    cases.sort();
    let generated = generated_toolchain_tests(&cases);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir set"));
    fs::write(out_dir.join("toolchain_cases.rs"), generated)
        .expect("generated toolchain cases should be written");
}

fn collect_cases(root: &Path, relative: &Path, cases: &mut Vec<PathBuf>) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", root.display());
    if root.join("case.toml").is_file() {
        println!(
            "cargo:rerun-if-changed={}",
            root.join("case.toml").display()
        );
        cases.push(relative.to_path_buf());
        return Ok(());
    }

    let mut entries = fs::read_dir(root)?.collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.path());
    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_cases(&path, &relative.join(entry.file_name()), cases)?;
        }
    }
    Ok(())
}

fn generated_toolchain_tests(cases: &[PathBuf]) -> String {
    let mut names = BTreeSet::new();
    let mut out = String::from("mod generated_toolchain_cases {\n    use super::*;\n\n");
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
