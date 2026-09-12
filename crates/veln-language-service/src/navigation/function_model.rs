#[derive(Clone, Debug)]
struct FunctionSymbol {
    module: String,
    name: String,
    declaration: NavigationLocation,
    package: Option<String>,
    package_origin: Option<PackageOrigin>,
    public: bool,
    standard_prelude: bool,
    declaration_kind: SymbolDeclarationKind,
    alias_target_module: Option<String>,
    alias_target_name: Option<String>,
}

#[derive(Clone, Debug)]
struct FunctionAliasSymbol {
    module: String,
    name: String,
    package: Option<String>,
    target_module: Option<String>,
    target_name: Option<String>,
    import_aliases: BTreeMap<String, String>,
}
