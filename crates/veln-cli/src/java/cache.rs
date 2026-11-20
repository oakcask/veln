use super::*;

pub(super) fn reuse_valid_cache_or_remove_invalid(
    lock_dir: &Path,
    cache_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
    hooks: &dyn JvmCacheHooks,
) -> Result<bool, String> {
    let _lock = JvmCacheLock::acquire(lock_dir).map_err(|error| error.to_string())?;
    hooks.after_initial_lock();
    if validated_cache_exists(cache_dir, program)? {
        return Ok(true);
    }
    remove_invalid_cache(cache_dir, hooks)?;
    Ok(false)
}

pub(super) fn publish_prepared_cache_under_lock(
    lock_dir: &Path,
    compile_dir: &Path,
    cache_dir: &Path,
    program: &veln_backend_jvm::JvmProgram,
    hooks: &dyn JvmCacheHooks,
) -> Result<CachePublish, String> {
    let _lock = JvmCacheLock::acquire(lock_dir).map_err(|error| {
        let _ = fs::remove_dir_all(compile_dir);
        error.to_string()
    })?;
    if let Err(error) = hooks.at_fault_point(JvmCacheFaultPoint::Publication) {
        let _ = fs::remove_dir_all(compile_dir);
        return Err(format!("could not publish JVM cache entry: {error}"));
    }
    publish_prepared_jvm_cache(compile_dir, cache_dir, program)
}

pub(super) fn remove_failed_cache_preparation(
    compile_dir: &Path,
    preparation: &Result<CacheCompilePreparation, String>,
) {
    if preparation.is_err() || matches!(preparation, Ok(CacheCompilePreparation::ToolError(_))) {
        let _ = fs::remove_dir_all(compile_dir);
    }
}
