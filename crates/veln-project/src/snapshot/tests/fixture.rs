use super::*;

pub(super) struct SnapshotFixture {
    root: PathBuf,
}

impl SnapshotFixture {
    pub(super) fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-package-snapshot-{label}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    pub(super) fn root(&self) -> &Path {
        &self.root
    }

    pub(super) fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub(super) fn write_bytes(&self, relative: &str, bytes: &[u8]) {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    #[cfg(windows)]
    pub(super) fn write_bytes_raw_wide_relative(&self, relative: &[u16], bytes: &[u8]) {
        use std::os::windows::ffi::OsStringExt;

        let path = self.root.join(std::ffi::OsString::from_wide(relative));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    #[cfg(unix)]
    pub(super) fn write_bytes_raw_relative(&self, relative: &[u8], bytes: &[u8]) {
        use std::os::unix::ffi::OsStrExt;

        let path = self.root.join(Path::new(OsStr::from_bytes(relative)));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, bytes).unwrap();
    }

    #[cfg(unix)]
    pub(super) fn mkfifo(&self, relative: &str) {
        let path = self.path(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let status = std::process::Command::new("mkfifo")
            .arg(&path)
            .status()
            .unwrap();
        assert!(status.success());
    }
}

#[cfg(windows)]
pub(super) fn ill_formed_wide_name(suffix: &str) -> Vec<u16> {
    let mut name = "ignored-".encode_utf16().collect::<Vec<_>>();
    name.push(0xD800);
    name.extend(suffix.encode_utf16());
    name
}

impl Drop for SnapshotFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
