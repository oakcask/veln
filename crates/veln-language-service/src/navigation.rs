#[cfg(test)]
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use crate::{DirectDependencySnapshot, EffectiveProjectSnapshot};
use veln_ast::{NameClass, QualifiedPathSegment};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::{
    BodyLine, ParseOutput, PublicAliasKind, SyntaxItem, SyntaxTree, Token, TokenKind,
    TypeVariantDecl, Visibility, lex, parse,
};

include!("navigation/model.rs");
include!("navigation/index.rs");
include!("navigation/rename_conflicts.rs");
include!("navigation/symbol_lookup.rs");
include!("navigation/symbol_references.rs");
include!("navigation/declarations.rs");
include!("navigation/handler_bindings.rs");
include!("navigation/references.rs");
include!("navigation/scopes.rs");
include!("navigation/token_roles.rs");
include!("navigation/source_paths.rs");

#[cfg(test)]
thread_local! {
    static FUNCTION_SCOPE_COLLECTIONS: Cell<usize> = const { Cell::new(0) };
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
