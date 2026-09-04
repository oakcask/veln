use std::env;
use std::fs;
use std::path::PathBuf;

use veln_repo_toolchain_case::inventory as toolchain_case_inventory;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir set"));
    for root in toolchain_case_inventory::DISCOVERY_ROOTS {
        println!(
            "cargo:rerun-if-changed={}",
            manifest_dir.join(root.relative).display()
        );
    }
    let generated = toolchain_case_inventory::generated_toolchain_tests_from_preflight(
        &manifest_dir,
        &toolchain_case_inventory::DISCOVERY_ROOTS,
        true,
    )
    .unwrap_or_else(|error| panic!("{error}"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir set"));
    fs::write(out_dir.join("toolchain_cases.rs"), generated)
        .expect("generated toolchain cases should be written");
}
