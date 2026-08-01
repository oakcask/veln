use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let source_root = Path::new("veln");
    println!("cargo:rerun-if-changed={}", source_root.display());

    let manifest = fs::read_to_string(source_root.join("veln.toml"))
        .expect("standard library manifest should be readable");
    let exports = manifest_exports(&manifest);
    let mut paths = Vec::new();
    collect_veln_sources(source_root, source_root, &mut paths);
    paths.sort();

    let mut generated = String::new();
    generated.push_str(&format!("const MANIFEST: &str = {manifest:?};\n"));
    generated.push_str("static EXPORTS: &[&str] = &[\n");
    for export in exports {
        generated.push_str(&format!("    {export:?},\n"));
    }
    generated.push_str("];\nstatic FILES: &[StdlibFile] = &[\n");
    for relative in paths {
        let text = fs::read_to_string(source_root.join(&relative))
            .expect("standard library source should be readable");
        generated.push_str(&format!(
            "    StdlibFile {{ path: {relative:?}, text: {text:?} }},\n"
        ));
    }
    generated.push_str("];\n");

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should be set"));
    fs::write(output.join("stdlib_bundle.rs"), generated)
        .expect("standard library bundle should be writable");
}

fn collect_veln_sources(root: &Path, directory: &Path, paths: &mut Vec<String>) {
    let entries = fs::read_dir(directory).expect("standard library directory should be readable");
    for entry in entries {
        let entry = entry.expect("standard library directory entry should be readable");
        let path = entry.path();
        if path.is_dir() && entry.file_name() == "target" {
            continue;
        }
        if path.is_dir() {
            collect_veln_sources(root, &path, paths);
            continue;
        }
        let relative = path
            .strip_prefix(root)
            .expect("standard library source should be below source root")
            .to_string_lossy()
            .replace('\\', "/");
        if relative.ends_with(".veln") && !relative.ends_with("_test.veln") {
            paths.push(relative);
        }
    }
}

fn manifest_exports(manifest: &str) -> Vec<String> {
    let mut in_lib = false;
    let mut exports = Vec::new();
    for line in manifest.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_lib = line == "[lib]";
            continue;
        }
        if !in_lib || !line.starts_with("exports") {
            continue;
        }
        let Some((_, values)) = line.split_once('=') else {
            continue;
        };
        exports.extend(
            values
                .split('"')
                .enumerate()
                .filter(|(index, _)| index % 2 == 1)
                .map(|(_, value)| value.to_string()),
        );
    }
    exports
}
