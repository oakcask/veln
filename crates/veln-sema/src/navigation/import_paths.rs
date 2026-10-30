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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolvedNavigationImportIdentity(usize);

#[derive(Clone, Copy, Debug)]
pub struct ResolvedImportPathPrefix<'a> {
    pub import_id: ResolvedNavigationImportIdentity,
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
                        import_id: ResolvedNavigationImportIdentity(index),
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
