use super::*;

pub(super) fn resolve_lsp_mcp_file_backed_assertions(
    path: &Path,
    expectations: &mut CaseExpectations,
    case_text_cache: &mut CaseTextCache,
) {
    for (index, assertion) in expectations.lsp_assertions.iter_mut().enumerate() {
        let selector = assertion.selector();
        resolve_protocol_file_backed_operation(
            path,
            "lsp_assert",
            index,
            &selector,
            &assertion.path,
            &mut assertion.operation,
            case_text_cache,
        );
    }
    for (index, assertion) in expectations.mcp_assertions.iter_mut().enumerate() {
        let selector = assertion.selector();
        resolve_protocol_file_backed_operation(
            path,
            "mcp_assert",
            index,
            &selector,
            &assertion.path,
            &mut assertion.operation,
            case_text_cache,
        );
    }
}

pub(super) fn resolve_protocol_file_backed_operation(
    path: &Path,
    section: &str,
    index: usize,
    selector: &str,
    pointer: &str,
    operation: &mut Option<RpcAssertionOperation>,
    case_text_cache: &mut CaseTextCache,
) {
    let Some(current) = operation.take() else {
        return;
    };
    *operation = Some(match current {
        RpcAssertionOperation::EqualsFileRef(reference) => {
            let context = assertion_context(section, index, selector, pointer, "equals_file");
            let text = case_text_cache.read_path_with_context(
                path,
                reference.line_number,
                &reference.relative,
                Some(&context),
            );
            RpcAssertionOperation::EqualsFile(text)
        }
        RpcAssertionOperation::EqualsJsonFileRef(reference) => {
            let context = assertion_context(section, index, selector, pointer, "equals_json_file");
            let text = case_text_cache.read_path_with_context(
                path,
                reference.line_number,
                &reference.relative,
                Some(&context),
            );
            RpcAssertionOperation::EqualsJsonFile(parse_json(&text).unwrap_or_else(|error| {
                manifest_error(
                    path,
                    reference.line_number,
                    format!("invalid {context} value: {error}"),
                )
            }))
        }
        operation => operation,
    });
}

pub(super) fn assertion_context(
    section: &str,
    index: usize,
    selector: &str,
    pointer: &str,
    operation: &str,
) -> String {
    format!(
        "{} {operation}",
        assertion_base_context(section, index, selector, pointer)
    )
}

pub(super) fn assertion_base_context(
    section: &str,
    index: usize,
    selector: &str,
    pointer: &str,
) -> String {
    format!("{section} {index} {selector} path `{pointer}`")
}

pub(super) fn value_assertion_context(
    section: &str,
    index: usize,
    path: &str,
    operation: &str,
) -> String {
    format!(
        "{} {operation}",
        value_assertion_base_context(section, index, path)
    )
}

pub(super) fn value_assertion_base_context(section: &str, index: usize, path: &str) -> String {
    format!("{section} {index} path `{path}`")
}

pub(super) fn unresolved_assertion_operation_context(
    section: &str,
    index: usize,
    operation: &str,
) -> String {
    format!("{section} {index} {operation}")
}

pub(super) fn validate_binary_fixture_field_path(
    path: &Path,
    fixture_index: usize,
    field_path: Option<&JsonValue>,
) {
    let Some(JsonValue::Array(segments)) = field_path else {
        manifest_error(
            path,
            0,
            format!("binary_fixture {fixture_index} `field_path` must be a JSON array"),
        );
    };
    for (segment_index, segment) in segments.iter().enumerate() {
        let JsonValue::Object(_) = segment else {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} must be an object"
                ),
            );
        };
        if segment
            .object_field("kind")
            .and_then(JsonValue::as_str)
            .is_none()
        {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} is missing string `kind`"
                ),
            );
        }
        if segment
            .object_field("name")
            .and_then(JsonValue::as_str)
            .is_none()
        {
            manifest_error(
                path,
                0,
                format!(
                    "binary_fixture {fixture_index} `field_path` segment {segment_index} is missing string `name`"
                ),
            );
        }
    }
}

pub(super) fn command_source_inputs(command: &[String]) -> Vec<PathBuf> {
    let Some(command_name) = command.first().map(String::as_str) else {
        return Vec::new();
    };
    match command_name {
        "run" => run_command_source_inputs(&command[1..]),
        "check" | "doc" | "fmt" | "metrics" | "test" => source_inputs_after_flags(&command[1..]),
        _ => Vec::new(),
    }
}

pub(super) fn run_command_source_inputs(args: &[String]) -> Vec<PathBuf> {
    let mut saw_entry = false;
    let mut inputs = Vec::new();
    for arg in args {
        if arg == "--" {
            break;
        }
        if arg == "--json" {
            continue;
        }
        if !saw_entry {
            saw_entry = true;
            continue;
        }
        inputs.push(PathBuf::from(arg));
    }
    inputs
}

pub(super) fn source_inputs_after_flags(args: &[String]) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        if arg == "--" {
            break;
        }
        if arg == "--json" {
            continue;
        }
        if matches!(
            arg.as_str(),
            "--baseline" | "--write-baseline" | "--jobs" | "-j"
        ) {
            let _ = args.next();
            continue;
        }
        if arg.starts_with("--baseline=")
            || arg.starts_with("--write-baseline=")
            || arg.starts_with("--jobs=")
        {
            continue;
        }
        inputs.push(PathBuf::from(arg));
    }
    inputs
}

pub(super) fn fixture_reference_module(
    project: &Project,
    first_input: Option<&PathBuf>,
) -> Option<String> {
    if let Some(first_input) = first_input {
        let source_path = if first_input.is_absolute() {
            first_input.clone()
        } else {
            project.root.join(first_input)
        };
        if source_path.is_file()
            && let Ok(source) = veln_source::SourceFile::read(&project.root, &source_path)
            && let Ok(module) = derive_source_module_path(&source)
        {
            return Some(module);
        }
    }
    project
        .files
        .first()
        .and_then(|source| derive_source_module_path(source).ok())
}

pub(super) fn validate_binary_fixture_schema_references(
    path: &Path,
    module: &SurfaceModule,
    current_module: Option<&str>,
    fixtures: &[BinaryFixtureExpectation],
) {
    let mut errors = Vec::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let Some(schema) = &fixture.schema else {
            continue;
        };
        match resolve_fixture_schema_reference(module, schema, current_module) {
            FixtureSchemaResolution::Resolved { name } => {
                if let Some(error) =
                    validate_binary_fixture_schema_field_path(index, &name, fixture)
                {
                    errors.push(error);
                }
            }
            FixtureSchemaResolution::Private => errors.push(format!(
                "binary_fixture {index} schema reference `{schema}` is private"
            )),
            FixtureSchemaResolution::WrongKind(kind) => errors.push(format!(
                "binary_fixture {index} schema reference `{schema}` is a {kind}, not a schema"
            )),
            FixtureSchemaResolution::Unresolved => errors.push(format!(
                "unresolved binary_fixture {index} schema reference `{schema}`"
            )),
        }
    }
    if !errors.is_empty() {
        manifest_error(path, 0, errors.join("\n"));
    }
}

pub(super) fn validate_binary_fixture_schema_field_path(
    fixture_index: usize,
    schema_name: &str,
    fixture: &BinaryFixtureExpectation,
) -> Option<String> {
    let field_path = fixture
        .byte_diagnostic
        .as_ref()
        .and_then(|diagnostic| diagnostic.field_path.as_ref())?;
    let segments = field_path.as_array()?;
    let first_schema = segments
        .first()
        .and_then(|segment| match segment.object_field("kind") {
            Some(kind) if kind.as_str() == Some("schema") => segment.object_field("name"),
            _ => None,
        })
        .and_then(JsonValue::as_str);
    if first_schema != Some(schema_name) {
        return Some(format!(
            "binary_fixture {fixture_index} `field_path` first segment must name schema `{schema_name}`"
        ));
    }
    None
}

pub(super) enum FixtureSchemaResolution {
    Resolved { name: String },
    Private,
    WrongKind(&'static str),
    Unresolved,
}

pub(super) fn resolve_fixture_schema_reference(
    module: &SurfaceModule,
    target: &str,
    current_module: Option<&str>,
) -> FixtureSchemaResolution {
    let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
    resolve_fixture_schema_segments(module, &segments, current_module, true, &mut Vec::new())
}

pub(super) fn resolve_fixture_schema_segments(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    match segments {
        [name] => resolve_fixture_schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return FixtureSchemaResolution::Unresolved;
            };
            resolve_fixture_schema_in_module(
                module,
                Some(&use_decl.name),
                name,
                false,
                visited_aliases,
            )
        }
        _ => FixtureSchemaResolution::Unresolved,
    }
}

pub(super) fn resolve_fixture_schema_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == Visibility::Public {
            FixtureSchemaResolution::Resolved {
                name: schema.name.clone().expect("schema should have a name"),
            }
        } else {
            FixtureSchemaResolution::Private
        };
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_fixture_schema_alias_target(module, alias, visited_aliases);
    }
    fixture_schema_wrong_kind(module, module_name, name).map_or(
        FixtureSchemaResolution::Unresolved,
        FixtureSchemaResolution::WrongKind,
    )
}

pub(super) fn resolve_fixture_schema_alias_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> FixtureSchemaResolution {
    let Some(name) = &alias.name else {
        return FixtureSchemaResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if visited_aliases.contains(&key) {
        return FixtureSchemaResolution::Unresolved;
    }
    visited_aliases.push(key);
    let resolution = resolve_fixture_schema_segments(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    resolution
}

pub(super) fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

pub(super) fn is_accumulating_manifest_key(section: Section, key: &str) -> bool {
    matches!(
        (section, key),
        (
            Section::Stdout | Section::Stderr,
            "contains"
                | "contains_file"
                | "contains_files"
                | "not_contains"
                | "not_contains_file"
                | "not_contains_files"
        ) | (
            Section::Help | Section::ManifestError,
            "contains" | "contains_file" | "contains_files"
        )
    )
}

pub(super) fn fixture_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == FunctionKind::Function
            && function.name.as_deref() == Some(name)
            && function.module_name.as_deref() == module_name
    }) {
        return Some("function");
    }
    if module.types.iter().any(|type_decl| {
        type_decl.name.as_deref() == Some(name) && type_decl.module_name.as_deref() == module_name
    }) {
        return Some("type");
    }
    if module.codecs.iter().any(|codec| {
        codec.name.as_deref() == Some(name) && codec.module_name.as_deref() == module_name
    }) {
        return Some("codec");
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.name.as_deref() == Some(name) && alias.module_name.as_deref() == module_name
    }) {
        return match alias.kind {
            PublicAliasKind::Function => Some("function"),
            PublicAliasKind::Type => Some("type"),
            PublicAliasKind::Schema => None,
        };
    }
    None
}

pub(super) fn parse_help_key(
    path: &Path,
    line_number: usize,
    help: &mut HelpExpectation,
    key: &str,
    value: &ManifestValue<'_>,
    case_text_cache: &mut CaseTextCache,
) {
    match key {
        "stream" => help.stream = OutputStream::parse(path, value),
        "summary" => help.summary = Some(parse_string(path, value)),
        "usage" => help.usage = Some(parse_string(path, value)),
        "commands" => help.commands = parse_string_array(path, value),
        "arguments" => help.arguments = parse_string_array(path, value),
        "options" => help.options = parse_string_array(path, value),
        "contains" => help.contains.extend(parse_string_array(path, value)),
        "contains_file" => help.contains.push(case_text_cache.read(path, value)),
        "contains_files" => help.contains.extend(case_text_cache.read_many(path, value)),
        _ => manifest_error(path, line_number, format!("unknown help key `{key}`")),
    }
}

pub(super) fn parse_stream_key(
    path: &Path,
    line_number: usize,
    stream: &mut StreamExpectation,
    key: &str,
    value: &ManifestValue<'_>,
    allow_json: bool,
    case_text_cache: &mut CaseTextCache,
) {
    match key {
        "format" => {
            let format = parse_string(path, value);
            stream.format = Some(match format.as_str() {
                "empty" => StreamFormat::Empty,
                "text" => StreamFormat::Text,
                "json" if allow_json => StreamFormat::Json,
                _ => manifest_error(
                    path,
                    line_number,
                    format!("unknown stream format `{format}`"),
                ),
            });
        }
        "equals_file" => stream.equals = Some(case_text_cache.read(path, value)),
        "contains" => stream.contains.extend(parse_string_array(path, value)),
        "contains_file" => stream.contains.push(case_text_cache.read(path, value)),
        "contains_files" => stream
            .contains
            .extend(case_text_cache.read_many(path, value)),
        "not_contains" => stream.not_contains.extend(parse_string_array(path, value)),
        "not_contains_file" => stream.not_contains.push(case_text_cache.read(path, value)),
        "not_contains_files" => stream
            .not_contains
            .extend(case_text_cache.read_many(path, value)),
        _ => manifest_error(path, line_number, format!("unknown stream key `{key}`")),
    }
}

pub(super) fn parse_value_contains_operation(
    path: &Path,
    value: &ManifestValue<'_>,
) -> ValueAssertionOperation {
    ValueAssertionOperation::Contains(parse_string(path, value))
}

pub(super) fn record_mcp_contains_assertion(
    assertion: &mut McpAssertion,
    path: &Path,
    value: &ManifestValue<'_>,
) {
    assertion.operation_count += 1;
    assertion.operation = Some(RpcAssertionOperation::Contains(parse_string(path, value)));
}
