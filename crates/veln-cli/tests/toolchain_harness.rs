use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Write;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use veln_analysis::{
    DoctestMode, checked_project_diagnostics, derive_source_module_path, load_surface_module,
};
use veln_ast::{FunctionKind, PublicAliasKind, SurfaceModule, UseDecl, Visibility};
use veln_diagnostics::{Diagnostic, Severity};
use veln_project::Project;

#[path = "toolchain_harness/assertion_json.rs"]
mod assertion_json;
#[cfg(all(unix, debug_assertions))]
#[path = "toolchain_harness/jvm_cache_coordination.rs"]
mod jvm_cache_coordination;
#[path = "toolchain_harness/manifest_preflight.rs"]
mod manifest_preflight;
#[path = "toolchain_harness/manifest_syntax.rs"]
mod manifest_syntax;
#[path = "toolchain_harness/result_value.rs"]
mod result_value;
#[path = "../toolchain_case_inventory.rs"]
mod toolchain_case_inventory;

use assertion_json::{JsonValue, escape_json_string, parse_json};
use manifest_syntax::{Statement as ManifestStatement, Value as ManifestValue};
use result_value::{parse_result_value, parse_veln_value};

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);
const SOURCE_DIAGNOSTIC_ARTIFACT_ENV: &str = "VELN_HARNESS_SOURCE_DIAGNOSTICS";

thread_local! {
    static TEST_GENERATED_TOOLCHAIN_CASES: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
}

include!(concat!(env!("OUT_DIR"), "/toolchain_cases.rs"));

#[path = "toolchain_harness/adapter_contexts.rs"]
mod adapter_contexts;
#[path = "toolchain_harness/assertion_models.rs"]
mod assertion_models;
#[path = "toolchain_harness/assertion_resolution.rs"]
mod assertion_resolution;
#[path = "toolchain_harness/binary_and_results.rs"]
mod binary_and_results;
#[path = "toolchain_harness/cache_and_metrics.rs"]
mod cache_and_metrics;
#[path = "toolchain_harness/case_expectations.rs"]
mod case_expectations;
#[path = "toolchain_harness/case_text.rs"]
mod case_text;
#[path = "toolchain_harness/common_operations.rs"]
mod common_operations;
#[path = "toolchain_harness/file_and_manifest_values.rs"]
mod file_and_manifest_values;
#[path = "toolchain_harness/fixture_helpers.rs"]
mod fixture_helpers;
#[path = "toolchain_harness/inventory_and_policy.rs"]
mod inventory_and_policy;
#[path = "toolchain_harness/json_assertions.rs"]
mod json_assertions;
#[path = "toolchain_harness/json_number_semantics.rs"]
mod json_number_semantics;
#[path = "toolchain_harness/lsp_transport.rs"]
mod lsp_transport;
#[path = "toolchain_harness/manifest_assertions.rs"]
mod manifest_assertions;
#[path = "toolchain_harness/manifest_parser.rs"]
mod manifest_parser;
#[path = "toolchain_harness/manifest_parser_assertions.rs"]
mod manifest_parser_assertions;
#[path = "toolchain_harness/manifest_parser_headers.rs"]
mod manifest_parser_headers;
#[path = "toolchain_harness/manifest_parser_outputs.rs"]
mod manifest_parser_outputs;
#[path = "toolchain_harness/manifest_strings.rs"]
mod manifest_strings;
#[path = "toolchain_harness/nested_json_assertions.rs"]
mod nested_json_assertions;
#[path = "toolchain_harness/output_assertions.rs"]
mod output_assertions;
#[path = "toolchain_harness/policy_guards.rs"]
mod policy_guards;
#[path = "toolchain_harness/project_fixture.rs"]
mod project_fixture;
#[path = "toolchain_harness/protocol_assertions.rs"]
mod protocol_assertions;
#[path = "toolchain_harness/protocol_file_equality.rs"]
mod protocol_file_equality;
#[path = "toolchain_harness/runtime_inventory.rs"]
mod runtime_inventory;
#[path = "toolchain_harness/runtime_result_shapes.rs"]
mod runtime_result_shapes;
#[path = "toolchain_harness/sidecar_resources.rs"]
mod sidecar_resources;
#[path = "toolchain_harness/source_error_guards.rs"]
mod source_error_guards;

use assertion_models::*;
use assertion_resolution::*;
use case_expectations::*;
use case_text::*;
use common_operations::*;
use file_and_manifest_values::*;
use fixture_helpers::*;
use json_assertions::*;
use json_number_semantics::*;
use lsp_transport::*;
use manifest_parser::*;
use output_assertions::*;
use project_fixture::*;
use protocol_assertions::*;
use runtime_inventory::*;
use sidecar_resources::*;
use source_error_guards::*;
