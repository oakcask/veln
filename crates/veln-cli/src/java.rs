use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, ExitStatus, Output};
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) enum JavaRunResult {
    Ran(Output),
    ToolError(String),
}

pub(crate) fn compile_and_run_java(
    build_dir: &Path,
    java: &veln_backend_jvm::JavaProgram,
    java_args: &[String],
) -> Result<ExitCode, String> {
    let result = compile_and_run_java_capture(build_dir, java, "veln run", java_args)?;
    let output = match result {
        JavaRunResult::Ran(output) => output,
        JavaRunResult::ToolError(message) => {
            eprintln!("{message}");
            return Ok(ExitCode::from(1));
        }
    };
    forward_process_output(&output)?;
    Ok(exit_code_from_status(output.status))
}

pub(crate) fn compile_and_run_java_capture(
    build_dir: &Path,
    java: &veln_backend_jvm::JavaProgram,
    command_name: &str,
    java_args: &[String],
) -> Result<JavaRunResult, String> {
    compile_and_run_java_capture_with_env(build_dir, java, command_name, &[], java_args)
}

pub(crate) fn compile_and_run_java_capture_with_env(
    _build_dir: &Path,
    java: &veln_backend_jvm::JavaProgram,
    command_name: &str,
    java_env: &[(&str, &OsStr)],
    java_args: &[String],
) -> Result<JavaRunResult, String> {
    let class_dir = match ensure_cached_java(java, command_name)? {
        CachedJava::Ready(path) => path,
        CachedJava::ToolError(message) => return Ok(JavaRunResult::ToolError(message)),
    };

    let mut command = ProcessCommand::new("java");
    command.arg("-cp").arg(&class_dir).arg("VelnEntry");
    command.args(java_args);
    for (name, value) in java_env {
        command.env(name, value);
    }
    let java_output = command.output();
    let java_output = match java_output {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(JavaRunResult::ToolError(format!(
                "veln: `java` was not found; install a JDK to use `{command_name}`"
            )));
        }
        Err(error) => return Err(error.to_string()),
    };
    Ok(JavaRunResult::Ran(java_output))
}

enum CachedJava {
    Ready(PathBuf),
    ToolError(String),
}

fn ensure_cached_java(
    java: &veln_backend_jvm::JavaProgram,
    command_name: &str,
) -> Result<CachedJava, String> {
    let cache_root = env::current_dir()
        .map_err(|error| error.to_string())?
        .join("target")
        .join("veln-cache")
        .join("java");
    fs::create_dir_all(&cache_root).map_err(|error| error.to_string())?;
    let key = java_cache_key(java);
    let cache_dir = cache_root.join(&key);
    let marker = cache_dir.join(".veln-cache-ok");
    if marker.is_file() {
        return Ok(CachedJava::Ready(cache_dir));
    }
    if cache_dir.exists() {
        fs::remove_dir_all(&cache_dir).map_err(|error| error.to_string())?;
    }

    let compile_dir =
        create_cache_compile_dir(&cache_root, &key).map_err(|error| error.to_string())?;
    for source in &java.sources {
        fs::write(compile_dir.join(&source.path), &source.contents)
            .map_err(|error| error.to_string())?;
    }

    let javac_output = ProcessCommand::new("javac")
        .args(java.sources.iter().map(|source| source.path.as_str()))
        .current_dir(&compile_dir)
        .output();
    let javac_output = match javac_output {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let _ = fs::remove_dir_all(&compile_dir);
            return Ok(CachedJava::ToolError(format!(
                "veln: `javac` was not found; install a JDK to use `{command_name}`"
            )));
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&compile_dir);
            return Err(error.to_string());
        }
    };
    if !javac_output.status.success() {
        let _ = fs::remove_dir_all(&compile_dir);
        return Ok(CachedJava::ToolError(format!(
            "veln: javac failed with status {}",
            javac_output.status
        )));
    }

    fs::write(&marker_for(&compile_dir), b"ok\n").map_err(|error| error.to_string())?;
    match fs::rename(&compile_dir, &cache_dir) {
        Ok(()) => Ok(CachedJava::Ready(cache_dir)),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
            fs::remove_dir_all(&compile_dir).map_err(|error| error.to_string())?;
            Ok(CachedJava::Ready(cache_dir))
        }
        Err(error) => {
            let _ = fs::remove_dir_all(&compile_dir);
            Err(error.to_string())
        }
    }
}

fn marker_for(dir: &Path) -> PathBuf {
    dir.join(".veln-cache-ok")
}

fn create_cache_compile_dir(cache_root: &Path, key: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = cache_root.join(format!("{key}.tmp-{}-{nanos}", std::process::id()));
    for attempt in 0..100 {
        let candidate = if attempt == 0 {
            base.clone()
        } else {
            cache_root.join(format!(
                "{key}.tmp-{}-{nanos}-{attempt}",
                std::process::id()
            ))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate cache compile directory",
    ))
}

fn java_cache_key(java: &veln_backend_jvm::JavaProgram) -> String {
    let mut hash = Fnv64::new();
    hash.write(b"veln-java-cache-v1\0");
    for source in &java.sources {
        hash.write(source.path.as_bytes());
        hash.write(b"\0");
        hash.write(source.contents.as_bytes());
        hash.write(b"\0");
    }
    format!("{:016x}", hash.finish())
}

struct Fnv64(u64);

impl Fnv64 {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }

    fn finish(self) -> u64 {
        self.0
    }
}

pub(crate) fn create_build_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    for attempt in 0..100 {
        let candidate = if attempt == 0 {
            base.clone()
        } else {
            env::temp_dir().join(format!("{prefix}-{}-{nanos}-{attempt}", std::process::id()))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not allocate build directory",
    ))
}

fn forward_process_output(output: &Output) -> Result<(), String> {
    io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| error.to_string())?;
    io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn exit_code_from_status(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        Some(_) | None => ExitCode::from(1),
    }
}
