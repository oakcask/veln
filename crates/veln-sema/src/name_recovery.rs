use std::collections::BTreeMap;

use veln_ast::{NameClass, NameOccurrence, PublicAlias, SurfaceModule, UseDecl};

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

pub(crate) fn use_decl_has_invalid_module_segment(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> bool {
    module.invalid_names.iter().any(|invalid| {
        invalid.class == NameClass::Module
            && (invalid.occurrence == NameOccurrence::PathSegment
                || (use_decl.alias_span.as_ref() == Some(&invalid.span)
                    && invalid.occurrence == NameOccurrence::Declaration))
            && invalid.span.file == use_decl.span.file
            && use_decl.span.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= use_decl.span.end.offset
    })
}

pub(crate) fn use_decl_has_invalid_alias(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    let Some(alias_span) = &use_decl.alias_span else {
        return false;
    };
    module.invalid_names.iter().any(|invalid| {
        invalid.class == NameClass::Module
            && invalid.occurrence == NameOccurrence::Declaration
            && invalid.span == *alias_span
    })
}

pub(crate) fn use_decl_has_duplicate_target(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    if use_decl.recovered {
        return false;
    }
    module.uses.iter().any(|other| {
        !std::ptr::eq(other, use_decl)
            && other.origin == veln_ast::UseOrigin::Source
            && use_decl.origin == veln_ast::UseOrigin::Source
            && !other.recovered
            && other.module_name == use_decl.module_name
            && other.written_name == use_decl.written_name
            && other.package == use_decl.package
    })
}

pub(crate) fn use_decl_has_reserved_alias(use_decl: &UseDecl) -> bool {
    use_decl.origin == veln_ast::UseOrigin::Source
        && use_decl.has_explicit_alias()
        && use_decl.alias == "prelude"
}

pub(crate) fn use_decl_has_source_alias_collision(
    module: &SurfaceModule,
    use_decl: &UseDecl,
) -> bool {
    if use_decl.recovered || use_decl_has_reserved_alias(use_decl) {
        return false;
    }
    let matches = module.uses.iter().filter(|other| {
        other.origin == veln_ast::UseOrigin::Source
            && !other.recovered
            && !use_decl_has_reserved_alias(other)
            && other.module_name == use_decl.module_name
            && other.alias == use_decl.alias
    });
    let mut count = 0usize;
    let mut has_explicit = false;
    for other in matches {
        count += 1;
        has_explicit |= other.has_explicit_alias();
    }
    count > 1 && has_explicit
}

pub(crate) fn use_decl_is_recovery_candidate(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    use_decl_has_invalid_module_segment(module, use_decl)
        && !use_decl.recovered
        && !use_decl_has_reserved_alias(use_decl)
        && !use_decl_has_duplicate_target(module, use_decl)
        && !use_decl_has_source_alias_collision(module, use_decl)
}

pub(crate) fn use_decl_is_quarantined(module: &SurfaceModule, use_decl: &UseDecl) -> bool {
    use_decl.recovered
        || use_decl_has_invalid_module_segment(module, use_decl)
        || use_decl_has_invalid_alias(module, use_decl)
        || use_decl_has_reserved_alias(use_decl)
        || use_decl_has_duplicate_target(module, use_decl)
}

pub(crate) fn use_decl_contributes_bare_exports(use_decl: &UseDecl) -> bool {
    !use_decl.has_explicit_alias()
}

pub(crate) fn normal_use_decls(module: &SurfaceModule) -> Vec<UseDecl> {
    module
        .uses
        .iter()
        .filter(|use_decl| !use_decl_is_quarantined(module, use_decl))
        .map(resolved_import_use_decl)
        .collect()
}

pub(crate) fn resolved_import_use_decl(use_decl: &UseDecl) -> UseDecl {
    let mut resolved = use_decl.clone();
    resolved.name = resolved_import_module_name(use_decl, use_decl.module_name.as_deref());
    resolved
}

pub(crate) fn normal_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    let candidates = module
        .uses
        .iter()
        .filter(|use_decl| !use_decl_is_quarantined(module, use_decl))
        .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
        .collect::<Vec<_>>();
    let mut exact = candidates.iter().copied().filter(|use_decl| {
        use_decl_matches_exact_import_path(use_decl, &module_path, current_module)
    });
    if let Some(first) = exact.next() {
        return exact.next().is_none().then_some(first);
    }
    if source_alias_is_colliding(module, &module_path, current_module) {
        return None;
    }
    let mut qualified = candidates
        .into_iter()
        .filter(|use_decl| simple_import_alias_matches(use_decl, &module_path));
    let first = qualified.next()?;
    qualified.next().is_none().then_some(first)
}

fn source_alias_is_colliding(
    module: &SurfaceModule,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    module.uses.iter().any(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && use_decl.alias == module_path
            && use_decl_has_source_alias_collision(module, use_decl)
    })
}

pub(crate) fn select_use_decl_for_path<'a>(
    uses: impl Iterator<Item = &'a UseDecl>,
    module_path: &str,
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let candidates = uses
        .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
        .collect::<Vec<_>>();
    let mut exact = candidates.iter().copied().filter(|use_decl| {
        use_decl_matches_exact_import_path(use_decl, module_path, current_module)
    });
    if let Some(first) = exact.next() {
        return exact.next().is_none().then_some(first);
    }
    let mut qualified = candidates
        .into_iter()
        .filter(|use_decl| simple_import_alias_matches(use_decl, module_path));
    let first = qualified.next()?;
    qualified.next().is_none().then_some(first)
}

pub(crate) fn schema_composition_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    normal_imported_use_for_path(module, segments, current_module).or_else(|| {
        let module_path = segments.join("::");
        let mut candidates = module.uses.iter().filter(|use_decl| {
            use_decl.package.is_none()
                && use_decl.module_name.as_deref() == current_module
                && !use_decl_is_quarantined(module, use_decl)
                && !use_decl.has_explicit_alias()
                && use_decl.alias == module_path
        });
        let candidate = candidates.next()?;
        candidates.next().is_none().then_some(candidate)
    })
}

pub(crate) fn resolved_import_module_name(
    use_decl: &UseDecl,
    current_module: Option<&str>,
) -> String {
    if (use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        || (use_decl.package.is_none()
            && current_module.is_some_and(|module| module.starts_with("std::"))))
        && !use_decl.name.starts_with("std::")
    {
        format!("std::{}", use_decl.name)
    } else {
        use_decl.name.clone()
    }
}

pub(crate) fn use_decl_matches_import_path(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    use_decl_matches_exact_import_path(use_decl, module_path, current_module)
        || (use_decl.module_name.as_deref() == current_module
            && simple_import_alias_matches(use_decl, module_path))
}

pub(crate) fn use_decl_matches_exact_import_path(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    if use_decl.module_name.as_deref() != current_module {
        return false;
    }
    if use_decl.written_name == module_path {
        return true;
    }
    if use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        && use_decl
            .written_name
            .strip_prefix("std::")
            .is_some_and(|package_relative| package_relative == module_path)
    {
        return true;
    }
    if use_decl.package.is_some()
        || !current_module.is_some_and(|module| module.starts_with("std::"))
    {
        return false;
    }
    use_decl
        .written_name
        .strip_prefix("std::")
        .is_some_and(|package_relative| package_relative == module_path)
}

fn simple_import_alias_matches(use_decl: &UseDecl, module_path: &str) -> bool {
    use_decl.alias == module_path
        && (use_decl.has_explicit_alias()
            || use_decl.package.is_some()
            || !use_decl.name.contains("::")
            || use_decl.origin == veln_ast::UseOrigin::ImplicitStandardPrelude)
}

pub(crate) fn use_decl_matches_implicit_import_alias(
    use_decl: &UseDecl,
    module_path: &str,
    current_module: Option<&str>,
) -> bool {
    use_decl.module_name.as_deref() == current_module
        && simple_import_alias_matches(use_decl, module_path)
}
