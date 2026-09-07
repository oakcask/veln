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
mod protocol;
mod references;
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

fn initialized_server(workspace: &TempWorkspace) -> Server {
    let base = WorkspaceBase::open(workspace.root.clone()).unwrap();
    let selection = Selection::discover(base.path()).unwrap();
    Server {
        base,
        selection,
        initialized: true,
        language_resources: LanguageResources::for_test(Vec::new(), Vec::new()),
    }
}

fn initialized_server_with_embedded_resources(workspace: &TempWorkspace) -> Server {
    let mut server = initialized_server(workspace);
    server.language_resources = LanguageResources::checked().unwrap();
    server
}

#[test]
fn initialized_server_avoids_embedded_language_resources() {
    let workspace = TempWorkspace::new("minimal-language-resources");
    let server = initialized_server(&workspace);

    assert!(
        server.language_resources.list_result()["resources"]
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
