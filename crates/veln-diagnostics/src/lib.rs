//! Stable diagnostic envelopes and detail payloads.

mod envelope;
mod json;
mod severity;

pub use envelope::{Diagnostic, DiagnosticEnvelope, SCHEMA_VERSION, ToolInfo, diagnostic_to_json};
pub use json::JsonValue;
pub use severity::{CheckStatus, DiagnosticKind, Severity};
