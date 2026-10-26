use std::collections::{BTreeMap, BTreeSet};

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
    let quarantined = module
        .uses
        .iter()
        .map(|use_decl| use_decl_is_quarantined(module, use_decl))
        .collect::<Vec<_>>();
    let mut qualifier_groups = BTreeMap::<(Option<String>, String), (usize, bool)>::new();
    let mut exact_groups = BTreeMap::<(Option<String>, String), (usize, bool)>::new();
    for (use_decl, quarantined) in module.uses.iter().zip(&quarantined) {
        if use_decl.origin == veln_ast::UseOrigin::Source
            && !use_decl.recovered
            && !use_decl_has_reserved_alias(use_decl)
        {
            let group = qualifier_groups
                .entry((use_decl.module_name.clone(), use_decl.alias.clone()))
                .or_default();
            group.0 += 1;
            group.1 |= use_decl.has_explicit_alias();
        }
        if use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.recovered {
            let mut exact_paths = vec![use_decl.written_name.clone()];
            if (use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
                || (use_decl.package.is_none()
                    && use_decl
                        .module_name
                        .as_deref()
                        .is_some_and(|module| module.starts_with("std::"))))
                && let Some(relative) = use_decl.written_name.strip_prefix("std::")
            {
                exact_paths.push(relative.to_string());
            }
            for path in exact_paths {
                let group = exact_groups
                    .entry((use_decl.module_name.clone(), path))
                    .or_default();
                group.0 += 1;
                group.1 |= *quarantined;
            }
        }
    }
    let colliding_qualifiers = qualifier_groups
        .into_iter()
        .filter_map(|(key, (count, has_explicit))| (count > 1 && has_explicit).then_some(key))
        .collect::<BTreeSet<_>>();
    let blocked_exact_paths = exact_groups
        .into_iter()
        .filter_map(|(key, (count, quarantined))| (count != 1 || quarantined).then_some(key))
        .collect::<BTreeSet<_>>();
    module
        .uses
        .iter()
        .zip(quarantined)
        .filter(|(_, quarantined)| !*quarantined)
        .map(|(use_decl, _)| {
            let mut resolved = resolved_import_use_decl(use_decl);
            let qualifier = (use_decl.module_name.clone(), use_decl.alias.clone());
            if colliding_qualifiers.contains(&qualifier) || blocked_exact_paths.contains(&qualifier)
            {
                resolved.alias.clear();
            }
            resolved
        })
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
    let exact = module
        .uses
        .iter()
        .filter(|use_decl| {
            !use_decl.recovered
                && use_decl_matches_exact_import_path(use_decl, &module_path, current_module)
        })
        .collect::<Vec<_>>();
    if !exact.is_empty() {
        return (exact.len() == 1 && !use_decl_is_quarantined(module, exact[0]))
            .then_some(exact[0]);
    }
    let candidates = module
        .uses
        .iter()
        .filter(|use_decl| !use_decl_is_quarantined(module, use_decl))
        .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
        .collect::<Vec<_>>();
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
