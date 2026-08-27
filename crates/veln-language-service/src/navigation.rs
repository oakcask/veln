use std::collections::BTreeSet;
use std::sync::Arc;

use crate::{DirectDependencySnapshot, EffectiveProjectSnapshot};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourcePath, SourceSpan};
use veln_syntax::{
    BodyLine, ParseOutput, PublicAliasKind, SyntaxItem, SyntaxTree, Token, TokenKind,
    TypeVariantDecl, Visibility, lex, parse,
};

include!("navigation/model.rs");
include!("navigation/index.rs");
include!("navigation/declarations.rs");
include!("navigation/handler_bindings.rs");
include!("navigation/references.rs");
include!("navigation/scopes.rs");
include!("navigation/token_roles.rs");
include!("navigation/source_paths.rs");

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
