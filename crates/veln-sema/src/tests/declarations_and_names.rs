use super::*;
use crate::types::environment::TypeEnvironment;
use crate::types::private_inference::private_inference_counters;
use crate::types::signatures::{HandlerPathResolution, SchemaReferenceErrorKind};
use veln_ast::{InvalidName, NameClass, NameOccurrence};

mod aliases;
mod effects_and_handlers;
mod holes;
mod local_inference;
mod private_inference_performance;
mod private_signature_inference;
mod schema_aliases_and_payloads;
mod schema_primitives;
mod scopes_and_patterns;
mod test_declarations;
