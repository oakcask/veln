use veln_ast::{SurfaceModule, Visibility};

use super::*;
use crate::adt::{AdtDescriptor, AdtVariantDescriptor, AdtVariantKind};
use crate::builtin_type_syntax::{BUILTIN_TYPE_SYNTAX_DESCRIPTORS, BuiltinTypeSyntaxDescriptor};
use crate::source_less_names::{InvalidStandardSymbolReason, SourceLessNameClass};
use crate::standard_symbols::{StandardSymbolKind, StandardSymbolStability};

#[path = "tests/consumers.rs"]
mod consumers;
#[path = "tests/fixture.rs"]
mod fixture;
#[path = "tests/key_validation.rs"]
mod key_validation;
#[path = "tests/publication.rs"]
mod publication;

use fixture::*;
