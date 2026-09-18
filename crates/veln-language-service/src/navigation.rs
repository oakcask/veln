#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use crate::{DirectDependencySnapshot, EffectiveProjectSnapshot};
use veln_ast::{InvalidName, NameClass, QualifiedPathSegment};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{
    BodyLine, Expr, ExprKind, FunctionDecl, ParseOutput, PublicAliasKind, SyntaxItem, SyntaxTree,
    Token, TokenKind, TypeVariantDecl, Visibility, lex, parse,
};

include!("navigation/model.rs");
include!("navigation/source_indexing.rs");
include!("navigation/package_schemas.rs");
include!("navigation/index.rs");
include!("navigation/selection.rs");
include!("navigation/recovery.rs");
include!("navigation/rename_shared.rs");
include!("navigation/recovery_rename_conflicts.rs");
include!("navigation/function_rename_conflicts.rs");
include!("navigation/rename_conflicts.rs");
include!("navigation/rename_visibility.rs");
include!("navigation/symbol_lookup.rs");
include!("navigation/symbol_references.rs");
include!("navigation/declarations.rs");
include!("navigation/function_alias_declarations.rs");
include!("navigation/recovery_declarations.rs");
include!("navigation/handler_bindings.rs");
include!("navigation/references.rs");
include!("navigation/scopes.rs");
include!("navigation/token_roles.rs");
include!("navigation/source_paths.rs");

#[cfg(test)]
#[path = "navigation/classification_tests.rs"]
mod classification_tests;

#[cfg(test)]
thread_local! {
    static FUNCTION_SCOPE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static TYPE_REFERENCE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static CONSTRUCTOR_REFERENCE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_SOURCE_INDEXES: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_SOURCE_PARSES: Cell<usize> = const { Cell::new(0) };
    static WORKSPACE_SOURCE_PARSES: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_PATH_CLASSIFICATIONS: Cell<usize> = const { Cell::new(0) };
    static PATH_CLASSIFICATION_CONTEXTS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_DECLARATION_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_ALIAS_ELIGIBILITY_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_COMPOSITION_DECLARATION_VISITS: Cell<usize> = const { Cell::new(0) };
    static SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn record_schema_alias_declaration_visit() {
    SCHEMA_ALIAS_DECLARATION_VISITS.set(SCHEMA_ALIAS_DECLARATION_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_declaration_visits() {
    SCHEMA_ALIAS_DECLARATION_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_declaration_visits() -> usize {
    SCHEMA_ALIAS_DECLARATION_VISITS.get()
}

#[cfg(test)]
fn record_schema_alias_eligibility_visit() {
    SCHEMA_ALIAS_ELIGIBILITY_VISITS.set(SCHEMA_ALIAS_ELIGIBILITY_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_eligibility_visits() {
    SCHEMA_ALIAS_ELIGIBILITY_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_eligibility_visits() -> usize {
    SCHEMA_ALIAS_ELIGIBILITY_VISITS.get()
}

#[cfg(test)]
fn record_schema_composition_declaration_visit() {
    SCHEMA_COMPOSITION_DECLARATION_VISITS.set(SCHEMA_COMPOSITION_DECLARATION_VISITS.get() + 1);
}

#[cfg(test)]
fn record_schema_composition_field_token_visits(count: usize) {
    SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.set(SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.get() + count);
}

#[cfg(test)]
pub(crate) fn reset_schema_composition_index_work() {
    SCHEMA_COMPOSITION_DECLARATION_VISITS.set(0);
    SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_composition_index_work() -> (usize, usize) {
    (
        SCHEMA_COMPOSITION_DECLARATION_VISITS.get(),
        SCHEMA_COMPOSITION_FIELD_TOKEN_VISITS.get(),
    )
}

#[cfg(test)]
fn record_function_scope_collection() {
    FUNCTION_SCOPE_COLLECTIONS.set(FUNCTION_SCOPE_COLLECTIONS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_function_scope_collections() {
    FUNCTION_SCOPE_COLLECTIONS.set(0);
}

#[cfg(test)]
pub(crate) fn function_scope_collections() -> usize {
    FUNCTION_SCOPE_COLLECTIONS.get()
}

#[cfg(test)]
fn record_type_reference_collection() {
    TYPE_REFERENCE_COLLECTIONS.set(TYPE_REFERENCE_COLLECTIONS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_type_reference_collections() {
    TYPE_REFERENCE_COLLECTIONS.set(0);
}

#[cfg(test)]
pub(crate) fn type_reference_collections() -> usize {
    TYPE_REFERENCE_COLLECTIONS.get()
}

#[cfg(test)]
fn record_constructor_reference_collection() {
    CONSTRUCTOR_REFERENCE_COLLECTIONS.set(CONSTRUCTOR_REFERENCE_COLLECTIONS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_constructor_reference_collections() {
    CONSTRUCTOR_REFERENCE_COLLECTIONS.set(0);
}

#[cfg(test)]
pub(crate) fn constructor_reference_collections() -> usize {
    CONSTRUCTOR_REFERENCE_COLLECTIONS.get()
}

#[cfg(test)]
fn record_dependency_source_index() {
    DEPENDENCY_SOURCE_INDEXES.set(DEPENDENCY_SOURCE_INDEXES.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_dependency_source_indexes() {
    DEPENDENCY_SOURCE_INDEXES.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_source_indexes() -> usize {
    DEPENDENCY_SOURCE_INDEXES.get()
}

#[cfg(test)]
fn record_dependency_source_parse() {
    DEPENDENCY_SOURCE_PARSES.set(DEPENDENCY_SOURCE_PARSES.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_dependency_source_parses() {
    DEPENDENCY_SOURCE_PARSES.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_source_parses() -> usize {
    DEPENDENCY_SOURCE_PARSES.get()
}

#[cfg(test)]
fn record_workspace_source_parse() {
    WORKSPACE_SOURCE_PARSES.set(WORKSPACE_SOURCE_PARSES.get() + 1);
}

#[cfg(test)]
fn reset_workspace_source_parses() {
    WORKSPACE_SOURCE_PARSES.set(0);
}

#[cfg(test)]
fn workspace_source_parses() -> usize {
    WORKSPACE_SOURCE_PARSES.get()
}

#[cfg(test)]
fn record_dependency_path_classifications(count: usize) {
    DEPENDENCY_PATH_CLASSIFICATIONS.set(DEPENDENCY_PATH_CLASSIFICATIONS.get() + count);
}

#[cfg(test)]
pub(crate) fn reset_dependency_path_classifications() {
    DEPENDENCY_PATH_CLASSIFICATIONS.set(0);
}

#[cfg(test)]
pub(crate) fn dependency_path_classifications() -> usize {
    DEPENDENCY_PATH_CLASSIFICATIONS.get()
}

#[cfg(test)]
fn record_schema_alias_import_index_entries(count: usize) {
    SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.set(SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.get() + count);
}

#[cfg(test)]
fn record_schema_alias_import_route_lookup() {
    SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.set(SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_import_work() {
    SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.set(0);
    SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_import_work() -> (usize, usize) {
    (
        SCHEMA_ALIAS_IMPORT_INDEX_ENTRIES.get(),
        SCHEMA_ALIAS_IMPORT_ROUTE_LOOKUPS.get(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_file_supplies_every_declaration_kind() {
        let (file, declarations, _) = index_workspace_source(SourceFile::new(
            "main.veln",
            concat!(
                "pub type Item\n",
                "  pub Value(value: Int)\n",
                "end\n\n",
                "pub type Exported = Item\n\n",
                "pub fn identity(value: Item) -> Item\n",
                "  value\n",
                "end\n",
            ),
        ));

        assert!(!file.tokens.is_empty());
        assert_eq!(declarations.functions[0].name, "identity");
        assert_eq!(declarations.types[0].name, "Item");
        assert_eq!(declarations.constructors[0].name, "Value");
        assert_eq!(declarations.type_aliases[0].name, "Exported");
    }

    #[test]
    fn package_declarations_share_dependency_origin_metadata() {
        let dependency = crate::tests::dependency_snapshot(
            "example/pkg",
            &[(
                "prelude.veln",
                concat!(
                    "pub type Item\n",
                    "  pub Value(value: Int)\n",
                    "end\n\n",
                    "pub type Exported = Item\n\n",
                    "pub fn identity(value: Item) -> Item\n",
                    "  value\n",
                    "end\n",
                ),
            )],
            ["prelude.veln"],
        );
        let (source, entry) = dependency.indexed_sources().next().unwrap();
        let (file, parsed) = indexed_dependency_source(&dependency, source, entry.uri());
        let declarations = file_declarations(&file, &parsed.tree);

        fn assert_dependency_origin(
            package: &Option<String>,
            origin: Option<PackageOrigin>,
            source: &NavigationSource,
        ) {
            assert_eq!(package.as_deref(), Some("example/pkg"));
            assert_eq!(origin, Some(PackageOrigin::DirectDependency));
            assert!(matches!(source, NavigationSource::Package { .. }));
        }

        let function = &declarations.functions[0];
        assert_dependency_origin(
            &function.package,
            function.package_origin,
            &function.declaration.source,
        );
        let symbol_type = &declarations.types[0];
        assert_dependency_origin(
            &symbol_type.package,
            symbol_type.package_origin,
            &symbol_type.declaration.source,
        );
        let constructor = &declarations.constructors[0];
        assert_dependency_origin(
            &constructor.package,
            constructor.package_origin,
            &constructor.declaration.source,
        );
        let alias = &declarations.type_aliases[0];
        assert_dependency_origin(
            &alias.package,
            alias.package_origin,
            &alias.declaration.source,
        );
    }

    #[test]
    fn workspace_index_parses_each_source_once() {
        let snapshot = EffectiveProjectSnapshot::new(vec![
            SourceFile::new("main.veln", "fn main() -> Int\n  helper()\nend\n"),
            SourceFile::new("helper.veln", "fn helper() -> Int\n  1\nend\n"),
        ]);
        reset_workspace_source_parses();

        snapshot.navigation_index();

        assert_eq!(workspace_source_parses(), 2);
    }
}
