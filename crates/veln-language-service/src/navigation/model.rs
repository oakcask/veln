pub struct SourcePosition {
    pub source: SourcePath,
    /// One-based source line.
    pub line: usize,
    /// One-based Unicode-scalar source column.
    pub column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Type,
    Function,
    Constructor,
    HandlerContextParameter,
    HandlerOperationClauseParameter,
}

impl SymbolKind {
    pub fn rename_name_class(&self) -> RenameNameClass {
        match self {
            Self::Type => RenameNameClass::Type,
            Self::Constructor => RenameNameClass::Constructor,
            Self::Function => RenameNameClass::Function,
            Self::HandlerContextParameter | Self::HandlerOperationClauseParameter => {
                RenameNameClass::ValueBinding
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameNameClass {
    Type,
    Constructor,
    Function,
    ValueBinding,
}

impl RenameNameClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Constructor => "constructor",
            Self::Function => "function",
            Self::ValueBinding => "value_binding",
        }
    }

    pub fn required_initial(self) -> RenameRequiredInitial {
        match self {
            Self::Type | Self::Constructor => RenameRequiredInitial::AsciiUppercase,
            Self::Function | Self::ValueBinding => RenameRequiredInitial::AsciiLowercase,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameRequiredInitial {
    AsciiUppercase,
    AsciiLowercase,
}

impl RenameRequiredInitial {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AsciiUppercase => "ascii_uppercase",
            Self::AsciiLowercase => "ascii_lowercase",
        }
    }

    fn accepts(self, name: &str) -> bool {
        let Some(initial) = name.chars().next() else {
            return false;
        };
        match self {
            Self::AsciiUppercase => initial.is_ascii_uppercase(),
            Self::AsciiLowercase => initial.is_ascii_lowercase(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenameFailure {
    pub code: &'static str,
    pub symbol_class: RenameNameClass,
    pub requested_name: String,
    pub kind: RenameFailureKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameFailureKind {
    InvalidCase {
        required_initial: RenameRequiredInitial,
    },
    Conflict {
        conflicting_declaration: Box<NavigationLocation>,
        affected_scope: Box<RenameAffectedScope>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenameAffectedScope {
    Module {
        name: String,
    },
    Lexical {
        file: String,
        start_offset: usize,
        end_offset: usize,
    },
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
    pub classified_path_segment: Option<QualifiedPathSegment>,
    pub definition: NavigationLocation,
    pub references: Vec<SourceSpan>,
}

pub fn validate_rename(
    result: &NavigationResult,
    requested_name: &str,
) -> Result<(), RenameFailure> {
    let symbol_class = result.selected_symbol.kind.rename_name_class();
    let required_initial = symbol_class.required_initial();
    if !required_initial.accepts(requested_name) {
        return Err(RenameFailure {
            code: "rename.invalid_case",
            symbol_class,
            requested_name: requested_name.to_string(),
            kind: RenameFailureKind::InvalidCase { required_initial },
        });
    }
    Ok(())
}

pub fn validate_rename_in_snapshot(
    snapshot: &EffectiveProjectSnapshot,
    result: &NavigationResult,
    requested_name: &str,
) -> Result<(), RenameFailure> {
    validate_rename(result, requested_name)?;
    let symbol_class = result.selected_symbol.kind.rename_name_class();
    if requested_name == result.selected_symbol.name {
        return Ok(());
    }
    if let Some((conflicting_declaration, affected_scope)) = snapshot
        .navigation_index()
        .rename_conflict(result, requested_name)
    {
        return Err(RenameFailure {
            code: "rename.conflict",
            symbol_class,
            requested_name: requested_name.to_string(),
            kind: RenameFailureKind::Conflict {
                conflicting_declaration: Box::new(conflicting_declaration),
                affected_scope: Box::new(affected_scope),
            },
        });
    }
    Ok(())
}

pub fn navigate(
    snapshot: &EffectiveProjectSnapshot,
    position: SourcePosition,
) -> Option<NavigationResult> {
    let request = snapshot
        .navigation_index()
        .symbol_at_position(position.source.as_str(), &position)?;
    let definition = request.symbol.definition();
    let selected_symbol = request.symbol.selected_symbol(definition.clone());
    let mut references = request.symbol.references(&request.index);
    sort_locations(&mut references);
    Some(NavigationResult {
        selected_symbol,
        selection: request.selection,
        classified_path_segment: request.classified_path_segment,
        definition,
        references,
    })
}

impl Symbol {
    fn definition(&self) -> NavigationLocation {
        match self {
            Self::Type(symbol) => symbol.declaration.clone(),
            Self::Function(symbol) => symbol.declaration.clone(),
            Self::Constructor(symbol) => symbol.declaration.clone(),
            Self::Local(symbol) => workspace_location(symbol.declaration.clone()),
        }
    }

    fn selected_symbol(&self, declaration: NavigationLocation) -> SelectedSymbol {
        SelectedSymbol {
            kind: self.kind(),
            name: self.name().to_string(),
            declaration,
        }
    }

    fn kind(&self) -> SymbolKind {
        match self {
            Self::Type(_) => SymbolKind::Type,
            Self::Function(_) => SymbolKind::Function,
            Self::Constructor(_) => SymbolKind::Constructor,
            Self::Local(symbol) => symbol.kind.symbol_kind(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Type(symbol) => &symbol.name,
            Self::Function(symbol) => &symbol.name,
            Self::Constructor(symbol) => &symbol.name,
            Self::Local(symbol) => &symbol.name,
        }
    }

    fn references(&self, index: &SymbolIndex) -> Vec<SourceSpan> {
        match self {
            Self::Type(symbol) => index.type_references(symbol),
            Self::Function(symbol) => index.function_references(symbol),
            Self::Constructor(symbol) => index.constructor_references(symbol),
            Self::Local(symbol) => index.local_references(symbol, false),
        }
    }
}

impl LocalSymbolKind {
    fn symbol_kind(&self) -> SymbolKind {
        match self {
            Self::HandlerContextParameter => SymbolKind::HandlerContextParameter,
            Self::HandlerOperationClauseParameter => SymbolKind::HandlerOperationClauseParameter,
        }
    }
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
struct TypeSymbol {
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
struct ClassifiedNavigationSegment {
    segment: QualifiedPathSegment,
    symbol: Option<Symbol>,
}

impl ClassifiedNavigationSegment {
    fn into_selected_symbol(self) -> Option<SelectedNavigationSymbol> {
        let Self { segment, symbol } = self;
        debug_assert!(segment_role_matches_symbol(&segment, symbol.as_ref()?));
        Some(SelectedNavigationSymbol {
            symbol: symbol?,
            classified_path_segment: Some(segment),
        })
    }
}

#[derive(Debug)]
struct SelectedNavigationSymbol {
    symbol: Symbol,
    classified_path_segment: Option<QualifiedPathSegment>,
}

impl SelectedNavigationSymbol {
    fn bare(symbol: Symbol) -> Self {
        Self {
            symbol,
            classified_path_segment: None,
        }
    }
}

fn self_role_for_symbol(symbol: Option<&Symbol>) -> Option<NameClass> {
    match symbol? {
        Symbol::Type(_) => Some(NameClass::Type),
        Symbol::Function(_) => Some(NameClass::Function),
        Symbol::Constructor(_) => Some(NameClass::Constructor),
        Symbol::Local(_) => None,
    }
}

fn segment_role_matches_symbol(segment: &QualifiedPathSegment, symbol: &Symbol) -> bool {
    if self_role_for_symbol(Some(symbol)).is_some_and(|role| role == segment.role) {
        return true;
    }
    matches!(
        (segment.role, symbol),
        (NameClass::ValueBinding, Symbol::Function(_))
    )
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
    classified_path_segment: Option<QualifiedPathSegment>,
}

#[derive(Clone, Debug)]
enum Symbol {
    Type(TypeSymbol),
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
    tokens: Vec<Token>,
    module: String,
    companion_target_module: Option<String>,
    uses: BTreeSet<String>,
    external_uses: BTreeSet<(String, String)>,
    import_aliases: BTreeMap<String, String>,
    external_import_aliases: BTreeMap<String, (String, String)>,
    invalid_declaration_names: Vec<SourceSpan>,
    classified_path_segments: Vec<QualifiedPathSegment>,
    origin: IndexedOrigin,
}

#[derive(Default)]
struct FileDeclarations {
    functions: Vec<FunctionSymbol>,
    types: Vec<TypeSymbol>,
    constructors: Vec<ConstructorSymbol>,
    type_aliases: Vec<TypeAliasSymbol>,
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
pub(crate) struct SymbolIndex {
    files: Vec<IndexedFile>,
    functions: Vec<FunctionSymbol>,
    types: Vec<TypeSymbol>,
    constructors: Vec<ConstructorSymbol>,
    type_aliases: Vec<TypeAliasSymbol>,
}

#[derive(Debug)]
struct FunctionScope {
    body_start: usize,
    end: usize,
    params: BTreeSet<String>,
    result_binding: Option<String>,
    local_bindings: Vec<LocalBinding>,
}

#[derive(Debug)]
struct LocalBinding {
    name: String,
    declaration_start: usize,
    declaration_end: usize,
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct ClauseBinding {
    name: String,
    declaration: SourceSpan,
    start: usize,
    end: usize,
    kind: LocalSymbolKind,
}
