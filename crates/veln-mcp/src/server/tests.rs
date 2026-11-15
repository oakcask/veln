use super::*;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

mod check_project;
mod definition;
mod dependency_resources;
mod language_tools;
mod lifecycle;
mod outcome;
mod package_documentation_resources;
mod presentation;
mod protocol;
mod references;
mod references_standard_library_aliases;
mod references_support;
mod references_workspace_schema_aliases;
mod rename;
mod resources;

fn parse_responses(output: Vec<u8>) -> Vec<Value> {
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect()
}

struct TempWorkspace {
    root: PathBuf,
}

struct UnresolvableDependencyTypeAliasCase {
    name: &'static str,
    dependency_sources: &'static [(&'static str, &'static str)],
}

fn unresolvable_dependency_type_alias_cases() -> [UnresolvableDependencyTypeAliasCase; 3] {
    [
        UnresolvableDependencyTypeAliasCase {
            name: "invalid-module-target",
            dependency_sources: &[
                ("model.veln", "use Bad\n\npub type Alias = Bad::Target\n"),
                ("Bad.veln", "pub type Target\nend\n"),
            ],
        },
        UnresolvableDependencyTypeAliasCase {
            name: "parse-diagnostic-target",
            dependency_sources: &[
                (
                    "model.veln",
                    "use broken\n\npub type Alias = broken::Target\n",
                ),
                ("broken.veln", "pub type Target\n  pub Ready(Int)\n"),
            ],
        },
        UnresolvableDependencyTypeAliasCase {
            name: "ambiguous-target",
            dependency_sources: &[(
                "model.veln",
                concat!(
                    "pub type Target\n",
                    "end\n\n",
                    "pub type Target\n",
                    "end\n\n",
                    "pub type Alias = Target\n",
                ),
            )],
        },
    ]
}

fn write_unresolvable_dependency_type_alias_workspace(
    tool: &str,
    case: &UnresolvableDependencyTypeAliasCase,
) -> TempWorkspace {
    let workspace = TempWorkspace::new(&format!("{tool}-dependency-type-alias-{}", case.name));
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use model from \"example/dep\"\n\n",
            "fn read(input: model::Alias) -> model::Alias\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    for &(path, text) in case.dependency_sources {
        workspace.write(&format!("vendor/dep/{path}"), text);
    }
    workspace
}

fn initialized_server(workspace: &TempWorkspace) -> Server {
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    Server {
        base,
        selection,
        initialized: true,
        language_resources: minimal_language_resources(),
        capture_cache: CaptureCache::default(),
    }
}

fn minimal_language_resources() -> LanguageResources {
    LanguageResources::for_test(Vec::new(), Vec::new())
}

fn initialized_server_with_embedded_resources(workspace: &TempWorkspace) -> Server {
    let mut server = initialized_server(workspace);
    server.language_resources = LanguageResources::checked().unwrap();
    server
}

#[test]
fn minimal_language_resources_avoid_embedded_resources() {
    let resources = minimal_language_resources();

    assert!(
        resources.list_result()["resources"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

impl TempWorkspace {
    fn new(name: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-mcp-server-{name}-{}-{nonce}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn write(&self, relative: &str, contents: &str) {
        self.write_bytes(relative, contents.as_bytes());
    }

    fn write_bytes(&self, relative: &str, contents: &[u8]) {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn mkdir(&self, relative: &str) {
        fs::create_dir_all(self.root.join(relative)).unwrap();
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
