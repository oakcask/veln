use super::*;

#[derive(Clone, Copy, Debug)]
enum DocSchemaResolution {
    Resolved,
    Private,
    WrongKind(&'static str),
    Unresolved,
}

pub(super) fn doc_schema_reference_diagnostics(
    module: &SurfaceModule,
    sources: &[ParsedDocSource<'_>],
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for source in sources {
        for reference in doc_schema_references(source.source) {
            match resolve_doc_schema_reference(
                module,
                &reference.target,
                source.module_name.as_deref(),
                &mut Vec::new(),
            ) {
                DocSchemaResolution::Resolved => {}
                DocSchemaResolution::Private => diagnostics.push(private_doc_schema_diagnostic(
                    &reference.target,
                    reference.span,
                )),
                DocSchemaResolution::WrongKind(actual_kind) => {
                    diagnostics.push(doc_schema_kind_mismatch_diagnostic(
                        &reference.target,
                        actual_kind,
                        reference.span,
                    ))
                }
                DocSchemaResolution::Unresolved => diagnostics.push(
                    unresolved_doc_schema_diagnostic(&reference.target, reference.span),
                ),
            }
        }
    }
    diagnostics
}

fn doc_schema_references(source: &SourceFile) -> Vec<veln_syntax::DocumentationSchemaReference> {
    let lines = source.text().split_inclusive('\n').collect::<Vec<_>>();
    let mut references = Vec::new();
    let mut line_start = 0;
    for line in lines {
        let trimmed = line.trim_start();
        let indent_len = line.len() - trimmed.len();
        if let Some(content) = trimmed.strip_prefix("##") {
            let content_start = line_start + indent_len + "##".len();
            references.extend(extract_documentation_schema_references(
                source,
                content,
                content_start,
            ));
        }
        line_start += line.len();
    }
    references
}

pub(super) fn render_doc_schema_references(line: &str) -> String {
    let mut rendered = String::new();
    let mut cursor = 0;
    while let Some(relative_start) = line[cursor..].find("{@schema") {
        let marker_start = cursor + relative_start;
        let after_marker = marker_start + "{@schema".len();
        let Some(next) = line[after_marker..].chars().next() else {
            break;
        };
        if !next.is_whitespace() {
            rendered.push_str(&line[cursor..after_marker]);
            cursor = after_marker;
            continue;
        }
        let after_space = after_marker
            + line[after_marker..]
                .char_indices()
                .find(|(_, ch)| !ch.is_whitespace())
                .map_or(0, |(index, _)| index);
        let Some(relative_end) = line[after_space..].find('}') else {
            break;
        };
        let marker_end = after_space + relative_end;
        let target = line[after_space..marker_end].trim();
        rendered.push_str(&line[cursor..marker_start]);
        if target.is_empty() {
            rendered.push_str(&line[marker_start..=marker_end]);
        } else {
            rendered.push('`');
            rendered.push_str(target);
            rendered.push('`');
        }
        cursor = marker_end + 1;
    }
    rendered.push_str(&line[cursor..]);
    rendered
}

fn resolve_doc_schema_reference(
    module: &SurfaceModule,
    target: &str,
    current_module: Option<&str>,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
    resolve_doc_schema_segments(module, &segments, current_module, true, visited_aliases)
}

fn resolve_doc_schema_segments(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    match segments {
        [name] => resolve_doc_schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let Some(imported_module) = veln_sema::normal_imported_module_for_path(
                module,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return DocSchemaResolution::Unresolved;
            };
            resolve_doc_schema_in_module(
                module,
                Some(&imported_module),
                name,
                false,
                visited_aliases,
            )
        }
        _ => DocSchemaResolution::Unresolved,
    }
}

fn resolve_doc_schema_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == veln_ast::Visibility::Public {
            DocSchemaResolution::Resolved
        } else {
            DocSchemaResolution::Private
        };
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == AstPublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_doc_schema_alias_target(module, alias, visited_aliases);
    }
    doc_schema_wrong_kind(module, module_name, name).map_or(
        DocSchemaResolution::Unresolved,
        DocSchemaResolution::WrongKind,
    )
}

fn resolve_doc_schema_alias_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> DocSchemaResolution {
    let Some(name) = &alias.name else {
        return DocSchemaResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if visited_aliases.contains(&key) {
        return DocSchemaResolution::Unresolved;
    }
    visited_aliases.push(key);
    let resolution = resolve_doc_schema_segments(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    resolution
}

fn doc_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == veln_ast::FunctionKind::Function
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
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.name.as_deref() == Some(name) && alias.module_name.as_deref() == module_name
    }) {
        return match alias.kind {
            AstPublicAliasKind::Function => Some("function"),
            AstPublicAliasKind::Type => Some("type"),
            AstPublicAliasKind::Schema => None,
        };
    }
    None
}

fn unresolved_doc_schema_diagnostic(target: &str, span: veln_source::SourceSpan) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("unresolved documentation schema reference `{target}`"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(target.to_string())),
        ]),
    )
}

fn private_doc_schema_diagnostic(target: &str, span: veln_source::SourceSpan) -> Diagnostic {
    Diagnostic::new(
        "name.visibility",
        Severity::Error,
        DiagnosticKind::Name,
        format!("documentation schema reference `{target}` is private"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(target.to_string())),
            ("visibility", JsonValue::string("private")),
        ]),
    )
}

fn doc_schema_kind_mismatch_diagnostic(
    target: &str,
    actual_kind: &'static str,
    span: veln_source::SourceSpan,
) -> Diagnostic {
    Diagnostic::new(
        "name.kind_mismatch",
        Severity::Error,
        DiagnosticKind::Name,
        format!("documentation schema reference `{target}` is a {actual_kind}, not a schema"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("expected_kind", JsonValue::string("schema")),
            ("actual_kind", JsonValue::string(actual_kind)),
            ("target", JsonValue::string(target.to_string())),
        ]),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lowered_module(source: &str) -> SurfaceModule {
        let source = SourceFile::new("app.veln", source);
        let parsed = parse(&source);
        let mut module = veln_ast::lower_surface_ast(&parsed.tree);
        for target in [
            "alpha", "beta", "api", "Api", "fallback", "one::api", "two::api",
        ] {
            let target_source = SourceFile::new(
                format!("{target}.veln"),
                "pub schema Packet\n\tvalue: Int\nend\n",
            );
            let target_parsed = parse(&target_source);
            let target_module = veln_ast::lower_surface_ast_with_module_identity(
                &target_parsed.tree,
                target.to_string(),
                target_source.span(veln_source::TextRange::new(0, 0)),
            );
            module.schemas.extend(target_module.schemas);
        }
        module
    }

    fn assert_unresolved(module: &SurfaceModule, target: &str) {
        assert!(matches!(
            resolve_doc_schema_reference(module, target, Some("app"), &mut Vec::new()),
            DocSchemaResolution::Unresolved
        ));
    }

    #[test]
    fn documentation_schema_selection_rejects_blocked_and_quarantined_imports() {
        for (source, targets) in [
            (
                "mod app\nuse alpha as wire\nuse beta as wire\n",
                &["wire::Packet"][..],
            ),
            (
                "mod app\nuse api as first from \"dep\"\nuse api as second from \"dep\"\n",
                &["api::Packet", "first::Packet", "second::Packet"][..],
            ),
            ("mod app\nuse Api as wire\n", &["wire::Packet"][..]),
            ("mod app\nuse api as prelude\n", &["prelude::Packet"][..]),
            (
                "mod app\nuse api as wire as nested\n",
                &["wire::Packet", "api::Packet"][..],
            ),
        ] {
            let module = lowered_module(source);
            for target in targets {
                assert_unresolved(&module, target);
            }
        }

        let mut ambiguous_exact = lowered_module(concat!(
            "mod app\n",
            "use api as first from \"one\"\n",
            "use api as second from \"two\"\n",
            "use fallback as api\n",
        ));
        ambiguous_exact.uses[0].name = "one::api".to_string();
        ambiguous_exact.uses[1].name = "two::api".to_string();
        assert_unresolved(&ambiguous_exact, "api::Packet");
    }
}
