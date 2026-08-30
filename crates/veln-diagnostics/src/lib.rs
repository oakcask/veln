//! Stable diagnostic envelopes and detail payloads.

mod envelope;
mod json;
mod severity;
mod toolchain;

pub use envelope::{Diagnostic, DiagnosticEnvelope, SCHEMA_VERSION, ToolInfo, diagnostic_to_json};
pub use json::{JsonValue, parse_json_value, source_span_to_json, write_json_string};
pub use severity::{CheckStatus, DiagnosticKind, Severity};
pub use toolchain::{
    ToolchainSymbolNameClass, ToolchainSymbolNameFailureReason,
    toolchain_invalid_symbol_case_diagnostic,
};
