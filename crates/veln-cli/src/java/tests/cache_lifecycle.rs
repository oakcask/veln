use super::*;

#[test]
fn cache_compile_dirs_are_unique_and_markers_stay_inside_them() {
    let root = temp_root("compile-dir");

    let first = create_cache_compile_dir(&root, "cache-key").expect("first dir should be created");
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
    let second =
        cached_path(ensure_cached_jvm_classes_in(&root, &program).expect("cache should be reused"));

    assert_eq!(first, second);
    assert_eq!(ready_cache_entries(&root), vec![first]);

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn occupied_cache_lock_reaches_focused_timeout() {
    let root = temp_root("cache-lock-timeout");
    let lock_dir = root.join("cache-key.lock");
    fs::create_dir(&lock_dir).expect("occupied lock should be created");

    let error = match JvmCacheLock::acquire_with_timeout(&lock_dir, Duration::ZERO) {
        Ok(_) => panic!("occupied lock should time out"),
        Err(error) => error,
    };

    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert_eq!(
        error.to_string(),
        "timed out waiting for JVM cache coordination"
    );
    assert!(lock_dir.is_dir());
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
        ensure_cached_jvm_classes_in(&root, &initial).expect("initial cache should be prepared"),
    );
    let second = cached_path(
        ensure_cached_jvm_classes_in(&root, &changed).expect("changed cache should be prepared"),
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

#[cfg(any(unix, windows))]
#[test]
fn invalid_entry_removal_failure_stops_java_and_later_revalidates_entry() {
    let root = temp_root("cache-removal-failure");
    let cache_root = root.join("cache");
    let program = jvm_program(&[("VelnEntry.class", b"entry")]);
    let cache_dir = cached_path(
        ensure_cached_jvm_classes_in(&cache_root, &program).expect("cache should be prepared"),
    );
    fs::write(cache_dir.join(JVM_ENTRY_CLASS), b"poisoned")
        .expect("cache entry should be poisoned");
    let java_launcher = write_recording_fake_java(&root.join("tools"));
    let java_marker = root.join("java-started");
    let execution = JvmExecution {
        cache_root: cache_root.clone(),
        java_launcher,
    };
    let fault = FailOnceHook::new(JvmCacheFaultPoint::InvalidEntryRemoval);

    let error = prepare_and_run_jvm_capture_with_execution_and_hooks(
        &execution,
        &program,
        "veln run",
        &[("JAVA_MARKER", java_marker.as_os_str())],
        &[],
        &fault,
    )
    .expect_err("cache removal failure should stop the command");

    assert!(error.contains("could not remove invalid JVM cache entry"));
    assert!(
        !java_marker.exists(),
        "Java must not start after cache failure"
    );
    assert_eq!(
        fs::read(cache_dir.join(JVM_ENTRY_CLASS)).expect("invalid entry should remain"),
        b"poisoned"
    );

    let repaired = cached_path(
        ensure_cached_jvm_classes_in(&cache_root, &program)
            .expect("later invocation should repair the entry"),
    );
    assert_eq!(repaired, cache_dir);
    assert!(
        validate_cached_jvm_classes(&repaired, &program).expect("repaired entry should validate")
    );

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn prepared_entry_validation_failure_cleans_up_and_allows_retry_and_reuse() {
    let root = temp_root("cache-prepared-validation-failure");
    let program = jvm_program(&[("VelnEntry.class", b"entry")]);
    let cache_dir = cached_path(
        ensure_cached_jvm_classes_in(&root, &program).expect("cache should be prepared"),
    );
    fs::write(cache_dir.join(JVM_ENTRY_CLASS), b"poisoned")
        .expect("cache entry should be poisoned");
    let fault = FailOnceHook::new(JvmCacheFaultPoint::PreparedEntryValidation);

    let error = ensure_cached_jvm_classes_in_with_hooks(&root, &program, &fault)
        .expect_err("prepared entry validation should fail");

    assert!(error.contains("could not prepare JVM cache entry"));
    assert!(
        cache_root_paths(&root).is_empty(),
        "failure should leave a miss"
    );

    let regenerated = cached_path(
        ensure_cached_jvm_classes_in(&root, &program)
            .expect("later invocation should regenerate the entry"),
    );
    let hit_hook = CountingHook::new(None);
    let reused = cached_path(
        ensure_cached_jvm_classes_in_with_hooks(&root, &program, &hit_hook)
            .expect("regenerated entry should be reusable"),
    );
    assert_eq!(reused, regenerated);
    assert_eq!(hit_hook.prepare_count(), 0);
    assert!(
        validate_cached_jvm_classes(&regenerated, &program)
            .expect("regenerated entry should validate")
    );

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn publication_failure_leaves_selected_root_as_miss_and_allows_retry() {
    let root = temp_root("cache-publication-failure");
    let program = jvm_program(&[("VelnEntry.class", b"entry")]);
    let fault = FailOnceHook::new(JvmCacheFaultPoint::Publication);

    let error = ensure_cached_jvm_classes_in_with_hooks(&root, &program, &fault)
        .expect_err("publication should fail");

    assert!(error.contains("could not publish JVM cache entry"));
    assert!(
        cache_root_paths(&root).is_empty(),
        "selected root should contain no partial entry"
    );

    let published = cached_path(
        ensure_cached_jvm_classes_in(&root, &program)
            .expect("later invocation should retry publication"),
    );
    let reused = cached_path(
        ensure_cached_jvm_classes_in(&root, &program).expect("published entry should be reusable"),
    );
    assert_eq!(reused, published);
    assert_eq!(ready_cache_entries(&root), vec![published]);

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
fn cached_jvm_classes_repair_invalid_and_incomplete_entries() {
    let root = temp_root("cache-repair");
    let program = jvm_program(&[("VelnEntry.class", b"entry")]);

    let cache_dir = cached_path(
        ensure_cached_jvm_classes_in(&root, &program).expect("cache should be prepared"),
    );
    fs::write(cache_dir.join("VelnEntry.class"), b"poisoned").expect("class should be poisoned");
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
    fs::write(cache_dir.join("VelnEntry.class"), b"poisoned").expect("class should be poisoned");
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

#[test]
fn failed_writer_preserves_concurrently_published_winner_byte_for_byte() {
    let root = temp_root("cache-failed-writer-isolation");
    let program = Arc::new(jvm_program(&[("VelnEntry.class", b"entry")]));
    let failed_writer_hook = Arc::new(PauseThenFailPublicationHook::new());

    let failed_writer = {
        let root = root.clone();
        let program = Arc::clone(&program);
        let failed_writer_hook = Arc::clone(&failed_writer_hook);
        thread::spawn(move || {
            ensure_cached_jvm_classes_in_with_hooks(&root, &program, &*failed_writer_hook)
        })
    };
    failed_writer_hook.wait_until_reached_publish();

    let winner = cached_path(
        ensure_cached_jvm_classes_in(&root, &program).expect("winner should publish cache"),
    );
    let winner_before_failure = cache_snapshot(&winner);
    failed_writer_hook.release_publish();
    let error = failed_writer
        .join()
        .expect("failed writer should finish")
        .expect_err("failed writer publication should report an error");

    assert!(error.contains("could not publish JVM cache entry"));
    assert_eq!(cache_snapshot(&winner), winner_before_failure);
    assert!(validate_cached_jvm_classes(&winner, &program).expect("winner should validate"));
    assert_eq!(cache_root_paths(&root), vec![winner.clone()]);

    let hit_hook = CountingHook::new(None);
    let reused = cached_path(
        ensure_cached_jvm_classes_in_with_hooks(&root, &program, &hit_hook)
            .expect("winner should remain reusable"),
    );
    assert_eq!(reused, winner);
    assert_eq!(hit_hook.prepare_count(), 0);

    fs::remove_dir_all(root).expect("test root should be removed");
}
