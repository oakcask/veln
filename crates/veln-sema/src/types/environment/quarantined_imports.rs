use super::*;

impl TypeEnvironment {
    pub(crate) fn quarantined_import_call_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: usize,
    ) -> usize {
        let Some((use_decl, name)) = self.quarantined_import_for_segments(segments, current_module)
        else {
            return 0;
        };
        let module_name = use_decl.name.as_str();
        self.functions_named(name)
            .filter(|function| {
                function.module_name.as_deref() == Some(module_name)
                    && function_signature_accepts_arg_count(function, arg_count)
                    && function.visibility == Visibility::Public
                    && !self.imported_codec_helper_is_hidden(function, use_decl)
            })
            .count()
    }

    pub(crate) fn quarantined_import_value_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> usize {
        let Some((use_decl, name)) = self.quarantined_import_for_segments(segments, current_module)
        else {
            return 0;
        };
        let module_name = use_decl.name.as_str();
        self.functions_named(name)
            .filter(|function| {
                function.module_name.as_deref() == Some(module_name)
                    && function.visibility == Visibility::Public
                    && !self.imported_codec_helper_is_hidden(function, use_decl)
            })
            .count()
    }

    pub(super) fn quarantined_import_effect_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> usize {
        let Some((use_decl, name)) = self.quarantined_import_for_segments(segments, current_module)
        else {
            return 0;
        };
        let module_name = use_decl.name.as_str();
        self.effects
            .iter()
            .filter(|effect| {
                effect.name == name
                    && effect.module_name.as_deref() == Some(module_name)
                    && effect.visibility == Visibility::Public
            })
            .count()
    }

    pub(super) fn quarantined_import_handler_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> usize {
        let Some((use_decl, name)) = self.quarantined_import_for_segments(segments, current_module)
        else {
            return 0;
        };
        let module_name = use_decl.name.as_str();
        self.handlers
            .iter()
            .filter(|handler| {
                handler.name == name
                    && handler.module_name.as_deref() == Some(module_name)
                    && handler.visibility == Visibility::Public
            })
            .count()
    }

    pub(crate) fn quarantined_import_constructor_recovery_candidate_count(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        arg_count: Option<usize>,
    ) -> usize {
        let [alias, constructor_segments @ ..] = segments else {
            return 0;
        };
        if constructor_segments.is_empty() {
            return 0;
        }
        self.import_constructor_recoveries
            .iter()
            .filter(|(key, _)| {
                key.current_module.as_deref() == current_module
                    && key.alias == *alias
                    && key.constructor_segments == constructor_segments
                    && arg_count.is_none_or(|count| key.field_count == count)
            })
            .map(|(_, count)| *count)
            .sum()
    }

    #[cfg(test)]
    pub(crate) fn quarantined_import_type_recovery_candidate_count(
        &self,
        type_name: &str,
        current_module: Option<&str>,
        args_len: usize,
    ) -> usize {
        let segments = type_name
            .split("::")
            .map(str::to_string)
            .collect::<Vec<_>>();
        let Some((use_decl, name)) =
            self.quarantined_import_for_segments(&segments, current_module)
        else {
            return 0;
        };
        let module_name = Some(use_decl.name.as_str());
        self.type_symbols
            .iter()
            .filter(|symbol| {
                symbol.name == name
                    && symbol.module_name.as_deref() == module_name
                    && self.symbol_is_visible(*symbol, module_name, current_module)
                    && self
                        .adts
                        .descriptor_for_type_path(
                            type_name,
                            args_len,
                            current_module,
                            &self.quarantined_uses,
                        )
                        .is_some()
            })
            .count()
    }

    fn quarantined_import_for_segments<'a>(
        &'a self,
        segments: &'a [String],
        current_module: Option<&str>,
    ) -> Option<(&'a UseDecl, &'a str)> {
        match segments {
            [_, .., name] => imported_use_for_path(
                &self.quarantined_uses,
                &segments[..segments.len() - 1],
                current_module,
            )
            .map(|use_decl| (use_decl, name.as_str())),
            _ => None,
        }
    }
}

fn function_signature_accepts_arg_count(function: &FunctionSignature, arg_count: usize) -> bool {
    if function.variadic.is_some() {
        arg_count >= function.params.len()
    } else {
        arg_count == function.params.len()
    }
}
