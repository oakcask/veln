#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use crate::{DirectDependencySnapshot, EffectiveProjectSnapshot};
use veln_ast::{InvalidName, NameClass, QualifiedPathSegment};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{
    BodyLine, FunctionDecl, ParseOutput, PublicAliasKind, SyntaxItem, SyntaxTree, Token, TokenKind,
    TypeVariantDecl, Visibility, lex, parse,
};

include!("navigation/model.rs");
include!("navigation/source_indexing.rs");
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
include!("navigation/recovery_declarations.rs");
include!("navigation/handler_bindings.rs");
include!("navigation/references.rs");
include!("navigation/scopes.rs");
include!("navigation/token_roles.rs");
include!("navigation/source_paths.rs");

#[cfg(test)]
thread_local! {
    static FUNCTION_SCOPE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static TYPE_REFERENCE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_SOURCE_INDEXES: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_SOURCE_PARSES: Cell<usize> = const { Cell::new(0) };
    static DEPENDENCY_PATH_CLASSIFICATIONS: Cell<usize> = const { Cell::new(0) };
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
mod tests {
    use super::*;

    #[test]
    fn prepared_file_supplies_every_declaration_kind() {
        let (file, declarations) = index_workspace_source(SourceFile::new(
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
}
