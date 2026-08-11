use std::collections::BTreeSet;
use std::fs;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug)]
pub(crate) struct DiscoveryRoot {
    pub(crate) id: &'static str,
    pub(crate) relative: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CaseDescriptor {
    pub(crate) id: String,
    pub(crate) root_id: &'static str,
    pub(crate) case_relative: PathBuf,
    pub(crate) manifest_relative: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Preflight {
    pub(crate) cases: Vec<CaseDescriptor>,
}

pub(crate) const DISCOVERY_ROOTS: [DiscoveryRoot; 2] = [
    DiscoveryRoot {
        id: "crates/veln-cli/tests/toolchain_cases",
        relative: "tests/toolchain_cases",
    },
    DiscoveryRoot {
        id: "examples/specification",
        relative: "../../examples/specification",
    },
];

pub(crate) fn run_preflight(manifest_dir: &Path) -> Result<Preflight, String> {
    run_preflight_with_roots(manifest_dir, &DISCOVERY_ROOTS)
}

pub(crate) fn run_preflight_with_roots(
    manifest_dir: &Path,
    roots: &[DiscoveryRoot],
) -> Result<Preflight, String> {
    run_preflight_with_roots_and_policy(manifest_dir, roots, policy_enforcement_enabled())
}

pub(crate) fn run_preflight_with_roots_and_policy(
    manifest_dir: &Path,
    roots: &[DiscoveryRoot],
    enforce_policy: bool,
) -> Result<Preflight, String> {
    let mut errors = validate_roots(manifest_dir, roots);
    let mut cases = Vec::new();
    if errors.is_empty() {
        for root in roots {
            let root_path = manifest_dir.join(root.relative);
            match discover_root(root, &root_path) {
                Ok(mut discovered) => cases.append(&mut discovered),
                Err(mut discovered_errors) => errors.append(&mut discovered_errors),
            }
        }
    }
    cases.sort_by(|left, right| left.id.cmp(&right.id));

    if errors.is_empty() && enforce_policy {
        errors.extend(scan_policy(manifest_dir, &cases));
    }

    if errors.is_empty() {
        Ok(Preflight { cases })
    } else {
        Err(render_preflight_errors(errors))
    }
}

pub(crate) fn compare_generated_inventory(
    manifest_dir: &Path,
    generated: &[&str],
) -> Result<Preflight, String> {
    compare_generated_inventory_with_policy(manifest_dir, generated, policy_enforcement_enabled())
}

pub(crate) fn compare_generated_inventory_with_policy(
    manifest_dir: &Path,
    generated: &[&str],
    enforce_policy: bool,
) -> Result<Preflight, String> {
    let preflight =
        run_preflight_with_roots_and_policy(manifest_dir, &DISCOVERY_ROOTS, enforce_policy)?;
    let current = preflight
        .cases
        .iter()
        .map(|case| case.manifest_relative.to_string_lossy().replace('\\', "/"))
        .collect::<BTreeSet<_>>();
    let generated = generated.iter().map(|path| (*path).to_string()).collect();
    if current == generated {
        return Ok(preflight);
    }

    let mut errors = Vec::new();
    for added in current.difference(&generated) {
        errors.push(format!(
            "{added}: rebuild the toolchain harness because this case manifest was added after test generation"
        ));
    }
    for removed in generated.difference(&current) {
        errors.push(format!(
            "{removed}: rebuild the toolchain harness because this generated case manifest is no longer discovered"
        ));
    }
    Err(render_preflight_errors(errors))
}

fn validate_roots(manifest_dir: &Path, roots: &[DiscoveryRoot]) -> Vec<String> {
    let mut errors = Vec::new();
    let mut canonical = Vec::new();
    for root in roots {
        let path = manifest_dir.join(root.relative);
        match fs::canonicalize(&path) {
            Ok(path) => canonical.push((root.id, path)),
            Err(error) => errors.push(format!(
                "{}: discovery root must be readable before toolchain case generation: {error}",
                root.id
            )),
        }
    }
    for left_index in 0..canonical.len() {
        for right_index in left_index + 1..canonical.len() {
            let (left_id, left) = &canonical[left_index];
            let (right_id, right) = &canonical[right_index];
            if left == right || left.starts_with(right) || right.starts_with(left) {
                errors.push(format!(
                    "{left_id} and {right_id}: configured discovery roots overlap; move one root so each case has one owner"
                ));
            }
        }
    }
    errors
}

fn discover_root(
    root: &DiscoveryRoot,
    root_path: &Path,
) -> Result<Vec<CaseDescriptor>, Vec<String>> {
    let mut errors = Vec::new();
    let mut cases = Vec::new();
    let root_manifest = root_path.join("case.toml");
    let root_ancestor = if root_manifest.exists() {
        errors.push(format!(
            "{}: remove root-level case.toml or move it below a case directory; discovery roots are containers",
            root.id
        ));
        Some(PathBuf::from("case.toml"))
    } else {
        None
    };
    discover_dir(
        root,
        root_path,
        Path::new(""),
        root_ancestor,
        &mut cases,
        &mut errors,
    );
    if errors.is_empty() {
        Ok(cases)
    } else {
        Err(errors)
    }
}

fn discover_dir(
    root: &DiscoveryRoot,
    dir: &Path,
    relative: &Path,
    ancestor_manifest: Option<PathBuf>,
    cases: &mut Vec<CaseDescriptor>,
    errors: &mut Vec<String>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(format!(
                "{}: read discovery directory failed: {error}",
                display_root_path(root.id, relative)
            ));
            return;
        }
    };
    let mut entries = match entries.collect::<Result<Vec<_>, _>>() {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(format!(
                "{}: enumerate discovery directory failed: {error}",
                display_root_path(root.id, relative)
            ));
            return;
        }
    };
    entries.sort_by_key(|entry| entry.file_name());

    let mut local_manifest = ancestor_manifest.clone();
    for entry in &entries {
        if entry.file_name() == "case.toml" {
            let manifest_relative = relative.join("case.toml");
            if let Some(ancestor) = &ancestor_manifest {
                errors.push(format!(
                    "{}: nested case.toml is below {}; remove the nested manifest or move it to a sibling case directory",
                    display_root_path(root.id, &manifest_relative),
                    display_root_path(root.id, ancestor)
                ));
            } else if !relative.as_os_str().is_empty() {
                let manifest_relative_to_crate = Path::new(root.relative).join(&manifest_relative);
                cases.push(CaseDescriptor {
                    id: format!("{}/{}", root.id, slash_path(relative)),
                    root_id: root.id,
                    case_relative: relative.to_path_buf(),
                    manifest_relative: manifest_relative_to_crate
                        .parent()
                        .expect("case manifest should have a directory")
                        .to_path_buf(),
                });
                local_manifest = Some(manifest_relative);
            }
        }
    }

    for entry in entries {
        let entry_relative = relative.join(entry.file_name());
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(format!(
                    "{}: inspect discovery entry failed: {error}",
                    display_root_path(root.id, &entry_relative)
                ));
                continue;
            }
        };
        if is_link_like(&metadata) {
            errors.push(format!(
                "{}: replace the link or reparse point with a regular fixture entry; discovery never follows entries that can hide or escape a case",
                display_root_path(root.id, &entry_relative)
            ));
            continue;
        }
        if metadata.is_dir() {
            discover_dir(
                root,
                &entry.path(),
                &entry_relative,
                local_manifest.clone(),
                cases,
                errors,
            );
        }
    }
}

fn scan_policy(manifest_dir: &Path, cases: &[CaseDescriptor]) -> Vec<String> {
    let mut errors = Vec::new();
    for case in cases {
        let manifest = manifest_dir.join(&case.manifest_relative).join("case.toml");
        let text = match fs::read_to_string(&manifest) {
            Ok(text) => text,
            Err(error) => {
                errors.push(format!(
                    "{}: read manifest failed before policy validation: {error}",
                    case.id
                ));
                continue;
            }
        };
        let findings = match panic::catch_unwind(AssertUnwindSafe(|| {
            crate::manifest_syntax::manifest_policy_findings(&manifest, &text)
        })) {
            Ok(findings) => findings,
            Err(panic) => {
                errors.push(format!(
                    "{}: manifest policy scan failed before command generation: {}",
                    case.id,
                    panic_message(panic)
                ));
                continue;
            }
        };
        for finding in findings {
            errors.push(format!(
                "{}:{}:{}-{} field `{}` contains {} `{}`; use physical multiline text or a sidecar so line structure remains reviewable",
                case.id,
                finding.line,
                finding.start,
                finding.end,
                finding.field,
                finding.category,
                finding.spelling.escape_debug()
            ));
        }
    }
    errors
}

fn panic_message(panic: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&str>() {
        return message.to_string();
    }
    "non-string panic".to_string()
}

fn policy_enforcement_enabled() -> bool {
    std::env::var("VELN_TOOLCHAIN_CASE_POLICY").is_ok_and(|value| value == "deny")
}

fn render_preflight_errors(errors: Vec<String>) -> String {
    let mut report = format!(
        "toolchain case preflight found {} problem(s) in the authoritative case inventory",
        errors.len()
    );
    report.push_str(
        "\nmove encoded line structure to physical multiline manifest values or sidecars, replace links with regular fixture entries, and rebuild so policy and execution use one visible portable case set",
    );
    for error in errors {
        report.push('\n');
        report.push_str("- ");
        report.push_str(&error);
    }
    report
}

fn display_root_path(root_id: &str, relative: &Path) -> String {
    if relative.as_os_str().is_empty() {
        root_id.to_string()
    } else {
        format!("{root_id}/{}", slash_path(relative))
    }
}

pub(crate) fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(unix)]
fn is_link_like(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(windows)]
fn is_link_like(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::FileTypeExt;

    let file_type = metadata.file_type();
    file_type.is_symlink() || file_type.is_symlink_dir() || file_type.is_symlink_file()
}
