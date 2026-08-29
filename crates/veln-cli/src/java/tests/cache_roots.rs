use super::*;

#[test]
fn explicit_cache_root_is_complete_and_preserves_lexical_components() {
    let root = temp_root("override-root");
    let cache_override = root.join("segment").join("..").join("selected");

    let selected = resolve_veln_cache_root_from(
        CacheHost::Other,
        Some(cache_override.clone().into_os_string()),
        None,
        None,
        None,
    )
    .expect("absolute override should be selected");

    assert_eq!(selected, cache_override);
    assert!(!selected.ends_with("veln"));
}

#[test]
fn invalid_cache_override_never_falls_back_to_host_base() {
    let host_base = temp_root("override-precedence");
    for cache_override in [OsString::new(), OsString::from("relative-cache")] {
        let error = resolve_veln_cache_root_from(
            CacheHost::Unix,
            Some(cache_override),
            Some(host_base.clone().into_os_string()),
            Some(host_base.clone().into_os_string()),
            None,
        )
        .expect_err("invalid override should fail");
        assert!(error.contains("invalid VELN_CACHE_DIR"));
    }
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn unix_cache_root_uses_xdg_then_absolute_home_fallback() {
    let root = temp_root("unix-defaults");
    let xdg = root.join("xdg");
    let home = root.join("home");

    let selected_xdg = resolve_veln_cache_root_from(
        CacheHost::Unix,
        None,
        Some(xdg.clone().into_os_string()),
        Some(home.clone().into_os_string()),
        None,
    )
    .expect("XDG cache root should resolve");
    assert_eq!(selected_xdg, xdg.join("veln"));

    for unusable_xdg in [
        None,
        Some(OsString::new()),
        Some(OsString::from("relative")),
    ] {
        let selected_home = resolve_veln_cache_root_from(
            CacheHost::Unix,
            None,
            unusable_xdg,
            Some(home.clone().into_os_string()),
            None,
        )
        .expect("HOME fallback should resolve");
        assert_eq!(selected_home, home.join(".cache").join("veln"));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn macos_cache_root_uses_home_library_caches() {
    let home = temp_root("macos-default");
    let selected = resolve_veln_cache_root_from(
        CacheHost::Macos,
        None,
        None,
        Some(home.clone().into_os_string()),
        None,
    )
    .expect("macOS cache root should resolve");
    assert_eq!(selected, home.join("Library/Caches/veln"));
}

#[cfg(windows)]
#[test]
fn windows_cache_root_uses_local_app_data() {
    let local_app_data = temp_root("windows-default");
    let selected = resolve_veln_cache_root_from(
        CacheHost::Windows,
        None,
        None,
        None,
        Some(local_app_data.clone().into_os_string()),
    )
    .expect("Windows cache root should resolve");
    assert_eq!(selected, local_app_data.join("veln"));
}

#[test]
fn unavailable_host_cache_base_has_no_local_fallback() {
    let error = resolve_veln_cache_root_from(CacheHost::Other, None, None, None, None)
        .expect_err("unsupported host should require an override");
    assert!(error.contains("user cache directory is unavailable"));
}

#[cfg(unix)]
#[test]
fn non_unicode_absolute_override_remains_a_native_path() {
    use std::os::unix::ffi::OsStringExt;

    let value = OsString::from_vec(b"/tmp/veln-cache-\xff".to_vec());
    let selected =
        resolve_veln_cache_root_from(CacheHost::Other, Some(value.clone()), None, None, None)
            .expect("native absolute override should resolve");
    assert_eq!(selected.into_os_string(), value);
}

#[cfg(unix)]
#[test]
fn non_unicode_unix_base_remains_a_native_path() {
    use std::os::unix::ffi::OsStringExt;

    let value = OsString::from_vec(b"/tmp/veln-xdg-\xff".to_vec());
    let selected =
        resolve_veln_cache_root_from(CacheHost::Unix, None, Some(value.clone()), None, None)
            .expect("native XDG base should resolve");
    assert_eq!(selected, PathBuf::from(value).join("veln"));
}

#[cfg(windows)]
#[test]
fn non_unicode_windows_values_remain_native_paths() {
    use std::os::windows::ffi::OsStringExt;

    let mut units = "C:\\veln-cache-".encode_utf16().collect::<Vec<_>>();
    units.push(0xd800);
    let value = OsString::from_wide(&units);

    let selected_override =
        resolve_veln_cache_root_from(CacheHost::Windows, Some(value.clone()), None, None, None)
            .expect("native Windows override should resolve");
    assert_eq!(selected_override.clone().into_os_string(), value);

    let selected_base = resolve_veln_cache_root_from(
        CacheHost::Windows,
        None,
        None,
        None,
        Some(selected_override.clone().into_os_string()),
    )
    .expect("native LOCALAPPDATA should resolve");
    assert_eq!(selected_base, selected_override.join("veln"));
}
