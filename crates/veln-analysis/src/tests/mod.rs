use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use veln_ast::{FunctionKind, SurfaceModule};
use veln_diagnostics::{DiagnosticKind, JsonValue, Severity, diagnostic_to_json};
use veln_project::Project;
use veln_source::{LineCol, SourceFile, SourcePath, SourceSpan};
use veln_syntax::{
    ParseDiagnostic, ParseRepairCandidate, ParseRepairEdit, Recovery, RecoveryStrategy,
    UnexpectedToken,
};

use super::*;

mod diagnostic_conversion;
mod project_cache;
mod reachability;
mod support;

use support::*;
