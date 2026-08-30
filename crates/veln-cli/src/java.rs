use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, ExitStatus, Output};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};
use veln_project::LowerHexBytes;

const JVM_CACHE_MARKER: &str = ".veln-cache-ok";
const JVM_CACHE_MANIFEST: &str = ".veln-cache-manifest";
const JVM_CACHE_MANIFEST_HEADER: &[u8] = b"veln-jvm-class-cache-manifest/v1\n";
const JVM_CACHE_VERSION: &[u8] = b"veln-jvm-class-cache-v3\0";
const JVM_CACHE_PREPARE_ATTEMPTS: usize = 3;
const JVM_CACHE_LOCK_WAIT: Duration = Duration::from_secs(60);
const JVM_CACHE_LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);
const JVM_ENTRY_CLASS: &str = "VelnEntry.class";

#[derive(Debug)]
pub(crate) enum JvmRunResult {
    Ran(Output),
    ToolError(String),
}

#[derive(Clone)]
pub(crate) struct JvmExecution {
    cache_root: PathBuf,
    java_launcher: PathBuf,
}

#[cfg(test)]
impl JvmExecution {
    pub(crate) fn for_test(cache_root: PathBuf) -> Self {
        Self {
            cache_root,
            java_launcher: PathBuf::from("java"),
        }
    }
}

pub(crate) enum JvmExecutionPreparation {
    Ready(JvmExecution),
    ToolError(String),
}

pub(crate) fn prepare_jvm_execution(command_name: &str) -> Result<JvmExecutionPreparation, String> {
    let Some(java_launcher) = find_java_launcher() else {
        return Ok(JvmExecutionPreparation::ToolError(missing_java_message(
            command_name,
        )));
    };
    let cache_root = resolve_veln_cache_root()?.join("jvm");
    fs::create_dir_all(&cache_root)
        .map_err(|error| format!("could not prepare the Veln cache root: {error}"))?;
    if !cache_root.is_dir() {
        return Err(
            "could not prepare the Veln cache root: the selected root is not a directory"
                .to_string(),
        );
    }
    Ok(JvmExecutionPreparation::Ready(JvmExecution {
        cache_root,
        java_launcher,
    }))
}

pub(crate) fn prepare_and_run_jvm_capture_with_env(
    _build_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
    command_name: &str,
    java_env: &[(&str, &OsStr)],
    java_args: &[String],
) -> Result<JvmRunResult, String> {
    let execution = match prepare_jvm_execution(command_name)? {
        JvmExecutionPreparation::Ready(execution) => execution,
        JvmExecutionPreparation::ToolError(message) => {
            return Ok(JvmRunResult::ToolError(message));
        }
    };
    prepare_and_run_jvm_capture_with_execution(
        &execution,
        program,
        command_name,
        java_env,
        java_args,
    )
}

pub(crate) fn prepare_and_run_jvm_capture_with_execution(
    execution: &JvmExecution,
    program: &veln_backend_jvm::JvmProgram,
    command_name: &str,
    java_env: &[(&str, &OsStr)],
    java_args: &[String],
) -> Result<JvmRunResult, String> {
    prepare_and_run_jvm_capture_with_execution_and_hooks(
        execution,
        program,
        command_name,
        java_env,
        java_args,
        &NoJvmCacheHooks,
    )
}

fn prepare_and_run_jvm_capture_with_execution_and_hooks(
    execution: &JvmExecution,
    program: &veln_backend_jvm::JvmProgram,
    command_name: &str,
    java_env: &[(&str, &OsStr)],
    java_args: &[String],
    hooks: &dyn JvmCacheHooks,
) -> Result<JvmRunResult, String> {
    let class_dir =
        match ensure_cached_jvm_classes_in_with_hooks(&execution.cache_root, program, hooks)? {
            CachedJvmClasses::Ready(path) => path,
            CachedJvmClasses::ToolError(message) => return Ok(JvmRunResult::ToolError(message)),
        };

    run_jvm_class_dir(
        execution.java_launcher.as_os_str(),
        &class_dir,
        command_name,
        java_env,
        java_args,
    )
}

fn run_jvm_class_dir(
    java_launcher: &OsStr,
    class_dir: &Path,
    command_name: &str,
    java_env: &[(&str, &OsStr)],
    java_args: &[String],
) -> Result<JvmRunResult, String> {
    let mut command = ProcessCommand::new(java_launcher);
    command.arg("-cp").arg(class_dir).arg("VelnEntry");
    command.args(java_args);
    for (name, value) in java_env {
        command.env(name, value);
    }
    let java_output = command.output();
    let java_output = match java_output {
        Ok(output) => output,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(JvmRunResult::ToolError(missing_java_message(command_name)));
        }
        Err(error) => return Err(error.to_string()),
    };
    Ok(JvmRunResult::Ran(java_output))
}

#[derive(Debug)]
enum CachedJvmClasses {
    Ready(PathBuf),
    ToolError(String),
}

mod environment;
use environment::*;

#[cfg(test)]
fn ensure_cached_jvm_classes_in(
    cache_root: &Path,
    program: &veln_backend_jvm::JvmProgram,
) -> Result<CachedJvmClasses, String> {
    ensure_cached_jvm_classes_in_with_hooks(cache_root, program, &NoJvmCacheHooks)
}

fn ensure_cached_jvm_classes_in_with_hooks(
    cache_root: &Path,
    program: &veln_backend_jvm::JvmProgram,
    hooks: &dyn JvmCacheHooks,
) -> Result<CachedJvmClasses, String> {
    fs::create_dir_all(cache_root)
        .map_err(|error| format!("could not prepare the Veln cache root: {error}"))?;
    let key = jvm_class_cache_key(program);
    let cache_dir = cache_root.join(&key);
    let lock_dir = cache_lock_dir(cache_root, &key);

    for _ in 0..JVM_CACHE_PREPARE_ATTEMPTS {
        {
            let _lock = JvmCacheLock::acquire(&lock_dir).map_err(|error| error.to_string())?;
            hooks.after_initial_lock();
            if validated_cache_exists(&cache_dir, program)? {
                return Ok(CachedJvmClasses::Ready(cache_dir));
            }
            remove_invalid_cache(&cache_dir, hooks)?;
        }

        hooks.before_prepare();
        let compile_dir = match prepare_jvm_cache_compile_dir(cache_root, &key, program, hooks)? {
            CacheCompilePreparation::Ready(path) => path,
            CacheCompilePreparation::ToolError(message) => {
                return Ok(CachedJvmClasses::ToolError(message));
            }
        };

        hooks.before_publish_lock();
        let _lock = match JvmCacheLock::acquire(&lock_dir) {
            Ok(lock) => lock,
            Err(error) => {
                let _ = fs::remove_dir_all(&compile_dir);
                return Err(error.to_string());
            }
        };
        if let Err(error) = hooks.at_fault_point(JvmCacheFaultPoint::Publication) {
            let _ = fs::remove_dir_all(&compile_dir);
            return Err(format!("could not publish JVM cache entry: {error}"));
        }
        match publish_prepared_jvm_cache(&compile_dir, &cache_dir, program)? {
            CachePublish::Published | CachePublish::ReusedValidated => {
                return Ok(CachedJvmClasses::Ready(cache_dir));
            }
            CachePublish::LostInvalidRace => {}
        }
    }

    Err("could not prepare validated JVM class cache entry".to_string())
}

trait JvmCacheHooks {
    fn after_initial_lock(&self) {}
    fn before_prepare(&self) {}
    fn before_publish_lock(&self) {}
    fn at_fault_point(&self, _point: JvmCacheFaultPoint) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum JvmCacheFaultPoint {
    InvalidEntryRemoval,
    PreparedEntryValidation,
    Publication,
}

struct NoJvmCacheHooks;

impl JvmCacheHooks for NoJvmCacheHooks {}

enum CacheCompilePreparation {
    Ready(PathBuf),
    ToolError(String),
}

fn validated_cache_exists(
    cache_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
) -> Result<bool, String> {
    validate_cached_jvm_classes(cache_dir, program).map_err(|error| error.to_string())
}

fn remove_invalid_cache(cache_dir: &Path, hooks: &dyn JvmCacheHooks) -> Result<(), String> {
    if cache_dir.exists() {
        hooks
            .at_fault_point(JvmCacheFaultPoint::InvalidEntryRemoval)
            .and_then(|()| fs::remove_dir_all(cache_dir))
            .map_err(|error| format!("could not remove invalid JVM cache entry: {error}"))?;
    }
    Ok(())
}

fn prepare_jvm_cache_compile_dir(
    cache_root: &Path,
    key: &str,
    program: &veln_backend_jvm::JvmProgram,
    hooks: &dyn JvmCacheHooks,
) -> Result<CacheCompilePreparation, String> {
    let compile_dir =
        create_cache_compile_dir(cache_root, key).map_err(|error| error.to_string())?;
    let preparation = (|| {
        write_cached_jvm_classes(&compile_dir, program).map_err(|error| error.to_string())?;
        if !compile_dir.join(JVM_ENTRY_CLASS).is_file() {
            return Ok(CacheCompilePreparation::ToolError(
                "veln: JVM class preparation did not produce an entry class".to_string(),
            ));
        }
        write_jvm_cache_metadata(&compile_dir, program)?;
        hooks
            .at_fault_point(JvmCacheFaultPoint::PreparedEntryValidation)
            .map_err(|error| error.to_string())?;
        if !validate_cached_jvm_classes(&compile_dir, program).map_err(|error| error.to_string())? {
            return Err("prepared JVM cache entry did not pass validation".to_string());
        }
        Ok(CacheCompilePreparation::Ready(compile_dir.clone()))
    })();
    if preparation.is_err() || matches!(&preparation, Ok(CacheCompilePreparation::ToolError(_))) {
        let _ = fs::remove_dir_all(&compile_dir);
    }
    preparation.map_err(|error| format!("could not prepare JVM cache entry: {error}"))
}

fn write_jvm_cache_metadata(
    compile_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
) -> Result<(), String> {
    fs::write(
        manifest_for(compile_dir),
        render_jvm_cache_manifest(program),
    )
    .map_err(|error| error.to_string())?;
    fs::write(marker_for(compile_dir), b"ok\n").map_err(|error| error.to_string())
}

fn publish_prepared_jvm_cache(
    compile_dir: &Path,
    cache_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
) -> Result<CachePublish, String> {
    let published = publish_cached_jvm_classes(compile_dir, cache_dir, program);
    if published.is_err() {
        let _ = fs::remove_dir_all(compile_dir);
    }
    published.map_err(|error| format!("could not publish JVM cache entry: {error}"))
}

fn write_cached_jvm_classes(
    compile_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
) -> io::Result<()> {
    for class in &program.classes {
        let path = compile_dir.join(&class.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &class.contents)?;
    }
    Ok(())
}

fn validate_cached_jvm_classes(
    cache_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
) -> io::Result<bool> {
    if !marker_for(cache_dir).is_file() || !manifest_for(cache_dir).is_file() {
        return Ok(false);
    }
    if fs::read(manifest_for(cache_dir))? != render_jvm_cache_manifest(program) {
        return Ok(false);
    }

    let expected_digests = jvm_cache_class_digests(program);
    let mut expected_paths = expected_digests
        .iter()
        .map(|class| class.path.to_string())
        .collect::<Vec<_>>();
    expected_paths.sort();

    let Some(mut actual_paths) = collect_cache_file_paths(cache_dir)? else {
        return Ok(false);
    };
    actual_paths.sort();
    if actual_paths != expected_paths {
        return Ok(false);
    }

    for class in expected_digests {
        let path = cache_dir.join(class.path);
        let contents = fs::read(path)?;
        if contents.len() != class.byte_len || Sha256::digest(&contents) != class.digest {
            return Ok(false);
        }
    }

    Ok(true)
}

enum CachePublish {
    Published,
    ReusedValidated,
    LostInvalidRace,
}

fn publish_cached_jvm_classes(
    compile_dir: &Path,
    cache_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
) -> io::Result<CachePublish> {
    if validate_cached_jvm_classes(cache_dir, program)? {
        fs::remove_dir_all(compile_dir)?;
        return Ok(CachePublish::ReusedValidated);
    }
    remove_invalid_cache_io(cache_dir)?;

    match fs::rename(compile_dir, cache_dir) {
        Ok(()) => Ok(CachePublish::Published),
        Err(error) if is_cache_publish_collision(&error) => {
            fs::remove_dir_all(compile_dir)?;
            if validate_cached_jvm_classes(cache_dir, program)? {
                return Ok(CachePublish::ReusedValidated);
            }
            if cache_dir.exists() {
                fs::remove_dir_all(cache_dir)?;
            }
            Ok(CachePublish::LostInvalidRace)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(compile_dir);
            Err(error)
        }
    }
}

struct JvmCacheLock {
    dir: PathBuf,
}

impl JvmCacheLock {
    fn acquire(dir: &Path) -> io::Result<Self> {
        Self::acquire_with_timeout(dir, cache_lock_wait())
    }

    fn acquire_with_timeout(dir: &Path, timeout: Duration) -> io::Result<Self> {
        let deadline = Instant::now() + timeout;
        loop {
            match fs::create_dir(dir) {
                Ok(()) => {
                    let lock = Self {
                        dir: dir.to_path_buf(),
                    };
                    pause_after_cache_lock_for_test()?;
                    return Ok(lock);
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "timed out waiting for JVM cache coordination",
                        ));
                    }
                    thread::sleep(JVM_CACHE_LOCK_RETRY_DELAY);
                }
                Err(error) => return Err(error),
            }
        }
    }
}

fn cache_lock_wait() -> Duration {
    #[cfg(debug_assertions)]
    if let Some(wait_ms) = env::var_os("VELN_INTERNAL_TEST_CACHE_LOCK_WAIT_MS")
        .and_then(|value| value.into_string().ok())
        .and_then(|value| value.parse::<u64>().ok())
    {
        return Duration::from_millis(wait_ms);
    }
    JVM_CACHE_LOCK_WAIT
}

fn pause_after_cache_lock_for_test() -> io::Result<()> {
    #[cfg(debug_assertions)]
    if let Some(marker) = env::var_os("VELN_INTERNAL_TEST_CACHE_LOCK_READY") {
        fs::write(marker, b"ready\n")?;
        thread::sleep(Duration::from_secs(30));
    }
    Ok(())
}

impl Drop for JvmCacheLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.dir);
    }
}

fn cache_lock_dir(cache_root: &Path, key: &str) -> PathBuf {
    cache_root.join(format!("{key}.lock"))
}

fn remove_invalid_cache_io(cache_dir: &Path) -> io::Result<()> {
    match fs::remove_dir_all(cache_dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn is_cache_publish_collision(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::AlreadyExists | io::ErrorKind::DirectoryNotEmpty
    )
}

fn marker_for(dir: &Path) -> PathBuf {
    dir.join(JVM_CACHE_MARKER)
}

fn manifest_for(dir: &Path) -> PathBuf {
    dir.join(JVM_CACHE_MANIFEST)
}

fn collect_cache_file_paths(root: &Path) -> io::Result<Option<Vec<String>>> {
    let mut paths = Vec::new();
    if collect_cache_file_paths_from(root, root, &mut paths)? {
        Ok(Some(paths))
    } else {
        Ok(None)
    }
}

fn collect_cache_file_paths_from(
    root: &Path,
    dir: &Path,
    paths: &mut Vec<String>,
) -> io::Result<bool> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            if !collect_cache_file_paths_from(root, &path, paths)? {
                return Ok(false);
            }
            continue;
        }
        if !file_type.is_file() {
            return Ok(false);
        }
        if path == marker_for(root) || path == manifest_for(root) {
            continue;
        }
        let relative = path.strip_prefix(root).map_err(io::Error::other)?;
        let relative = relative
            .components()
            .map(|component| component.as_os_str().to_string_lossy())
            .collect::<Vec<_>>()
            .join("/");
        paths.push(relative);
    }
    Ok(true)
}

fn render_jvm_cache_manifest(program: &veln_backend_jvm::JvmProgram) -> Vec<u8> {
    let mut manifest = Vec::new();
    manifest.extend_from_slice(JVM_CACHE_MANIFEST_HEADER);
    manifest.extend_from_slice(format!("classes {}\n", program.classes.len()).as_bytes());
    for class in jvm_cache_class_digests(program) {
        manifest.extend_from_slice(
            format!(
                "{} {} {} ",
                class.path.len(),
                class.byte_len,
                format_args!("{:x}", LowerHexBytes(&class.digest))
            )
            .as_bytes(),
        );
        manifest.extend_from_slice(class.path.as_bytes());
        manifest.push(b'\n');
    }
    manifest
}

struct JvmCacheClassDigest<'a> {
    path: &'a str,
    byte_len: usize,
    digest: sha2::digest::Output<Sha256>,
}

fn jvm_cache_class_digests(program: &veln_backend_jvm::JvmProgram) -> Vec<JvmCacheClassDigest<'_>> {
    program
        .classes
        .iter()
        .map(|class| JvmCacheClassDigest {
            path: &class.path,
            byte_len: class.contents.len(),
            digest: Sha256::digest(&class.contents),
        })
        .collect()
}

fn create_cache_compile_dir(cache_root: &Path, key: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = cache_root.join(format!("{key}.tmp-{}-{nanos}", std::process::id()));
    create_unique_dir(
        base,
        |attempt| {
            cache_root.join(format!(
                "{key}.tmp-{}-{nanos}-{attempt}",
                std::process::id()
            ))
        },
        "could not allocate cache compile directory",
    )
}

fn jvm_class_cache_key(program: &veln_backend_jvm::JvmProgram) -> String {
    let mut hash = Sha256::new();
    hash.update(JVM_CACHE_VERSION);
    for class in &program.classes {
        hash.update((class.path.len() as u64).to_be_bytes());
        hash.update(class.path.as_bytes());
        hash.update((class.contents.len() as u64).to_be_bytes());
        hash.update(&class.contents);
    }
    let digest = hash.finalize();
    format!("{:x}", LowerHexBytes(&digest))
}

pub(crate) fn create_build_dir(prefix: &str) -> io::Result<PathBuf> {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let base = env::temp_dir().join(format!("{prefix}-{}-{nanos}", std::process::id()));
    create_unique_dir(
        base,
        |attempt| {
            env::temp_dir().join(format!("{prefix}-{}-{nanos}-{attempt}", std::process::id()))
        },
        "could not allocate build directory",
    )
}

fn create_unique_dir(
    base: PathBuf,
    retry_candidate: impl Fn(usize) -> PathBuf,
    exhausted_message: &'static str,
) -> io::Result<PathBuf> {
    for attempt in 0..100 {
        let candidate = if attempt == 0 {
            base.clone()
        } else {
            retry_candidate(attempt)
        };
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        exhausted_message,
    ))
}

pub(crate) fn forward_process_output(output: &Output) -> Result<(), String> {
    io::stdout()
        .write_all(&output.stdout)
        .map_err(|error| error.to_string())?;
    io::stderr()
        .write_all(&output.stderr)
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn exit_code_from_status(status: ExitStatus) -> ExitCode {
    match status.code() {
        Some(code) if (0..=255).contains(&code) => ExitCode::from(code as u8),
        Some(_) | None => ExitCode::from(1),
    }
}

fn missing_java_message(command_name: &str) -> String {
    format!("veln: `java` was not found; install a JDK to use `{command_name}`")
}

#[cfg(test)]
#[path = "java/tests.rs"]
mod tests;
