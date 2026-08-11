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
    run_preflight_with_roots_and_policy(manifest_dir, roots, true)
}

pub(crate) fn run_preflight_with_roots_and_policy(
    manifest_dir: &Path,
    roots: &[DiscoveryRoot],
    enforce_policy: bool,
) -> Result<Preflight, String> {
    let validation = validate_roots(manifest_dir, roots);
    let mut errors = validation.errors;
    let mut cases = Vec::new();
    if !validation.has_overlap {
        for root in validation.readable_roots {
            let root_path = manifest_dir.join(root.relative);
            let mut discovered = discover_root(root, &root_path);
            cases.append(&mut discovered.cases);
            errors.append(&mut discovered.errors);
        }
    }
    cases.sort_by(|left, right| left.id.cmp(&right.id));

    if enforce_policy {
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
    compare_generated_inventory_with_policy(manifest_dir, generated, true)
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
        errors.push(PreflightError::manifest(
            added.clone(),
            format!(
                "{added}: rebuild the toolchain harness because this case manifest was added after test generation"
            ),
        ));
    }
    for removed in generated.difference(&current) {
        errors.push(PreflightError::manifest(
            removed.clone(),
            format!(
                "{removed}: rebuild the toolchain harness because this generated case manifest is no longer discovered"
            ),
        ));
    }
    Err(render_preflight_errors(errors))
}

pub(crate) fn generated_toolchain_tests_from_preflight(
    manifest_dir: &Path,
    roots: &[DiscoveryRoot],
    enforce_policy: bool,
) -> Result<String, String> {
    let preflight = run_preflight_with_roots_and_policy(manifest_dir, roots, enforce_policy)?;
    let cases = preflight
        .cases
        .iter()
        .map(|case| case.manifest_relative.clone())
        .collect::<Vec<_>>();
    Ok(generated_toolchain_tests(&cases))
}

pub(crate) fn generated_toolchain_tests(cases: &[PathBuf]) -> String {
    let mut names = BTreeSet::new();
    let mut out = String::from(
        "mod toolchain_semantic_baseline {\n    include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/tests/toolchain_semantic_baseline/mod.rs\"));\n}\n\nconst GENERATED_TOOLCHAIN_CASES: &[&str] = &[\n",
    );
    for case in cases {
        let case = slash_path(case);
        out.push_str(&format!("    {case:?},\n"));
    }
    out.push_str("];\n\nmod generated_toolchain_cases {\n    use super::*;\n\n");
    for case in cases {
        let name = unique_test_name(case, &mut names);
        let case = slash_path(case);
        out.push_str("    #[test]\n");
        out.push_str(&format!("    fn {name}() {{\n"));
        out.push_str(&format!(
            "        run_case(&toolchain_case_path({case:?}));\n"
        ));
        out.push_str("    }\n\n");
    }
    out.push_str("}\n");
    out
}

fn unique_test_name(case: &Path, names: &mut BTreeSet<String>) -> String {
    let raw = case.to_string_lossy();
    let mut name = String::from("toolchain_case_");
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            name.push(ch.to_ascii_lowercase());
        } else if !name.ends_with('_') {
            name.push('_');
        }
    }
    while name.ends_with('_') {
        name.pop();
    }

    let hash = fnv1a(raw.as_bytes());
    let max_prefix_len = 96usize.saturating_sub(17);
    if name.len() > max_prefix_len {
        name.truncate(max_prefix_len);
        while name.ends_with('_') {
            name.pop();
        }
    }
    name.push_str(&format!("_{hash:016x}"));

    assert!(
        names.insert(name.clone()),
        "duplicate generated test `{name}`"
    );
    name
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

struct RootValidation<'a> {
    errors: Vec<PreflightError>,
    readable_roots: Vec<&'a DiscoveryRoot>,
    has_overlap: bool,
}

#[derive(Clone, Debug)]
struct PreflightError {
    affected_manifest: Option<String>,
    message: String,
}

impl PreflightError {
    fn inventory(message: impl Into<String>) -> Self {
        Self {
            affected_manifest: None,
            message: message.into(),
        }
    }

    fn manifest(affected_manifest: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            affected_manifest: Some(affected_manifest.into()),
            message: message.into(),
        }
    }
}

fn validate_roots<'a>(manifest_dir: &Path, roots: &'a [DiscoveryRoot]) -> RootValidation<'a> {
    let mut errors = Vec::new();
    let mut canonical = Vec::new();
    for root in roots {
        let path = manifest_dir.join(root.relative);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if is_link_like(&metadata) {
                    errors.push(PreflightError::inventory(format!(
                        "{}: replace the link or reparse point with a regular discovery root; discovery never follows roots that can hide or escape the configured case tree",
                        root.id
                    )));
                    continue;
                }
            }
            Err(error) => {
                errors.push(PreflightError::inventory(format!(
                    "{}: discovery root must be readable before toolchain case generation: {error}",
                    root.id
                )));
                continue;
            }
        }
        match fs::canonicalize(&path) {
            Ok(path) => canonical.push((root, path)),
            Err(error) => errors.push(PreflightError::inventory(format!(
                "{}: discovery root must be readable before toolchain case generation: {error}",
                root.id
            ))),
        }
    }
    let mut has_overlap = false;
    for left_index in 0..canonical.len() {
        for right_index in left_index + 1..canonical.len() {
            let (left_root, left) = &canonical[left_index];
            let (right_root, right) = &canonical[right_index];
            if left == right || left.starts_with(right) || right.starts_with(left) {
                has_overlap = true;
                errors.push(PreflightError::inventory(format!(
                    "{} and {}: configured discovery roots overlap; move one root so each case has one owner",
                    left_root.id, right_root.id
                )));
            }
        }
    }
    RootValidation {
        errors,
        readable_roots: canonical.into_iter().map(|(root, _)| root).collect(),
        has_overlap,
    }
}

struct DiscoveryResult {
    cases: Vec<CaseDescriptor>,
    errors: Vec<PreflightError>,
}

fn discover_root(root: &DiscoveryRoot, root_path: &Path) -> DiscoveryResult {
    let mut errors = Vec::new();
    let mut cases = Vec::new();
    let root_manifest = root_path.join("case.toml");
    let root_ancestor = match fs::symlink_metadata(&root_manifest) {
        Ok(metadata) => {
            if is_link_like(&metadata) {
                errors.push(PreflightError::inventory(format!(
                    "{}/case.toml: replace the link or reparse point with a regular fixture entry; discovery never follows entries that can hide or escape a case",
                    root.id
                )));
                None
            } else {
                if !metadata.is_file() {
                    errors.push(PreflightError::inventory(format!(
                        "{}/case.toml: case.toml must be a regular file",
                        root.id
                    )));
                }
                errors.push(PreflightError::inventory(format!(
                    "{}: remove root-level case.toml or move it below a case directory; discovery roots are containers",
                    root.id
                )));
                Some(PathBuf::from("case.toml"))
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => {
            errors.push(PreflightError::inventory(format!(
                "{}: inspect root-level case.toml failed: {error}",
                root.id
            )));
            None
        }
    };
    discover_dir(
        root,
        root_path,
        Path::new(""),
        root_ancestor,
        &mut cases,
        &mut errors,
    );
    DiscoveryResult { cases, errors }
}

fn discover_dir(
    root: &DiscoveryRoot,
    dir: &Path,
    relative: &Path,
    ancestor_manifest: Option<PathBuf>,
    cases: &mut Vec<CaseDescriptor>,
    errors: &mut Vec<PreflightError>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(PreflightError::inventory(format!(
                "{}: read discovery directory failed: {error}",
                display_root_path(root.id, relative)
            )));
            return;
        }
    };
    let mut entries = match entries.collect::<Result<Vec<_>, _>>() {
        Ok(entries) => entries,
        Err(error) => {
            errors.push(PreflightError::inventory(format!(
                "{}: enumerate discovery directory failed: {error}",
                display_root_path(root.id, relative)
            )));
            return;
        }
    };
    entries.sort_by_key(|entry| entry.file_name());

    let mut classified = Vec::new();
    for entry in entries {
        let entry_relative = relative.join(entry.file_name());
        let metadata = match fs::symlink_metadata(entry.path()) {
            Ok(metadata) => metadata,
            Err(error) => {
                errors.push(PreflightError::inventory(format!(
                    "{}: inspect discovery entry failed: {error}",
                    display_root_path(root.id, &entry_relative)
                )));
                continue;
            }
        };
        if is_link_like(&metadata) {
            errors.push(PreflightError::inventory(format!(
                "{}: replace the link or reparse point with a regular fixture entry; discovery never follows entries that can hide or escape a case",
                display_root_path(root.id, &entry_relative)
            )));
            continue;
        }
        if !metadata.is_dir() && !metadata.is_file() {
            errors.push(PreflightError::inventory(format!(
                "{}: replace the non-regular fixture entry with a regular file or directory",
                display_root_path(root.id, &entry_relative)
            )));
            continue;
        }
        classified.push((entry, entry_relative, metadata));
    }

    let mut local_manifest = ancestor_manifest.clone();
    for (entry, _, metadata) in &classified {
        if entry.file_name() == "case.toml" {
            let manifest_relative = relative.join("case.toml");
            if !metadata.is_file() {
                errors.push(PreflightError::inventory(format!(
                    "{}: case.toml must be a regular file",
                    display_root_path(root.id, &manifest_relative)
                )));
                continue;
            }
            if let Some(ancestor) = &ancestor_manifest {
                errors.push(PreflightError::inventory(format!(
                    "{}: nested case.toml is below {}; remove the nested manifest or move it to a sibling case directory",
                    display_root_path(root.id, &manifest_relative),
                    display_root_path(root.id, ancestor)
                )));
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

    for (entry, entry_relative, metadata) in classified {
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

fn scan_policy(manifest_dir: &Path, cases: &[CaseDescriptor]) -> Vec<PreflightError> {
    let mut errors = Vec::new();
    for case in cases {
        let manifest = manifest_dir.join(&case.manifest_relative).join("case.toml");
        let text = match fs::read_to_string(&manifest) {
            Ok(text) => text,
            Err(error) => {
                errors.push(PreflightError::manifest(
                    case.id.clone(),
                    format!(
                        "{}: read manifest failed before policy validation: {error}",
                        case.id
                    ),
                ));
                continue;
            }
        };
        let scan = match panic::catch_unwind(AssertUnwindSafe(|| {
            crate::manifest_syntax::manifest_policy_scan(&manifest, &text)
        })) {
            Ok(scan) => scan,
            Err(panic) => {
                errors.push(PreflightError::manifest(
                    case.id.clone(),
                    format!(
                        "{}: manifest policy scan failed before command generation: {}",
                        case.id,
                        panic_message(panic)
                    ),
                ));
                continue;
            }
        };
        for finding in scan.findings {
            errors.push(PreflightError::manifest(
                case.id.clone(),
                format!(
                    "{}:{}:{}-{} field `{}` contains {} `{}`; use physical multiline text or a sidecar so line structure remains reviewable",
                    case.id,
                    finding.line,
                    finding.start,
                    finding.end,
                    finding.field,
                    finding.category,
                    finding.spelling.escape_debug()
                ),
            ));
        }
        if let Some(error) = scan.error {
            errors.push(PreflightError::manifest(
                case.id.clone(),
                format!(
                    "{}: manifest policy scan failed before command generation: {error}",
                    case.id
                ),
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

fn render_preflight_errors(errors: Vec<PreflightError>) -> String {
    let affected_manifests = errors
        .iter()
        .filter_map(|error| error.affected_manifest.as_deref())
        .collect::<BTreeSet<_>>()
        .len();
    let mut report = format!(
        "toolchain case preflight found {} problem(s) affecting {} manifest(s) in the authoritative case inventory",
        errors.len(),
        affected_manifests
    );
    report.push_str(
        "\nmove encoded line structure to physical multiline manifest values or sidecars, replace links with regular fixture entries, and rebuild so policy and execution use one visible portable case set",
    );
    for error in errors {
        report.push('\n');
        report.push_str("- ");
        report.push_str(&error.message);
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
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}
