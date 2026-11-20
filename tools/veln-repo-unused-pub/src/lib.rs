use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use syn::visit::{self, Visit};

#[derive(Debug, PartialEq)]
pub struct Finding {
    pub file: PathBuf,
    pub line: usize,
    pub kind: &'static str,
    pub name: String,
    pub evidence: &'static str,
}

#[derive(Debug)]
pub struct Report {
    pub files: usize,
    pub definitions: usize,
    pub findings: Vec<Finding>,
}

#[derive(Default)]
struct References {
    production: bool,
    test: bool,
}

#[derive(Default)]
struct Index {
    definitions: Vec<Finding>,
    references: BTreeMap<String, References>,
}

pub fn scan(roots: &[PathBuf]) -> Result<Report, String> {
    let mut files = BTreeSet::new();
    for root in roots {
        collect_files(root, &mut files)?;
    }
    if files.is_empty() {
        return Err("no Rust files found; provide source roots such as crates tools".into());
    }
    let mut index = Index::default();
    for file in &files {
        let source = fs::read_to_string(file)
            .map_err(|error| format!("cannot read {}: {error}", file.display()))?;
        index.add(file, &source)?;
    }
    Ok(index.finish(files.len()))
}

fn collect_files(path: &Path, files: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    // Do not follow links into other worktrees or generated build trees.
    if metadata.is_symlink() {
        return Ok(());
    }
    if metadata.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
        files.insert(path.to_path_buf());
    } else if metadata.is_dir() {
        if path
            .file_name()
            .is_some_and(|name| matches!(name.to_str(), Some("target" | ".git" | "node_modules")))
        {
            return Ok(());
        }
        for entry in fs::read_dir(path)
            .map_err(|error| format!("cannot list {}: {error}", path.display()))?
        {
            let entry =
                entry.map_err(|error| format!("cannot list {}: {error}", path.display()))?;
            collect_files(&entry.path(), files)?;
        }
    }
    Ok(())
}

impl Index {
    fn add(&mut self, file: &Path, source: &str) -> Result<(), String> {
        let syntax = syn::parse_file(source)
            .map_err(|error| format!("cannot parse {}: {error}", file.display()))?;
        let test = file.components().any(|part| part.as_os_str() == "tests")
            || file
                .file_stem()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "tests" || name.ends_with("_tests"));
        Scanner {
            index: self,
            file,
            test,
            owner: None,
            trait_impl: false,
        }
        .visit_file(&syntax);
        Ok(())
    }

    fn finish(self, files: usize) -> Report {
        let definitions = self.definitions.len();
        let mut findings: Vec<_> = self
            .definitions
            .into_iter()
            .filter_map(|mut finding| {
                let references = self.references.get(&finding.name);
                if references.is_some_and(|refs| refs.production) {
                    return None;
                }
                finding.evidence = if references.is_some_and(|refs| refs.test) {
                    "test-only-references"
                } else {
                    "no-references"
                };
                Some(finding)
            })
            .collect();
        findings.sort_by(|a, b| (&a.file, a.line, &a.name).cmp(&(&b.file, b.line, &b.name)));
        Report {
            files,
            definitions,
            findings,
        }
    }
}

struct Scanner<'a> {
    index: &'a mut Index,
    file: &'a Path,
    test: bool,
    owner: Option<String>,
    trait_impl: bool,
}

impl Scanner<'_> {
    fn reference(&mut self, ident: &proc_macro2::Ident) {
        let name = ident.to_string();
        if self.owner.as_ref() == Some(&name) {
            return;
        }
        let refs = self.index.references.entry(name).or_default();
        if self.test {
            refs.test = true;
        } else {
            refs.production = true;
        }
    }

    fn definition(
        &mut self,
        ident: &proc_macro2::Ident,
        vis: &syn::Visibility,
        kind: &'static str,
    ) {
        if !self.test && matches!(vis, syn::Visibility::Public(_)) {
            self.index.definitions.push(Finding {
                file: self.file.to_path_buf(),
                line: ident.span().start().line,
                name: ident.to_string(),
                kind,
                evidence: "no-references",
            });
        }
    }

    fn tokens(&mut self, tokens: TokenStream) {
        for token in tokens {
            match token {
                TokenTree::Ident(ident) => self.reference(&ident),
                TokenTree::Group(group) => self.tokens(group.stream()),
                _ => {}
            }
        }
    }
}

fn test_attribute(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("test")
            || matches!(&attr.meta, syn::Meta::List(meta)
            if meta.path.is_ident("cfg") && meta.tokens.to_string() == "test")
    })
}

impl<'ast> Visit<'ast> for Scanner<'_> {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.test |= test_attribute(&file.attrs);
        visit::visit_file(self, file);
    }

    fn visit_item(&mut self, item: &'ast syn::Item) {
        let (attrs, definition) = match item {
            syn::Item::Fn(node) => (&node.attrs, Some((&node.sig.ident, &node.vis, "fn"))),
            syn::Item::Struct(node) => (&node.attrs, Some((&node.ident, &node.vis, "struct"))),
            syn::Item::Enum(node) => (&node.attrs, Some((&node.ident, &node.vis, "enum"))),
            syn::Item::Trait(node) => (&node.attrs, Some((&node.ident, &node.vis, "trait"))),
            syn::Item::Type(node) => (&node.attrs, Some((&node.ident, &node.vis, "type"))),
            syn::Item::Const(node) => (&node.attrs, Some((&node.ident, &node.vis, "const"))),
            syn::Item::Static(node) => (&node.attrs, Some((&node.ident, &node.vis, "static"))),
            syn::Item::Mod(node) => (&node.attrs, None),
            syn::Item::Impl(node) => (&node.attrs, None),
            syn::Item::Use(node) => (&node.attrs, None),
            syn::Item::Macro(node) => (&node.attrs, None),
            _ => {
                visit::visit_item(self, item);
                return;
            }
        };
        let previous_test = self.test;
        self.test |= test_attribute(attrs);
        let previous_owner = self.owner.clone();
        if let Some((ident, vis, kind)) = definition {
            self.definition(ident, vis, kind);
            self.owner = Some(ident.to_string());
        }
        visit::visit_item(self, item);
        self.owner = previous_owner;
        self.test = previous_test;
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        let previous = self.trait_impl;
        self.trait_impl = item.trait_.is_some();
        // An impl header alone does not establish a consumer of the type.
        self.visit_generics(&item.generics);
        if let Some((path, _)) = &item.trait_ {
            self.visit_path(path);
        }
        for member in &item.items {
            self.visit_impl_item(member);
        }
        self.trait_impl = previous;
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        let previous_test = self.test;
        self.test |= test_attribute(&item.attrs);
        if !self.trait_impl {
            self.definition(&item.sig.ident, &item.vis, "method");
        }
        let previous_owner = self.owner.replace(item.sig.ident.to_string());
        visit::visit_impl_item_fn(self, item);
        self.owner = previous_owner;
        self.test = previous_test;
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        for segment in &path.segments {
            self.reference(&segment.ident);
        }
        visit::visit_path(self, path);
    }

    fn visit_expr_method_call(&mut self, item: &'ast syn::ExprMethodCall) {
        self.reference(&item.method);
        visit::visit_expr_method_call(self, item);
    }

    fn visit_use_tree(&mut self, item: &'ast syn::UseTree) {
        match item {
            syn::UseTree::Path(node) => self.reference(&node.ident),
            syn::UseTree::Name(node) => self.reference(&node.ident),
            syn::UseTree::Rename(node) => self.reference(&node.ident),
            _ => {}
        }
        visit::visit_use_tree(self, item);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        self.visit_path(&item.path);
        self.tokens(item.tokens.clone());
    }

    fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
        // Prose and string literals are not evidence of executable consumers.
        if !attr.path().is_ident("doc")
            && let syn::Meta::List(meta) = &attr.meta
        {
            self.tokens(meta.tokens.clone());
        }
    }
}

#[cfg(test)]
mod tests;
