use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use veln_ast::{
    decode_surface_module, encode_surface_module, lower_surface_ast_with_module_identity,
};
use veln_source::{SourceFile, TextRange};
use veln_syntax::parse;

fn main() {
    let source_root = Path::new("veln");
    println!("cargo:rerun-if-changed={}", source_root.display());

    let output = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR should be set"));
    build_standard_library_bundle(source_root, &output);
}

struct BundleInputs {
    manifest: String,
    exports: Vec<String>,
    paths: Vec<String>,
}

pub(crate) fn build_standard_library_bundle(source_root: &Path, output: &Path) {
    let inputs = load_bundle_inputs(source_root);
    let mut generated = render_source_tables(&inputs);
    write_lowered_modules(
        source_root,
        &output.join("lowered"),
        &inputs.paths,
        &mut generated,
    );
    generated.push_str("];\n");

    fs::write(output.join("stdlib_bundle.rs"), generated)
        .expect("standard library bundle should be writable");
}

fn load_bundle_inputs(source_root: &Path) -> BundleInputs {
    let manifest = fs::read_to_string(source_root.join("veln.toml"))
        .expect("standard library manifest should be readable");
    let exports = manifest_exports(&manifest);
    let mut paths = Vec::new();
    collect_veln_sources(source_root, source_root, &mut paths);
    paths.sort();
    BundleInputs {
        manifest,
        exports,
        paths,
    }
}

fn render_source_tables(inputs: &BundleInputs) -> String {
    let mut generated = String::new();
    generated.push_str(&format!("const MANIFEST: &str = {:?};\n", inputs.manifest));
    generated.push_str("static EXPORTS: &[&str] = &[\n");
    for export in &inputs.exports {
        generated.push_str(&format!("    {export:?},\n"));
    }
    generated.push_str("];\nstatic FILES: &[StdlibFile] = &[\n");
    for relative in &inputs.paths {
        generated.push_str(&format!(
            "    StdlibFile {{ path: {relative:?}, text: include_str!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/veln/{relative}\")) }},\n"
        ));
    }
    generated.push_str("];\nstatic LOWERED_FILES: &[StdlibLoweredFile] = &[\n");
    generated
}

fn write_lowered_modules(
    source_root: &Path,
    lowered_output: &Path,
    paths: &[String],
    generated: &mut String,
) {
    for relative in paths {
        write_lowered_module(source_root, lowered_output, relative);
        generated.push_str(&format!(
            "    StdlibLoweredFile {{ path: {relative:?}, module: include_bytes!(concat!(env!(\"OUT_DIR\"), \"/lowered/{relative}.bin\")) }},\n"
        ));
    }
}

fn write_lowered_module(source_root: &Path, lowered_output: &Path, relative: &str) {
    let text = fs::read_to_string(source_root.join(relative))
        .expect("standard library source should be readable");
    let lowered = lowered_standard_module(relative, &text);
    let encoded = encode_surface_module(&lowered);
    let decoded = decode_surface_module(&encoded)
        .expect("generated standard library lowered module should decode");
    assert_eq!(
        format!("{lowered:?}"),
        format!("{decoded:?}"),
        "generated standard library lowered module should round-trip for {relative}"
    );

    let lowered_path = lowered_output.join(format!("{relative}.bin"));
    fs::create_dir_all(
        lowered_path
            .parent()
            .expect("lowered standard library path should have a parent"),
    )
    .expect("lowered standard library output directory should be writable");
    fs::write(&lowered_path, encoded).expect("lowered standard library module should be writable");
}

fn lowered_standard_module(path: &str, text: &str) -> veln_ast::SurfaceModule {
    let source = SourceFile::new(path, text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "standard library source should parse cleanly: {path}: {:?}",
        parsed.diagnostics
    );
    let module_name = format!(
        "std::{}",
        path.strip_suffix(".veln")
            .expect("standard library source should use .veln suffix")
            .replace('/', "::")
    );
    let mut lowered = lower_surface_ast_with_module_identity(
        &parsed.tree,
        module_name,
        source.span(TextRange::new(0, 0)),
    );
    for use_decl in &mut lowered.uses {
        let imported = use_decl.name.clone();
        use_decl.name = format!("std::{imported}");
    }
    lowered
}

pub(crate) fn collect_veln_sources(root: &Path, directory: &Path, paths: &mut Vec<String>) {
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
        if is_distribution_source(&relative) {
            paths.push(relative);
        }
    }
}

pub(crate) fn is_distribution_source(path: &str) -> bool {
    path.ends_with(".veln") && !path.ends_with("_test.veln") && !path.ends_with(".test.veln")
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
