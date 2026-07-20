use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

pub(crate) struct DependencySummary {
    pub(crate) text: String,
    pub(crate) cycle_count: usize,
}

pub(crate) fn collect_summary(
    files: impl IntoIterator<Item = PathBuf>,
    hotspot_limit: usize,
    cycle_limit: usize,
) -> Result<DependencySummary, String> {
    let mut sources = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        sources.push(SourceFile { path, source });
    }
    let graph = DependencyGraph::from_sources(&sources)?;
    Ok(DependencySummary {
        text: graph.render_summary(hotspot_limit, cycle_limit),
        cycle_count: graph.cyclic_components().len(),
    })
}

pub(crate) fn emit_summary(summary: &str) -> Result<(), String> {
    match env::var("GITHUB_STEP_SUMMARY") {
        Ok(path) => {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(|error| format!("failed to open GitHub step summary: {error}"))?;
            file.write_all(summary.as_bytes())
                .map_err(|error| format!("failed to write GitHub step summary: {error}"))
        }
        Err(_) => {
            println!("{summary}");
            Ok(())
        }
    }
}

#[derive(Debug)]
struct SourceFile {
    path: PathBuf,
    source: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct ModuleKey {
    crate_src_root: PathBuf,
    segments: Vec<String>,
}

impl ModuleKey {
    fn crate_import_name(&self) -> Option<String> {
        crate_import_name(&self.crate_src_root)
    }
}

#[derive(Debug)]
struct DependencyNode {
    key: ModuleKey,
    file: PathBuf,
}

#[derive(Debug)]
struct DependencyGraph {
    nodes: Vec<DependencyNode>,
    incoming: Vec<Vec<usize>>,
    outgoing: Vec<Vec<usize>>,
}

impl DependencyGraph {
    fn from_sources(sources: &[SourceFile]) -> Result<Self, String> {
        let mut nodes = Vec::new();
        let mut module_index = std::collections::BTreeMap::new();
        let mut source_to_node = Vec::new();
        for source in sources {
            let Some(key) = module_key_for_path(&source.path) else {
                source_to_node.push(None);
                continue;
            };
            let index = if let Some(existing) = module_index.get(&key) {
                *existing
            } else {
                let index = nodes.len();
                module_index.insert(key.clone(), index);
                nodes.push(DependencyNode {
                    key,
                    file: source.path.clone(),
                });
                index
            };
            source_to_node.push(Some(index));
        }

        let mut crate_roots = std::collections::BTreeMap::new();
        for node in &nodes {
            if let Some(import_name) = node.key.crate_import_name() {
                crate_roots.insert(import_name, node.key.crate_src_root.clone());
            }
        }

        let mut edges = std::collections::BTreeSet::new();
        for (source_index, source) in sources.iter().enumerate() {
            let Some(graph_source_index) = source_to_node[source_index] else {
                continue;
            };
            let syntax = syn::parse_file(&source.source)
                .map_err(|error| format!("failed to parse {}: {error}", source.path.display()))?;
            let current = &nodes[graph_source_index].key;
            let mut visitor = DependencyVisitor {
                crate_roots: &crate_roots,
                current,
                edges: &mut edges,
                inline_modules: Vec::new(),
                module_index: &module_index,
                source_index: graph_source_index,
            };
            visitor.visit_file(&syntax);
        }

        let mut incoming = vec![Vec::new(); nodes.len()];
        let mut outgoing = vec![Vec::new(); nodes.len()];
        for (source, target) in edges {
            outgoing[source].push(target);
            incoming[target].push(source);
        }
        for edges in incoming.iter_mut().chain(outgoing.iter_mut()) {
            edges.sort_unstable();
        }

        Ok(Self {
            nodes,
            incoming,
            outgoing,
        })
    }

    fn edge_count(&self) -> usize {
        self.outgoing.iter().map(Vec::len).sum()
    }

    fn render_summary(&self, hotspot_limit: usize, cycle_limit: usize) -> String {
        let mut summary = String::new();
        summary.push_str("## Dependency Graph Refactor Signal\n\n");
        summary.push_str("Inspect dependency hotspots before broad Rust refactors; files with both high incoming and outgoing dependencies tend to combine caller impact with callee coordination cost.\n\n");
        summary.push_str(&format!(
            "- Rust files analyzed: {}\n- Internal dependency edges: {}\n- Strongly connected groups: {}\n\n",
            self.nodes.len(),
            self.edge_count(),
            self.cyclic_components().len()
        ));

        let hotspots = self.hotspots(hotspot_limit);
        summary.push_str("### Highest dependency pressure\n\n");
        if hotspots.is_empty() {
            summary.push_str(
                "No files have both incoming and outgoing internal dependencies in this scan.\n\n",
            );
        } else {
            summary.push_str("| File | In | Out | Pressure |\n");
            summary.push_str("| --- | ---: | ---: | ---: |\n");
            for hotspot in hotspots {
                summary.push_str(&format!(
                    "| `{}` | {} | {} | {} |\n",
                    markdown_escape(&display_path(&self.nodes[hotspot.index].file)),
                    hotspot.incoming,
                    hotspot.outgoing,
                    hotspot.pressure
                ));
            }
            summary.push('\n');
        }

        let cycles = self.cyclic_components();
        summary.push_str("### Dependency cycles\n\n");
        if cycles.is_empty() {
            summary.push_str(
                "No internal Rust source cycles detected by this import and path scan.\n",
            );
        } else {
            summary.push_str("Inspect these cycles before moving boundaries; breaking a cycle usually needs a clearer owner or a smaller shared interface.\n\n");
            for (index, cycle) in cycles.iter().take(cycle_limit).enumerate() {
                let files = cycle
                    .iter()
                    .map(|node| {
                        format!(
                            "`{}`",
                            markdown_escape(&display_path(&self.nodes[*node].file))
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" -> ");
                summary.push_str(&format!("{}. {files}\n", index + 1));
            }
            if cycles.len() > cycle_limit {
                summary.push_str(&format!(
                    "\n{} more cycle groups omitted from the summary.\n",
                    cycles.len() - cycle_limit
                ));
            }
        }

        summary.push('\n');
        summary
    }

    fn hotspots(&self, limit: usize) -> Vec<DependencyHotspot> {
        let mut hotspots = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, _)| {
                let incoming = self.incoming[index].len();
                let outgoing = self.outgoing[index].len();
                let pressure = incoming * outgoing;
                (pressure > 0).then_some(DependencyHotspot {
                    index,
                    incoming,
                    outgoing,
                    pressure,
                })
            })
            .collect::<Vec<_>>();
        hotspots.sort_by(|left, right| {
            right
                .pressure
                .cmp(&left.pressure)
                .then_with(|| right.incoming.cmp(&left.incoming))
                .then_with(|| right.outgoing.cmp(&left.outgoing))
                .then_with(|| {
                    self.nodes[left.index]
                        .file
                        .cmp(&self.nodes[right.index].file)
                })
        });
        hotspots.truncate(limit);
        hotspots
    }

    fn cyclic_components(&self) -> Vec<Vec<usize>> {
        let mut tarjan = Tarjan::new(&self.outgoing);
        let mut components = tarjan.components();
        components.retain(|component| component.len() > 1);
        for component in &mut components {
            component.sort_by(|left, right| self.nodes[*left].file.cmp(&self.nodes[*right].file));
        }
        components.sort_by(|left, right| {
            right
                .len()
                .cmp(&left.len())
                .then_with(|| self.nodes[left[0]].file.cmp(&self.nodes[right[0]].file))
        });
        components
    }
}

#[derive(Debug)]
struct DependencyHotspot {
    index: usize,
    incoming: usize,
    outgoing: usize,
    pressure: usize,
}

struct DependencyVisitor<'a> {
    crate_roots: &'a std::collections::BTreeMap<String, PathBuf>,
    current: &'a ModuleKey,
    edges: &'a mut std::collections::BTreeSet<(usize, usize)>,
    inline_modules: Vec<String>,
    module_index: &'a std::collections::BTreeMap<ModuleKey, usize>,
    source_index: usize,
}

impl DependencyVisitor<'_> {
    fn current_key(&self) -> ModuleKey {
        let mut current = self.current.clone();
        current.segments.extend(self.inline_modules.iter().cloned());
        current
    }

    fn record_path(&mut self, path: &syn::Path) {
        let segments = path
            .segments
            .iter()
            .map(|segment| segment.ident.to_string())
            .collect::<Vec<_>>();
        let current = self.current_key();
        let Some(key) = resolve_dependency_path(&current, self.crate_roots, &segments) else {
            return;
        };
        self.record_key(key);
    }

    fn record_key(&mut self, mut key: ModuleKey) {
        loop {
            if let Some(target) = self.module_index.get(&key) {
                if *target != self.source_index {
                    self.edges.insert((self.source_index, *target));
                }
                return;
            }

            if key.segments.pop().is_none() {
                return;
            }
        }
    }

    fn record_use_tree(&mut self, prefix: Vec<String>, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(path) => {
                let mut next = prefix;
                next.push(path.ident.to_string());
                self.record_use_tree(next, &path.tree);
            }
            syn::UseTree::Name(name) => {
                let mut next = prefix;
                next.push(name.ident.to_string());
                self.record_segments(next);
            }
            syn::UseTree::Rename(rename) => {
                let mut next = prefix;
                next.push(rename.ident.to_string());
                self.record_segments(next);
            }
            syn::UseTree::Glob(_) => self.record_segments(prefix),
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.record_use_tree(prefix.clone(), item);
                }
            }
        }
    }

    fn record_segments(&mut self, segments: Vec<String>) {
        let current = self.current_key();
        let Some(key) = resolve_dependency_path(&current, self.crate_roots, &segments) else {
            return;
        };
        self.record_key(key);
    }
}

impl<'ast> Visit<'ast> for DependencyVisitor<'_> {
    fn visit_visibility(&mut self, _visibility: &'ast syn::Visibility) {}

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        if item.content.is_none() {
            return;
        }
        self.inline_modules.push(item.ident.to_string());
        visit::visit_item_mod(self, item);
        self.inline_modules.pop();
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        self.record_use_tree(Vec::new(), &item.tree);
        visit::visit_item_use(self, item);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        self.record_path(path);
        visit::visit_path(self, path);
    }
}

fn resolve_dependency_path(
    current: &ModuleKey,
    crate_roots: &std::collections::BTreeMap<String, PathBuf>,
    segments: &[String],
) -> Option<ModuleKey> {
    let first = segments.first()?;
    match first.as_str() {
        "crate" => Some(ModuleKey {
            crate_src_root: current.crate_src_root.clone(),
            segments: segments[1..].to_vec(),
        }),
        "self" => {
            let mut resolved = current.segments.clone();
            resolved.extend_from_slice(&segments[1..]);
            Some(ModuleKey {
                crate_src_root: current.crate_src_root.clone(),
                segments: resolved,
            })
        }
        "super" => {
            let mut resolved = current.segments.clone();
            let mut rest_start = 0;
            while segments
                .get(rest_start)
                .is_some_and(|segment| segment == "super")
            {
                resolved.pop()?;
                rest_start += 1;
            }
            resolved.extend_from_slice(&segments[rest_start..]);
            Some(ModuleKey {
                crate_src_root: current.crate_src_root.clone(),
                segments: resolved,
            })
        }
        name => crate_roots.get(name).map(|crate_src_root| ModuleKey {
            crate_src_root: crate_src_root.clone(),
            segments: segments[1..].to_vec(),
        }),
    }
}

fn module_key_for_path(path: &Path) -> Option<ModuleKey> {
    let crate_src_root = crate_src_root_from_path(path)?;
    let relative = path.strip_prefix(&crate_src_root).ok()?;
    let mut segments = relative
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => value.to_str().map(ToString::to_string),
            _ => None,
        })
        .collect::<Vec<_>>();
    let file = segments.pop()?;
    match file.as_str() {
        "lib.rs" | "main.rs" | "mod.rs" => {}
        _ => {
            let stem = Path::new(&file).file_stem()?.to_str()?.to_string();
            segments.push(stem);
        }
    }
    Some(ModuleKey {
        crate_src_root,
        segments,
    })
}

fn crate_src_root_from_path(path: &Path) -> Option<PathBuf> {
    let mut current = PathBuf::new();
    let mut candidates = Vec::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if component.as_os_str() == OsStr::new("src") {
            candidates.push(current.clone());
        }
    }

    for candidate in candidates.iter().rev() {
        if candidate
            .parent()
            .is_some_and(|parent| parent.join("Cargo.toml").is_file())
        {
            return Some(candidate.clone());
        }
    }

    candidates.pop()
}

fn crate_import_name(crate_src_root: &Path) -> Option<String> {
    Some(
        crate_src_root
            .parent()?
            .file_name()?
            .to_str()?
            .replace('-', "_"),
    )
}

fn display_path(path: &Path) -> String {
    let relative = if path.is_absolute() {
        env::current_dir()
            .ok()
            .and_then(|current| path.strip_prefix(current).ok().map(Path::to_path_buf))
            .unwrap_or_else(|| path.to_path_buf())
    } else {
        path.to_path_buf()
    };
    relative.display().to_string()
}

fn markdown_escape(value: &str) -> String {
    value.replace('|', "\\|")
}

struct Tarjan<'a> {
    graph: &'a [Vec<usize>],
    index: usize,
    indices: Vec<Option<usize>>,
    lowlinks: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    components: Vec<Vec<usize>>,
}

impl<'a> Tarjan<'a> {
    fn new(graph: &'a [Vec<usize>]) -> Self {
        Self {
            graph,
            index: 0,
            indices: vec![None; graph.len()],
            lowlinks: vec![0; graph.len()],
            on_stack: vec![false; graph.len()],
            stack: Vec::new(),
            components: Vec::new(),
        }
    }

    fn components(&mut self) -> Vec<Vec<usize>> {
        for node in 0..self.graph.len() {
            if self.indices[node].is_none() {
                self.connect(node);
            }
        }
        std::mem::take(&mut self.components)
    }

    fn connect(&mut self, node: usize) {
        self.indices[node] = Some(self.index);
        self.lowlinks[node] = self.index;
        self.index += 1;
        self.stack.push(node);
        self.on_stack[node] = true;

        for target in &self.graph[node] {
            if self.indices[*target].is_none() {
                self.connect(*target);
                self.lowlinks[node] = self.lowlinks[node].min(self.lowlinks[*target]);
            } else if self.on_stack[*target] {
                let target_index = self.indices[*target].expect("stacked node has index");
                self.lowlinks[node] = self.lowlinks[node].min(target_index);
            }
        }

        if self.lowlinks[node] == self.indices[node].expect("connected node has index") {
            let mut component = Vec::new();
            while let Some(stack_node) = self.stack.pop() {
                self.on_stack[stack_node] = false;
                component.push(stack_node);
                if stack_node == node {
                    break;
                }
            }
            self.components.push(component);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_dependency_hotspots_from_rust_paths() {
        let sources = vec![
            SourceFile {
                path: PathBuf::from("crates/sample/src/lib.rs"),
                source: "mod a; mod b; pub fn root() {}".to_string(),
            },
            SourceFile {
                path: PathBuf::from("crates/sample/src/a.rs"),
                source: "use crate::b::Bee; pub fn a() { crate::b::run(); }".to_string(),
            },
            SourceFile {
                path: PathBuf::from("crates/sample/src/b.rs"),
                source: "use crate::a; pub struct Bee; pub fn run() { a::a(); }".to_string(),
            },
            SourceFile {
                path: PathBuf::from("crates/sample/build.rs"),
                source: "fn main() {}".to_string(),
            },
        ];

        let graph = DependencyGraph::from_sources(&sources).unwrap();
        let summary = graph.render_summary(5, 5);

        assert_eq!(graph.edge_count(), 2);
        assert!(summary.contains("crates/sample/src/a.rs"));
        assert!(summary.contains("crates/sample/src/b.rs"));
        assert!(summary.contains("Strongly connected groups: 1"));
    }

    #[test]
    fn ignores_restricted_visibility_paths() {
        let sources = vec![
            SourceFile {
                path: PathBuf::from("crates/sample/src/lib.rs"),
                source: "mod item;".to_string(),
            },
            SourceFile {
                path: PathBuf::from("crates/sample/src/item.rs"),
                source: "pub(crate) fn visible() {} pub(in crate) struct Item;".to_string(),
            },
        ];

        let graph = DependencyGraph::from_sources(&sources).unwrap();

        assert_eq!(graph.edge_count(), 0);
        assert!(graph.cyclic_components().is_empty());
    }

    #[test]
    fn resolves_super_from_inline_modules_without_root_edges() {
        let sources = vec![
            SourceFile {
                path: PathBuf::from("crates/sample/src/lib.rs"),
                source: "mod item;".to_string(),
            },
            SourceFile {
                path: PathBuf::from("crates/sample/src/item.rs"),
                source: "fn visible() {} mod tests { use super::*; fn check() { visible(); } }"
                    .to_string(),
            },
        ];

        let graph = DependencyGraph::from_sources(&sources).unwrap();

        assert_eq!(graph.edge_count(), 0);
        assert!(graph.cyclic_components().is_empty());
    }
}
