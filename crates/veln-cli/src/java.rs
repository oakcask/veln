use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, ExitStatus, Output};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

const JVM_CACHE_MARKER: &str = ".veln-cache-ok";
const JVM_CACHE_MANIFEST: &str = ".veln-cache-manifest";
const JVM_CACHE_MANIFEST_HEADER: &[u8] = b"veln-jvm-class-cache-manifest/v1\n";
const JVM_CACHE_VERSION: &[u8] = b"veln-jvm-class-cache-v3\0";
const JVM_CACHE_PREPARE_ATTEMPTS: usize = 3;
const JVM_ENTRY_CLASS: &str = "VelnEntry.class";

pub(crate) enum JvmRunResult {
    Ran(Output),
    ToolError(String),
}

pub(crate) fn prepare_and_run_jvm_capture_with_env(
    _build_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
    command_name: &str,
    java_env: &[(&str, &OsStr)],
    java_args: &[String],
) -> Result<JvmRunResult, String> {
    let class_dir = match ensure_cached_jvm_classes(program)? {
        CachedJvmClasses::Ready(path) => path,
        CachedJvmClasses::ToolError(message) => return Ok(JvmRunResult::ToolError(message)),
    };

    run_jvm_class_dir(
        OsStr::new("java"),
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

enum CachedJvmClasses {
    Ready(PathBuf),
    ToolError(String),
}

fn ensure_cached_jvm_classes(
    program: &veln_backend_jvm::JvmProgram,
) -> Result<CachedJvmClasses, String> {
    let cache_root = env::current_dir()
        .map_err(|error| error.to_string())?
        .join("target")
        .join("veln-cache")
        .join("jvm");
    ensure_cached_jvm_classes_in(&cache_root, program)
}

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
    fs::create_dir_all(cache_root).map_err(|error| error.to_string())?;
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
            remove_invalid_cache(&cache_dir)?;
        }

        hooks.before_prepare();
        let compile_dir = match prepare_jvm_cache_compile_dir(cache_root, &key, program)? {
            CacheCompilePreparation::Ready(path) => path,
            CacheCompilePreparation::ToolError(message) => {
                return Ok(CachedJvmClasses::ToolError(message));
            }
        };

        hooks.before_publish_lock();
        let _lock = JvmCacheLock::acquire(&lock_dir).map_err(|error| error.to_string())?;
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

fn remove_invalid_cache(cache_dir: &Path) -> Result<(), String> {
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn prepare_jvm_cache_compile_dir(
    cache_root: &Path,
    key: &str,
    program: &veln_backend_jvm::JvmProgram,
) -> Result<CacheCompilePreparation, String> {
    let compile_dir =
        create_cache_compile_dir(cache_root, key).map_err(|error| error.to_string())?;
    if let Err(error) = write_cached_jvm_classes(&compile_dir, program) {
        let _ = fs::remove_dir_all(&compile_dir);
        return Err(error.to_string());
    }
    if !compile_dir.join(JVM_ENTRY_CLASS).is_file() {
        let _ = fs::remove_dir_all(&compile_dir);
        return Ok(CacheCompilePreparation::ToolError(
            "veln: JVM class preparation did not produce an entry class".to_string(),
        ));
    }
    write_jvm_cache_metadata(&compile_dir, program)?;
    Ok(CacheCompilePreparation::Ready(compile_dir))
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
    publish_cached_jvm_classes(compile_dir, cache_dir, program).map_err(|error| error.to_string())
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
        if contents.len() != class.byte_len || sha256(&contents) != class.digest {
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
        loop {
            match fs::create_dir(dir) {
                Ok(()) => {
                    return Ok(Self {
                        dir: dir.to_path_buf(),
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    thread::yield_now();
                }
                Err(error) => return Err(error),
            }
        }
    }
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
                hex(&class.digest)
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
    digest: [u8; 32],
}

fn jvm_cache_class_digests(program: &veln_backend_jvm::JvmProgram) -> Vec<JvmCacheClassDigest<'_>> {
    program
        .classes
        .iter()
        .map(|class| JvmCacheClassDigest {
            path: &class.path,
            byte_len: class.contents.len(),
            digest: sha256(&class.contents),
        })
        .collect()
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

fn jvm_class_cache_key(program: &veln_backend_jvm::JvmProgram) -> String {
    let mut hash = Sha256::new();
    hash.update(JVM_CACHE_VERSION);
    for class in &program.classes {
        hash.update(&(class.path.len() as u64).to_be_bytes());
        hash.update(class.path.as_bytes());
        hash.update(&(class.contents.len() as u64).to_be_bytes());
        hash.update(&class.contents);
    }
    hex(&hash.finalize())
}

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes)
}

fn hex(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

struct Sha256 {
    state: [u32; 8],
    buffer: [u8; 64],
    buffer_len: usize,
    length_bits: u64,
}

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buffer: [0; 64],
            buffer_len: 0,
            length_bits: 0,
        }
    }

    fn digest(bytes: &[u8]) -> [u8; 32] {
        let mut hash = Self::new();
        hash.update(bytes);
        hash.finalize()
    }

    fn update(&mut self, mut bytes: &[u8]) {
        self.length_bits = self.length_bits.wrapping_add((bytes.len() as u64) * 8);
        if self.buffer_len > 0 {
            let copied = (64 - self.buffer_len).min(bytes.len());
            self.buffer[self.buffer_len..self.buffer_len + copied]
                .copy_from_slice(&bytes[..copied]);
            self.buffer_len += copied;
            bytes = &bytes[copied..];
            if self.buffer_len == 64 {
                let block = self.buffer;
                self.compress(&block);
                self.buffer_len = 0;
            } else {
                return;
            }
        }
        while bytes.len() >= 64 {
            let block: [u8; 64] = bytes[..64].try_into().expect("block length should match");
            self.compress(&block);
            bytes = &bytes[64..];
        }
        self.buffer[..bytes.len()].copy_from_slice(bytes);
        self.buffer_len = bytes.len();
    }

    fn finalize(mut self) -> [u8; 32] {
        self.buffer[self.buffer_len] = 0x80;
        self.buffer_len += 1;
        if self.buffer_len > 56 {
            self.buffer[self.buffer_len..].fill(0);
            let block = self.buffer;
            self.compress(&block);
            self.buffer_len = 0;
        }
        self.buffer[self.buffer_len..56].fill(0);
        self.buffer[56..].copy_from_slice(&self.length_bits.to_be_bytes());
        let block = self.buffer;
        self.compress(&block);

        let mut out = [0; 32];
        for (chunk, word) in out.chunks_exact_mut(4).zip(self.state) {
            chunk.copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        const K: [u32; 64] = [
            0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
            0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
            0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
            0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
            0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
            0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
            0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
            0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
            0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
            0xc67178f2,
        ];
        let mut w = [0; 64];
        for (i, chunk) in block.chunks_exact(4).enumerate().take(16) {
            w[i] = u32::from_be_bytes(chunk.try_into().expect("chunk length should match"));
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let sum1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(sum1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let sum0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = sum0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        for (state, value) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *state = state.wrapping_add(value);
        }
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
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier, Condvar, Mutex};
    use std::thread;

    use veln_backend_jvm::{JvmClassFile, JvmProgram};

    static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

    fn jvm_program(classes: &[(&str, &[u8])]) -> JvmProgram {
        JvmProgram {
            classes: classes
                .iter()
                .map(|(path, contents)| JvmClassFile {
                    path: (*path).to_string(),
                    contents: (*contents).to_vec(),
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

    fn cached_path(result: CachedJvmClasses) -> PathBuf {
        match result {
            CachedJvmClasses::Ready(path) => path,
            CachedJvmClasses::ToolError(message) => panic!("unexpected tool error: {message}"),
        }
    }

    fn ready_cache_entries(root: &Path) -> Vec<PathBuf> {
        let mut entries = fs::read_dir(root)
            .expect("cache root should be readable")
            .filter_map(Result::ok)
            .filter(|entry| marker_for(&entry.path()).is_file())
            .map(|entry| entry.path())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    struct CountingHook {
        before_prepare: Option<Arc<Barrier>>,
        prepare_count: AtomicUsize,
    }

    impl CountingHook {
        fn new(before_prepare: Option<Arc<Barrier>>) -> Self {
            Self {
                before_prepare,
                prepare_count: AtomicUsize::new(0),
            }
        }

        fn prepare_count(&self) -> usize {
            self.prepare_count.load(Ordering::SeqCst)
        }
    }

    impl JvmCacheHooks for CountingHook {
        fn before_prepare(&self) {
            self.prepare_count.fetch_add(1, Ordering::SeqCst);
            if let Some(barrier) = &self.before_prepare {
                barrier.wait();
            }
        }
    }

    struct PauseBeforePublishHook {
        state: Arc<(Mutex<PauseBeforePublishState>, Condvar)>,
    }

    struct PauseBeforePublishState {
        reached_publish: bool,
        release_publish: bool,
    }

    impl PauseBeforePublishHook {
        fn new() -> Self {
            Self {
                state: Arc::new((
                    Mutex::new(PauseBeforePublishState {
                        reached_publish: false,
                        release_publish: false,
                    }),
                    Condvar::new(),
                )),
            }
        }

        fn wait_until_reached_publish(&self) {
            let (state, cvar) = &*self.state;
            let mut state = state.lock().expect("publish pause state should lock");
            while !state.reached_publish {
                state = cvar.wait(state).expect("publish pause state should relock");
            }
        }

        fn release_publish(&self) {
            let (state, cvar) = &*self.state;
            let mut state = state.lock().expect("publish pause state should lock");
            state.release_publish = true;
            cvar.notify_all();
        }
    }

    impl JvmCacheHooks for PauseBeforePublishHook {
        fn before_publish_lock(&self) {
            let (state, cvar) = &*self.state;
            let mut state = state.lock().expect("publish pause state should lock");
            state.reached_publish = true;
            cvar.notify_all();
            while !state.release_publish {
                state = cvar.wait(state).expect("publish pause state should relock");
            }
        }
    }

    #[cfg(unix)]
    fn write_fake_java(root: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let tool = root.join("java");
        fs::write(&tool, "#!/bin/sh\nexit 0\n").expect("fake java should be written");
        let mut permissions = fs::metadata(&tool)
            .expect("fake java metadata should be available")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&tool, permissions).expect("fake java should be executable");
        tool
    }

    #[cfg(windows)]
    fn write_fake_java(root: &Path) -> PathBuf {
        let tool = root.join("java.cmd");
        fs::write(&tool, "@echo off\r\nexit /b 0\r\n").expect("fake java should be written");
        tool
    }

    #[cfg(not(any(unix, windows)))]
    fn write_fake_java(_root: &Path) -> PathBuf {
        panic!("fake java is not supported on this platform");
    }

    #[test]
    fn jvm_class_cache_key_tracks_class_path_contents_and_order() {
        let base = jvm_program(&[
            ("VelnProgram.class", b"class VelnProgram {}"),
            ("VelnRuntime.class", b"class VelnRuntime {}"),
        ]);
        let changed_contents = jvm_program(&[
            ("VelnProgram.class", b"class VelnProgram { int value; }"),
            ("VelnRuntime.class", b"class VelnRuntime {}"),
        ]);
        let changed_path = jvm_program(&[
            ("Entry.class", b"class VelnProgram {}"),
            ("VelnRuntime.class", b"class VelnRuntime {}"),
        ]);
        let changed_order = jvm_program(&[
            ("VelnRuntime.class", b"class VelnRuntime {}"),
            ("VelnProgram.class", b"class VelnProgram {}"),
        ]);

        let base_key = jvm_class_cache_key(&base);

        assert_eq!(base_key.len(), 64);
        assert!(base_key.chars().all(|ch| ch.is_ascii_hexdigit()));
        assert_ne!(base_key, jvm_class_cache_key(&changed_contents));
        assert_ne!(base_key, jvm_class_cache_key(&changed_path));
        assert_ne!(base_key, jvm_class_cache_key(&changed_order));
    }

    #[test]
    fn jvm_runner_reports_missing_java_launcher() {
        let root = temp_root("missing-java");
        let result = run_jvm_class_dir(
            root.join("missing-java").as_os_str(),
            &root,
            "veln run",
            &[],
            &[],
        )
        .expect("runner should handle missing launcher");

        match result {
            JvmRunResult::ToolError(message) => {
                assert_eq!(
                    message,
                    "veln: `java` was not found; install a JDK to use `veln run`"
                );
            }
            JvmRunResult::Ran(_) => panic!("missing launcher should not run"),
        }

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn jvm_runner_accepts_harness_owned_success_launcher() {
        let root = temp_root("fake-java");
        let fake_java = write_fake_java(&root);
        let result = run_jvm_class_dir(
            fake_java.as_os_str(),
            &root,
            "veln test",
            &[],
            &["arg".to_string()],
        )
        .expect("fake launcher should run");

        match result {
            JvmRunResult::Ran(output) => {
                assert!(output.status.success());
                assert_eq!(output.stdout, b"");
                assert_eq!(output.stderr, b"");
            }
            JvmRunResult::ToolError(message) => panic!("unexpected tool error: {message}"),
        }

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn sha256_matches_known_digest_and_incremental_writes() {
        assert_eq!(
            hex(&Sha256::digest(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );

        let mut all_at_once = Sha256::new();
        all_at_once.update(b"left-right");

        let mut incremental = Sha256::new();
        incremental.update(b"left");
        incremental.update(b"-");
        incremental.update(b"right");

        assert_eq!(all_at_once.finalize(), incremental.finalize());
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
        assert_eq!(marker_for(&first), first.join(JVM_CACHE_MARKER));
        assert_eq!(marker_for(&second), second.join(JVM_CACHE_MARKER));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn cache_validation_rejects_missing_manifest_and_poisoned_classes() {
        let root = temp_root("cache-validation");
        let program = jvm_program(&[
            ("VelnEntry.class", b"entry"),
            ("support/VelnRuntime.class", b"runtime"),
        ]);
        write_cached_jvm_classes(&root, &program).expect("classes should be written");
        fs::write(marker_for(&root), b"ok\n").expect("marker should be written");

        assert!(
            !validate_cached_jvm_classes(&root, &program).expect("cache should be checked"),
            "marker-only cache should not validate"
        );

        fs::write(manifest_for(&root), render_jvm_cache_manifest(&program))
            .expect("manifest should be written");
        assert!(
            validate_cached_jvm_classes(&root, &program).expect("cache should be checked"),
            "complete cache should validate"
        );

        fs::write(root.join("VelnEntry.class"), b"poisoned").expect("class should be poisoned");
        assert!(
            !validate_cached_jvm_classes(&root, &program).expect("cache should be checked"),
            "poisoned cache should not validate"
        );

        write_cached_jvm_classes(&root, &program).expect("classes should be restored");
        fs::write(root.join("extra.class"), b"extra").expect("extra class should be written");
        assert!(
            !validate_cached_jvm_classes(&root, &program).expect("cache should be checked"),
            "cache with an unexpected class should not validate"
        );

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn cached_jvm_classes_reuse_prepared_entry() {
        let root = temp_root("cache-reuse");
        let program = jvm_program(&[("VelnEntry.class", b"entry")]);

        let first = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be prepared"),
        );
        let second = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be reused"),
        );

        assert_eq!(first, second);
        assert_eq!(ready_cache_entries(&root), vec![first]);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn concurrent_warm_same_key_hits_reuse_valid_entry_without_rebuild() {
        let root = temp_root("cache-warm-concurrent");
        let program = Arc::new(jvm_program(&[("VelnEntry.class", b"entry")]));
        let cache_dir = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be prepared"),
        );
        let hook = Arc::new(CountingHook::new(None));
        let start = Arc::new(Barrier::new(5));

        let handles = (0..4)
            .map(|_| {
                let root = root.clone();
                let program = Arc::clone(&program);
                let hook = Arc::clone(&hook);
                let start = Arc::clone(&start);
                thread::spawn(move || {
                    start.wait();
                    cached_path(
                        ensure_cached_jvm_classes_in_with_hooks(&root, &program, &*hook)
                            .expect("warm cache should be reused"),
                    )
                })
            })
            .collect::<Vec<_>>();
        start.wait();

        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker should finish"))
            .collect::<Vec<_>>();

        assert!(results.iter().all(|path| path == &cache_dir));
        assert_eq!(hook.prepare_count(), 0);
        assert!(validate_cached_jvm_classes(&cache_dir, &program).expect("cache should validate"));
        assert_eq!(ready_cache_entries(&root), vec![cache_dir]);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn cached_jvm_classes_use_new_entry_after_program_changes() {
        let root = temp_root("cache-source-change");
        let initial = jvm_program(&[("VelnEntry.class", b"entry")]);
        let changed = jvm_program(&[("VelnEntry.class", b"changed entry")]);

        let first = cached_path(
            ensure_cached_jvm_classes_in(&root, &initial)
                .expect("initial cache should be prepared"),
        );
        let second = cached_path(
            ensure_cached_jvm_classes_in(&root, &changed)
                .expect("changed cache should be prepared"),
        );

        assert_ne!(first, second);
        assert_eq!(ready_cache_entries(&root), vec![first, second]);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn concurrent_cold_different_key_publications_both_validate() {
        let root = temp_root("cache-cold-different-key-concurrent");
        let left = Arc::new(jvm_program(&[("VelnEntry.class", b"left")]));
        let right = Arc::new(jvm_program(&[("VelnEntry.class", b"right")]));
        let hook = Arc::new(CountingHook::new(Some(Arc::new(Barrier::new(2)))));

        let left_handle = {
            let root = root.clone();
            let left = Arc::clone(&left);
            let hook = Arc::clone(&hook);
            thread::spawn(move || {
                cached_path(
                    ensure_cached_jvm_classes_in_with_hooks(&root, &left, &*hook)
                        .expect("left cache should be prepared"),
                )
            })
        };
        let right_handle = {
            let root = root.clone();
            let right = Arc::clone(&right);
            let hook = Arc::clone(&hook);
            thread::spawn(move || {
                cached_path(
                    ensure_cached_jvm_classes_in_with_hooks(&root, &right, &*hook)
                        .expect("right cache should be prepared"),
                )
            })
        };

        let left_cache = left_handle.join().expect("left worker should finish");
        let right_cache = right_handle.join().expect("right worker should finish");

        assert_ne!(left_cache, right_cache);
        assert!(validate_cached_jvm_classes(&left_cache, &left).expect("left should validate"));
        assert!(validate_cached_jvm_classes(&right_cache, &right).expect("right should validate"));
        let mut expected_entries = vec![left_cache, right_cache];
        expected_entries.sort();
        assert_eq!(ready_cache_entries(&root), expected_entries);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn concurrent_cold_same_key_publication_reuses_winner() {
        let root = temp_root("cache-cold-same-key-concurrent");
        let program = Arc::new(jvm_program(&[("VelnEntry.class", b"entry")]));
        let hook = Arc::new(CountingHook::new(Some(Arc::new(Barrier::new(4)))));

        let handles = (0..4)
            .map(|_| {
                let root = root.clone();
                let program = Arc::clone(&program);
                let hook = Arc::clone(&hook);
                thread::spawn(move || {
                    cached_path(
                        ensure_cached_jvm_classes_in_with_hooks(&root, &program, &*hook)
                            .expect("same-key cache should be prepared"),
                    )
                })
            })
            .collect::<Vec<_>>();

        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker should finish"))
            .collect::<Vec<_>>();
        let cache_dir = results[0].clone();

        assert!(results.iter().all(|path| path == &cache_dir));
        assert_eq!(hook.prepare_count(), 4);
        assert!(validate_cached_jvm_classes(&cache_dir, &program).expect("cache should validate"));
        assert_eq!(ready_cache_entries(&root), vec![cache_dir]);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn cached_jvm_classes_repair_invalid_and_incomplete_entries() {
        let root = temp_root("cache-repair");
        let program = jvm_program(&[("VelnEntry.class", b"entry")]);

        let cache_dir = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be prepared"),
        );
        fs::write(cache_dir.join("VelnEntry.class"), b"poisoned")
            .expect("class should be poisoned");
        let repaired = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be repaired"),
        );
        assert_eq!(repaired, cache_dir);
        assert_eq!(
            fs::read(cache_dir.join("VelnEntry.class")).expect("class should be readable"),
            b"entry"
        );

        fs::remove_file(cache_dir.join("VelnEntry.class")).expect("class should be removed");
        let repaired = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be repaired"),
        );
        assert_eq!(repaired, cache_dir);
        assert_eq!(
            fs::read(cache_dir.join("VelnEntry.class")).expect("class should be readable"),
            b"entry"
        );

        fs::remove_file(manifest_for(&cache_dir)).expect("manifest should be removed");
        let repaired = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be repaired"),
        );
        assert_eq!(repaired, cache_dir);
        assert_eq!(
            fs::read(manifest_for(&cache_dir)).expect("manifest should be readable"),
            render_jvm_cache_manifest(&program)
        );
        assert_eq!(ready_cache_entries(&root), vec![cache_dir]);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn concurrent_invalid_same_key_repair_converges_on_valid_entry() {
        let root = temp_root("cache-repair-concurrent");
        let program = Arc::new(jvm_program(&[("VelnEntry.class", b"entry")]));
        let cache_dir = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("cache should be prepared"),
        );
        fs::write(cache_dir.join("VelnEntry.class"), b"poisoned")
            .expect("class should be poisoned");
        let hook = Arc::new(CountingHook::new(Some(Arc::new(Barrier::new(4)))));

        let handles = (0..4)
            .map(|_| {
                let root = root.clone();
                let program = Arc::clone(&program);
                let hook = Arc::clone(&hook);
                thread::spawn(move || {
                    cached_path(
                        ensure_cached_jvm_classes_in_with_hooks(&root, &program, &*hook)
                            .expect("same-key cache should be repaired"),
                    )
                })
            })
            .collect::<Vec<_>>();

        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("worker should finish"))
            .collect::<Vec<_>>();

        assert!(results.iter().all(|path| path == &cache_dir));
        assert_eq!(hook.prepare_count(), 4);
        assert!(validate_cached_jvm_classes(&cache_dir, &program).expect("cache should validate"));
        assert_eq!(
            fs::read(cache_dir.join("VelnEntry.class")).expect("class should be readable"),
            b"entry"
        );
        assert_eq!(ready_cache_entries(&root), vec![cache_dir]);

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn cache_publish_revalidates_winning_entry_after_race() {
        let root = temp_root("cache-publish-race");
        let program = jvm_program(&[("VelnEntry.class", b"entry")]);
        let compile_dir =
            create_cache_compile_dir(&root, "cache-key").expect("compile dir should be created");
        write_cached_jvm_classes(&compile_dir, &program).expect("classes should be written");
        fs::write(
            manifest_for(&compile_dir),
            render_jvm_cache_manifest(&program),
        )
        .expect("manifest should be written");
        fs::write(marker_for(&compile_dir), b"ok\n").expect("marker should be written");

        let cache_dir = root.join("cache-key");
        fs::create_dir(&cache_dir).expect("winning cache dir should be created");
        write_cached_jvm_classes(&cache_dir, &program).expect("winning classes should be written");
        fs::write(
            manifest_for(&cache_dir),
            render_jvm_cache_manifest(&program),
        )
        .expect("winning manifest should be written");
        fs::write(marker_for(&cache_dir), b"ok\n").expect("winning marker should be written");

        assert!(matches!(
            publish_cached_jvm_classes(&compile_dir, &cache_dir, &program)
                .expect("race should be handled"),
            CachePublish::ReusedValidated
        ));
        assert!(!compile_dir.exists());
        assert!(validate_cached_jvm_classes(&cache_dir, &program).expect("cache should validate"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn cache_publish_replaces_invalid_entry_before_publish() {
        let root = temp_root("cache-publish-invalid");
        let program = jvm_program(&[("VelnEntry.class", b"entry")]);
        let compile_dir =
            create_cache_compile_dir(&root, "cache-key").expect("compile dir should be created");
        write_cached_jvm_classes(&compile_dir, &program).expect("classes should be written");
        fs::write(
            manifest_for(&compile_dir),
            render_jvm_cache_manifest(&program),
        )
        .expect("manifest should be written");
        fs::write(marker_for(&compile_dir), b"ok\n").expect("marker should be written");

        let cache_dir = root.join("cache-key");
        fs::create_dir(&cache_dir).expect("winning cache dir should be created");
        fs::write(marker_for(&cache_dir), b"ok\n").expect("winning marker should be written");

        assert!(matches!(
            publish_cached_jvm_classes(&compile_dir, &cache_dir, &program)
                .expect("race should be handled"),
            CachePublish::Published
        ));
        assert!(!compile_dir.exists());
        assert!(validate_cached_jvm_classes(&cache_dir, &program).expect("cache should validate"));

        fs::remove_dir_all(root).expect("test root should be removed");
    }

    #[test]
    fn publish_loser_revalidates_winner_after_controlled_interleaving() {
        let root = temp_root("cache-publish-interleaving");
        let program = Arc::new(jvm_program(&[("VelnEntry.class", b"entry")]));
        let loser_hook = Arc::new(PauseBeforePublishHook::new());

        let loser_handle = {
            let root = root.clone();
            let program = Arc::clone(&program);
            let loser_hook = Arc::clone(&loser_hook);
            thread::spawn(move || {
                cached_path(
                    ensure_cached_jvm_classes_in_with_hooks(&root, &program, &*loser_hook)
                        .expect("loser should reuse published cache"),
                )
            })
        };
        loser_hook.wait_until_reached_publish();

        let winner = cached_path(
            ensure_cached_jvm_classes_in(&root, &program).expect("winner should publish cache"),
        );
        loser_hook.release_publish();
        let loser = loser_handle.join().expect("loser should finish");

        assert_eq!(loser, winner);
        assert!(validate_cached_jvm_classes(&winner, &program).expect("cache should validate"));
        assert_eq!(ready_cache_entries(&root), vec![winner]);

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
