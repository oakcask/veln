use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=runtime/VelnRuntime.java");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source_path = manifest_dir.join("runtime").join("VelnRuntime.java");
    let runtime_source =
        fs::read_to_string(&source_path).expect("runtime source should be readable");

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
}
