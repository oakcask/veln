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
    let generated = toolchain_case_inventory::generated_toolchain_tests(&cases);
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir set"));
    fs::write(out_dir.join("toolchain_cases.rs"), generated)
        .expect("generated toolchain cases should be written");
}

fn manifest_error(path: &Path, line_number: usize, message: impl std::fmt::Display) -> ! {
    if line_number == 0 {
        panic!("{}: {message}", path.display());
    }
    panic!("{}:{line_number}: {message}", path.display());
}
