use std::collections::BTreeMap;

use veln_ast::{NameClass, NameOccurrence, PublicAlias, SurfaceModule, UseDecl};

use super::import_classification;

pub(crate) struct InvalidAliasTargetIndex {
    spans_by_file: BTreeMap<String, Vec<(usize, usize)>>,
}

impl InvalidAliasTargetIndex {
    pub(crate) fn new(module: &SurfaceModule) -> Self {
        let mut spans_by_file = BTreeMap::<String, Vec<_>>::new();
        for invalid in &module.invalid_names {
            if invalid.occurrence == NameOccurrence::AliasTarget {
                spans_by_file
                    .entry(invalid.span.file.as_str().to_owned())
                    .or_default()
                    .push((invalid.span.start.offset, invalid.span.end.offset));
            }
        }
        for spans in spans_by_file.values_mut() {
            spans.sort_unstable_by_key(|(start, _)| *start);
        }
        Self { spans_by_file }
    }

    pub(crate) fn contains(&self, alias: &PublicAlias) -> bool {
        let Some(spans) = self.spans_by_file.get(alias.span.file.as_str()) else {
            return false;
        };
        let first = spans.partition_point(|(start, _)| *start < alias.span.start.offset);
        spans.get(first).is_some_and(|(start, end)| {
            *start < alias.span.end.offset && *end <= alias.span.end.offset
        })
    }
}

pub(crate) fn public_alias_has_invalid_target_leaf(
    module: &SurfaceModule,
    alias: &PublicAlias,
    class: Option<NameClass>,
) -> bool {
    module.invalid_names.iter().any(|invalid| {
        invalid.occurrence == NameOccurrence::AliasTarget
            && class.is_none_or(|class| invalid.class == class)
            && invalid.span.file == alias.span.file
            && alias.span.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= alias.span.end.offset
    })
}

#[cfg(test)]
fn use_decl_has_invalid_module_segment(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    import_classification(module).has_invalid_module_segment(use_decl)
}

#[cfg(test)]
fn use_decl_has_invalid_alias(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    let Some(alias_span) = &use_decl.alias_span else {
        return false;
    };
    module.invalid_names.iter().any(|invalid| {
        invalid.class == NameClass::Module
            && invalid.occurrence == NameOccurrence::Declaration
            && invalid.span == *alias_span
    })
}

#[cfg(test)]
fn use_decl_has_duplicate_target(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    import_classification(module).has_duplicate_target(use_decl)
}

pub(crate) fn use_decl_has_reserved_alias(use_decl: &UseDecl) -> bool {
    use_decl.origin == veln_ast::UseOrigin::Source
        && use_decl.has_explicit_alias()
        && use_decl.alias == "prelude"
}

pub(crate) fn use_decl_is_recovery_candidate(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    import_classification(module).is_recovery_candidate(use_decl)
}

#[cfg(test)]
pub(super) fn use_decl_is_quarantined(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    use_decl.recovered
        || use_decl_has_invalid_module_segment(module, use_decl)
        || use_decl_has_invalid_alias(module, use_decl)
        || use_decl_has_reserved_alias(use_decl)
        || use_decl_has_duplicate_target(module, use_decl)
}

pub(crate) fn use_decl_contributes_bare_exports(use_decl: &UseDecl) -> bool {
    !use_decl.has_explicit_alias()
}
