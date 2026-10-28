use veln_ast::{SurfaceModule, UseOrigin};
use veln_source::SourceSpan;

use crate::name_recovery::{ImportPrefixSelection, NormalImportSelectionIndex, normal_use_decls};

#[derive(Clone, Debug)]
pub struct ResolvedExplicitImportAlias {
    pub declaring_module: String,
    pub name: String,
    pub span: SourceSpan,
    pub target_module: String,
    pub target_package: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ResolvedNavigationImport {
    pub declaring_module: String,
    pub qualifier: String,
    pub target_module: String,
    pub target_package: Option<String>,
    pub explicit_alias: bool,
}

#[derive(Clone, Debug)]
pub enum ResolvedImportPath {
    Exact {
        target_module: String,
        target_package: Option<String>,
    },
    ExplicitAlias(ResolvedExplicitImportAlias),
}

#[derive(Clone, Debug)]
pub enum ResolvedImportPathSelection {
    Resolved(ResolvedImportPath),
    Blocked,
    Missing,
}

#[derive(Clone, Copy, Debug)]
pub struct ResolvedImportPathPrefix<'a> {
    pub target_module: &'a str,
    pub target_package: Option<&'a str>,
    pub alias_span: Option<&'a SourceSpan>,
    pub consumed_segments: usize,
}

#[derive(Clone, Copy, Debug)]
pub enum ResolvedImportPathPrefixSelection<'a> {
    Resolved(ResolvedImportPathPrefix<'a>),
    Blocked,
    Missing,
}

pub struct ResolvedImportPathIndex<'a> {
    imports: NormalImportSelectionIndex<'a>,
}

impl<'a> ResolvedImportPathIndex<'a> {
    pub fn new(module: &'a SurfaceModule) -> Self {
        Self {
            imports: NormalImportSelectionIndex::new_with_prefix_paths(module),
        }
    }

    pub fn resolve(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<ResolvedImportPath> {
        let use_decl = self
            .imports
            .imported_use_for_path(segments, current_module, None)?;
        resolved_import_path_from_use(use_decl, segments)
    }

    pub fn resolve_prefixes<'s>(
        &self,
        segments: impl IntoIterator<Item = &'s str>,
        current_module: Option<&str>,
    ) -> Option<ResolvedImportPath> {
        let segments = segments.into_iter().collect::<Vec<_>>();
        let ImportPrefixSelection::Selected {
            index,
            explicit_alias,
            consumed_segments,
        } = self
            .imports
            .select_import_prefixes(segments.iter().copied(), current_module, None)
        else {
            return None;
        };
        let use_decl = self.imports.selected_use(index)?;
        Some(resolved_import_path_from_selection(
            use_decl,
            explicit_alias,
            &segments[consumed_segments..],
        ))
    }

    pub fn select_prefixes<'s>(
        &self,
        segments: impl IntoIterator<Item = &'s str>,
        current_module: Option<&str>,
    ) -> ResolvedImportPathSelection {
        let segments = segments.into_iter().collect::<Vec<_>>();
        match self
            .imports
            .select_import_prefixes(segments.iter().copied(), current_module, None)
        {
            ImportPrefixSelection::Selected {
                index,
                explicit_alias,
                consumed_segments,
            } => {
                let Some(use_decl) = self.imports.selected_use(index) else {
                    return ResolvedImportPathSelection::Missing;
                };
                ResolvedImportPathSelection::Resolved(resolved_import_path_from_selection(
                    use_decl,
                    explicit_alias,
                    &segments[consumed_segments..],
                ))
            }
            ImportPrefixSelection::Blocked => ResolvedImportPathSelection::Blocked,
            ImportPrefixSelection::Missing => ResolvedImportPathSelection::Missing,
        }
    }

    pub fn select_prefix_sequence(
        &self,
        segments: &[&str],
        current_module: Option<&str>,
    ) -> Vec<ResolvedImportPathPrefixSelection<'a>> {
        self.imports
            .select_import_prefix_sequence(segments, current_module, None)
            .into_iter()
            .map(|selection| match selection {
                ImportPrefixSelection::Selected {
                    index,
                    explicit_alias,
                    consumed_segments,
                } => {
                    let Some(use_decl) = self.imports.selected_use(index) else {
                        return ResolvedImportPathPrefixSelection::Missing;
                    };
                    ResolvedImportPathPrefixSelection::Resolved(ResolvedImportPathPrefix {
                        target_module: &use_decl.name,
                        target_package: use_decl.package.as_deref(),
                        alias_span: explicit_alias
                            .then_some(use_decl.alias_span.as_ref())
                            .flatten(),
                        consumed_segments,
                    })
                }
                ImportPrefixSelection::Blocked => ResolvedImportPathPrefixSelection::Blocked,
                ImportPrefixSelection::Missing => ResolvedImportPathPrefixSelection::Missing,
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct ImportNamespaceParticipant {
    pub declaring_module: String,
    pub qualifier: String,
    pub declaration_span: SourceSpan,
    pub explicit_alias_span: Option<SourceSpan>,
}

pub fn resolved_explicit_import_aliases(
    module: &SurfaceModule,
) -> Vec<ResolvedExplicitImportAlias> {
    normal_use_decls(module)
        .into_iter()
        .filter(|use_decl| use_decl.has_explicit_alias() && !use_decl.alias.is_empty())
        .filter_map(|use_decl| {
            Some(ResolvedExplicitImportAlias {
                declaring_module: use_decl.module_name?,
                name: use_decl.alias,
                span: use_decl.alias_span?,
                target_module: use_decl.name,
                target_package: use_decl.package,
            })
        })
        .collect()
}

pub fn resolved_navigation_imports(module: &SurfaceModule) -> Vec<ResolvedNavigationImport> {
    normal_use_decls(module)
        .into_iter()
        .filter_map(|use_decl| {
            let explicit_alias = use_decl.has_explicit_alias();
            Some(ResolvedNavigationImport {
                declaring_module: use_decl.module_name?,
                qualifier: use_decl.alias,
                target_module: use_decl.name,
                target_package: use_decl.package,
                explicit_alias,
            })
        })
        .collect()
}

fn resolved_import_path_from_use(
    use_decl: &veln_ast::UseDecl,
    segments: &[String],
) -> Option<ResolvedImportPath> {
    if use_decl.has_explicit_alias() && segments.join("::") == use_decl.alias {
        return Some(ResolvedImportPath::ExplicitAlias(
            ResolvedExplicitImportAlias {
                declaring_module: use_decl.module_name.clone()?,
                name: use_decl.alias.clone(),
                span: use_decl.alias_span.clone()?,
                target_module: use_decl.name.clone(),
                target_package: use_decl.package.clone(),
            },
        ));
    }
    Some(ResolvedImportPath::Exact {
        target_module: use_decl.name.clone(),
        target_package: use_decl.package.clone(),
    })
}

fn resolved_import_path_from_selection(
    use_decl: &veln_ast::UseDecl,
    explicit_alias: bool,
    suffix: &[&str],
) -> ResolvedImportPath {
    let target_module = if suffix.is_empty() {
        use_decl.name.clone()
    } else {
        format!("{}::{}", use_decl.name, suffix.join("::"))
    };
    if explicit_alias {
        ResolvedImportPath::ExplicitAlias(ResolvedExplicitImportAlias {
            declaring_module: use_decl.module_name.clone().unwrap_or_default(),
            name: use_decl.alias.clone(),
            span: use_decl
                .alias_span
                .clone()
                .unwrap_or_else(|| use_decl.span.clone()),
            target_module,
            target_package: use_decl.package.clone(),
        })
    } else {
        ResolvedImportPath::Exact {
            target_module,
            target_package: use_decl.package.clone(),
        }
    }
}

pub fn import_namespace_participants(module: &SurfaceModule) -> Vec<ImportNamespaceParticipant> {
    let mut participants = Vec::new();
    for use_decl in module
        .uses
        .iter()
        .filter(|use_decl| use_decl.origin == UseOrigin::Source && !use_decl.recovered)
    {
        let Some(declaring_module) = use_decl.module_name.clone() else {
            continue;
        };
        let target_span = use_decl
            .name_spans
            .last()
            .cloned()
            .unwrap_or_else(|| use_decl.span.clone());
        for qualifier in use_decl.exact_paths() {
            participants.push(ImportNamespaceParticipant {
                declaring_module: declaring_module.clone(),
                qualifier,
                declaration_span: target_span.clone(),
                explicit_alias_span: None,
            });
        }
        participants.push(ImportNamespaceParticipant {
            declaring_module,
            qualifier: use_decl.alias.clone(),
            declaration_span: use_decl
                .alias_span
                .clone()
                .unwrap_or_else(|| target_span.clone()),
            explicit_alias_span: use_decl.alias_span.clone(),
        });
    }
    participants.sort_by(|left, right| {
        left.declaration_span
            .file
            .cmp(&right.declaration_span.file)
            .then_with(|| {
                left.declaration_span
                    .start
                    .offset
                    .cmp(&right.declaration_span.start.offset)
            })
            .then_with(|| left.qualifier.cmp(&right.qualifier))
    });
    participants
}
