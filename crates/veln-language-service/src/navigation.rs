use std::collections::BTreeSet;
use std::sync::Arc;

use crate::{DirectDependencySnapshot, EffectiveProjectSnapshot};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, SourcePath, SourceSpan};
use veln_syntax::{
    BodyLine, PublicAliasKind, SyntaxItem, Token, TokenKind, TypeVariantDecl, Visibility, lex,
    parse,
};

include!("navigation/model.rs");
include!("navigation/index.rs");
include!("navigation/declarations.rs");
include!("navigation/handler_bindings.rs");
include!("navigation/references.rs");
include!("navigation/scopes.rs");
include!("navigation/token_roles.rs");
include!("navigation/source_paths.rs");
