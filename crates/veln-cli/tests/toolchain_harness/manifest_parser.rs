use super::*;

pub(super) fn parse_manifest(path: &Path, text: &str) -> CaseManifest {
    let statements = manifest_syntax::parse_document(path, text);
    manifest_preflight::validate(path, &statements);
    let mut parser = ManifestParser::new(path);
    for statement in statements {
        match statement {
            ManifestStatement::Section { name, line } => {
                parser.parse_section_header(&name, line);
            }
            ManifestStatement::Assignment { key, line, value } => {
                parser.parse_section_key(line, key, &value);
            }
        }
    }
    parser.finish()
}

pub(super) struct ManifestParser<'a> {
    pub(super) path: &'a Path,
    pub(super) command: Option<Vec<String>>,
    pub(super) cwd: Option<PathBuf>,
    pub(super) stdin: Option<String>,
    pub(super) stdin_jsonrpc_file: Option<String>,
    pub(super) stdin_jsonrpc_workspace_file_uri_directives: Vec<WorkspaceFileUriDirective>,
    pub(super) exit: Option<i32>,
    pub(super) repeat: usize,
    pub(super) env: Vec<(String, String)>,
    pub(super) source_errors: SourceErrorExpectation,
    pub(super) stdout: StreamExpectation,
    pub(super) stderr: StreamExpectation,
    pub(super) help: Option<HelpExpectation>,
    pub(super) json_assertions: Vec<JsonAssertion>,
    pub(super) result_value_assertions: Vec<ResultValueAssertion>,
    pub(super) lsp_assertions: Vec<LspAssertion>,
    pub(super) mcp_assertions: Vec<McpAssertion>,
    pub(super) file_assertions: Vec<FileAssertion>,
    pub(super) diagnostics: Vec<DiagnosticExpectation>,
    pub(super) manifest_error: Option<ManifestErrorExpectation>,
    pub(super) binary_fixtures: Vec<BinaryFixtureExpectation>,
    pub(super) output_chunk_lists: Vec<OutputChunkListExpectation>,
    pub(super) tools: ToolSetup,
    pub(super) requires: Requirements,
    pub(super) skip: SkipRules,
    pub(super) section: Section,
    pub(super) seen_assignments: BTreeSet<String>,
    pub(super) stdin_operand_count: usize,
    pub(super) case_text_cache: CaseTextCache,
}
