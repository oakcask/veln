use super::*;

#[derive(Clone, Copy)]
pub(super) enum CacheHost {
    Unix,
    Macos,
    Windows,
    Other,
}

pub(super) fn resolve_veln_cache_root() -> Result<PathBuf, String> {
    let host = if cfg!(target_os = "macos") {
        CacheHost::Macos
    } else if cfg!(windows) {
        CacheHost::Windows
    } else if cfg!(unix) {
        CacheHost::Unix
    } else {
        CacheHost::Other
    };
    resolve_veln_cache_root_from(
        host,
        env::var_os("VELN_CACHE_DIR"),
        env::var_os("XDG_CACHE_HOME"),
        env::var_os("HOME"),
        env::var_os("LOCALAPPDATA"),
    )
}

pub(super) fn resolve_veln_cache_root_from(
    host: CacheHost,
    cache_override: Option<OsString>,
    xdg_cache_home: Option<OsString>,
    home: Option<OsString>,
    local_app_data: Option<OsString>,
) -> Result<PathBuf, String> {
    if let Some(cache_override) = cache_override {
        return usable_absolute_path(&cache_override).ok_or_else(|| {
            "invalid VELN_CACHE_DIR: expected a non-empty absolute path".to_string()
        });
    }

    let base = match host {
        CacheHost::Unix => usable_absolute_path_option(xdg_cache_home)
            .or_else(|| usable_absolute_path_option(home).map(|path| path.join(".cache"))),
        CacheHost::Macos => {
            usable_absolute_path_option(home).map(|path| path.join("Library").join("Caches"))
        }
        CacheHost::Windows => usable_absolute_path_option(local_app_data),
        CacheHost::Other => None,
    };
    base.map(|path| path.join("veln")).ok_or_else(|| {
        "user cache directory is unavailable; set VELN_CACHE_DIR to a non-empty absolute path"
            .to_string()
    })
}

pub(super) fn usable_absolute_path_option(value: Option<OsString>) -> Option<PathBuf> {
    value.and_then(|value| usable_absolute_path(&value))
}

pub(super) fn usable_absolute_path(value: &OsStr) -> Option<PathBuf> {
    if value.is_empty() {
        return None;
    }
    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
}

pub(super) fn find_java_launcher() -> Option<PathBuf> {
    find_java_launcher_in_path(&env::var_os("PATH")?)
}

pub(super) fn find_java_launcher_in_path(path: &OsStr) -> Option<PathBuf> {
    env::split_paths(path)
        .flat_map(|directory| java_launcher_candidates(&directory))
        .find(|candidate| {
            #[cfg(unix)]
            {
                is_executable_file(candidate)
            }
            #[cfg(not(unix))]
            {
                is_executable_file(candidate)
            }
        })
}

#[cfg(windows)]
pub(super) fn java_launcher_candidates(directory: &Path) -> Vec<PathBuf> {
    ["java.exe", "java.cmd", "java.bat", "java"]
        .into_iter()
        .map(|name| directory.join(name))
        .collect()
}

#[cfg(not(windows))]
pub(super) fn java_launcher_candidates(directory: &Path) -> Vec<PathBuf> {
    vec![directory.join("java")]
}

#[cfg(unix)]
pub(super) fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    current_process_can_execute(path)
}

#[cfg(not(unix))]
pub(super) fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
pub(super) fn current_process_can_execute(path: &Path) -> bool {
    ProcessCommand::new("/bin/sh")
        .arg("-c")
        .arg("test -x \"$1\"")
        .arg("veln-java-access-check")
        .arg(path)
        .status()
        .is_ok_and(|status| status.success())
}
