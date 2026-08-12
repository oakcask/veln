use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

use syn::visit::{self, Visit};

#[derive(Debug, PartialEq)]
pub(crate) struct DependencyReport {
    pub(crate) file_count: usize,
    pub(crate) edge_count: usize,
    pub(crate) hotspots: Vec<DependencyHotspot>,
    pub(crate) cycles: Vec<Vec<PathBuf>>,
}

#[derive(Debug, PartialEq)]
pub(crate) struct DependencyHotspot {
    pub(crate) path: PathBuf,
    pub(crate) incoming: usize,
    pub(crate) outgoing: usize,
    pub(crate) pressure: usize,
}

pub(crate) fn collect_report(
    files: impl IntoIterator<Item = PathBuf>,
) -> Result<DependencyReport, String> {
    let mut sources = Vec::new();
    for path in files {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
        sources.push(SourceFile { path, source });
    }
    DependencyGraph::from_sources(&sources).map(|graph| graph.report())
}

impl DependencyReport {
    pub(crate) fn render_human(&self, hotspot_limit: usize, cycle_limit: usize) -> String {
        let mut output = String::new();
        output.push_str("Dependency graph\n");
        output.push_str(&format!(
            "  files: {}, internal edges: {}, cycles: {}\n",
            self.file_count,
            self.edge_count,
            self.cycles.len()
        ));

        output.push_str("\nHighest dependency pressure\n");
        if self.hotspots.is_empty() {
            output.push_str("  none\n");
        } else {
            for hotspot in self.hotspots.iter().take(hotspot_limit) {
                output.push_str(&format!(
                    "  {} in={} out={} pressure={}\n",
                    display_path(&hotspot.path),
                    hotspot.incoming,
                    hotspot.outgoing,
                    hotspot.pressure
                ));
            }
            if self.hotspots.len() > hotspot_limit {
                output.push_str(&format!(
                    "  ... {} more hotspot(s)\n",
                    self.hotspots.len() - hotspot_limit
                ));
            }
        }

        output.push_str("\nDependency cycles\n");
        if self.cycles.is_empty() {
            output.push_str("  none\n");
        } else {
            for cycle in self.cycles.iter().take(cycle_limit) {
                let paths = cycle
                    .iter()
                    .map(|path| display_path(path))
                    .collect::<Vec<_>>()
                    .join(" -> ");
                output.push_str(&format!("  {paths}\n"));
            }
            if self.cycles.len() > cycle_limit {
                output.push_str(&format!(
                    "  ... {} more cycle group(s)\n",
                    self.cycles.len() - cycle_limit
                ));
            }
        }
        output
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

struct ModuleCatalog {
    nodes: Vec<DependencyNode>,
    module_index: std::collections::BTreeMap<ModuleKey, usize>,
    source_to_node: Vec<Option<usize>>,
    crate_roots: std::collections::BTreeMap<String, PathBuf>,
}

impl ModuleCatalog {
    fn from_sources(sources: &[SourceFile]) -> Self {
        let mut nodes = Vec::new();
        let mut module_index = std::collections::BTreeMap::new();
        let source_to_node = sources
            .iter()
            .map(|source| {
                let key = module_key_for_path(&source.path)?;
                Some(*module_index.entry(key.clone()).or_insert_with(|| {
                    let index = nodes.len();
                    nodes.push(DependencyNode {
                        key,
                        file: source.path.clone(),
                    });
                    index
                }))
            })
            .collect();
        let crate_roots = nodes
            .iter()
            .filter_map(|node| {
                node.key
                    .crate_import_name()
                    .map(|name| (name, node.key.crate_src_root.clone()))
            })
            .collect();
        Self {
            nodes,
            module_index,
            source_to_node,
            crate_roots,
        }
    }
}

#[derive(Debug)]
struct DependencyGraph {
    nodes: Vec<DependencyNode>,
    incoming: Vec<Vec<usize>>,
    outgoing: Vec<Vec<usize>>,
}

impl DependencyGraph {
    fn from_sources(sources: &[SourceFile]) -> Result<Self, String> {
        let catalog = ModuleCatalog::from_sources(sources);
        let edges = collect_dependency_edges(sources, &catalog)?;
        let (incoming, outgoing) = build_adjacency(catalog.nodes.len(), edges);
        Ok(Self {
            nodes: catalog.nodes,
            incoming,
            outgoing,
        })
    }

    fn edge_count(&self) -> usize {
        self.outgoing.iter().map(Vec::len).sum()
    }

    fn report(&self) -> DependencyReport {
        let hotspots = self
            .hotspots()
            .into_iter()
            .map(|hotspot| DependencyHotspot {
                path: self.nodes[hotspot.index].file.clone(),
                incoming: hotspot.incoming,
                outgoing: hotspot.outgoing,
                pressure: hotspot.pressure,
            })
            .collect();
        let cycles = self
            .cyclic_components()
            .into_iter()
            .map(|cycle| {
                cycle
                    .into_iter()
                    .map(|node| self.nodes[node].file.clone())
                    .collect()
            })
            .collect();
        DependencyReport {
            file_count: self.nodes.len(),
            edge_count: self.edge_count(),
            hotspots,
            cycles,
        }
    }

    fn hotspots(&self) -> Vec<RankedDependencyHotspot> {
        let mut hotspots = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, _)| {
                let incoming = self.incoming[index].len();
                let outgoing = self.outgoing[index].len();
                let pressure = incoming * outgoing;
                (pressure > 0).then_some(RankedDependencyHotspot {
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

fn collect_dependency_edges(
    sources: &[SourceFile],
    catalog: &ModuleCatalog,
) -> Result<std::collections::BTreeSet<(usize, usize)>, String> {
    let mut edges = std::collections::BTreeSet::new();
    for (source_index, source) in sources.iter().enumerate() {
        let Some(graph_source_index) = catalog.source_to_node[source_index] else {
            continue;
        };
        let syntax = syn::parse_file(&source.source)
            .map_err(|error| format!("failed to parse {}: {error}", source.path.display()))?;
        let current = &catalog.nodes[graph_source_index].key;
        let mut visitor = DependencyVisitor {
            crate_roots: &catalog.crate_roots,
            current,
            edges: &mut edges,
            inline_modules: Vec::new(),
            module_index: &catalog.module_index,
            source_index: graph_source_index,
        };
        visitor.visit_file(&syntax);
    }

    Ok(edges)
}

fn build_adjacency(
    node_count: usize,
    edges: std::collections::BTreeSet<(usize, usize)>,
) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let mut incoming = vec![Vec::new(); node_count];
    let mut outgoing = vec![Vec::new(); node_count];
    for (source, target) in edges {
        outgoing[source].push(target);
        incoming[target].push(source);
    }
    for edges in incoming.iter_mut().chain(outgoing.iter_mut()) {
        edges.sort_unstable();
    }

    (incoming, outgoing)
}

#[derive(Debug)]
struct RankedDependencyHotspot {
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
        let report = graph.report();
        let summary = report.render_human(5, 5);

        assert_eq!(graph.edge_count(), 2);
        assert_eq!(report.hotspots.len(), 2);
        assert_eq!(report.cycles.len(), 1);
        assert!(summary.contains("crates/sample/src/a.rs"));
        assert!(summary.contains("crates/sample/src/b.rs"));
        assert!(summary.contains("cycles: 1"));
    }

    #[test]
    fn resolves_dependencies_across_crate_module_catalogs() {
        let sources = vec![
            SourceFile {
                path: PathBuf::from("crates/alpha/src/lib.rs"),
                source: "use beta::support; pub fn alpha() { support::beta(); }".to_string(),
            },
            SourceFile {
                path: PathBuf::from("crates/beta/src/lib.rs"),
                source: "pub mod support;".to_string(),
            },
            SourceFile {
                path: PathBuf::from("crates/beta/src/support.rs"),
                source: "pub fn beta() { alpha::alpha(); }".to_string(),
            },
        ];

        let graph = DependencyGraph::from_sources(&sources).unwrap();
        let report = graph.report();

        assert_eq!(graph.edge_count(), 2);
        assert_eq!(report.cycles.len(), 1);
        assert_eq!(
            report.cycles[0],
            vec![
                PathBuf::from("crates/alpha/src/lib.rs"),
                PathBuf::from("crates/beta/src/support.rs"),
            ]
        );
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
