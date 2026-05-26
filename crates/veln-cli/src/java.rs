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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use veln_backend_jvm::{JavaProgram, JavaSourceFile};

    static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

    fn java_program(sources: &[(&str, &str)]) -> JavaProgram {
        JavaProgram {
            sources: sources
                .iter()
                .map(|(path, contents)| JavaSourceFile {
                    path: (*path).to_string(),
                    contents: (*contents).to_string(),
                })
                .collect(),
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "veln-cli-java-test-{name}-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test root should be created");
        root
    }

    #[test]
    fn java_cache_key_tracks_source_path_contents_and_order() {
        let base = java_program(&[
            ("VelnProgram.java", "class VelnProgram {}"),
            ("VelnRuntime.java", "class VelnRuntime {}"),
        ]);
        let changed_contents = java_program(&[
            ("VelnProgram.java", "class VelnProgram { int value; }"),
            ("VelnRuntime.java", "class VelnRuntime {}"),
        ]);
        let changed_path = java_program(&[
            ("Entry.java", "class VelnProgram {}"),
            ("VelnRuntime.java", "class VelnRuntime {}"),
        ]);
        let changed_order = java_program(&[
            ("VelnRuntime.java", "class VelnRuntime {}"),
            ("VelnProgram.java", "class VelnProgram {}"),
        ]);

        let base_key = java_cache_key(&base);

        assert_eq!(base_key.len(), 16);
        assert!(base_key.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_ne!(base_key, java_cache_key(&changed_contents));
        assert_ne!(base_key, java_cache_key(&changed_path));
        assert_ne!(base_key, java_cache_key(&changed_order));
    }

    #[test]
    fn fnv64_hash_matches_incremental_writes() {
        let mut all_at_once = Fnv64::new();
        all_at_once.write(b"left-right");

        let mut incremental = Fnv64::new();
        incremental.write(b"left");
        incremental.write(b"-");
        incremental.write(b"right");

        assert_eq!(all_at_once.finish(), incremental.finish());
    }

    #[test]
    fn cache_compile_dirs_are_unique_and_markers_stay_inside_them() {
        let root = temp_root("compile-dir");

        let first =
            create_cache_compile_dir(&root, "cache-key").expect("first dir should be created");
        let second =
            create_cache_compile_dir(&root, "cache-key").expect("second dir should be created");

        assert_ne!(first, second);
        assert!(first.is_dir());
        assert!(second.is_dir());
        assert_eq!(marker_for(&first), first.join(".veln-cache-ok"));
        assert_eq!(marker_for(&second), second.join(".veln-cache-ok"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[cfg(unix)]
    #[test]
    fn exit_code_from_status_maps_success_failure_and_signal_statuses() {
        use std::os::unix::process::ExitStatusExt;

        assert_eq!(
            exit_code_from_status(ExitStatus::from_raw(0)),
            ExitCode::from(0)
        );
        assert_eq!(
            exit_code_from_status(ExitStatus::from_raw(42 << 8)),
            ExitCode::from(42)
        );
        assert_eq!(
            exit_code_from_status(ExitStatus::from_raw(9)),
            ExitCode::from(1)
        );
    }
}
