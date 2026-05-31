use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

const RUNTIME_FRAGMENTS: &[&str] = &[
    "values.java.inc",
    "concurrency.java.inc",
    "effects.java.inc",
    "accessors.java.inc",
    "collections.java.inc",
    "option_result.java.inc",
    "operators.java.inc",
    "stdio.java.inc",
    "diagnostics.java.inc",
    "interop.java.inc",
];

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let runtime_source_dir = manifest_dir.join("src").join("runtime");
    let runtime_source = runtime_source(&runtime_source_dir);

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let runtime_dir = out_dir.join("runtime");
    fs::create_dir_all(&runtime_dir).expect("runtime output dir should be created");
    fs::write(runtime_dir.join("VelnRuntime.java"), runtime_source)
        .expect("runtime source should be written");

    let output = Command::new("javac")
        .arg("--release")
        .arg("8")
        .arg("VelnRuntime.java")
        .current_dir(&runtime_dir)
        .output()
        .expect("javac should run; install a JDK to build veln-backend-jvm");
    if !output.status.success() {
        panic!(
            "javac failed while building JVM runtime classes:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    write_runtime_class_manifest(&out_dir, &runtime_dir);
}

fn runtime_source(runtime_source_dir: &Path) -> String {
    let mut source = String::from("public final class VelnRuntime {\n");
    for fragment in RUNTIME_FRAGMENTS {
        let path = runtime_source_dir.join(fragment);
        println!("cargo:rerun-if-changed=src/runtime/{fragment}");
        let text = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!("failed to read runtime fragment `{fragment}`: {error}")
        });
        source.push_str(&text);
        if !text.ends_with('\n') {
            source.push('\n');
        }
    }
    source.push_str("}\n");
    source
}

fn write_runtime_class_manifest(out_dir: &Path, runtime_dir: &Path) {
    let mut class_names = fs::read_dir(runtime_dir)
        .expect("runtime output dir should be readable")
        .map(|entry| {
            let entry = entry.expect("runtime output entry should be readable");
            entry.file_name().to_string_lossy().into_owned()
        })
        .filter(|name| name.ends_with(".class"))
        .collect::<Vec<_>>();
    class_names.sort();

    if !class_names.iter().any(|name| name == "VelnRuntime.class") {
        panic!(
            "javac did not produce VelnRuntime.class; inspect the runtime fragment compile output"
        );
    }

    let mut manifest = Vec::new();
    writeln!(manifest, "const RUNTIME_CLASSES: &[(&str, &[u8])] = &[").unwrap();
    for name in class_names {
        writeln!(
            manifest,
            "    ({name:?}, include_bytes!(concat!(env!(\"OUT_DIR\"), \"/runtime/{name}\"))),"
        )
        .unwrap();
    }
    writeln!(manifest, "];").unwrap();

    fs::write(out_dir.join("runtime_classes.rs"), manifest)
        .expect("runtime class manifest should be written");
}
