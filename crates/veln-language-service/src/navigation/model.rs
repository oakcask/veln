pub struct SourcePosition {
    pub source: SourcePath,
    /// One-based source line.
    pub line: usize,
    /// One-based Unicode-scalar source column.
    pub column: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Schema,
    Effect,
    Handler,
    EffectOperation,
    Type,
    Function,
    Constructor,
    ValueBinding,
    HandlerContextParameter,
    HandlerOperationClauseParameter,
}

impl SymbolKind {
    pub fn is_renamable(&self) -> bool {
        !matches!(
            self,
            Self::Schema | Self::Effect | Self::Handler | Self::EffectOperation
        )
    }

    pub fn rename_name_class(&self) -> RenameNameClass {
        match self {
            Self::Schema | Self::Effect | Self::Handler | Self::EffectOperation => {
                RenameNameClass::CasingNeutral
            }
            Self::Type => RenameNameClass::Type,
            Self::Constructor => RenameNameClass::Constructor,
            Self::Function => RenameNameClass::Function,
            Self::ValueBinding
            | Self::HandlerContextParameter
            | Self::HandlerOperationClauseParameter => {
                RenameNameClass::ValueBinding
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameNameClass {
    CasingNeutral,
    Type,
    Constructor,
    Function,
    ValueBinding,
}

impl RenameNameClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CasingNeutral => "casing_neutral",
            Self::Type => "type",
            Self::Constructor => "constructor",
            Self::Function => "function",
            Self::ValueBinding => "value_binding",
        }
    }

    pub fn required_initial(self) -> RenameRequiredInitial {
        match self {
            Self::CasingNeutral => RenameRequiredInitial::Any,
            Self::Type | Self::Constructor => RenameRequiredInitial::AsciiUppercase,
            Self::Function | Self::ValueBinding => RenameRequiredInitial::AsciiLowercase,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameRequiredInitial {
    Any,
    AsciiUppercase,
    AsciiLowercase,
}

impl RenameRequiredInitial {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::AsciiUppercase => "ascii_uppercase",
            Self::AsciiLowercase => "ascii_lowercase",
        }
    }

    fn accepts(self, name: &str) -> bool {
        let Some(initial) = name.chars().next() else {
            return false;
        };
        match self {
            Self::Any => true,
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
    pub declaration_kind: SymbolDeclarationKind,
    pub package_origin: Option<PackageOrigin>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NavigationSource {
    Workspace,
    Package { uri: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolDeclarationKind {
    Declaration,
    PublicAlias,
    Recovery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PackageOrigin {
    DirectDependency,
    StandardLibrary,
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
    pub reference_eligible: bool,
    pub is_recovery: bool,
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
    if !snapshot.navigation_index_is_prepared()
        && navigation_selection_is_unsupported(snapshot, &position)
    {
        return None;
    }
    let schema_candidate = schema_navigation_candidate(snapshot, &position);
    if !snapshot.navigation_index_is_prepared() && schema_candidate {
        if let Some(result) = navigate_in_index(
            snapshot.direct_dependency_schema_navigation_index(),
            &position,
        ) && result.selected_symbol.kind == SymbolKind::Schema
            && result.selected_symbol.package_origin == Some(PackageOrigin::DirectDependency)
        {
            return Some(result);
        }
        return navigate_in_index(snapshot.schema_navigation_index(), &position);
    }
    navigate_in_index(snapshot.navigation_index(), &position)
}

pub fn navigate_for_rename(
    snapshot: &EffectiveProjectSnapshot,
    position: SourcePosition,
) -> Option<NavigationResult> {
    let index = snapshot.navigation_index();
    if let Some((alias, selection)) = index.workspace_type_alias_for_rename(&position) {
        let definition = alias.declaration.clone();
        let selected_symbol = Symbol::TypeAlias(alias.clone()).selected_symbol(definition.clone());
        let mut references = index.workspace_type_alias_references(&alias);
        sort_locations(&mut references);
        return Some(NavigationResult {
            selected_symbol,
            selection,
            classified_path_segment: None,
            definition,
            references,
            reference_eligible: true,
            is_recovery: false,
        });
    }
    let mut result = navigate_in_index(Arc::clone(&index), &position)?;
    if let Some(symbol) = index.selected_function(&result)
        && symbol.declaration_kind == SymbolDeclarationKind::PublicAlias
    {
        result.references = index.workspace_function_alias_references(&symbol);
        sort_locations(&mut result.references);
    }
    Some(result)
}

fn navigation_selection_is_unsupported(
    snapshot: &EffectiveProjectSnapshot,
    position: &SourcePosition,
) -> bool {
    let Some(source) = snapshot.workspace_source(&position.source) else {
        return false;
    };
    let tokens = lex(source).tokens;
    let Some(offset) = offset_for_position(source.text(), position) else {
        return true;
    };
    let Some((token_index, _)) = identifier_token_at(&tokens, offset) else {
        return true;
    };
    line_tokens_before(&tokens, token_index)
        .iter()
        .any(|token| token.kind == TokenKind::Use)
}

fn schema_navigation_candidate(
    snapshot: &EffectiveProjectSnapshot,
    position: &SourcePosition,
) -> bool {
    let Some(source) = snapshot.workspace_source(&position.source) else {
        return false;
    };
    let Some(line) = source.text().lines().nth(position.line.saturating_sub(1)) else {
        return false;
    };
    let likely_operation =
        (line.contains("decode") || line.contains("encode")) && line.contains("from");
    if !likely_operation && !line.contains(':') {
        return false;
    }
    let tokens = lex(source).tokens;
    let Some(offset) = offset_for_position(source.text(), position) else {
        return false;
    };
    let inside_schema = position_inside_schema_declaration(source, position.line);
    let Some((token_index, _)) = identifier_token_at(&tokens, offset) else {
        return likely_operation || (line.contains(':') && inside_schema);
    };
    let mut leaf_index = token_index;
    while let Some(next) = next_path_segment_index(&tokens, leaf_index) {
        leaf_index = next;
    }
    is_schema_operation_path_leaf_candidate_token(&tokens, leaf_index)
        || (inside_schema && is_schema_composition_path_leaf_token(&tokens, leaf_index))
        || (line.contains(':')
            && (inside_schema_declaration(&tokens, leaf_index) || inside_schema))
}

fn position_inside_schema_declaration(source: &SourceFile, line: usize) -> bool {
    source
        .text()
        .lines()
        .take(line)
        .filter(|line| {
            !line.trim().is_empty()
                && !line.starts_with(char::is_whitespace)
                && !line.trim_start().starts_with('#')
        })
        .last()
        .is_some_and(|line| {
            line.starts_with("schema ") || line.starts_with("pub schema ")
        })
}

fn navigate_in_index(
    index: Arc<SymbolIndex>,
    position: &SourcePosition,
) -> Option<NavigationResult> {
    let request = index.symbol_at_position(position.source.as_str(), position)?;
    let definition = request.symbol.definition();
    let selected_symbol = request.symbol.selected_symbol(definition.clone());
    let reference_eligible = request.symbol.reference_eligible(&request.index);
    let mut references = if request.references_supported && reference_eligible {
        request.symbol.references(&request.index)
    } else {
        Vec::new()
    };
    sort_locations(&mut references);
    Some(NavigationResult {
        selected_symbol,
        selection: request.selection,
        classified_path_segment: request.classified_path_segment,
        definition,
        references,
        reference_eligible,
        is_recovery: request.symbol.is_recovery(),
    })
}

pub fn definition_at(
    snapshot: &EffectiveProjectSnapshot,
    position: SourcePosition,
) -> Option<NavigationLocation> {
    let index = snapshot.navigation_index();
    let request = index.symbol_at_position(position.source.as_str(), &position)?;
    request
        .symbol
        .definition_supported(&request.index)
        .then(|| request.symbol.definition())
}

impl Symbol {
    fn reference_eligible(&self, index: &SymbolIndex) -> bool {
        match self {
            Self::Effect(symbol) => index.effect_references_supported(symbol),
            Self::SchemaAlias(symbol) if symbol.package.is_none() => {
                index.workspace_schema_alias_is_eligible(
                    &symbol.module,
                    &symbol.name,
                    &symbol.declaration,
                )
            }
            _ => true,
        }
    }

    fn definition_supported(&self, index: &SymbolIndex) -> bool {
        match self {
            Self::SchemaAlias(_) => false,
            Self::TypeAlias(symbol) if symbol.package.is_some() => {
                index.type_alias_definition_supported(symbol)
            }
            _ => true,
        }
    }

    fn definition(&self) -> NavigationLocation {
        match self {
            Self::Schema(symbol) | Self::SchemaAlias(symbol) => symbol.declaration.clone(),
            Self::Effect(symbol) => symbol.declaration.clone(),
            Self::Handler(symbol) => symbol.declaration.clone(),
            Self::EffectOperation(symbol) => symbol.declaration.clone(),
            Self::Type(symbol) => symbol.declaration.clone(),
            Self::TypeAlias(symbol) => symbol.declaration.clone(),
            Self::Function(symbol) => symbol.declaration.clone(),
            Self::Constructor(symbol) => symbol.declaration.clone(),
            Self::Local(symbol) => workspace_location(symbol.declaration.clone()),
            Self::Recovery(symbol) => workspace_location(symbol.declaration.clone()),
        }
    }

    fn selected_symbol(&self, declaration: NavigationLocation) -> SelectedSymbol {
        SelectedSymbol {
            kind: self.kind(),
            name: self.name().to_string(),
            declaration,
            declaration_kind: self.declaration_kind(),
            package_origin: self.package_origin(),
        }
    }

    fn kind(&self) -> SymbolKind {
        match self {
            Self::Schema(_) | Self::SchemaAlias(_) => SymbolKind::Schema,
            Self::Effect(_) => SymbolKind::Effect,
            Self::Handler(_) => SymbolKind::Handler,
            Self::EffectOperation(_) => SymbolKind::EffectOperation,
            Self::Type(_) | Self::TypeAlias(_) => SymbolKind::Type,
            Self::Function(_) => SymbolKind::Function,
            Self::Constructor(_) => SymbolKind::Constructor,
            Self::Local(symbol) => symbol.kind.symbol_kind(),
            Self::Recovery(symbol) => symbol.kind,
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Schema(symbol) | Self::SchemaAlias(symbol) => &symbol.name,
            Self::Effect(symbol) => &symbol.name,
            Self::Handler(symbol) => &symbol.name,
            Self::EffectOperation(symbol) => &symbol.name,
            Self::Type(symbol) => &symbol.name,
            Self::TypeAlias(symbol) => &symbol.name,
            Self::Function(symbol) => &symbol.name,
            Self::Constructor(symbol) => &symbol.name,
            Self::Local(symbol) => &symbol.name,
            Self::Recovery(symbol) => &symbol.name,
        }
    }

    fn references(&self, index: &SymbolIndex) -> Vec<SourceSpan> {
        match self {
            Self::Schema(symbol) => index.schema_references(symbol),
            Self::SchemaAlias(symbol) => index.schema_alias_references(symbol),
            Self::Effect(symbol) => index.effect_references(symbol),
            Self::Handler(_) | Self::EffectOperation(_) => Vec::new(),
            Self::Type(symbol) => index.type_references(symbol),
            Self::TypeAlias(symbol) => index.type_alias_references(symbol),
            Self::Function(symbol) => index.function_references(symbol),
            Self::Constructor(symbol) => index.constructor_references(symbol),
            Self::Local(symbol) => index.local_references(symbol, false),
            Self::Recovery(symbol) => index.recovery_references(symbol),
        }
    }

    fn is_recovery(&self) -> bool {
        matches!(self, Self::Recovery(_))
    }

    fn declaration_kind(&self) -> SymbolDeclarationKind {
        match self {
            Self::Function(symbol) => symbol.declaration_kind,
            Self::SchemaAlias(_) | Self::TypeAlias(_) => SymbolDeclarationKind::PublicAlias,
            Self::Constructor(symbol) => symbol.declaration_kind,
            Self::Recovery(_) => SymbolDeclarationKind::Recovery,
            _ => SymbolDeclarationKind::Declaration,
        }
    }

    fn package_origin(&self) -> Option<PackageOrigin> {
        match self {
            Self::Schema(symbol) | Self::SchemaAlias(symbol) => symbol.package_origin,
            Self::Function(symbol) => symbol.package_origin,
            Self::Type(symbol) => symbol.package_origin,
            Self::TypeAlias(symbol) => symbol.package_origin,
            Self::Constructor(symbol) => symbol.package_origin,
            _ => None,
        }
    }
}

impl LocalSymbolKind {
    fn symbol_kind(&self) -> SymbolKind {
        match self {
            Self::ValueBinding => SymbolKind::ValueBinding,
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
    alias_target_module: Option<String>,
    alias_target_name: Option<String>,
    declaration: NavigationLocation,
    package: Option<String>,
    package_origin: Option<PackageOrigin>,
    public: bool,
    standard_prelude: bool,
    declaration_kind: SymbolDeclarationKind,
    invalid_declaration_name: bool,
}

#[derive(Clone, Debug)]
struct PackageFunctionTarget {
    module: String,
    name: String,
    package: String,
    package_origin: PackageOrigin,
}

#[derive(Clone, Debug)]
struct PackageSchemaTarget {
    module: String,
    name: String,
    package: String,
    package_origin: PackageOrigin,
    public: bool,
    exported: bool,
}

#[derive(Clone, Debug)]
struct PackageTypeTarget {
    module: String,
    name: String,
    package: String,
    package_origin: PackageOrigin,
}

#[derive(Clone, Debug)]
struct PackageConstructorTarget {
    module: String,
    type_name: String,
    name: String,
    package: String,
    package_origin: PackageOrigin,
}

#[derive(Clone, Debug)]
struct TypeSymbol {
    module: String,
    name: String,
    declaration: NavigationLocation,
    package: Option<String>,
    package_origin: Option<PackageOrigin>,
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
    package_origin: Option<PackageOrigin>,
    public: bool,
    standard_prelude: bool,
    declaration_kind: SymbolDeclarationKind,
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
        Symbol::Schema(_)
        | Symbol::SchemaAlias(_)
        | Symbol::Effect(_)
        | Symbol::Handler(_)
        | Symbol::EffectOperation(_) => {
            None
        }
        Symbol::Type(_) | Symbol::TypeAlias(_) => Some(NameClass::Type),
        Symbol::Function(_) => Some(NameClass::Function),
        Symbol::Constructor(_) => Some(NameClass::Constructor),
        Symbol::Recovery(symbol) => symbol.name_class(),
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
    declaration: NavigationLocation,
    target_module: Option<String>,
    target_name: String,
    package: Option<String>,
    package_origin: Option<PackageOrigin>,
    standard_prelude: bool,
}

#[derive(Clone, Debug)]
enum TypeConflictCandidate {
    Type(TypeSymbol),
    Alias(TypeAliasSymbol),
}

impl TypeConflictCandidate {
    fn declaration(&self) -> NavigationLocation {
        match self {
            Self::Type(symbol) => symbol.declaration.clone(),
            Self::Alias(symbol) => symbol.declaration.clone(),
        }
    }

    fn is_selected_type(&self, selected: &TypeSymbol) -> bool {
        match self {
            Self::Type(symbol) => same_type(symbol, selected),
            Self::Alias(symbol) => symbol.declaration == selected.declaration,
        }
    }
}

#[derive(Debug)]
struct SymbolRequest {
    index: Arc<SymbolIndex>,
    symbol: Symbol,
    selection: SourceSpan,
    classified_path_segment: Option<QualifiedPathSegment>,
    references_supported: bool,
}

#[derive(Clone, Debug)]
enum Symbol {
    Schema(NeutralSymbol),
    SchemaAlias(NeutralSymbol),
    Effect(NeutralSymbol),
    Handler(NeutralSymbol),
    EffectOperation(EffectOperationSymbol),
    Type(TypeSymbol),
    TypeAlias(TypeAliasSymbol),
    Function(FunctionSymbol),
    Constructor(ConstructorSymbol),
    Local(LocalSymbol),
    Recovery(RecoverySymbol),
}

#[derive(Clone, Debug)]
struct NeutralSymbol {
    module: String,
    name: String,
    declaration: NavigationLocation,
    package: Option<String>,
    package_origin: Option<PackageOrigin>,
    public: bool,
    standard_prelude: bool,
    alias_target_module: Option<String>,
    alias_target_name: Option<String>,
}

#[derive(Clone, Debug)]
struct EffectOperationSymbol {
    module: String,
    effect_name: String,
    name: String,
    declaration: NavigationLocation,
    package: Option<String>,
}

#[derive(Clone, Debug)]
struct LocalSymbol {
    name: String,
    declaration: SourceSpan,
    scope_file: String,
    scope_start: usize,
    scope_end: usize,
    declaration_scope_start: usize,
    declaration_scope_end: usize,
    kind: LocalSymbolKind,
}

#[derive(Clone, Debug)]
struct RecoverySymbol {
    name: String,
    declaration: SourceSpan,
    source_file: String,
    scope_start: usize,
    scope_end: usize,
    declaration_scope_start: usize,
    declaration_scope_end: usize,
    public: bool,
    kind: SymbolKind,
}

impl RecoverySymbol {
    fn name_class(&self) -> Option<NameClass> {
        match self.kind {
            SymbolKind::Schema
            | SymbolKind::Effect
            | SymbolKind::Handler
            | SymbolKind::EffectOperation => None,
            SymbolKind::Type => Some(NameClass::Type),
            SymbolKind::Function => Some(NameClass::Function),
            SymbolKind::Constructor => Some(NameClass::Constructor),
            SymbolKind::ValueBinding
            | SymbolKind::HandlerContextParameter
            | SymbolKind::HandlerOperationClauseParameter => {
                Some(NameClass::ValueBinding)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LocalSymbolKind {
    ValueBinding,
    HandlerContextParameter,
    HandlerOperationClauseParameter,
}

#[derive(Clone, Debug)]
struct IndexedFile {
    source: SourceFile,
    tokens: Vec<Token>,
    module: String,
    companion_target_module: Option<String>,
    uses: BTreeSet<String>,
    external_uses: BTreeSet<(String, String)>,
    import_aliases: BTreeMap<String, String>,
    external_import_aliases: BTreeMap<String, (String, String)>,
    schema_alias_external_imports: Vec<ExternalImport>,
    invalid_declaration_names: Vec<SourceSpan>,
    recovery_symbols: Vec<RecoverySymbol>,
    recovered_effect_declarations: Vec<SourceSpan>,
    schema_operation_leaf_ranges: BTreeSet<(usize, usize)>,
    schema_composition_leaf_spans: Vec<SourceSpan>,
    effect_reference_ranges: BTreeSet<(usize, usize)>,
    classified_path_segments: Vec<QualifiedPathSegment>,
    type_reference_locations: OnceLock<TypeReferenceLocations>,
    navigation_isolated: bool,
    origin: IndexedOrigin,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExternalImport {
    module: String,
    package: String,
    alias: String,
    syntax_valid: bool,
}

#[derive(Clone, Debug, Default)]
struct SchemaAliasModuleImports {
    workspace_imports: BTreeSet<String>,
    workspace_imports_by_alias: BTreeMap<String, BTreeSet<String>>,
    external_imports_by_module: BTreeMap<String, BTreeSet<(String, String)>>,
    external_imports_by_alias: BTreeMap<String, BTreeSet<(String, String)>>,
    valid_external_imports_by_module: BTreeMap<String, BTreeSet<(String, String)>>,
    valid_external_imports_by_alias: BTreeMap<String, BTreeSet<(String, String)>>,
}

#[derive(Clone, Debug, Default)]
struct BareSchemaAliasIndex {
    workspace_aliases: BTreeMap<(String, String), NeutralSymbol>,
    workspace_schemas: BTreeSet<(String, String)>,
    workspace_alias_declarations: BTreeSet<(String, String)>,
    standard_prelude_aliases: BTreeMap<String, Vec<NeutralSymbol>>,
    standard_prelude_alias_declarations: BTreeSet<String>,
}

#[derive(Clone, Debug, Default)]
struct SchemaOperationLookupIndex {
    workspace_schemas: BTreeMap<(String, String), Vec<NeutralSymbol>>,
    package_schemas: BTreeMap<(String, String, String), Vec<NeutralSymbol>>,
    workspace_aliases: BTreeMap<(String, String), Vec<NeutralSymbol>>,
    package_aliases: BTreeMap<(String, String, String), Vec<NeutralSymbol>>,
    package_alias_declarations: BTreeSet<(String, String, String)>,
}

#[derive(Clone, Debug)]
struct SchemaCompositionReference {
    span: SourceSpan,
    target: SchemaReferenceTarget,
}

#[derive(Clone, Debug)]
enum SchemaReferenceTarget {
    Schema(NeutralSymbol),
    Alias(NeutralSymbol),
}

#[derive(Clone, Debug, Default)]
struct FileDeclarations {
    schemas: Vec<NeutralSymbol>,
    schema_aliases: Vec<NeutralSymbol>,
    schema_alias_blockers: Vec<NeutralSymbol>,
    package_schema_alias_declarations: Vec<PackageSchemaAliasDeclaration>,
    package_schema_targets: Vec<PackageSchemaTarget>,
    recovered_package_schema_targets: Vec<PackageSchemaTarget>,
    resolved_package_schema_aliases: Vec<ResolvedPackageSchemaAlias>,
    effects: Vec<NeutralSymbol>,
    handlers: Vec<NeutralSymbol>,
    operations: Vec<EffectOperationSymbol>,
    functions: Vec<FunctionSymbol>,
    package_function_targets: Vec<PackageFunctionTarget>,
    package_type_targets: Vec<PackageTypeTarget>,
    package_constructor_targets: Vec<PackageConstructorTarget>,
    types: Vec<TypeSymbol>,
    constructors: Vec<ConstructorSymbol>,
    type_aliases: Vec<TypeAliasSymbol>,
}

#[derive(Clone, Debug)]
struct PackageSchemaAliasDeclaration {
    module: String,
    name: String,
    package: String,
    package_origin: PackageOrigin,
    exported: bool,
}

#[derive(Clone, Debug)]
struct ResolvedPackageSchemaAlias {
    package: String,
    package_origin: PackageOrigin,
    alias_module: Option<String>,
    alias_name: String,
    target_exported: bool,
    direct_target_module: Option<String>,
    direct_target_name: String,
    direct_target_is_alias: bool,
}

#[derive(Clone, Debug)]
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
pub(crate) struct IndexedDependencies {
    files: Vec<IndexedFile>,
    declarations: FileDeclarations,
    module: veln_ast::SurfaceModule,
}

#[derive(Debug)]
pub(crate) struct SymbolIndex {
    files: Vec<IndexedFile>,
    schemas: Vec<NeutralSymbol>,
    schema_aliases: Vec<NeutralSymbol>,
    package_schemas: BTreeMap<(PackageOrigin, String, String, String), NeutralSymbol>,
    effects: Vec<NeutralSymbol>,
    handlers: Vec<NeutralSymbol>,
    operations: Vec<EffectOperationSymbol>,
    functions: Vec<FunctionSymbol>,
    package_function_targets: Vec<PackageFunctionTarget>,
    package_type_targets: Vec<PackageTypeTarget>,
    package_constructor_targets: Vec<PackageConstructorTarget>,
    types: Vec<TypeSymbol>,
    constructors: Vec<ConstructorSymbol>,
    type_aliases: Vec<TypeAliasSymbol>,
    type_indices_by_name: BTreeMap<String, Vec<usize>>,
    type_alias_indices_by_name: BTreeMap<String, Vec<usize>>,
    package_type_alias_indices_by_name: BTreeMap<String, Vec<usize>>,
    workspace_type_indices_by_module_and_name: BTreeMap<(String, String), Vec<usize>>,
    workspace_type_alias_indices_by_module_and_name: BTreeMap<(String, String), Vec<usize>>,
    package_type_alias_indices_by_module_and_name: BTreeMap<(String, String), Vec<usize>>,
    schema_composition_references: Vec<SchemaCompositionReference>,
    schema_alias_module_imports: BTreeMap<String, SchemaAliasModuleImports>,
    bare_schema_alias_index: BareSchemaAliasIndex,
    schema_operation_lookup_index: SchemaOperationLookupIndex,
    function_rename_index: OnceLock<FunctionRenameIndex>,
}

#[derive(Debug)]
struct FunctionRenameIndex {
    scopes_by_file: BTreeMap<String, Vec<FunctionScope>>,
    handler_files: BTreeSet<String>,
}

type TypeReferenceLocations = Vec<(String, usize, SourceSpan)>;

#[derive(Debug)]
struct FunctionScope {
    body_start: usize,
    end: usize,
    params: Vec<ScopedBinding>,
    result_binding: Option<ScopedBinding>,
    local_bindings: Vec<LocalBinding>,
    local_bindings_by_name: BTreeMap<String, Vec<usize>>,
}

#[derive(Debug)]
struct ScopedBinding {
    name: String,
    declaration_start: usize,
    declaration_end: usize,
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
