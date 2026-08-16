//! Editor- and transport-neutral definition and reference services for Veln.

mod package_documentation;
mod virtual_source;

pub use package_documentation::{
    PackageDocAlias, PackageDocCatalog, PackageDocDeclaration, PackageDocDiagnostic,
    PackageDocDiagnosticSpan, PackageDocDoctest, PackageDocExpectedOutput,
    PackageDocFunctionContract, PackageDocGeneration, PackageDocGenerationStatus,
    PackageDocGeneratorContract, PackageDocMetadata, PackageDocModule, PackageDocReference,
    PackageDocResult, PackageDocResultKind, PackageDocTypeConstructor,
};
pub use virtual_source::{VirtualSourceCatalog, VirtualSourceCatalogError, VirtualSourceEntry};

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::sync::{Arc, OnceLock};

use veln_project::{
    CapturedPackageSnapshot, CapturedPackageSource, PackageIdentity, ProjectManifest,
    classify_companion_source,
};
use veln_source::{SourceFile, SourcePath, SourceSpan};
use veln_syntax::{PublicAliasKind, SyntaxItem, Token, TokenKind, Visibility, lex, parse};

#[cfg(test)]
mod navigation_stats {
    use std::cell::Cell;

    thread_local! {
        static CONSTRUCTOR_CANDIDATES_CONSIDERED: Cell<usize> = const { Cell::new(0) };
    }

    pub fn reset() {
        CONSTRUCTOR_CANDIDATES_CONSIDERED.set(0);
    }

    pub fn record_constructor_candidate() {
        CONSTRUCTOR_CANDIDATES_CONSIDERED.with(|value| value.set(value.get() + 1));
    }

    pub fn constructor_candidates_considered() -> usize {
        CONSTRUCTOR_CANDIDATES_CONSIDERED.get()
    }
}

#[derive(Clone, Debug)]
pub struct EffectiveProjectSnapshot {
    sources: Vec<SourceFile>,
    direct_dependencies: Vec<DirectDependencySnapshot>,
    standard_library: Option<DirectDependencySnapshot>,
    navigation_index: OnceLock<Arc<SymbolIndex>>,
}

impl EffectiveProjectSnapshot {
    pub fn new(sources: Vec<SourceFile>) -> Self {
        Self {
            sources,
            direct_dependencies: Vec::new(),
            standard_library: None,
            navigation_index: OnceLock::new(),
        }
    }

    pub fn with_direct_dependencies(
        sources: Vec<SourceFile>,
        direct_dependencies: Vec<DirectDependencySnapshot>,
    ) -> Self {
        Self {
            sources,
            direct_dependencies,
            standard_library: None,
            navigation_index: OnceLock::new(),
        }
    }

    pub fn with_standard_library(mut self, standard_library: DirectDependencySnapshot) -> Self {
        self.standard_library = Some(standard_library);
        self
    }

    pub fn with_workspace_overlays(&self, overlays: impl IntoIterator<Item = SourceFile>) -> Self {
        let mut sources = self.sources.clone();
        for overlay in overlays {
            let source_path = overlay.path().as_str().to_string();
            if let Some(existing) = sources
                .iter_mut()
                .find(|source| source.path().as_str() == source_path)
            {
                *existing = overlay;
            } else {
                sources.push(overlay);
            }
        }
        sources.sort_by(|left, right| left.path().as_str().cmp(right.path().as_str()));
        Self {
            sources,
            direct_dependencies: self.direct_dependencies.clone(),
            standard_library: self.standard_library.clone(),
            navigation_index: OnceLock::new(),
        }
    }

    fn navigation_index(&self) -> Arc<SymbolIndex> {
        self.navigation_index
            .get_or_init(|| {
                Arc::new(SymbolIndex::new(
                    self.sources.clone(),
                    self.direct_dependencies.clone(),
                    self.standard_library.clone(),
                ))
            })
            .clone()
    }

    pub fn resolve_virtual_source(&self, uri: &str) -> Option<&[u8]> {
        self.direct_dependencies
            .iter()
            .find_map(|dependency| dependency.resolve_virtual_source(uri))
            .or_else(|| {
                self.standard_library
                    .as_ref()
                    .and_then(|standard_library| standard_library.resolve_virtual_source(uri))
            })
    }
}

#[derive(Clone, Debug)]
pub struct DirectDependencySnapshot {
    identity: PackageIdentity,
    snapshot: CapturedPackageSnapshot,
    exported_sources: BTreeSet<String>,
    virtual_sources: VirtualSourceCatalog,
    standard_library: bool,
}

impl DirectDependencySnapshot {
    pub fn from_validated_manifest(
        expected_identity: &PackageIdentity,
        snapshot: CapturedPackageSnapshot,
        manifest: ProjectManifest,
    ) -> Result<Self, DirectDependencySnapshotError> {
        let actual_identity = manifest
            .package
            .fields
            .iter()
            .find(|field| field.key == "name")
            .ok_or(DirectDependencySnapshotError::MissingPackageName)?;
        if actual_identity.value != expected_identity.as_str() {
            return Err(DirectDependencySnapshotError::PackageNameMismatch {
                expected: expected_identity.as_str().to_string(),
                actual: actual_identity.value.clone(),
            });
        }
        let exported_sources = manifest
            .lib
            .exports
            .into_iter()
            .map(|export| SourcePath::new(export.path).as_str().to_string())
            .collect();
        let virtual_sources =
            VirtualSourceCatalog::new([(expected_identity.clone(), snapshot.clone())])?;
        Ok(Self {
            identity: expected_identity.clone(),
            snapshot,
            exported_sources,
            virtual_sources,
            standard_library: false,
        })
    }

    pub fn from_validated_standard_library(
        snapshot: CapturedPackageSnapshot,
        manifest: ProjectManifest,
    ) -> Result<Self, DirectDependencySnapshotError> {
        let identity = PackageIdentity::embedded_standard();
        let mut standard_library = Self::from_validated_manifest(&identity, snapshot, manifest)?;
        standard_library.standard_library = true;
        Ok(standard_library)
    }

    fn indexed_sources(
        &self,
    ) -> impl Iterator<Item = (&CapturedPackageSource, &VirtualSourceEntry)> {
        self.snapshot
            .sources()
            .iter()
            .enumerate()
            .map(|(source_index, source)| {
                let entry = self
                    .virtual_sources
                    .entry_for_source(0, source_index)
                    .expect("direct dependency catalog contains every captured source");
                (source, entry)
            })
    }

    fn resolve_virtual_source(&self, uri: &str) -> Option<&[u8]> {
        self.virtual_sources.resolve(uri)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DirectDependencySnapshotError {
    MissingPackageName,
    PackageNameMismatch { expected: String, actual: String },
    VirtualSourceCatalog(VirtualSourceCatalogError),
}

impl fmt::Display for DirectDependencySnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPackageName => {
                write!(formatter, "direct dependency manifest has no package name")
            }
            Self::PackageNameMismatch { expected, actual } => write!(
                formatter,
                "direct dependency package name `{actual}` does not match requested package `{expected}`"
            ),
            Self::VirtualSourceCatalog(error) => error.fmt(formatter),
        }
    }
}

impl Error for DirectDependencySnapshotError {}

impl From<VirtualSourceCatalogError> for DirectDependencySnapshotError {
    fn from(error: VirtualSourceCatalogError) -> Self {
        Self::VirtualSourceCatalog(error)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourcePosition {
    pub source: SourcePath,
    /// One-based source line.
    pub line: usize,
    /// One-based Unicode-scalar source column.
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Constructor,
    HandlerContextParameter,
    HandlerOperationClauseParameter,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedSymbol {
    pub kind: SymbolKind,
    pub name: String,
    pub declaration: NavigationLocation,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationSource {
    Workspace,
    Package { uri: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationLocation {
    pub source: NavigationSource,
    pub span: SourceSpan,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavigationResult {
    pub selected_symbol: SelectedSymbol,
    pub selection: SourceSpan,
    pub definition: NavigationLocation,
    pub references: Vec<SourceSpan>,
}

pub fn navigate(
    snapshot: &EffectiveProjectSnapshot,
    position: SourcePosition,
) -> Option<NavigationResult> {
    let request = snapshot
        .navigation_index()
        .symbol_at_position(position.source.as_str(), &position)?;
    let definition = match &request.symbol {
        Symbol::Function(symbol) => symbol.declaration.clone(),
        Symbol::Constructor(symbol) => symbol.declaration.clone(),
        Symbol::Local(symbol) => workspace_location(symbol.declaration.clone()),
    };
    let selected_symbol = match &request.symbol {
        Symbol::Function(symbol) => SelectedSymbol {
            kind: SymbolKind::Function,
            name: symbol.name.clone(),
            declaration: definition.clone(),
        },
        Symbol::Constructor(symbol) => SelectedSymbol {
            kind: SymbolKind::Constructor,
            name: symbol.name.clone(),
            declaration: definition.clone(),
        },
        Symbol::Local(symbol) => SelectedSymbol {
            kind: match symbol.kind {
                LocalSymbolKind::HandlerContextParameter => SymbolKind::HandlerContextParameter,
                LocalSymbolKind::HandlerOperationClauseParameter => {
                    SymbolKind::HandlerOperationClauseParameter
                }
            },
            name: symbol.name.clone(),
            declaration: definition.clone(),
        },
    };
    let mut references = match &request.symbol {
        Symbol::Function(symbol) => request
            .index
            .files
            .iter()
            .flat_map(|file| request.index.references_in_file(file, symbol))
            .collect(),
        Symbol::Constructor(_) => Vec::new(),
        Symbol::Local(symbol) => request.index.local_references(symbol, false),
    };
    sort_locations(&mut references);
    Some(NavigationResult {
        selected_symbol,
        selection: request.selection,
        definition,
        references,
    })
}

fn sort_locations(locations: &mut Vec<SourceSpan>) {
    locations.sort_by(|left, right| {
        left.file
            .as_str()
            .cmp(right.file.as_str())
            .then(left.start.offset.cmp(&right.start.offset))
            .then(left.end.offset.cmp(&right.end.offset))
    });
    locations.dedup_by(|left, right| {
        left.file == right.file
            && left.start.offset == right.start.offset
            && left.end.offset == right.end.offset
    });
}

#[derive(Clone, Debug)]
struct FunctionSymbol {
    module: String,
    name: String,
    declaration: NavigationLocation,
    package: Option<String>,
    public: bool,
    standard_prelude: bool,
}

#[derive(Clone, Debug)]
struct ConstructorSymbol {
    module: String,
    type_name: String,
    name: String,
    declaration: NavigationLocation,
    package: Option<String>,
    public: bool,
    standard_prelude: bool,
}

#[derive(Clone, Debug)]
struct TypeAliasSymbol {
    module: String,
    name: String,
    target_module: Option<String>,
    target_name: String,
    package: Option<String>,
    standard_prelude: bool,
}

#[derive(Debug)]
struct SymbolRequest {
    index: Arc<SymbolIndex>,
    symbol: Symbol,
    selection: SourceSpan,
}

#[derive(Clone, Debug)]
enum Symbol {
    Function(FunctionSymbol),
    Constructor(ConstructorSymbol),
    Local(LocalSymbol),
}

#[derive(Clone, Debug)]
struct LocalSymbol {
    name: String,
    declaration: SourceSpan,
    scope_file: String,
    scope_start: usize,
    scope_end: usize,
    kind: LocalSymbolKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LocalSymbolKind {
    HandlerContextParameter,
    HandlerOperationClauseParameter,
}

#[derive(Debug)]
struct IndexedFile {
    source: SourceFile,
    module: String,
    companion_target_module: Option<String>,
    uses: BTreeSet<String>,
    external_uses: BTreeSet<(String, String)>,
    origin: IndexedOrigin,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum IndexedSourceOrigin {
    Workspace,
    Package { identity: String },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct IndexedSourceKey {
    origin: IndexedSourceOrigin,
    path: String,
}

#[derive(Debug)]
enum IndexedOrigin {
    Workspace,
    Package {
        identity: String,
        uri: String,
        exported: bool,
        standard_library: bool,
    },
}

#[derive(Debug)]
struct SymbolIndex {
    files: Vec<IndexedFile>,
    functions: Vec<FunctionSymbol>,
    constructors: ConstructorTable,
    type_aliases: Vec<TypeAliasSymbol>,
    functions_by_name: BTreeMap<String, Vec<usize>>,
    constructors_by_name: BTreeMap<String, Vec<usize>>,
    constructors_by_type_name: BTreeMap<String, Vec<usize>>,
    workspace_constructor_names_by_module: BTreeMap<String, BTreeSet<String>>,
    public_workspace_constructor_names_by_module: BTreeMap<String, BTreeSet<String>>,
    public_package_constructor_names_by_import: BTreeMap<(String, String), BTreeSet<String>>,
    public_reexported_constructor_names_by_import:
        BTreeMap<(String, Option<String>), BTreeSet<String>>,
    private_workspace_reexported_constructor_names_by_import_and_module:
        BTreeMap<(String, String), BTreeSet<String>>,
    type_aliases_by_target_name: BTreeMap<String, Vec<usize>>,
    visible_bare_constructor_names_by_source: BTreeMap<IndexedSourceKey, BTreeSet<String>>,
}

#[derive(Debug)]
struct FunctionScope {
    body_start: usize,
    end: usize,
    params: BTreeMap<String, bool>,
    result_binding: Option<String>,
    local_bindings: Vec<LocalBinding>,
}

#[derive(Debug)]
struct LocalBinding {
    name: String,
    start: usize,
    end: usize,
    callable: bool,
}

#[derive(Debug)]
struct ClauseBinding {
    name: String,
    declaration: SourceSpan,
    start: usize,
    end: usize,
    kind: LocalSymbolKind,
}

impl SymbolIndex {
    fn new(
        sources: Vec<SourceFile>,
        dependencies: Vec<DirectDependencySnapshot>,
        standard_library: Option<DirectDependencySnapshot>,
    ) -> Self {
        let mut files = sources
            .into_iter()
            .map(index_workspace_source)
            .collect::<Vec<_>>();
        for dependency in dependencies.into_iter().chain(standard_library) {
            index_dependency_sources(&mut files, dependency);
        }
        let functions = files
            .iter()
            .flat_map(function_declarations)
            .collect::<Vec<_>>();
        let constructors = files
            .iter()
            .flat_map(constructor_declarations)
            .collect::<Vec<_>>();
        let type_aliases = files
            .iter()
            .flat_map(type_alias_declarations)
            .collect::<Vec<_>>();
        let mut index = Self {
            functions_by_name: function_indices_by_name(&functions),
            constructors_by_name: constructor_indices_by_name(&constructors),
            constructors_by_type_name: constructor_indices_by_type_name(&constructors),
            workspace_constructor_names_by_module: workspace_constructor_names_by_module(
                &constructors,
                false,
            ),
            public_workspace_constructor_names_by_module: workspace_constructor_names_by_module(
                &constructors,
                true,
            ),
            public_package_constructor_names_by_import: public_package_constructor_names_by_import(
                &constructors,
            ),
            type_aliases_by_target_name: type_alias_indices_by_target_name(&type_aliases),
            public_reexported_constructor_names_by_import: BTreeMap::new(),
            private_workspace_reexported_constructor_names_by_import_and_module: BTreeMap::new(),
            functions,
            constructors: ConstructorTable::new(constructors),
            type_aliases,
            files,
            visible_bare_constructor_names_by_source: BTreeMap::new(),
        };
        index.public_reexported_constructor_names_by_import =
            public_reexported_constructor_names_by_import(&index);
        index.private_workspace_reexported_constructor_names_by_import_and_module =
            private_workspace_reexported_constructor_names_by_import_and_module(&index);
        index.visible_bare_constructor_names_by_source =
            visible_bare_constructor_names_by_source(&index);
        index
    }

    fn symbol_at_position(
        self: Arc<Self>,
        source_path: &str,
        position: &SourcePosition,
    ) -> Option<SymbolRequest> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == source_path)?;
        let offset = offset_for_position(file.source.text(), position)?;
        let tokens = lex(&file.source).tokens;
        let (token_index, token) = identifier_token_at(&tokens, offset)?;
        let selection = file.source.span(token.range);
        let name = file
            .source
            .text()
            .get(selection.start.offset..selection.end.offset)?
            .to_string();
        let symbol = self.symbol_for_selection(file, &tokens, token_index, &name, &selection)?;
        Some(SymbolRequest {
            index: self,
            symbol,
            selection,
        })
    }

    fn symbol_for_selection(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<Symbol> {
        if let Some(symbol) =
            handler_operation_clause_symbol(file, tokens, token_index, name, selection)
        {
            return Some(Symbol::Local(symbol));
        }

        if is_handler_operation_clause_operation_name(tokens, token_index) {
            return None;
        }

        if let Some(symbol) = self.function_declared_at(name, selection) {
            return Some(Symbol::Function(symbol));
        }

        if !is_call_target_token(tokens, token_index) {
            return None;
        }
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            return self.symbol_for_bare_call(file, tokens, token_index, name);
        };
        self.symbol_for_qualified_call(file, &qualifier, name)
    }

    fn function_declared_at(&self, name: &str, selection: &SourceSpan) -> Option<FunctionSymbol> {
        self.functions_named(name)
            .find(|symbol| {
                symbol.name == name
                    && symbol.package.is_none()
                    && symbol.declaration.span.file == selection.file
                    && symbol.declaration.span.start.offset == selection.start.offset
                    && symbol.declaration.span.end.offset == selection.end.offset
            })
            .cloned()
    }

    fn symbol_for_bare_call(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
    ) -> Option<Symbol> {
        if self.local_callable_binding_shadows_call_target(file, tokens, token_index, name) {
            return None;
        }
        if self.has_visible_bare_constructor_name(file, name) {
            if let Some(symbol) = self.constructor_for_bare_call(file, name) {
                return Some(Symbol::Constructor(symbol));
            }
            if self.has_ambiguous_constructor_for_bare_call(file, name) {
                return None;
            }
        }
        if self.local_binding_shadows_call_target(file, tokens, token_index, name) {
            return None;
        }
        if let Some(symbol) = self.functions_named(name).find(|symbol| {
            symbol.name == name && symbol.module == file.module && symbol.package.is_none()
        }) {
            return Some(Symbol::Function(symbol.clone()));
        }
        if self.has_visible_non_prelude_imported_function(file, name)
            || self.has_visible_non_prelude_imported_constructor(file, name)
        {
            return None;
        }
        self.functions_named(name)
            .find(|symbol| symbol.name == name && symbol.standard_prelude)
            .cloned()
            .map(Symbol::Function)
    }

    fn symbol_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<Symbol> {
        if let Some(symbol) = self.constructor_for_qualified_call(file, qualifier, name) {
            return Some(Symbol::Constructor(symbol));
        }
        if self.has_ambiguous_constructor_for_qualified_call(file, qualifier, name) {
            return None;
        }
        self.functions_named(name)
            .find(|symbol| match &symbol.package {
                Some(package) => {
                    symbol.name == name
                        && symbol.module == qualifier
                        && (symbol.standard_prelude
                            || file
                                .external_uses
                                .contains(&(symbol.module.clone(), package.clone())))
                }
                None => {
                    symbol.name == name
                        && symbol.module == qualifier
                        && file.uses.contains(&symbol.module)
                        && file
                            .companion_target_module
                            .as_ref()
                            .is_some_and(|target| target == &symbol.module)
                }
            })
            .cloned()
            .map(Symbol::Function)
    }

    fn constructor_for_bare_call(
        &self,
        file: &IndexedFile,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        let current_module_candidates: Vec<_> = self
            .constructors_named(name)
            .filter(|symbol| {
                symbol.name == name
                    && symbol.package.is_none()
                    && symbol.module == file.module
                    && visible_workspace_constructor_from(file, symbol)
            })
            .collect();
        if !current_module_candidates.is_empty() {
            return exactly_one_constructor(current_module_candidates.into_iter()).cloned();
        }

        let workspace_import_candidates: Vec<_> = self
            .constructors_named(name)
            .filter(|symbol| {
                symbol.name == name
                    && !symbol.standard_prelude
                    && symbol.package.is_none()
                    && symbol.module != file.module
                    && (file.uses.contains(&symbol.module)
                        || self.constructor_reexport_visible_from(file, symbol, None))
                    && visible_workspace_constructor_from(file, symbol)
            })
            .collect();
        if !workspace_import_candidates.is_empty() {
            return exactly_one_constructor(workspace_import_candidates.into_iter()).cloned();
        }

        let external_import_candidates = self.constructors_named(name).filter(|symbol| {
            symbol.name == name
                && !symbol.standard_prelude
                && symbol.public
                && symbol.package.as_ref().is_some_and(|package| {
                    file.external_uses
                        .contains(&(symbol.module.clone(), package.clone()))
                        || self.constructor_reexport_visible_from(file, symbol, Some(package))
                })
        });
        exactly_one_constructor(external_import_candidates).cloned()
    }

    fn has_ambiguous_constructor_for_bare_call(&self, file: &IndexedFile, name: &str) -> bool {
        at_least_two_constructors(self.constructors_named(name).filter(|symbol| {
            symbol.name == name
                && symbol.package.is_none()
                && symbol.module == file.module
                && visible_workspace_constructor_from(file, symbol)
        })) || at_least_two_constructors(self.constructors_named(name).filter(|symbol| {
            symbol.name == name
                && !symbol.standard_prelude
                && symbol.package.is_none()
                && symbol.module != file.module
                && (file.uses.contains(&symbol.module)
                    || self.constructor_reexport_visible_from(file, symbol, None))
                && visible_workspace_constructor_from(file, symbol)
        })) || at_least_two_constructors(self.constructors_named(name).filter(|symbol| {
            symbol.name == name
                && !symbol.standard_prelude
                && symbol.public
                && symbol.package.as_ref().is_some_and(|package| {
                    file.external_uses
                        .contains(&(symbol.module.clone(), package.clone()))
                        || self.constructor_reexport_visible_from(file, symbol, Some(package))
                })
        }))
    }

    fn constructor_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> Option<ConstructorSymbol> {
        exactly_one_constructor(self.constructors_named(name).filter(|symbol| {
            symbol.name == name
                && (constructor_qualifier_matches(symbol, qualifier)
                    || self.constructor_reexport_qualifier_matches(file, symbol, qualifier))
                && match &symbol.package {
                    Some(package) => {
                        symbol.standard_prelude
                            || file
                                .external_uses
                                .contains(&(symbol.module.clone(), package.clone()))
                            || self.constructor_reexport_visible_from(file, symbol, Some(package))
                    }
                    None => {
                        symbol.module == file.module
                            || ((file.uses.contains(&symbol.module)
                                || self.constructor_reexport_visible_from(file, symbol, None))
                                && visible_workspace_constructor_from(file, symbol))
                    }
                }
        }))
        .cloned()
    }

    fn has_ambiguous_constructor_for_qualified_call(
        &self,
        file: &IndexedFile,
        qualifier: &str,
        name: &str,
    ) -> bool {
        at_least_two_constructors(self.constructors_named(name).filter(|symbol| {
            symbol.name == name
                && (constructor_qualifier_matches(symbol, qualifier)
                    || self.constructor_reexport_qualifier_matches(file, symbol, qualifier))
                && match &symbol.package {
                    Some(package) => {
                        symbol.standard_prelude
                            || file
                                .external_uses
                                .contains(&(symbol.module.clone(), package.clone()))
                            || self.constructor_reexport_visible_from(file, symbol, Some(package))
                    }
                    None => {
                        symbol.module == file.module
                            || ((file.uses.contains(&symbol.module)
                                || self.constructor_reexport_visible_from(file, symbol, None))
                                && visible_workspace_constructor_from(file, symbol))
                    }
                }
        }))
    }
}

#[derive(Debug)]
struct ConstructorTable(Vec<ConstructorSymbol>);

impl ConstructorTable {
    fn new(constructors: Vec<ConstructorSymbol>) -> Self {
        Self(constructors)
    }

    #[allow(dead_code)]
    fn iter(&self) -> ConstructorTableIter<'_> {
        ConstructorTableIter {
            symbols: self.0.iter(),
        }
    }

    fn get(&self, index: usize) -> Option<&ConstructorSymbol> {
        self.0.get(index)
    }
}

#[allow(dead_code)]
struct ConstructorTableIter<'a> {
    symbols: std::slice::Iter<'a, ConstructorSymbol>,
}

impl<'a> Iterator for ConstructorTableIter<'a> {
    type Item = &'a ConstructorSymbol;

    fn next(&mut self) -> Option<Self::Item> {
        let symbol = self.symbols.next()?;
        #[cfg(test)]
        navigation_stats::record_constructor_candidate();
        Some(symbol)
    }
}

struct ConstructorCandidates<'a> {
    table: &'a ConstructorTable,
    indices: Option<std::slice::Iter<'a, usize>>,
}

impl<'a> Iterator for ConstructorCandidates<'a> {
    type Item = &'a ConstructorSymbol;

    fn next(&mut self) -> Option<Self::Item> {
        let indices = self.indices.as_mut()?;
        loop {
            let index = indices.next()?;
            let symbol = self.table.get(*index);
            if symbol.is_some() {
                #[cfg(test)]
                navigation_stats::record_constructor_candidate();
            }
            if let Some(symbol) = symbol {
                return Some(symbol);
            }
        }
    }
}

fn exactly_one_constructor<'a>(
    mut candidates: impl Iterator<Item = &'a ConstructorSymbol>,
) -> Option<&'a ConstructorSymbol> {
    let candidate = candidates.next()?;
    candidates.next().is_none().then_some(candidate)
}

fn at_least_two_constructors<'a>(
    mut candidates: impl Iterator<Item = &'a ConstructorSymbol>,
) -> bool {
    candidates.next().is_some() && candidates.next().is_some()
}

impl SymbolIndex {
    fn constructor_reexport_qualifier_matches(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        qualifier: &str,
    ) -> bool {
        self.type_aliases_targeting(&symbol.type_name).any(|alias| {
            type_alias_targets_constructor(alias, symbol)
                && (qualifier == alias.module
                    || qualifier == format!("{}::{}", alias.module, alias.name))
                && match &alias.package {
                    Some(alias_package) => file
                        .external_uses
                        .contains(&(alias.module.clone(), alias_package.clone())),
                    None => file.uses.contains(&alias.module) || file.module == alias.module,
                }
        })
    }

    fn has_visible_non_prelude_imported_constructor(&self, file: &IndexedFile, name: &str) -> bool {
        if !self.has_visible_bare_constructor_name(file, name) {
            return false;
        }
        self.constructors_named(name).any(|symbol| {
            if symbol.name != name || symbol.standard_prelude {
                return false;
            }
            if symbol.package.is_none() && symbol.module == file.module {
                return false;
            }
            match &symbol.package {
                Some(package) => {
                    symbol.public
                        && file
                            .external_uses
                            .contains(&(symbol.module.clone(), package.clone()))
                }
                None => {
                    (file.uses.contains(&symbol.module)
                        || self.constructor_reexport_visible_from(file, symbol, None))
                        && visible_workspace_constructor_from(file, symbol)
                }
            }
        })
    }

    fn constructor_reexport_visible_from(
        &self,
        file: &IndexedFile,
        symbol: &ConstructorSymbol,
        package: Option<&String>,
    ) -> bool {
        self.type_aliases_targeting(&symbol.type_name).any(|alias| {
            if !type_alias_targets_constructor(alias, symbol) {
                return false;
            }
            if alias.package.as_ref() != package {
                return false;
            }
            match &alias.package {
                Some(alias_package) => file
                    .external_uses
                    .contains(&(alias.module.clone(), alias_package.clone())),
                None => file.uses.contains(&alias.module),
            }
        })
    }

    fn has_visible_non_prelude_imported_function(&self, file: &IndexedFile, name: &str) -> bool {
        self.functions_named(name).any(|symbol| {
            if symbol.name != name || symbol.standard_prelude {
                return false;
            }
            if symbol.package.is_none() && symbol.module == file.module {
                return false;
            }
            if symbol.package.is_none() && !symbol.public {
                return false;
            }
            match &symbol.package {
                Some(package) => file
                    .external_uses
                    .contains(&(symbol.module.clone(), package.clone())),
                None => file.uses.contains(&symbol.module),
            }
        })
    }

    fn local_references(&self, symbol: &LocalSymbol, include_declaration: bool) -> Vec<SourceSpan> {
        let Some(file) = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == symbol.scope_file)
        else {
            return Vec::new();
        };
        let tokens = lex(&file.source).tokens;
        let mut spans = Vec::new();
        if include_declaration {
            spans.push(symbol.declaration.clone());
        }
        spans.extend(
            tokens
                .iter()
                .enumerate()
                .filter(|(index, token)| {
                    token.text == symbol.name
                        && token.kind == TokenKind::Ident
                        && token.range.start >= symbol.scope_start
                        && token.range.start < symbol.scope_end
                        && !is_field_name(&tokens, *index)
                        && !is_local_binding_name(&tokens, *index)
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || inside_handler_operation_clause_body(&tokens, token.range.start))
                        && !local_binding_shadows_name(
                            &tokens,
                            &symbol.name,
                            token.range.start,
                            symbol.scope_start,
                            symbol.scope_end,
                        )
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || !handler_operation_clause_parameter_shadows_name(
                                &tokens,
                                &symbol.name,
                                token.range.start,
                                symbol.scope_start,
                                symbol.scope_end,
                            ))
                })
                .map(|(_, token)| file.source.span(token.range)),
        );
        spans.sort_by_key(|span| span.start.offset);
        spans.dedup_by_key(|span| (span.start.offset, span.end.offset));
        spans
    }

    fn references_in_file(&self, file: &IndexedFile, symbol: &FunctionSymbol) -> Vec<SourceSpan> {
        if symbol.package.is_some() {
            return Vec::new();
        }
        if !matches!(file.origin, IndexedOrigin::Workspace) {
            return Vec::new();
        }
        if file.module == symbol.module {
            return self.bare_function_references_in_file(file, symbol);
        }
        if file.uses.contains(&symbol.module)
            && file
                .companion_target_module
                .as_ref()
                .is_some_and(|target| target == &symbol.module)
        {
            return self.qualified_function_references_in_file(file, symbol);
        }
        Vec::new()
    }

    fn bare_function_references_in_file(
        &self,
        file: &IndexedFile,
        symbol: &FunctionSymbol,
    ) -> Vec<SourceSpan> {
        let tokens = lex(&file.source).tokens;
        call_reference_token_indices(&tokens, &symbol.name)
            .into_iter()
            .filter(|index| {
                !is_call_target_token(&tokens, *index)
                    || matches!(
                        self.symbol_for_bare_call(file, &tokens, *index, &symbol.name),
                        Some(Symbol::Function(candidate))
                            if same_function_symbol(&candidate, symbol)
                    )
            })
            .map(|index| file.source.span(tokens[index].range))
            .collect()
    }

    fn local_binding_shadows_call_target(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        index: usize,
        name: &str,
    ) -> bool {
        let offset = tokens[index].range.start;
        let callable_values = self.callable_function_value_names(file);
        function_scopes(tokens, &callable_values)
            .iter()
            .any(|scope| {
                offset >= scope.body_start
                    && offset < scope.end
                    && scope.shadows(name, tokens, index)
            })
            || handler_operation_clause_bindings(file, tokens)
                .iter()
                .any(|binding| {
                    binding.name == name && offset >= binding.start && offset < binding.end
                })
    }

    fn local_callable_binding_shadows_call_target(
        &self,
        file: &IndexedFile,
        tokens: &[Token],
        index: usize,
        name: &str,
    ) -> bool {
        let offset = tokens[index].range.start;
        let callable_values = self.callable_function_value_names(file);
        function_scopes(tokens, &callable_values)
            .iter()
            .any(|scope| {
                offset >= scope.body_start
                    && offset < scope.end
                    && scope.callable_shadows(name, tokens, index)
            })
            || handler_operation_clause_bindings(file, tokens)
                .iter()
                .any(|binding| {
                    binding.name == name && offset >= binding.start && offset < binding.end
                })
    }

    fn callable_function_value_names(&self, file: &IndexedFile) -> BTreeSet<String> {
        self.functions
            .iter()
            .filter(|symbol| {
                symbol.package.is_none()
                    && (symbol.module == file.module
                        || file.uses.contains(&symbol.module)
                            && symbol.public
                            && file
                                .companion_target_module
                                .as_ref()
                                .is_some_and(|target| target == &symbol.module))
            })
            .map(|symbol| symbol.name.clone())
            .collect()
    }

    fn qualified_function_references_in_file(
        &self,
        file: &IndexedFile,
        symbol: &FunctionSymbol,
    ) -> Vec<SourceSpan> {
        let tokens = lex(&file.source).tokens;
        qualified_reference_token_indices(&tokens, &symbol.module, &symbol.name)
            .into_iter()
            .filter(|index| {
                let Some(qualifier) = qualifier_for_token(&tokens, *index) else {
                    return false;
                };
                matches!(
                    self.symbol_for_qualified_call(file, &qualifier, &symbol.name),
                    Some(Symbol::Function(candidate)) if same_function_symbol(&candidate, symbol)
                )
            })
            .map(|index| file.source.span(tokens[index].range))
            .collect()
    }

    fn functions_named(&self, name: &str) -> impl Iterator<Item = &FunctionSymbol> {
        self.functions_by_name
            .get(name)
            .into_iter()
            .flat_map(|indices| indices.iter().map(|index| &self.functions[*index]))
    }

    fn constructors_named(&self, name: &str) -> ConstructorCandidates<'_> {
        ConstructorCandidates {
            table: &self.constructors,
            indices: self
                .constructors_by_name
                .get(name)
                .map(|indices| indices.iter()),
        }
    }

    fn constructors_for_type(&self, name: &str) -> ConstructorCandidates<'_> {
        ConstructorCandidates {
            table: &self.constructors,
            indices: self
                .constructors_by_type_name
                .get(name)
                .map(|indices| indices.iter()),
        }
    }

    fn has_visible_bare_constructor_name(&self, file: &IndexedFile, name: &str) -> bool {
        self.visible_bare_constructor_names_by_source
            .get(&file.key())
            .is_some_and(|names| names.contains(name))
    }

    fn type_aliases_targeting(&self, name: &str) -> impl Iterator<Item = &TypeAliasSymbol> {
        self.type_aliases_by_target_name
            .get(name)
            .into_iter()
            .flat_map(|indices| indices.iter().map(|index| &self.type_aliases[*index]))
    }
}

impl IndexedFile {
    fn key(&self) -> IndexedSourceKey {
        let origin = match &self.origin {
            IndexedOrigin::Workspace => IndexedSourceOrigin::Workspace,
            IndexedOrigin::Package { identity, .. } => IndexedSourceOrigin::Package {
                identity: identity.clone(),
            },
        };
        IndexedSourceKey {
            origin,
            path: self.source.path().as_str().to_string(),
        }
    }
}

fn same_function_symbol(left: &FunctionSymbol, right: &FunctionSymbol) -> bool {
    left.name == right.name
        && left.module == right.module
        && left.package == right.package
        && left.declaration == right.declaration
}

fn index_workspace_source(source: SourceFile) -> IndexedFile {
    let path = source.path().as_str().to_string();
    let companion_target_module = classify_companion_source(&path)
        .and_then(|companion| module_name_from_path(&companion.target_path));
    let module = explicit_module_name(source.text())
        .or_else(|| module_name_from_path(&path))
        .unwrap_or_default();
    let (uses, external_uses) = use_modules(source.text());
    IndexedFile {
        source,
        module,
        companion_target_module,
        uses,
        external_uses,
        origin: IndexedOrigin::Workspace,
    }
}

fn index_dependency_sources(files: &mut Vec<IndexedFile>, dependency: DirectDependencySnapshot) {
    for (source, entry) in dependency.indexed_sources() {
        let text = std::str::from_utf8(source.bytes())
            .expect("captured package source text is valid UTF-8");
        let source_file = SourceFile::new(source.path(), text);
        let module = explicit_module_name(text)
            .or_else(|| module_name_from_path(source.path()))
            .unwrap_or_default();
        let (uses, external_uses) = use_modules(text);
        files.push(IndexedFile {
            source: source_file,
            module,
            companion_target_module: None,
            uses,
            external_uses,
            origin: IndexedOrigin::Package {
                identity: dependency.identity.as_str().to_string(),
                uri: entry.uri().to_string(),
                exported: dependency.exported_sources.contains(source.path()),
                standard_library: dependency.standard_library,
            },
        });
    }
}

fn function_indices_by_name(functions: &[FunctionSymbol]) -> BTreeMap<String, Vec<usize>> {
    let mut indices = BTreeMap::new();
    for (index, symbol) in functions.iter().enumerate() {
        indices
            .entry(symbol.name.clone())
            .or_insert_with(Vec::new)
            .push(index);
    }
    indices
}

fn constructor_indices_by_name(constructors: &[ConstructorSymbol]) -> BTreeMap<String, Vec<usize>> {
    let mut indices = BTreeMap::new();
    for (index, symbol) in constructors.iter().enumerate() {
        indices
            .entry(symbol.name.clone())
            .or_insert_with(Vec::new)
            .push(index);
    }
    indices
}

fn constructor_indices_by_type_name(
    constructors: &[ConstructorSymbol],
) -> BTreeMap<String, Vec<usize>> {
    let mut indices = BTreeMap::new();
    for (index, symbol) in constructors.iter().enumerate() {
        indices
            .entry(symbol.type_name.clone())
            .or_insert_with(Vec::new)
            .push(index);
    }
    indices
}

fn workspace_constructor_names_by_module(
    constructors: &[ConstructorSymbol],
    public_only: bool,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut names = BTreeMap::<String, BTreeSet<String>>::new();
    for symbol in constructors {
        if symbol.package.is_none() && !symbol.standard_prelude && (!public_only || symbol.public) {
            names
                .entry(symbol.module.clone())
                .or_default()
                .insert(symbol.name.clone());
        }
    }
    names
}

fn public_package_constructor_names_by_import(
    constructors: &[ConstructorSymbol],
) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut names = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for symbol in constructors {
        if symbol.public
            && !symbol.standard_prelude
            && let Some(package) = &symbol.package
        {
            names
                .entry((symbol.module.clone(), package.clone()))
                .or_default()
                .insert(symbol.name.clone());
        }
    }
    names
}

fn type_alias_indices_by_target_name(
    type_aliases: &[TypeAliasSymbol],
) -> BTreeMap<String, Vec<usize>> {
    let mut indices = BTreeMap::new();
    for (index, alias) in type_aliases.iter().enumerate() {
        indices
            .entry(alias.target_name.clone())
            .or_insert_with(Vec::new)
            .push(index);
    }
    indices
}

fn visible_bare_constructor_names_by_source(
    index: &SymbolIndex,
) -> BTreeMap<IndexedSourceKey, BTreeSet<String>> {
    index
        .files
        .iter()
        .map(|file| {
            let mut names = BTreeSet::new();
            names.extend(
                index
                    .workspace_constructor_names_by_module
                    .get(&file.module)
                    .into_iter()
                    .flat_map(|module_names| module_names.iter().cloned()),
            );
            for module in &file.uses {
                names.extend(
                    index
                        .public_workspace_constructor_names_by_module
                        .get(module)
                        .into_iter()
                        .flat_map(|module_names| module_names.iter().cloned()),
                );
            }
            for import in &file.external_uses {
                names.extend(
                    index
                        .public_package_constructor_names_by_import
                        .get(import)
                        .into_iter()
                        .flat_map(|module_names| module_names.iter().cloned()),
                );
            }
            names.extend(reexported_public_bare_constructor_names(index, file));
            names.extend(reexported_private_workspace_bare_constructor_names(
                index, file,
            ));
            (file.key(), names)
        })
        .collect()
}

fn public_reexported_constructor_names_by_import(
    index: &SymbolIndex,
) -> BTreeMap<(String, Option<String>), BTreeSet<String>> {
    let mut names = BTreeMap::<(String, Option<String>), BTreeSet<String>>::new();
    for alias in &index.type_aliases {
        for symbol in index.constructors_for_type(&alias.target_name) {
            if type_alias_targets_constructor(alias, symbol)
                && symbol.public
                && !symbol.standard_prelude
                && alias.package.as_ref() == symbol.package.as_ref()
            {
                names
                    .entry((alias.module.clone(), alias.package.clone()))
                    .or_default()
                    .insert(symbol.name.clone());
            }
        }
    }
    names
}

fn reexported_public_bare_constructor_names(
    index: &SymbolIndex,
    file: &IndexedFile,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for module in &file.uses {
        names.extend(
            index
                .public_reexported_constructor_names_by_import
                .get(&(module.clone(), None))
                .into_iter()
                .flat_map(|module_names| module_names.iter().cloned()),
        );
    }
    for (module, package) in &file.external_uses {
        names.extend(
            index
                .public_reexported_constructor_names_by_import
                .get(&(module.clone(), Some(package.clone())))
                .into_iter()
                .flat_map(|module_names| module_names.iter().cloned()),
        );
    }
    names
}

fn private_workspace_reexported_constructor_names_by_import_and_module(
    index: &SymbolIndex,
) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut names = BTreeMap::<(String, String), BTreeSet<String>>::new();
    for alias in index
        .type_aliases
        .iter()
        .filter(|alias| alias.package.is_none())
    {
        for symbol in index.constructors_for_type(&alias.target_name) {
            if type_alias_targets_constructor(alias, symbol)
                && !symbol.public
                && symbol.package.is_none()
                && !symbol.standard_prelude
            {
                names
                    .entry((alias.module.clone(), symbol.module.clone()))
                    .or_default()
                    .insert(symbol.name.clone());
            }
        }
    }
    names
}

fn reexported_private_workspace_bare_constructor_names(
    index: &SymbolIndex,
    file: &IndexedFile,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for module in &file.uses {
        names.extend(
            index
                .private_workspace_reexported_constructor_names_by_import_and_module
                .get(&(module.clone(), file.module.clone()))
                .into_iter()
                .flat_map(|module_names| module_names.iter().cloned()),
        );
    }
    names
}

fn visible_workspace_constructor_from(file: &IndexedFile, symbol: &ConstructorSymbol) -> bool {
    symbol.public || symbol.module == file.module
}

fn constructor_qualifier_matches(symbol: &ConstructorSymbol, qualifier: &str) -> bool {
    qualifier == symbol.module || qualifier == format!("{}::{}", symbol.module, symbol.type_name)
}

fn type_alias_targets_constructor(alias: &TypeAliasSymbol, symbol: &ConstructorSymbol) -> bool {
    if alias.standard_prelude != symbol.standard_prelude {
        return false;
    }
    if alias.target_name != symbol.type_name {
        return false;
    }
    match &alias.target_module {
        Some(module) => module == &symbol.module,
        None => alias.module == symbol.module,
    }
}

fn function_declarations(file: &IndexedFile) -> Vec<FunctionSymbol> {
    let mut functions = Vec::new();
    let tokens = lex(&file.source).tokens;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Fn
            && let Some(name) = next_non_layout_token(&tokens, index)
            && is_identifier(&name.text)
        {
            let public = previous_non_layout_token(&tokens, index)
                .is_some_and(|previous| previous.kind == TokenKind::Pub);
            let (declaration, package, standard_prelude) = match &file.origin {
                IndexedOrigin::Workspace => (
                    workspace_location(file.source.span(name.range)),
                    None,
                    false,
                ),
                IndexedOrigin::Package {
                    identity,
                    uri,
                    exported,
                    standard_library,
                } => {
                    if !exported || !public {
                        continue;
                    }
                    (
                        NavigationLocation {
                            source: NavigationSource::Package { uri: uri.clone() },
                            span: file.source.span(name.range),
                        },
                        Some(identity.clone()),
                        *standard_library && file.module == "prelude",
                    )
                }
            };
            functions.push(FunctionSymbol {
                module: file.module.clone(),
                name: name.text.clone(),
                declaration,
                package,
                public,
                standard_prelude,
            });
        }
    }
    functions
}

fn constructor_declarations(file: &IndexedFile) -> Vec<ConstructorSymbol> {
    let tokens = lex(&file.source).tokens;
    let tokens = tokens.as_slice();
    parse(&file.source)
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Type(type_decl) => Some(type_decl),
            _ => None,
        })
        .flat_map(|type_decl| {
            let type_public = type_decl.visibility == Visibility::Public;
            type_decl.variants.iter().filter_map(move |variant| {
                let name = variant.name.as_ref()?;
                let public = type_public && variant.visibility == Visibility::Public;
                let span = tokens
                    .iter()
                    .find(|token| {
                        token.kind == TokenKind::Ident
                            && token.text == *name
                            && token.range.start >= variant.span.start.offset
                            && token.range.end <= variant.span.end.offset
                    })
                    .map_or_else(
                        || variant.span.clone(),
                        |token| file.source.span(token.range),
                    );
                let (declaration, package, standard_prelude) = match &file.origin {
                    IndexedOrigin::Workspace => (workspace_location(span), None, false),
                    IndexedOrigin::Package {
                        identity,
                        uri,
                        exported,
                        standard_library,
                    } => {
                        if !exported || !public {
                            return None;
                        }
                        (
                            NavigationLocation {
                                source: NavigationSource::Package { uri: uri.clone() },
                                span,
                            },
                            Some(identity.clone()),
                            *standard_library && file.module == "prelude",
                        )
                    }
                };
                Some(ConstructorSymbol {
                    module: file.module.clone(),
                    type_name: type_decl.name.clone().unwrap_or_default(),
                    name: name.clone(),
                    declaration,
                    package,
                    public,
                    standard_prelude,
                })
            })
        })
        .collect()
}

fn type_alias_declarations(file: &IndexedFile) -> Vec<TypeAliasSymbol> {
    parse(&file.source)
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::PublicAlias(alias) if alias.kind == PublicAliasKind::Type => {
                let name = alias.name.clone()?;
                let target_name = alias.target.last()?.clone();
                let target_module = match alias.target.as_slice() {
                    [_] => None,
                    [segments @ .., _] => Some(segments.join("::")),
                    [] => None,
                };
                let (package, standard_prelude) = match &file.origin {
                    IndexedOrigin::Workspace => (None, false),
                    IndexedOrigin::Package {
                        identity,
                        exported,
                        standard_library,
                        ..
                    } => {
                        if !exported {
                            return None;
                        }
                        (
                            Some(identity.clone()),
                            *standard_library && file.module == "prelude",
                        )
                    }
                };
                Some(TypeAliasSymbol {
                    module: file.module.clone(),
                    name,
                    target_module,
                    target_name,
                    package,
                    standard_prelude,
                })
            }
            _ => None,
        })
        .collect()
}

fn handler_operation_clause_symbol(
    file: &IndexedFile,
    tokens: &[Token],
    token_index: usize,
    name: &str,
    selection: &SourceSpan,
) -> Option<LocalSymbol> {
    handler_operation_clause_bindings(file, tokens)
        .into_iter()
        .find(|binding| {
            let token_offset = tokens[token_index].range.start;
            binding.name == name
                && ((selection.start.offset >= binding.declaration.start.offset
                    && selection.start.offset < binding.declaration.end.offset)
                    || (token_offset >= binding.start
                        && token_offset < binding.end
                        && (binding.kind != LocalSymbolKind::HandlerContextParameter
                            || inside_handler_operation_clause_body(tokens, token_offset))
                        && !local_binding_shadows_name(
                            tokens,
                            &binding.name,
                            token_offset,
                            binding.start,
                            binding.end,
                        )))
        })
        .map(|binding| LocalSymbol {
            name: binding.name,
            declaration: binding.declaration,
            scope_file: file.source.path().as_str().to_string(),
            scope_start: binding.start,
            scope_end: binding.end,
            kind: binding.kind,
        })
}

fn handler_operation_clause_bindings(file: &IndexedFile, tokens: &[Token]) -> Vec<ClauseBinding> {
    let mut clause_bindings = Vec::new();
    for (arrow_index, arrow) in tokens.iter().enumerate() {
        if arrow.kind != TokenKind::FatArrow
            || !inside_top_level_block(tokens, arrow_index, TokenKind::Handler)
        {
            continue;
        }
        let line_start_index = line_start_index(tokens, arrow_index);
        let body_end =
            handler_operation_clause_body_end(tokens, arrow_index, file.source.text().len());
        let Some(lparen_index) = tokens[line_start_index..arrow_index]
            .iter()
            .position(|token| token.kind == TokenKind::LParen)
            .map(|index| line_start_index + index)
        else {
            continue;
        };
        let Some(rparen_index) = tokens[lparen_index + 1..arrow_index]
            .iter()
            .position(|token| token.kind == TokenKind::RParen)
            .map(|index| lparen_index + 1 + index)
        else {
            continue;
        };
        for token in &tokens[lparen_index + 1..rparen_index] {
            if token.kind == TokenKind::Ident && is_identifier(&token.text) {
                clause_bindings.push(ClauseBinding {
                    name: token.text.clone(),
                    declaration: file.source.span(token.range),
                    start: arrow.range.end,
                    end: body_end,
                    kind: LocalSymbolKind::HandlerOperationClauseParameter,
                });
            }
        }
    }
    clause_bindings.extend(handler_context_parameter_bindings(file, tokens));
    clause_bindings
}

fn handler_context_parameter_bindings(file: &IndexedFile, tokens: &[Token]) -> Vec<ClauseBinding> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind == TokenKind::Handler)
        .flat_map(|(handler_index, _)| {
            handler_context_parameter_bindings_for_handler(file, tokens, handler_index)
        })
        .collect()
}

fn handler_context_parameter_bindings_for_handler(
    file: &IndexedFile,
    tokens: &[Token],
    handler_index: usize,
) -> Vec<ClauseBinding> {
    let Some(body_start) = tokens[handler_index..]
        .iter()
        .find(|token| token.kind == TokenKind::Newline)
        .map(|token| token.range.end)
    else {
        return Vec::new();
    };
    let handler_end = function_scope_end(tokens, handler_index + 1).unwrap_or(body_start);
    let Some(lparen_index) = tokens[handler_index..]
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
        .map(|index| handler_index + index)
    else {
        return Vec::new();
    };
    let Some(rparen_index) = matching_rparen_index(tokens, lparen_index, tokens.len()) else {
        return Vec::new();
    };
    tokens[lparen_index + 1..rparen_index]
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind == TokenKind::Ident && is_identifier(&token.text))
        .filter(|(relative_index, _)| {
            let index = lparen_index + 1 + relative_index;
            next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
        })
        .map(|(_, token)| ClauseBinding {
            name: token.text.clone(),
            declaration: file.source.span(token.range),
            start: body_start,
            end: handler_end,
            kind: LocalSymbolKind::HandlerContextParameter,
        })
        .collect()
}

fn handler_operation_clause_body_end(
    tokens: &[Token],
    arrow_index: usize,
    file_end: usize,
) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[arrow_index + 1..].iter().enumerate() {
        let index = arrow_index + 1 + relative_index;
        match token.kind {
            TokenKind::Eof => return file_end,
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks = nested_blocks.saturating_sub(1),
            TokenKind::FatArrow if nested_blocks == 0 && !is_satisfy_arrow(tokens, index) => {
                return match_arm_pattern_start_from_arrow(tokens, token.range.start);
            }
            _ => {}
        }
    }
    file_end
}

fn line_start_index(tokens: &[Token], index: usize) -> usize {
    tokens[..index]
        .iter()
        .rposition(|token| token.kind == TokenKind::Newline)
        .map_or(0, |index| index + 1)
}

fn matching_rparen_index(tokens: &[Token], lparen_index: usize, end_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (relative_index, token) in tokens[lparen_index..end_index].iter().enumerate() {
        let index = lparen_index + relative_index;
        match token.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn call_reference_token_indices(tokens: &[Token], name: &str) -> Vec<usize> {
    let scopes = function_scopes(tokens, &BTreeSet::new());
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == name
                && is_identifier(&token.text)
                && previous_non_layout_token(tokens, *index)
                    .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
                && !is_field_name(tokens, *index)
                && !is_function_declaration_name(tokens, *index)
                && !is_parameter_name(tokens, *index)
                && !is_local_binding_name(tokens, *index)
                && !is_handler_operation_clause_operation_name(tokens, *index)
                && (token_scope(&scopes, token.range.start)
                    .is_some_and(|scope| !scope.shadows(name, tokens, *index))
                    || is_handler_operation_clause_call_target(tokens, *index)
                    || is_function_alias_target_reference(tokens, *index, name)
                    || is_codec_implementation_function_reference(tokens, *index, name))
        })
        .map(|(index, _)| index)
        .collect()
}

fn qualified_reference_token_indices(tokens: &[Token], module: &str, name: &str) -> Vec<usize> {
    let module_segments = module.split("::").collect::<Vec<_>>();
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == name
                && is_call_target_token(tokens, *index)
                && qualified_reference_matches(tokens, *index, &module_segments)
        })
        .map(|(index, _)| index)
        .collect()
}

fn function_scopes(tokens: &[Token], function_values: &BTreeSet<String>) -> Vec<FunctionScope> {
    let mut scopes = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.kind, TokenKind::Fn | TokenKind::Test) {
            continue;
        }
        let Some(body_start) = tokens[index..]
            .iter()
            .find(|token| token.kind == TokenKind::Newline)
            .map(|token| token.range.end)
        else {
            continue;
        };
        let end = function_scope_end(tokens, index + 1).unwrap_or(body_start);
        let params = parameter_bindings(tokens, index, body_start);
        let result_binding = result_binding_name(tokens, index, body_start);
        let local_bindings = local_bindings(tokens, body_start, end, &params, function_values);
        scopes.push(FunctionScope {
            body_start,
            end,
            params,
            result_binding,
            local_bindings,
        });
    }
    scopes
}

impl FunctionScope {
    fn shadows(&self, name: &str, tokens: &[Token], index: usize) -> bool {
        let offset = tokens[index].range.start;
        self.params.contains_key(name)
            || self
                .result_binding
                .as_deref()
                .is_some_and(|binding| binding == name && is_ensure_reference(tokens, index))
            || self.local_bindings.iter().any(|binding| {
                binding.name == name && binding.start <= offset && offset < binding.end
            })
    }

    fn callable_shadows(&self, name: &str, tokens: &[Token], index: usize) -> bool {
        let offset = tokens[index].range.start;
        self.params.get(name).copied().unwrap_or(false)
            || self.local_bindings.iter().any(|binding| {
                binding.callable
                    && binding.name == name
                    && binding.start <= offset
                    && offset < binding.end
            })
    }
}

fn function_scope_end(tokens: &[Token], start: usize) -> Option<usize> {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[start..].iter().enumerate() {
        let index = start + relative_index;
        match token.kind {
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return Some(token.range.start),
            TokenKind::End => nested_blocks -= 1,
            TokenKind::Eof => return None,
            _ => {}
        }
    }
    None
}

fn parameter_bindings(tokens: &[Token], start: usize, body_start: usize) -> BTreeMap<String, bool> {
    let mut names = BTreeMap::new();
    let mut depth = 0usize;
    let mut pending_parameter_name: Option<String> = None;
    let mut pending_type_start: Option<usize> = None;
    for (relative_index, token) in tokens[start..]
        .iter()
        .enumerate()
        .take_while(|(_, token)| token.range.start < body_start)
    {
        let index = start + relative_index;
        match token.kind {
            TokenKind::LParen => {
                depth += 1;
                if depth == 1 {
                    pending_parameter_name = None;
                    pending_type_start = None;
                }
            }
            TokenKind::RParen => {
                if depth == 1 {
                    insert_pending_parameter(
                        tokens,
                        &mut names,
                        pending_parameter_name.take(),
                        pending_type_start.take(),
                        index,
                    );
                }
                depth = depth.saturating_sub(1);
            }
            TokenKind::Comma if depth == 1 => {
                insert_pending_parameter(
                    tokens,
                    &mut names,
                    pending_parameter_name.take(),
                    pending_type_start.take(),
                    index,
                );
            }
            TokenKind::Ident if depth == 1 && pending_parameter_name.is_none() => {
                pending_parameter_name = Some(token.text.clone());
            }
            TokenKind::Colon if depth == 1 && pending_parameter_name.is_some() => {
                pending_type_start = Some(index + 1);
            }
            _ => {}
        }
    }
    names
}

fn insert_pending_parameter(
    tokens: &[Token],
    names: &mut BTreeMap<String, bool>,
    name: Option<String>,
    type_start: Option<usize>,
    end: usize,
) {
    let Some(name) = name else {
        return;
    };
    let callable = type_start.is_some_and(|start| type_range_contains_fn(tokens, start, end));
    names.insert(name, callable);
}

fn result_binding_name(tokens: &[Token], start: usize, body_start: usize) -> Option<String> {
    let arrow_index = tokens[start..]
        .iter()
        .position(|token| token.kind == TokenKind::Arrow)
        .map(|index| start + index)?;
    if tokens[arrow_index].range.start >= body_start {
        return None;
    }
    let candidate_index = next_non_layout_index(tokens, arrow_index)?;
    let candidate = &tokens[candidate_index];
    if candidate.kind != TokenKind::Ident || !is_identifier(&candidate.text) {
        return None;
    }
    next_non_layout_token(tokens, candidate_index)
        .is_some_and(|next| next.kind == TokenKind::Colon)
        .then(|| candidate.text.clone())
}

fn local_bindings(
    tokens: &[Token],
    body_start: usize,
    end: usize,
    params: &BTreeMap<String, bool>,
    function_values: &BTreeSet<String>,
) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    let mut callable_values = params
        .iter()
        .filter_map(|(name, callable)| callable.then_some(name.clone()))
        .collect::<BTreeSet<_>>();
    callable_values.extend(function_values.iter().cloned());
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= end
            || token.kind != TokenKind::Let
        {
            continue;
        }
        let binding_end = local_binding_scope_end(tokens, index, end);
        let binding_start = let_binding_scope_start(tokens, index);
        let callable = let_binding_is_callable(tokens, index, &callable_values);
        let names = let_binding_names(tokens, index);
        for name in names {
            if callable {
                callable_values.insert(name.clone());
            }
            bindings.push(LocalBinding {
                name,
                start: binding_start,
                end: binding_end,
                callable,
            });
        }
    }
    bindings.extend(match_arm_pattern_binding_names(tokens, body_start, end));
    bindings.extend(satisfy_candidate_binding_names(tokens, body_start, end));
    bindings
}

fn let_binding_is_callable(
    tokens: &[Token],
    let_index: usize,
    callable_values: &BTreeSet<String>,
) -> bool {
    let line_end = tokens[let_index + 1..]
        .iter()
        .position(|token| token.kind == TokenKind::Newline || token.kind == TokenKind::Eof)
        .map_or(tokens.len(), |relative| let_index + 1 + relative);
    if let Some(colon_index) = tokens[let_index + 1..line_end]
        .iter()
        .position(|token| token.kind == TokenKind::Colon)
        .map(|relative| let_index + 1 + relative)
        && let Some(equal_index) = tokens[colon_index + 1..line_end]
            .iter()
            .position(|token| token.kind == TokenKind::Equal)
            .map(|relative| colon_index + 1 + relative)
        && type_range_contains_fn(tokens, colon_index + 1, equal_index)
    {
        return true;
    }
    let Some(equal_index) = tokens[let_index + 1..line_end]
        .iter()
        .position(|token| token.kind == TokenKind::Equal)
        .map(|relative| let_index + 1 + relative)
    else {
        return false;
    };
    next_non_layout_index_before(tokens, equal_index, line_end).is_some_and(|value_index| {
        tokens[value_index].kind == TokenKind::Ident
            && callable_values.contains(&tokens[value_index].text)
    })
}

fn type_range_contains_fn(tokens: &[Token], start: usize, end: usize) -> bool {
    tokens[start..end]
        .iter()
        .any(|token| token.kind == TokenKind::Fn)
}

fn let_binding_scope_start(tokens: &[Token], let_index: usize) -> usize {
    tokens[let_index + 1..]
        .iter()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .last()
        .map(|token| token.range.end)
        .unwrap_or_else(|| tokens[let_index].range.end)
}

fn local_binding_scope_end(tokens: &[Token], let_index: usize, function_end: usize) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[let_index + 1..].iter().enumerate() {
        let index = let_index + 1 + relative_index;
        if token.range.start >= function_end {
            break;
        }
        match token.kind {
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::Else if nested_blocks == 0 => return token.range.start,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks -= 1,
            _ => {}
        }
    }
    function_end
}

fn let_binding_names(tokens: &[Token], let_index: usize) -> Vec<String> {
    let mut names = let_pattern_binding_names(tokens, let_index)
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    if let Some(name) = simple_let_binding_name(tokens, let_index)
        && !names.iter().any(|existing| existing == &name)
    {
        names.push(name);
    }
    names
}

fn simple_let_binding_name(tokens: &[Token], let_index: usize) -> Option<String> {
    let token_index = next_non_layout_index(tokens, let_index)?;
    let token = &tokens[token_index];
    (token.kind == TokenKind::Ident
        && is_identifier(&token.text)
        && next_non_layout_token(tokens, token_index)
            .is_some_and(|next| matches!(next.kind, TokenKind::Colon | TokenKind::Equal)))
    .then(|| token.text.clone())
}

fn local_binding_shadows_name(
    tokens: &[Token],
    name: &str,
    offset: usize,
    scope_start: usize,
    scope_end: usize,
) -> bool {
    local_bindings(
        tokens,
        scope_start,
        scope_end,
        &BTreeMap::new(),
        &BTreeSet::new(),
    )
    .iter()
    .any(|binding| binding.name == name && offset >= binding.start && offset < binding.end)
}

fn handler_operation_clause_parameter_shadows_name(
    tokens: &[Token],
    name: &str,
    offset: usize,
    scope_start: usize,
    scope_end: usize,
) -> bool {
    if offset < scope_start || offset >= scope_end {
        return false;
    }
    let file_end = tokens.last().map_or(scope_end, |token| token.range.end);
    tokens.iter().enumerate().any(|(arrow_index, arrow)| {
        if arrow.kind != TokenKind::FatArrow
            || !is_handler_operation_clause_arrow(tokens, arrow_index)
        {
            return false;
        }
        let Some((lparen_index, rparen_index)) =
            handler_operation_clause_parameter_range(tokens, arrow_index)
        else {
            return false;
        };
        let body_end = handler_operation_clause_body_end(tokens, arrow_index, file_end);
        offset >= tokens[lparen_index].range.start
            && offset < body_end
            && handler_operation_clause_parameter_names_in_range(tokens, lparen_index, rparen_index)
                .contains(name)
    })
}

fn handler_operation_clause_parameter_range(
    tokens: &[Token],
    arrow_index: usize,
) -> Option<(usize, usize)> {
    let lparen_index = tokens[..arrow_index]
        .iter()
        .rposition(|token| token.kind == TokenKind::LParen)?;
    let rparen_index = tokens[lparen_index + 1..arrow_index]
        .iter()
        .position(|token| token.kind == TokenKind::RParen)
        .map(|index| lparen_index + 1 + index)?;
    Some((lparen_index, rparen_index))
}

fn handler_operation_clause_parameter_names_in_range(
    tokens: &[Token],
    lparen_index: usize,
    rparen_index: usize,
) -> BTreeSet<String> {
    tokens[lparen_index + 1..rparen_index]
        .iter()
        .filter(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        .map(|token| token.text.clone())
        .collect()
}

fn let_pattern_binding_names(tokens: &[Token], let_index: usize) -> Vec<(String, usize)> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut index = let_index + 1;
    while index < tokens.len() {
        let token = &tokens[index];
        if token.kind == TokenKind::Eof || token.kind == TokenKind::Newline {
            break;
        }
        if depth == 0 && matches!(token.kind, TokenKind::Colon | TokenKind::Equal) {
            break;
        }
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Ident if is_pattern_binding_token(tokens, index) => {
                names.push((token.text.clone(), token.range.end));
            }
            _ => {}
        }
        index += 1;
    }
    names
}

fn match_arm_pattern_binding_names(
    tokens: &[Token],
    body_start: usize,
    function_end: usize,
) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= function_end
            || token.kind != TokenKind::FatArrow
            || !inside_match(tokens, index, body_start)
        {
            continue;
        }
        let scope_start = token.range.end;
        let scope_end = match_arm_scope_end(tokens, index + 1, function_end);
        let pattern_start = match_arm_pattern_start(tokens, index, body_start);
        for name in pattern_binding_names_in_range(tokens, pattern_start, index) {
            bindings.push(LocalBinding {
                name,
                start: scope_start,
                end: scope_end,
                callable: false,
            });
        }
    }
    bindings
}

fn satisfy_candidate_binding_names(
    tokens: &[Token],
    body_start: usize,
    function_end: usize,
) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= function_end
            || token.kind != TokenKind::Ident
            || token.text != "satisfy"
        {
            continue;
        }
        let Some(candidate_index) = next_non_layout_index(tokens, index) else {
            continue;
        };
        let candidate = &tokens[candidate_index];
        if candidate.kind != TokenKind::Ident || !is_identifier(&candidate.text) {
            continue;
        }
        let Some(arrow_index) = next_non_layout_index(tokens, candidate_index) else {
            continue;
        };
        if tokens[arrow_index].kind != TokenKind::FatArrow {
            continue;
        }
        let end = tokens[arrow_index + 1..]
            .iter()
            .find(|token| token.kind == TokenKind::Newline || token.range.start >= function_end)
            .map(|token| token.range.start)
            .unwrap_or(function_end);
        bindings.push(LocalBinding {
            name: candidate.text.clone(),
            start: tokens[arrow_index].range.end,
            end,
            callable: false,
        });
    }
    bindings
}

fn inside_match(tokens: &[Token], index: usize, body_start: usize) -> bool {
    let mut nested_blocks = 0usize;
    for token in tokens[..index]
        .iter()
        .rev()
        .take_while(|token| token.range.start >= body_start)
    {
        match token.kind {
            TokenKind::End => nested_blocks += 1,
            TokenKind::Match if nested_blocks == 0 => return true,
            TokenKind::If | TokenKind::Handler | TokenKind::Match => {
                nested_blocks = nested_blocks.saturating_sub(1);
            }
            _ => {}
        }
    }
    false
}

fn match_arm_scope_end(tokens: &[Token], start: usize, function_end: usize) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[start..].iter().enumerate() {
        let index = start + relative_index;
        if token.range.start >= function_end {
            break;
        }
        match token.kind {
            TokenKind::If | TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks -= 1,
            TokenKind::FatArrow if nested_blocks == 0 && !is_satisfy_arrow(tokens, index) => {
                return match_arm_pattern_start_from_arrow(tokens, token.range.start);
            }
            _ => {}
        }
    }
    function_end
}

fn match_arm_pattern_start(tokens: &[Token], arrow_index: usize, body_start: usize) -> usize {
    tokens[..arrow_index]
        .iter()
        .rev()
        .take_while(|token| token.range.start >= body_start)
        .find(|token| token.kind == TokenKind::Newline || token.kind == TokenKind::Match)
        .map_or(body_start, |token| token.range.end)
}

fn match_arm_pattern_start_from_arrow(tokens: &[Token], arrow_start: usize) -> usize {
    tokens
        .iter()
        .position(|token| token.range.start == arrow_start)
        .map_or(arrow_start, |index| {
            match_arm_pattern_start(tokens, index, 0)
        })
}

fn pattern_binding_names_in_range(tokens: &[Token], start: usize, end_index: usize) -> Vec<String> {
    tokens[..end_index]
        .iter()
        .enumerate()
        .filter(|(_, token)| token.range.start >= start)
        .filter(|(index, token)| {
            token.kind == TokenKind::Ident && is_pattern_binding_token(tokens, *index)
        })
        .map(|(_, token)| token.text.clone())
        .collect()
}

fn is_pattern_binding_token(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.kind == TokenKind::Ident
        && is_identifier(&token.text)
        && token.text != "true"
        && token.text != "false"
        && previous_non_layout_token(tokens, index)
            .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && next_non_layout_token(tokens, index)
            .is_none_or(|next| !matches!(next.kind, TokenKind::DoubleColon | TokenKind::Colon))
}

fn is_else_if(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| previous.kind == TokenKind::Else)
}

fn token_scope(scopes: &[FunctionScope], offset: usize) -> Option<&FunctionScope> {
    scopes
        .iter()
        .find(|scope| offset >= scope.body_start && offset < scope.end)
}

fn is_function_declaration_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| matches!(previous.kind, TokenKind::Fn | TokenKind::Test))
}

fn is_parameter_name(tokens: &[Token], index: usize) -> bool {
    next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
}

fn is_local_binding_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index).is_some_and(|previous| previous.kind == TokenKind::Let)
        || is_let_pattern_binding_name(tokens, index)
        || is_match_arm_pattern_binding_name(tokens, index)
        || is_satisfy_candidate_binding_name(tokens, index)
}

fn is_let_pattern_binding_name(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    if token.kind != TokenKind::Ident {
        return false;
    }
    let Some(let_index) = tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .take_while(|(_, token)| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .find_map(|(previous_index, token)| {
            (token.kind == TokenKind::Let).then_some(previous_index)
        })
    else {
        return false;
    };
    let_pattern_binding_names(tokens, let_index)
        .iter()
        .any(|(name, start)| name == &token.text && *start == token.range.end)
}

fn is_match_arm_pattern_binding_name(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.kind == TokenKind::Ident
        && tokens[index + 1..]
            .iter()
            .take_while(|next| next.kind != TokenKind::Newline && next.kind != TokenKind::Eof)
            .any(|next| next.kind == TokenKind::FatArrow)
        && is_pattern_binding_token(tokens, index)
}

fn is_satisfy_candidate_binding_name(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "satisfy")
        && next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::FatArrow)
}

fn is_satisfy_arrow(tokens: &[Token], index: usize) -> bool {
    let Some(candidate_index) = previous_non_layout_index(tokens, index) else {
        return false;
    };
    if tokens[candidate_index].kind != TokenKind::Ident {
        return false;
    }
    previous_non_layout_token(tokens, candidate_index)
        .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "satisfy")
}

fn is_field_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index).is_some_and(|previous| previous.kind == TokenKind::Dot)
        || next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
}

fn is_ensure_reference(tokens: &[Token], index: usize) -> bool {
    tokens[..index]
        .iter()
        .rev()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .any(|token| token.kind == TokenKind::Ensure)
}

fn is_function_alias_target_reference(tokens: &[Token], index: usize, name: &str) -> bool {
    tokens[index].text == name
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Equal)
        && tokens[..index]
            .iter()
            .rev()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .any(|token| token.kind == TokenKind::Fn)
}

fn is_codec_implementation_function_reference(tokens: &[Token], index: usize, name: &str) -> bool {
    tokens[index].text == name
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "with")
        && inside_codec_declaration(tokens, index)
}

fn is_call_target_token(tokens: &[Token], index: usize) -> bool {
    next_non_whitespace_token(tokens, index).is_some_and(|next| next.kind == TokenKind::LParen)
}

fn is_handler_operation_clause_call_target(tokens: &[Token], index: usize) -> bool {
    is_call_target_token(tokens, index)
        && inside_handler_operation_clause_body(tokens, tokens[index].range.start)
}

fn is_handler_operation_clause_operation_name(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index)
        .is_some_and(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        && tokens[index + 1..]
            .iter()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .position(|token| token.kind == TokenKind::FatArrow)
            .map(|relative_index| index + 1 + relative_index)
            .is_some_and(|arrow_index| {
                is_handler_operation_clause_arrow(tokens, arrow_index)
                    && line_tokens_before(tokens, arrow_index)
                        .iter()
                        .position(|token| {
                            !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline)
                        })
                        .is_some_and(|first_index| {
                            let line_start = line_start_index(tokens, arrow_index);
                            line_start + first_index == index
                        })
            })
}

fn inside_handler_operation_clause_body(tokens: &[Token], offset: usize) -> bool {
    let file_end = tokens.last().map_or(offset, |token| token.range.end);
    tokens.iter().enumerate().any(|(arrow_index, arrow)| {
        arrow.kind == TokenKind::FatArrow
            && is_handler_operation_clause_arrow(tokens, arrow_index)
            && offset >= arrow.range.end
            && offset < handler_operation_clause_body_end(tokens, arrow_index, file_end)
    })
}

fn is_handler_operation_clause_arrow(tokens: &[Token], arrow_index: usize) -> bool {
    if !inside_top_level_block(tokens, arrow_index, TokenKind::Handler) {
        return false;
    }
    let line_tokens = line_tokens_before(tokens, arrow_index);
    line_tokens
        .iter()
        .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .is_some_and(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        && line_tokens
            .iter()
            .any(|token| token.kind == TokenKind::LParen)
        && line_tokens
            .iter()
            .any(|token| token.kind == TokenKind::RParen)
}

fn line_tokens_before(tokens: &[Token], index: usize) -> &[Token] {
    &tokens[line_start_index(tokens, index)..index]
}

fn next_non_whitespace_token(tokens: &[Token], index: usize) -> Option<&Token> {
    tokens[index + 1..]
        .iter()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .find(|token| token.kind != TokenKind::Whitespace)
}

fn inside_codec_declaration(tokens: &[Token], index: usize) -> bool {
    inside_top_level_block(tokens, index, TokenKind::Codec)
}

fn inside_top_level_block(tokens: &[Token], index: usize, start_kind: TokenKind) -> bool {
    enclosing_top_level_block_index(tokens, index, start_kind).is_some()
}

fn enclosing_top_level_block_index(
    tokens: &[Token],
    index: usize,
    start_kind: TokenKind,
) -> Option<usize> {
    let mut nested_blocks = 0usize;
    for (candidate_index, token) in tokens[..index].iter().enumerate().rev() {
        match token.kind {
            TokenKind::End => nested_blocks += 1,
            kind if kind == start_kind && nested_blocks == 0 => return Some(candidate_index),
            TokenKind::Fn
            | TokenKind::Test
            | TokenKind::If
            | TokenKind::Match
            | TokenKind::Handler
            | TokenKind::Codec => nested_blocks = nested_blocks.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn identifier_token_at(tokens: &[Token], offset: usize) -> Option<(usize, &Token)> {
    tokens.iter().enumerate().find(|(_, token)| {
        token.kind == TokenKind::Ident
            && offset >= token.range.start
            && offset < token.range.end
            && is_identifier(&token.text)
    })
}

fn qualifier_for_token(tokens: &[Token], name_index: usize) -> Option<String> {
    let separator_index = previous_non_layout_index(tokens, name_index)?;
    if tokens[separator_index].kind != TokenKind::DoubleColon {
        return None;
    }
    let segment_index = previous_non_layout_index(tokens, separator_index)?;
    let mut segments = vec![tokens[segment_index].text.as_str()];
    let mut cursor = segment_index;
    while let Some(previous_separator) = previous_non_layout_index(tokens, cursor) {
        if tokens[previous_separator].kind != TokenKind::DoubleColon {
            break;
        }
        let Some(previous_segment) = previous_non_layout_index(tokens, previous_separator) else {
            break;
        };
        segments.push(tokens[previous_segment].text.as_str());
        cursor = previous_segment;
    }
    segments.reverse();
    Some(segments.join("::"))
}

fn qualified_reference_matches(
    tokens: &[Token],
    name_index: usize,
    module_segments: &[&str],
) -> bool {
    let mut expected_index = name_index;
    for expected_segment in module_segments.iter().rev() {
        let Some(separator_index) = previous_non_layout_index(tokens, expected_index) else {
            return false;
        };
        if tokens[separator_index].kind != TokenKind::DoubleColon {
            return false;
        }
        let Some(segment_index) = previous_non_layout_index(tokens, separator_index) else {
            return false;
        };
        if tokens[segment_index].text != *expected_segment {
            return false;
        }
        expected_index = segment_index;
    }
    previous_non_layout_token(tokens, expected_index)
        .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
}

fn next_non_layout_token(tokens: &[Token], index: usize) -> Option<&Token> {
    next_non_layout_index(tokens, index).map(|index| &tokens[index])
}

fn next_non_layout_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[index + 1..]
        .iter()
        .position(|token| !is_layout_token(token))
        .map(|relative_index| index + 1 + relative_index)
}

fn next_non_layout_index_before(tokens: &[Token], index: usize, end: usize) -> Option<usize> {
    tokens[index + 1..end]
        .iter()
        .position(|token| !is_layout_token(token))
        .map(|relative_index| index + 1 + relative_index)
}

fn previous_non_layout_token(tokens: &[Token], index: usize) -> Option<&Token> {
    let previous = previous_non_layout_index(tokens, index)?;
    Some(&tokens[previous])
}

fn previous_non_layout_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, token)| !is_layout_token(token))
        .map(|(index, _)| index)
}

fn is_layout_token(token: &Token) -> bool {
    is_layout_token_kind(token.kind)
}

fn is_layout_token_kind(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Whitespace | TokenKind::Newline)
}

fn explicit_module_name(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("mod ")?;
        leading_module_path(rest).map(str::to_string)
    })
}

fn module_name_from_path(path: &str) -> Option<String> {
    Some(path.strip_suffix(".veln")?.replace('/', "::"))
}

fn use_modules(text: &str) -> (BTreeSet<String>, BTreeSet<(String, String)>) {
    let mut local = BTreeSet::new();
    let mut external = BTreeSet::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("use ") else {
            continue;
        };
        let Some(module) = leading_module_path(rest) else {
            continue;
        };
        let suffix = rest[module.len()..].trim();
        if let Some(package) = suffix
            .strip_prefix("from ")
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.split_once('"').map(|(package, _)| package))
        {
            external.insert((module.to_string(), package.to_string()));
        } else {
            local.insert(module.to_string());
        }
    }
    (local, external)
}

fn workspace_location(span: SourceSpan) -> NavigationLocation {
    NavigationLocation {
        source: NavigationSource::Workspace,
        span,
    }
}

fn leading_module_path(input: &str) -> Option<&str> {
    let end = input
        .char_indices()
        .take_while(|(_, ch)| is_identifier_char(*ch) || *ch == ':')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&input[..end])
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(is_identifier_start) && chars.all(is_identifier_char)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_char(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

fn offset_for_position(text: &str, position: &SourcePosition) -> Option<usize> {
    let line_start = line_start_offset(text, position.line.checked_sub(1)?)?;
    let line = text[line_start..]
        .split_once('\n')
        .map_or(&text[line_start..], |(line, _)| line);
    let offset = line
        .char_indices()
        .nth(position.column.checked_sub(1)?)
        .map(|(index, _)| line_start + index)
        .unwrap_or(line_start + line.len());
    Some(offset)
}

fn line_start_offset(text: &str, zero_based_line: usize) -> Option<usize> {
    if zero_based_line == 0 {
        return Some(0);
    }
    let mut line = 0;
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            line += 1;
            if line == zero_based_line {
                return Some(index + 1);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use veln_project::{capture_package_snapshot, parse_manifest_text};

    use super::*;

    static NEXT_TEMP_ROOT: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn repeated_navigation_reuses_the_prepared_symbol_index() {
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            "fn identity(value: Int) -> Int\n  identity(value)\nend\n",
        )]);

        let first_index = snapshot.navigation_index();
        let first = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 4,
            },
        )
        .unwrap();
        let second_index = snapshot.navigation_index();
        let second = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 4,
            },
        )
        .unwrap();

        assert!(Arc::ptr_eq(&first_index, &second_index));
        assert_eq!(first, second);
    }

    #[test]
    fn function_definition_and_references_are_deterministic() {
        let result = query(
            vec![
                source(
                    "math.test.veln",
                    "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
                ),
                source(
                    "math.veln",
                    "fn increment(value: Int) -> Int\n  increment(value - 1)\nend\n",
                ),
            ],
            "math.test.veln",
            4,
            11,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_location(&result.definition, "math.veln", 1, 4);
        assert_eq!(
            locations(&result.references),
            [("math.test.veln", 4, 9), ("math.veln", 2, 3)]
        );
    }

    #[test]
    fn bare_function_references_scale_across_unrelated_constructor_names() {
        let size = 2000;
        let snapshot = dense_constructor_reference_snapshot(size);

        navigation_stats::reset();
        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 1,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.references.len(), size);
        assert_eq!(
            navigation_stats::constructor_candidates_considered(),
            0,
            "reference lookup scanned unrelated constructor candidates instead of using the name index"
        );
    }

    #[test]
    fn bare_function_references_skip_non_visible_same_named_constructors() {
        let size = 200;
        let snapshot = non_visible_same_named_constructor_reference_snapshot(size);

        navigation_stats::reset();
        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 1,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.references.len(), size);
        assert_eq!(
            navigation_stats::constructor_candidates_considered(),
            0,
            "reference lookup scanned same-named constructors that are not visible from the source file"
        );
    }

    #[test]
    fn constructor_reference_counter_observes_indexed_candidates() {
        let size = 2000;
        let snapshot = dense_constructor_call_snapshot(size);

        navigation_stats::reset();
        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: size + 9,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "main.veln", 2, 7);
        assert!(
            navigation_stats::constructor_candidates_considered() > 0,
            "constructor lookup bypassed the indexed candidate iterator"
        );
        assert!(
            navigation_stats::constructor_candidates_considered() < size / 10,
            "constructor lookup scanned unrelated constructor candidates instead of using the name index"
        );
    }

    #[test]
    fn ambiguous_same_scope_constructor_call_has_no_selected_symbol() {
        assert!(
            query(
                vec![source(
                    "main.veln",
                    concat!(
                        "type Left\n",
                        "  same(Int)\n",
                        "end\n\n",
                        "type Right\n",
                        "  same(Int)\n",
                        "end\n\n",
                        "fn main() -> Left\n",
                        "  same(1)\n",
                        "end\n",
                    ),
                )],
                "main.veln",
                10,
                4,
            )
            .is_none()
        );
    }

    #[test]
    fn ambiguous_current_module_constructor_call_does_not_fall_back_to_import() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "use model\n\n",
                    "type LocalToken\n",
                    "  byte(Int)\n",
                    "end\n\n",
                    "type OtherToken\n",
                    "  byte(Int)\n",
                    "end\n\n",
                    "pub fn main() -> LocalToken\n",
                    "  byte(1)\n",
                    "end\n\n",
                    "fn byte(value: Int) -> Int\n",
                    "  value\n",
                    "end\n",
                ),
            ),
            source(
                "model.veln",
                concat!("pub type ImportedToken\n", "  pub byte(Int)\n", "end\n"),
            ),
        ]);

        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 12,
                    column: 4,
                },
            )
            .is_none()
        );

        let function = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 15,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(function.selected_symbol.kind, SymbolKind::Function);
        assert!(function.references.is_empty());
    }

    #[test]
    fn bare_constructor_wins_over_same_named_function_call() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "type Shape\n",
                    "  same(Int)\n",
                    "end\n\n",
                    "fn same(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "fn main() -> Shape\n",
                    "  same(1)\n",
                    "end\n",
                ),
            )],
            "main.veln",
            10,
            4,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "main.veln", 2, 3);
        assert!(result.references.is_empty());
    }

    #[test]
    fn non_callable_local_bindings_do_not_shadow_constructor_call_navigation() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "type Token\n",
                "  pack(Int)\n",
                "end\n\n",
                "fn parameter_shadow(pack: Int) -> Token\n",
                "  pack(1)\n",
                "end\n\n",
                "fn local_shadow(value: Int) -> Token\n",
                "  let pack = value\n",
                "  pack(2)\n",
                "end\n",
            ),
        )];

        for (line, column) in [(6, 4), (11, 4)] {
            let result = query(sources.clone(), "main.veln", line, column).unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
            assert_location(&result.definition, "main.veln", 2, 3);
            assert!(result.references.is_empty());
        }
    }

    #[test]
    fn callable_local_function_value_shadows_constructor_call_navigation() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "type Token\n",
                "  pack(Int)\n",
                "end\n\n",
                "fn pack_value(value: Int) -> Int\n",
                "  value\n",
                "end\n\n",
                "fn local_shadow() -> Int\n",
                "  let pack = pack_value\n",
                "  pack(2)\n",
                "end\n",
            ),
        )];

        assert!(query(sources, "main.veln", 11, 4).is_none());
    }

    #[test]
    fn qualified_constructor_wins_over_same_named_function_call() {
        let result = query(
            vec![
                source(
                    "main.test.veln",
                    concat!(
                        "use main\n\n",
                        "test make_shape() -> main::Shape\n",
                        "  main::same(1)\n",
                        "end\n",
                    ),
                ),
                source(
                    "main.veln",
                    concat!(
                        "pub type Shape\n",
                        "  pub same(Int)\n",
                        "end\n\n",
                        "fn same(value: Int) -> Int\n",
                        "  value\n",
                        "end\n",
                    ),
                ),
            ],
            "main.test.veln",
            4,
            9,
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "main.veln", 2, 7);
        assert!(result.references.is_empty());
    }

    #[test]
    fn exact_companion_boundary_excludes_other_test_files() {
        let result = query(
            vec![
                source(
                    "other.test.veln",
                    "use math\n\ntest unrelated() -> Int\n  math::increment(2)\nend\n",
                ),
                source(
                    "math.veln",
                    "fn increment(value: Int) -> Int\n  value + 1\nend\n",
                ),
                source(
                    "math.test.veln",
                    "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
                ),
            ],
            "math.test.veln",
            4,
            11,
        )
        .unwrap();

        assert_eq!(locations(&result.references), [("math.test.veln", 4, 9)]);
    }

    #[test]
    fn handler_clause_binding_excludes_shadowing_patterns_and_fields() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "effect Choose\n",
                    "  pick(value: Bool) -> Int\n",
                    "end\n\n",
                    "handler choose() handles Choose\n",
                    "  pick(value) => match value\n",
                    "    true => value\n",
                    "    value => value\n",
                    "    false => record.value\n",
                    "  end\n",
                    "end\n",
                ),
            )],
            "main.veln",
            7,
            16,
        )
        .unwrap();

        assert_eq!(
            result.selected_symbol.kind,
            SymbolKind::HandlerOperationClauseParameter
        );
        assert_location(&result.definition, "main.veln", 6, 8);
        assert_eq!(
            locations(&result.references),
            [("main.veln", 6, 24), ("main.veln", 7, 13)]
        );
    }

    #[test]
    fn handler_context_binding_stays_in_clause_bodies() {
        let result = query(
            vec![source(
                "main.veln",
                concat!(
                    "effect Adjust\n",
                    "  amount(value: Int) -> Int\n",
                    "end\n\n",
                    "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                    "  amount(value) => callback(value)\n",
                    "end\n",
                ),
            )],
            "main.veln",
            6,
            22,
        )
        .unwrap();

        assert_eq!(
            result.selected_symbol.kind,
            SymbolKind::HandlerContextParameter
        );
        assert_location(&result.definition, "main.veln", 5, 16);
        assert_eq!(locations(&result.references), [("main.veln", 6, 20)]);
    }

    #[test]
    fn function_references_exclude_handler_local_callable_bindings() {
        let sources = vec![source(
            "main.veln",
            concat!(
                "fn callback(value: Int) -> Int\n",
                "  value\n",
                "end\n\n",
                "fn transform(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n\n",
                "effect Apply\n",
                "  run(transform: fn(Int) -> Int) -> Int\n",
                "end\n\n",
                "handler apply(callback: fn(Int) -> Int) handles Apply\n",
                "  run(transform) => callback(transform(1))\n",
                "end\n\n",
                "fn caller() -> Int\n",
                "  callback(1) + transform(2)\n",
                "end\n",
            ),
        )];

        let callback = query(sources.clone(), "main.veln", 1, 4).unwrap();
        assert_eq!(callback.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(locations(&callback.references), [("main.veln", 18, 3)]);

        let transform = query(sources, "main.veln", 5, 4).unwrap();
        assert_eq!(transform.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(locations(&transform.references), [("main.veln", 18, 17)]);
    }

    #[test]
    fn unsupported_positions_have_no_selected_symbol() {
        let sources = vec![source(
            "main.veln",
            "fn increment(value: Int) -> Int\n  value.field\n  \"increment()\"\n  # increment()\nend\n",
        )];

        for (line, column) in [(2, 9), (3, 5), (4, 5), (1, 1)] {
            assert!(query(sources.clone(), "main.veln", line, column).is_none());
        }
    }

    #[test]
    fn exported_direct_dependency_definition_has_virtual_location() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn increment(value: Int) -> Int\r\n  value + 1\r\nend\r\n",
            )],
            ["./math.veln"],
        );
        let result = dependency_query(dependency, "math::increment(1)").unwrap();

        assert_eq!(result.definition.span.file.as_str(), "math.veln");
        assert_eq!(result.definition.span.start.line, 1);
        assert_eq!(result.definition.span.start.column, 8);
        let NavigationSource::Package { uri } = &result.definition.source else {
            panic!("dependency definition did not use a package location");
        };
        assert!(uri.starts_with("veln-pkg:///example%2Fpkg/snapshot/"));
        assert!(uri.ends_with("/math.veln"));
        assert!(!uri.contains("veln-language-service-navigation"));
        assert!(result.references.is_empty());
    }

    #[test]
    fn standard_library_functions_resolve_through_implicit_and_explicit_imports() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    concat!(
                        "pub fn visible(value: Int) -> Int\n  value\nend\n\n",
                        "fn hidden(value: Int) -> Int\n  value\nend\n",
                    ),
                ),
                (
                    "api.veln",
                    "pub fn exported(value: Int) -> Int\n  value\nend\n",
                ),
                (
                    "private.veln",
                    "pub fn unavailable(value: Int) -> Int\n  value\nend\n",
                ),
            ],
            ["prelude.veln", "api.veln"],
        );
        let sources = vec![source(
            "main.veln",
            concat!(
                "use api from \"std\"\n\n",
                "pub fn main() -> Int\n",
                "  visible(1)\n",
                "  prelude::visible(1)\n",
                "  api::exported(1)\n",
                "end\n",
            ),
        )];
        let snapshot =
            EffectiveProjectSnapshot::new(sources).with_standard_library(standard_library);

        for (line, column, path, declaration_column) in [
            (4, 4, "prelude.veln", 8),
            (5, 12, "prelude.veln", 8),
            (6, 9, "api.veln", 8),
        ] {
            let result = navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line,
                    column,
                },
            )
            .unwrap();
            assert_eq!(result.definition.span.file.as_str(), path);
            assert_eq!(result.definition.span.start.column, declaration_column);
            let NavigationSource::Package { uri } = result.definition.source else {
                panic!("standard definition did not use a package location");
            };
            assert!(uri.starts_with("veln-pkg:///std/snapshot/"), "{uri}");
            assert!(uri.ends_with(path), "{uri}");
        }
    }

    #[test]
    fn workspace_function_wins_over_bare_standard_prelude_fallback() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn visible(value: Int) -> Int\n  value\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "fn visible(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n\n",
                "pub fn main() -> Int\n",
                "  visible(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 6,
                column: 4,
            },
        )
        .unwrap();

        assert_location(&result.definition, "main.veln", 1, 4);
    }

    #[test]
    fn standard_library_definition_requires_public_exported_visibility() {
        let standard_library = standard_library_snapshot(
            &[
                (
                    "prelude.veln",
                    "fn hidden(value: Int) -> Int\n  value\nend\n",
                ),
                (
                    "private.veln",
                    "pub fn unavailable(value: Int) -> Int\n  value\nend\n",
                ),
            ],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "use private from \"std\"\n\n",
                "pub fn main() -> Int\n",
                "  prelude::hidden(1)\n",
                "  private::unavailable(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        for (line, column) in [(4, 12), (5, 13)] {
            assert!(
                navigate(
                    &snapshot,
                    SourcePosition {
                        source: SourcePath::new("main.veln"),
                        line,
                        column,
                    },
                )
                .is_none()
            );
        }
    }

    #[test]
    fn standard_library_bare_prelude_fallback_respects_local_shadowing() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn byte(value: Int) -> Result<Byte, String>\n  prelude_builtin::byte(value)\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![source(
            "main.veln",
            concat!(
                "pub fn parameter_shadow(byte: fn(Int) -> Result<Byte, String>) -> Result<Byte, String>\n",
                "  byte(1)\n",
                "end\n\n",
                "pub fn local_shadow() -> Result<Byte, String>\n",
                "  let byte: fn(Int) -> Result<Byte, String> = prelude::byte\n",
                "  byte(1)\n",
                "end\n",
            ),
        )])
        .with_standard_library(standard_library);

        for (case, line, column) in [("parameter", 2, 4), ("local", 7, 4)] {
            assert!(
                navigate(
                    &snapshot,
                    SourcePosition {
                        source: SourcePath::new("main.veln"),
                        line,
                        column,
                    },
                )
                .is_none(),
                "accepted shadowed {case} call"
            );
        }
    }

    #[test]
    fn standard_library_bare_prelude_fallback_rejects_ambiguous_imports() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn vec_len(items: Vec<Int>) -> Int\n  0\nend\n",
            )],
            ["math.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn vec_len(items: Vec<A>) -> Int\n  prelude_builtin::vec_len(items)\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use math from \"example/pkg\"\n\n",
                    "pub fn main(items: Vec<Int>) -> Int\n",
                    "  vec_len(items)\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 4,
                    column: 4,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn imported_constructor_call_definition_wins_over_bare_prelude_fallback() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn byte(value: Int) -> Int\n  value\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "use model\n\n",
                    "pub fn main() -> Token\n",
                    "  byte(1)\n",
                    "end\n",
                ),
            ),
            source(
                "model.veln",
                concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
            ),
        ])
        .with_standard_library(standard_library);

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "model.veln", 2, 7);
    }

    #[test]
    fn reexported_constructor_call_definition_wins_over_bare_prelude_fallback() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn byte(value: Int) -> Int\n  value\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "use facade\n\n",
                    "pub fn bare() -> Token\n",
                    "  byte(1)\n",
                    "end\n\n",
                    "pub fn qualified() -> Token\n",
                    "  facade::byte(2)\n",
                    "end\n",
                ),
            ),
            source(
                "facade.veln",
                concat!("use model\n\n", "pub type Token = model::Token\n"),
            ),
            source(
                "model.veln",
                concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
            ),
        ])
        .with_standard_library(standard_library);

        for (line, column) in [(4, 4), (8, 11)] {
            let result = navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line,
                    column,
                },
            )
            .unwrap();

            assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
            assert_location(&result.definition, "model.veln", 2, 7);
        }
    }

    #[test]
    fn standard_library_bare_prelude_fallback_ignores_private_workspace_imports() {
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn byte(value: Int) -> Int\n  value\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::new(vec![
            source(
                "main.veln",
                concat!(
                    "use math\n\n",
                    "pub fn main() -> Int\n",
                    "  byte(1)\n",
                    "end\n",
                ),
            ),
            source("math.veln", "fn byte(value: Int) -> Int\n  0\nend\n"),
        ])
        .with_standard_library(standard_library);

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.definition.span.file.as_str(), "prelude.veln");
        let NavigationSource::Package { uri } = result.definition.source else {
            panic!("prelude definition did not use a package location");
        };
        assert!(uri.starts_with("veln-pkg:///std/snapshot/"), "{uri}");
    }

    #[test]
    fn standard_library_bare_prelude_fallback_rejects_same_module_package_imports() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn vec_len(items: Vec<Int>) -> Int\n  0\nend\n",
            )],
            ["math.veln"],
        );
        let standard_library = standard_library_snapshot(
            &[(
                "prelude.veln",
                "pub fn vec_len(items: Vec<A>) -> Int\n  prelude_builtin::vec_len(items)\nend\n",
            )],
            ["prelude.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "math.veln",
                concat!(
                    "use math from \"example/pkg\"\n\n",
                    "pub fn main(items: Vec<Int>) -> Int\n",
                    "  vec_len(items)\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        )
        .with_standard_library(standard_library);

        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("math.veln"),
                    line: 4,
                    column: 4,
                },
            )
            .is_none()
        );
    }

    #[test]
    fn dependency_definition_requires_exported_source_and_public_function() {
        let fixtures = [
            (
                "private declaration",
                "fn increment(value: Int) -> Int\n  value + 1\nend\n",
                vec!["math.veln"],
            ),
            (
                "unexported source",
                "pub fn increment(value: Int) -> Int\n  value + 1\nend\n",
                Vec::new(),
            ),
        ];

        for (case, source_text, exports) in fixtures {
            let dependency =
                dependency_snapshot("example/pkg", &[("math.veln", source_text)], exports);
            assert!(
                dependency_query(dependency, "math::increment(1)").is_none(),
                "accepted {case}"
            );
        }
    }

    #[test]
    fn dependency_definition_requires_exact_external_import() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn increment(value: Int) -> Int\n  value + 1\nend\n",
            )],
            ["math.veln"],
        );
        let cases = [
            (
                "missing import",
                "pub fn main() -> Int\n  increment(1)\nend\n",
                2,
                4,
            ),
            (
                "workspace unqualified same module",
                "module math\n\npub fn main() -> Int\n  increment(1)\nend\n",
                4,
                4,
            ),
            (
                "different package",
                "use math from \"other/pkg\"\n\npub fn main() -> Int\n  math::increment(1)\nend\n",
                4,
                10,
            ),
            (
                "different module",
                "use other from \"example/pkg\"\n\npub fn main() -> Int\n  other::increment(1)\nend\n",
                4,
                11,
            ),
        ];

        for (case, text, line, column) in cases {
            let result = navigate(
                &EffectiveProjectSnapshot::with_direct_dependencies(
                    vec![source("main.veln", text)],
                    vec![dependency.clone()],
                ),
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line,
                    column,
                },
            );
            assert!(result.is_none(), "accepted {case}");
        }
    }

    #[test]
    fn workspace_references_ignore_dependency_sources_with_matching_modules() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[(
                "math.veln",
                "pub fn increment(value: Int) -> Int\n  increment(value - 1)\nend\n",
            )],
            ["math.veln"],
        );

        let result = navigate(
            &EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source(
                    "math.veln",
                    "pub fn increment(value: Int) -> Int\n  value + 1\nend\n",
                )],
                vec![dependency],
            ),
            SourcePosition {
                source: SourcePath::new("math.veln"),
                line: 1,
                column: 8,
            },
        )
        .unwrap();

        assert_location(&result.definition, "math.veln", 1, 8);
        assert!(result.references.is_empty());
    }

    #[test]
    fn dependency_constructor_visibility_uses_origin_aware_source_keys() {
        let dependency = dependency_snapshot(
            "example/pkg",
            &[
                ("main.veln", "pub fn unrelated() -> Int\n  0\nend\n"),
                (
                    "wire.veln",
                    concat!("pub type Packet\n", "  pub target(Int)\n", "end\n"),
                ),
            ],
            ["main.veln", "wire.veln"],
        );
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source(
                "main.veln",
                concat!(
                    "use wire from \"example/pkg\"\n\n",
                    "fn target(value: Int) -> Int\n",
                    "  value\n",
                    "end\n\n",
                    "pub fn main() -> Packet\n",
                    "  target(1)\n",
                    "end\n",
                ),
            )],
            vec![dependency],
        );

        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 8,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(result.definition.span.file.as_str(), "wire.veln");
        let NavigationSource::Package { uri } = result.definition.source else {
            panic!("dependency constructor did not use a package location");
        };
        assert!(uri.ends_with("/wire.veln"), "{uri}");
    }

    #[test]
    fn direct_dependency_snapshot_derives_visibility_from_manifest() {
        let root = TempDependency::new(
            "example/pkg",
            &[("math.veln", "pub fn exposed() -> Int\n  1\nend\n")],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nname = \"example/pkg\"\n\n[lib]\nexports = [\"./math.veln\"]\n",
        );

        let dependency =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap();
        let result = dependency_query(dependency, "math::exposed()").unwrap();

        assert_eq!(result.definition.span.file.as_str(), "math.veln");
    }

    #[test]
    fn direct_dependency_snapshot_rejects_mismatched_manifest_identity() {
        let root = TempDependency::new(
            "other/pkg",
            &[("math.veln", "pub fn exposed() -> Int\n  1\nend\n")],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nname = \"other/pkg\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        );

        let error =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap_err();

        assert_eq!(
            error,
            DirectDependencySnapshotError::PackageNameMismatch {
                expected: "example/pkg".to_string(),
                actual: "other/pkg".to_string(),
            }
        );
    }

    #[test]
    fn direct_dependency_snapshot_rejects_manifest_without_package_name() {
        let root = TempDependency::new(
            "example/pkg",
            &[("math.veln", "pub fn exposed() -> Int\n  1\nend\n")],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nversion = \"0.1.0\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        );

        let error =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap_err();

        assert_eq!(error, DirectDependencySnapshotError::MissingPackageName);
    }

    #[test]
    fn dependency_virtual_sources_retain_nonexported_and_private_source_bytes() {
        let root = TempDependency::new(
            "example/pkg",
            &[
                ("public.veln", "pub fn exposed() -> Int\r\n  1\r\nend\r\n"),
                ("internal.veln", "fn hidden() -> Int\r\n  2\r\nend\r\n"),
            ],
        );
        let identity = PackageIdentity::new("example/pkg").unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let manifest = parse_manifest_text(
            "veln.toml",
            "[package]\nname = \"example/pkg\"\n\n[lib]\nexports = [\"public.veln\"]\n",
        );
        let dependency =
            DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest)
                .unwrap();
        let retained = dependency
            .virtual_sources
            .entries()
            .map(|entry| entry.uri().to_string())
            .collect::<Vec<_>>();
        let project = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![source("main.veln", "pub fn main() -> Int\n  0\nend\n")],
            vec![dependency],
        );

        assert_eq!(retained.len(), 2);
        for uri in retained {
            let expected = if uri.ends_with("/internal.veln") {
                b"fn hidden() -> Int\r\n  2\r\nend\r\n".as_slice()
            } else if uri.ends_with("/public.veln") {
                b"pub fn exposed() -> Int\r\n  1\r\nend\r\n".as_slice()
            } else {
                panic!("unexpected retained source URI {uri}");
            };
            assert_eq!(project.resolve_virtual_source(&uri), Some(expected));
        }
    }

    fn source(path: &str, text: &str) -> SourceFile {
        SourceFile::new(path, text)
    }

    fn query(
        sources: Vec<SourceFile>,
        source_path: &str,
        line: usize,
        column: usize,
    ) -> Option<NavigationResult> {
        navigate(
            &EffectiveProjectSnapshot::new(sources),
            SourcePosition {
                source: SourcePath::new(source_path),
                line,
                column,
            },
        )
    }

    fn assert_location(location: &NavigationLocation, path: &str, line: usize, column: usize) {
        assert_eq!(location.source, NavigationSource::Workspace);
        assert_eq!(location.span.file.as_str(), path);
        assert_eq!(
            (location.span.start.line, location.span.start.column),
            (line, column)
        );
    }

    fn locations(spans: &[SourceSpan]) -> Vec<(&str, usize, usize)> {
        spans
            .iter()
            .map(|span| (span.file.as_str(), span.start.line, span.start.column))
            .collect()
    }

    fn dependency_query(
        dependency: DirectDependencySnapshot,
        expression: &str,
    ) -> Option<NavigationResult> {
        let text =
            format!("use math from \"example/pkg\"\n\npub fn main() -> Int\n  {expression}\nend\n");
        navigate(
            &EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("main.veln", &text)],
                vec![dependency],
            ),
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 10,
            },
        )
    }

    fn dense_constructor_reference_snapshot(size: usize) -> EffectiveProjectSnapshot {
        let mut text =
            String::from("fn target(value: Int) -> Int\n  value\nend\n\npub type Token\n");
        for index in 0..size {
            text.push_str(&format!("  pub C{index}(Int)\n"));
        }
        text.push_str("end\n\npub fn caller() -> Int\n");
        for _ in 0..size {
            text.push_str("  target(0)\n");
        }
        text.push_str("end\n");
        EffectiveProjectSnapshot::new(vec![source("main.veln", &text)])
    }

    fn dense_constructor_call_snapshot(size: usize) -> EffectiveProjectSnapshot {
        let mut text = String::from("pub type Target\n  pub target(Int)\nend\n\npub type Noise\n");
        for index in 0..size {
            text.push_str(&format!("  pub C{index}(Int)\n"));
        }
        text.push_str("end\n\npub fn caller() -> Target\n  target(0)\nend\n");
        EffectiveProjectSnapshot::new(vec![source("main.veln", &text)])
    }

    fn non_visible_same_named_constructor_reference_snapshot(
        size: usize,
    ) -> EffectiveProjectSnapshot {
        let mut sources = Vec::new();
        let mut main =
            String::from("fn target(value: Int) -> Int\n  value\nend\n\nfn caller() -> Int\n");
        for _ in 0..size {
            main.push_str("  target(0)\n");
        }
        main.push_str("end\n");
        sources.push(source("main.veln", &main));
        for index in 0..size {
            sources.push(source(
                &format!("unused{index}.veln"),
                "type Token\n  target(Int)\nend\n",
            ));
        }
        EffectiveProjectSnapshot::new(sources)
    }

    #[test]
    fn reexported_constructor_visibility_index_does_not_scan_per_source() {
        let size = 400;
        let snapshot = reexported_constructor_visibility_snapshot(size);

        navigation_stats::reset();
        let result = navigate(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("caller0.veln"),
                line: 4,
                column: 4,
            },
        )
        .unwrap();

        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_location(&result.definition, "model.veln", 2, 7);
        assert!(
            navigation_stats::constructor_candidates_considered() < size / 20,
            "re-exported constructor visibility scanned constructor candidates once per source"
        );
    }

    fn reexported_constructor_visibility_snapshot(size: usize) -> EffectiveProjectSnapshot {
        let mut sources = vec![
            source(
                "facade.veln",
                concat!("use model\n\n", "pub type Token = model::Token\n"),
            ),
            source(
                "model.veln",
                concat!("pub type Token\n", "  pub target(Int)\n", "end\n"),
            ),
        ];
        for index in 0..size {
            sources.push(source(
                &format!("caller{index}.veln"),
                concat!(
                    "use facade\n\n",
                    "pub fn caller() -> Token\n",
                    "  target(0)\n",
                    "end\n",
                ),
            ));
        }
        EffectiveProjectSnapshot::new(sources)
    }

    fn dependency_snapshot(
        identity: &str,
        sources: &[(&str, &str)],
        exports: impl IntoIterator<Item = &'static str>,
    ) -> DirectDependencySnapshot {
        let root = TempDependency::new(identity, sources);
        let identity = PackageIdentity::new(identity).unwrap();
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let exports = exports
            .into_iter()
            .map(|export| format!("\"{export}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let manifest = parse_manifest_text(
            "veln.toml",
            &format!(
                "[package]\nname = \"{}\"\n\n[lib]\nexports = [{}]\n",
                identity.as_str(),
                exports,
            ),
        );
        DirectDependencySnapshot::from_validated_manifest(&identity, snapshot, manifest).unwrap()
    }

    fn standard_library_snapshot(
        sources: &[(&str, &str)],
        exports: impl IntoIterator<Item = &'static str>,
    ) -> DirectDependencySnapshot {
        let root = TempDependency::new("std", sources);
        let snapshot = capture_package_snapshot(&root.path).unwrap();
        let exports = exports
            .into_iter()
            .map(|export| format!("\"{export}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let manifest = parse_manifest_text(
            "veln.toml",
            &format!("[package]\nname = \"std\"\n\n[lib]\nexports = [{exports}]\n"),
        );
        DirectDependencySnapshot::from_validated_standard_library(snapshot, manifest).unwrap()
    }

    struct TempDependency {
        path: PathBuf,
    }

    impl TempDependency {
        fn new(identity: &str, sources: &[(&str, &str)]) -> Self {
            let id = NEXT_TEMP_ROOT.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "veln-language-service-navigation-{}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            fs::write(
                path.join("veln.toml"),
                format!("[package]\nname = \"{identity}\"\n"),
            )
            .unwrap();
            for (relative, text) in sources {
                let source_path = path.join(relative);
                if let Some(parent) = source_path.parent() {
                    fs::create_dir_all(parent).unwrap();
                }
                fs::write(source_path, text).unwrap();
            }
            Self { path }
        }
    }

    impl Drop for TempDependency {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
