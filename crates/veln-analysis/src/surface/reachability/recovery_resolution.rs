use super::*;

impl<'a> ReachableInvalidNameSelector<'a> {
    pub(super) fn has_valid_function(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> bool {
        self.visible_functions(segments, current_module)
            .into_iter()
            .any(|function| {
                function
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
                    && arg_count
                        .is_none_or(|count| function_shape(function).accepts_arg_count(count))
            })
    }

    pub(super) fn has_valid_function_alias(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        self.visible_aliases(segments, current_module, PublicAliasKind::Function)
            .into_iter()
            .any(|alias| {
                alias
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
            })
    }

    pub(super) fn has_valid_type_alias(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> bool {
        self.visible_aliases(segments, current_module, PublicAliasKind::Type)
            .into_iter()
            .any(|alias| {
                alias
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
            })
    }

    pub(super) fn has_valid_type(&self, segments: &[String], current_module: Option<&str>) -> bool {
        self.visible_types(segments, current_module)
            .into_iter()
            .any(|type_decl| {
                type_decl
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
            })
    }

    pub(super) fn has_valid_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> bool {
        self.visible_constructor_variants(segments, current_module)
            .into_iter()
            .any(|(type_decl, variant)| {
                type_decl
                    .name
                    .as_ref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                    && variant.name.as_ref().is_some_and(|name| {
                        name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                    })
                    && arg_count.is_none_or(|count| variant.fields.len() == count)
            })
    }

    pub(super) fn constructor_recovery_candidate(
        type_decl: &veln_ast::TypeDecl,
        variant: &veln_ast::TypeVariantDecl,
        arg_count: Option<usize>,
    ) -> bool {
        let invalid_type = type_decl
            .name
            .as_ref()
            .is_some_and(|name| !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase));
        let invalid_constructor = variant
            .name
            .as_ref()
            .is_some_and(|name| !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase));
        (invalid_type || invalid_constructor)
            && arg_count.is_none_or(|count| variant.fields.len() == count)
    }

    pub(super) fn constructor_recovery_spans(
        &self,
        type_decl: &veln_ast::TypeDecl,
        variant: &veln_ast::TypeVariantDecl,
    ) -> Vec<ReachableInvalidNameSpan> {
        self.invalid_names
            .iter()
            .copied()
            .filter(|invalid| {
                (invalid.class == veln_ast::NameClass::Type
                    && span_contains(&type_decl.span, &invalid.span)
                    && type_decl.name.as_deref() == Some(invalid.name.as_str()))
                    || (invalid.class == veln_ast::NameClass::Constructor
                        && span_contains(&variant.span, &invalid.span)
                        && variant.name.as_deref() == Some(invalid.name.as_str()))
            })
            .map(|invalid| ReachableInvalidNameSpan::Name(invalid.span.clone()))
            .collect()
    }

    pub(super) fn function_recovery_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> Vec<ReachableRecoveryCandidate> {
        self.visible_functions(segments, current_module)
            .into_iter()
            .filter(|function| {
                function.name.as_ref().is_some_and(|name| {
                    !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                }) && arg_count
                    .is_none_or(|count| function_shape(function).accepts_arg_count(count))
            })
            .map(|function| {
                ReachableRecoveryCandidate::new(vec![ReachableInvalidNameSpan::Declaration(
                    function.span.clone(),
                )])
            })
            .chain(
                self.visible_aliases(segments, current_module, PublicAliasKind::Function)
                    .into_iter()
                    .filter(|alias| {
                        alias.name.as_ref().is_some_and(|name| {
                            !name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
                        })
                    })
                    .map(|alias| {
                        ReachableRecoveryCandidate::new(vec![
                            ReachableInvalidNameSpan::Declaration(alias.span.clone()),
                        ])
                    }),
            )
            .collect()
    }

    pub(super) fn select_unique_type_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let candidates = self
            .visible_types(segments, current_module)
            .into_iter()
            .filter(|type_decl| {
                type_decl.name.as_ref().is_some_and(|name| {
                    !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                })
            })
            .map(|type_decl| type_decl.span.clone())
            .map(ReachableInvalidNameSpan::Declaration)
            .chain(
                self.visible_aliases(segments, current_module, PublicAliasKind::Type)
                    .into_iter()
                    .filter(|alias| {
                        alias.name.as_ref().is_some_and(|name| {
                            !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
                        })
                    })
                    .map(|alias| ReachableInvalidNameSpan::Declaration(alias.span.clone())),
            )
            .collect::<Vec<_>>();
        push_unique_reachable_invalid_name_span(candidates, spans);
    }

    pub(super) fn select_unique_constructor_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let candidates = self.constructor_recovery_candidates(segments, current_module, arg_count);
        push_unique_constructor_recovery_spans(candidates, spans);
    }

    pub(super) fn constructor_recovery_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> Vec<ReachableRecoveryCandidate> {
        self.visible_constructor_variants(segments, current_module)
            .into_iter()
            .filter(|(type_decl, variant)| {
                Self::constructor_recovery_candidate(type_decl, variant, arg_count)
            })
            .map(|(type_decl, variant)| {
                ReachableRecoveryCandidate::new(self.constructor_recovery_spans(type_decl, variant))
            })
            .collect()
    }

    pub(super) fn select_unique_value_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let mut candidates = self.constructor_recovery_candidates(segments, current_module, None);
        candidates.extend(self.function_recovery_candidates(segments, current_module, None));
        push_unique_constructor_recovery_spans(candidates, spans);
    }

    pub(super) fn select_unique_call_recovery(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: usize,
        spans: &mut Vec<ReachableInvalidNameSpan>,
    ) {
        let mut candidates =
            self.function_recovery_candidates(segments, current_module, Some(arg_count));
        candidates.extend(self.constructor_recovery_candidates(
            segments,
            current_module,
            Some(arg_count),
        ));
        push_unique_constructor_recovery_spans(candidates, spans);
    }

    pub(super) fn visible_functions(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&'a Function> {
        self.visible_named_candidates(
            &self.functions_by_name,
            segments,
            current_module,
            |function, target| {
                function.kind == FunctionKind::Function
                    && declaration_visible(
                        function.module_name.as_deref(),
                        function.visibility,
                        target,
                        current_module,
                    )
            },
        )
    }

    pub(super) fn visible_aliases(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        kind: PublicAliasKind,
    ) -> Vec<&'a veln_ast::PublicAlias> {
        self.visible_named_candidates(
            &self.aliases_by_name,
            segments,
            current_module,
            |alias, target| {
                alias.kind == kind
                    && declaration_visible(
                        alias.module_name.as_deref(),
                        Visibility::Public,
                        target,
                        current_module,
                    )
            },
        )
    }

    pub(super) fn visible_types(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<&'a veln_ast::TypeDecl> {
        self.visible_named_candidates(
            &self.types_by_name,
            segments,
            current_module,
            |type_decl, target| {
                declaration_visible(
                    type_decl.module_name.as_deref(),
                    type_decl.visibility,
                    target,
                    current_module,
                )
            },
        )
    }

    pub(super) fn visible_constructor_variants(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Vec<(&'a veln_ast::TypeDecl, &'a veln_ast::TypeVariantDecl)> {
        self.visible_named_candidates(
            &self.constructors_by_name,
            segments,
            current_module,
            |(type_decl, _), target| {
                declaration_visible(
                    type_decl.module_name.as_deref(),
                    type_decl.visibility,
                    target,
                    current_module,
                )
            },
        )
    }

    fn visible_named_candidates<T: Copy>(
        &self,
        index: &HashMap<(Option<String>, String), Vec<T>>,
        segments: &[String],
        current_module: Option<&str>,
        mut visible: impl FnMut(T, Option<&str>) -> bool,
    ) -> Vec<T> {
        let target = visible_path_target(&self.uses, segments, current_module);
        let Some(leaf) = path_leaf(segments).map(str::to_string) else {
            return Vec::new();
        };
        index
            .get(&(target.clone(), leaf))
            .into_iter()
            .flatten()
            .copied()
            .inspect(|_| {
                #[cfg(test)]
                reachability_counters::record_recovery_selector_candidate_scan();
            })
            .filter(|candidate| visible(*candidate, target.as_deref()))
            .collect()
    }

    pub(super) fn visible_handler(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&'a veln_ast::HandlerDecl> {
        let target = visible_path_target(&self.uses, segments, current_module);
        self.handlers.iter().copied().find(|handler| {
            handler.name.as_deref() == path_leaf(segments)
                && (declaration_visible(
                    handler.module_name.as_deref(),
                    handler.visibility,
                    target.as_deref(),
                    current_module,
                ) || companion_target_handler_visible(
                    handler,
                    target.as_deref(),
                    current_module,
                    &self.companion_access_targets,
                ))
        })
    }
}

pub(super) fn companion_target_handler_visible(
    handler: &veln_ast::HandlerDecl,
    target_module: Option<&str>,
    current_module: Option<&str>,
    companion_access_targets: &HashMap<String, String>,
) -> bool {
    if handler.visibility == Visibility::Public || target_module != handler.module_name.as_deref() {
        return false;
    }
    current_module.is_some_and(|current_module| {
        handler.module_name.as_ref().is_some_and(|handler_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed_target| allowed_target == handler_module)
        })
    })
}

impl FunctionShape {
    pub(super) fn accepts_arg_count(&self, arg_count: usize) -> bool {
        self.variadic.is_some() && arg_count >= self.fixed_arity
            || self.variadic.is_none() && arg_count == self.fixed_arity
    }
}

pub(super) fn visible_path_target(
    uses: &[&UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<String> {
    match segments {
        [_] => current_module.map(str::to_string),
        [_, .., _] => imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
            .map(|use_decl| use_decl.name.clone()),
        _ => None,
    }
}

pub(super) fn path_leaf(segments: &[String]) -> Option<&str> {
    segments.last().map(String::as_str)
}

pub(super) fn same_module_recovery_path(segments: &[String]) -> bool {
    matches!(segments, [_])
}

pub(super) fn declaration_visible(
    declaration_module: Option<&str>,
    visibility: Visibility,
    target_module: Option<&str>,
    current_module: Option<&str>,
) -> bool {
    match target_module {
        Some(target_module) if Some(target_module) != current_module => {
            declaration_module == Some(target_module) && visibility == Visibility::Public
        }
        Some(target_module) => declaration_module == Some(target_module),
        None => current_module.is_none() && declaration_module.is_none(),
    }
}

pub(super) fn push_unique_reachable_invalid_name_span(
    mut candidates: Vec<ReachableInvalidNameSpan>,
    spans: &mut Vec<ReachableInvalidNameSpan>,
) {
    dedup_reachable_invalid_name_spans(&mut candidates);
    if let [span] = candidates.as_slice() {
        spans.push(span.clone());
    }
}

pub(super) fn push_unique_constructor_recovery_spans(
    candidates: Vec<ReachableRecoveryCandidate>,
    spans: &mut Vec<ReachableInvalidNameSpan>,
) {
    if let [candidate] = candidates.as_slice() {
        let mut candidate_spans = candidate.spans.clone();
        dedup_reachable_invalid_name_spans(&mut candidate_spans);
        spans.extend(candidate_spans);
    }
}

pub(super) fn dedup_reachable_invalid_name_spans(spans: &mut Vec<ReachableInvalidNameSpan>) {
    let mut seen = Vec::<ReachableInvalidNameSpan>::new();
    spans.retain(|span| {
        if seen.iter().any(|known| known == span) {
            false
        } else {
            seen.push(span.clone());
            true
        }
    });
}

pub(super) fn collect_pattern_binding_names(pattern: &Pattern, bindings: &mut Vec<String>) {
    pattern.for_each_binding(&mut |name| bindings.push(name.to_string()));
}

pub(super) fn span_contains(container: &SourceSpan, span: &SourceSpan) -> bool {
    container.file == span.file
        && container.start.offset <= span.start.offset
        && span.end.offset <= container.end.offset
}
