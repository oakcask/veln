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

struct FailOnceHook {
    point: JvmCacheFaultPoint,
    remaining: AtomicUsize,
}

impl FailOnceHook {
    fn new(point: JvmCacheFaultPoint) -> Self {
        Self {
            point,
            remaining: AtomicUsize::new(1),
        }
    }
}

impl JvmCacheHooks for FailOnceHook {
    fn at_fault_point(&self, point: JvmCacheFaultPoint) -> io::Result<()> {
        if point == self.point && self.remaining.swap(0, Ordering::SeqCst) == 1 {
            return Err(io::Error::other(format!(
                "injected JVM cache fault at {point:?}"
            )));
        }
        Ok(())
    }
}

struct PauseThenFailPublicationHook {
    pause: PauseBeforePublishHook,
}

impl PauseThenFailPublicationHook {
    fn new() -> Self {
        Self {
            pause: PauseBeforePublishHook::new(),
        }
    }

    fn wait_until_reached_publish(&self) {
        self.pause.wait_until_reached_publish();
    }

    fn release_publish(&self) {
        self.pause.release_publish();
    }
}

impl JvmCacheHooks for PauseThenFailPublicationHook {
    fn before_publish_lock(&self) {
        self.pause.before_publish_lock();
    }

    fn at_fault_point(&self, point: JvmCacheFaultPoint) -> io::Result<()> {
        if point == JvmCacheFaultPoint::Publication {
            return Err(io::Error::other("injected failed writer publication"));
        }
        Ok(())
    }
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

fn cache_snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn collect(root: &Path, dir: &Path, snapshot: &mut Vec<(String, Vec<u8>)>) {
        for entry in fs::read_dir(dir).expect("snapshot directory should be readable") {
            let entry = entry.expect("snapshot entry should be readable");
            let path = entry.path();
            if path.is_dir() {
                collect(root, &path, snapshot);
            } else {
                let relative = path
                    .strip_prefix(root)
                    .expect("snapshot path should be below root")
                    .to_string_lossy()
                    .replace('\\', "/");
                snapshot.push((
                    relative,
                    fs::read(path).expect("snapshot file should be readable"),
                ));
            }
        }
    }

    let mut snapshot = Vec::new();
    collect(root, root, &mut snapshot);
    snapshot.sort_by(|left, right| left.0.cmp(&right.0));
    snapshot
}

fn cache_root_paths(root: &Path) -> Vec<PathBuf> {
    let mut paths = fs::read_dir(root)
        .expect("cache root should be readable")
        .map(|entry| entry.expect("cache root entry should be readable").path())
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[path = "tests/cache_lifecycle.rs"]
mod cache_lifecycle;
#[path = "tests/cache_roots.rs"]
mod cache_roots;
#[path = "tests/launcher.rs"]
mod launcher;
#[path = "tests/process_status.rs"]
mod process_status;

#[cfg(unix)]
fn write_fake_java(root: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(root).expect("fake java directory should be created");
    let tool = root.join("java");
    fs::write(&tool, "#!/bin/sh\nexit 0\n").expect("fake java should be written");
    let mut permissions = fs::metadata(&tool)
        .expect("fake java metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).expect("fake java should be executable");
    tool
}

#[cfg(unix)]
fn write_recording_fake_java(root: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    fs::create_dir_all(root).expect("fake java directory should be created");
    let tool = root.join("java");
    fs::write(&tool, "#!/bin/sh\nprintf started > \"$JAVA_MARKER\"\n")
        .expect("recording fake java should be written");
    let mut permissions = fs::metadata(&tool)
        .expect("recording fake java metadata should be available")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&tool, permissions).expect("recording fake java should be executable");
    tool
}

#[cfg(windows)]
fn write_fake_java(root: &Path) -> PathBuf {
    fs::create_dir_all(root).expect("fake java directory should be created");
    let tool = root.join("java.cmd");
    fs::write(&tool, "@echo off\r\nexit /b 0\r\n").expect("fake java should be written");
    tool
}

#[cfg(windows)]
fn write_recording_fake_java(root: &Path) -> PathBuf {
    fs::create_dir_all(root).expect("fake java directory should be created");
    let tool = root.join("java.cmd");
    fs::write(&tool, "@echo off\r\n>\"%JAVA_MARKER%\" echo started\r\n")
        .expect("recording fake java should be written");
    tool
}

#[cfg(not(any(unix, windows)))]
fn write_fake_java(_root: &Path) -> PathBuf {
    panic!("fake java is not supported on this platform");
}

#[cfg(not(any(unix, windows)))]
fn write_recording_fake_java(_root: &Path) -> PathBuf {
    panic!("recording fake java is not supported on this platform");
}
