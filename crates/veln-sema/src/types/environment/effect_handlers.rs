use super::*;

impl TypeEnvironment {
    #[cfg(test)]
    pub(crate) fn user_effect_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> Option<&EffectSignature> {
        self.resolve_user_effect_path(segments, current_module)
            .found()
    }

    pub(crate) fn user_effect_path_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> Option<&EffectSignature> {
        self.resolve_user_effect_path_in_owner(segments, current_module, current_package)
            .found()
    }

    #[cfg(test)]
    pub(crate) fn resolve_user_effect_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> UserEffectPathResolution<'_> {
        self.resolve_user_effect_path_in_owner(segments, current_module, None)
    }

    pub(crate) fn resolve_user_effect_path_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> UserEffectPathResolution<'_> {
        match segments {
            [name] => self
                .effects
                .iter()
                .find(|effect| {
                    (effect.qualified_name == *name
                        || (effect.name == *name
                            && effect.module_name.as_deref() == current_module))
                        && effect.package_name.as_deref() == current_package
                })
                .map_or(
                    UserEffectPathResolution::Missing,
                    UserEffectPathResolution::Found,
                ),
            [_, .., name] => self.resolve_qualified_user_effect_path(
                segments,
                name,
                current_module,
                current_package,
            ),
            _ => UserEffectPathResolution::Missing,
        }
    }

    fn resolve_qualified_user_effect_path(
        &self,
        segments: &[String],
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> UserEffectPathResolution<'_> {
        let Some(use_decl) = imported_use_for_path_in_owner(
            &self.uses,
            &segments[..segments.len() - 1],
            current_module,
            current_package,
        ) else {
            return self.missing_or_quarantined_effect_path(
                segments,
                current_module,
                current_package,
            );
        };
        let module_name = use_decl.name.as_str();
        let Some(effect) = self.effects.iter().find(|effect| {
            effect.name == name
                && effect.module_name.as_deref() == Some(module_name)
                && effect.package_name == use_decl.resolved_target().package
        }) else {
            return UserEffectPathResolution::Missing;
        };
        if imported_effect_is_visible(
            use_decl,
            current_module,
            module_name,
            effect.visibility,
            &self.companion_effect_access_targets,
        ) {
            return UserEffectPathResolution::Found(effect);
        }
        self.private_effect_path_mismatch(effect, use_decl, current_module)
    }

    fn missing_or_quarantined_effect_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> UserEffectPathResolution<'_> {
        if self.quarantined_import_effect_recovery_candidate_count(
            segments,
            current_module,
            current_package,
        ) == 1
        {
            UserEffectPathResolution::QuarantinedImportTarget
        } else {
            UserEffectPathResolution::Missing
        }
    }

    fn private_effect_path_mismatch<'a>(
        &'a self,
        effect: &'a EffectSignature,
        use_decl: &UseDecl,
        current_module: Option<&str>,
    ) -> UserEffectPathResolution<'a> {
        if effect.visibility != Visibility::Public
            && use_decl.package.is_none()
            && let Some(access) =
                current_module.and_then(|module| self.companion_effect_access_targets.get(module))
            && access.target_module != use_decl.name
        {
            return UserEffectPathResolution::PrivateCompanionTargetMismatch { effect, access };
        }
        UserEffectPathResolution::Missing
    }

    pub(crate) fn visible_user_effects(
        &self,
        current_module: Option<&str>,
    ) -> Vec<&EffectSignature> {
        self.effects
            .iter()
            .filter(|effect| {
                effect.module_name.as_deref() == current_module
                    || effect.visibility == Visibility::Public
                    || current_module
                        .and_then(|module| self.companion_effect_access_targets.get(module))
                        .is_some_and(|access| {
                            effect.module_name.as_deref() == Some(access.target_module.as_str())
                        })
            })
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn handler_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
    ) -> HandlerPathResolution<'_> {
        self.handler_path_in_owner(segments, current_module, None)
    }

    pub(crate) fn handler_path_in_owner(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> HandlerPathResolution<'_> {
        match segments {
            [name] => self.unqualified_handler_path(name, current_module, current_package),
            [_, .., name] => {
                self.qualified_handler_path(segments, name, current_module, current_package)
            }
            _ => HandlerPathResolution::Missing,
        }
    }

    fn unqualified_handler_path(
        &self,
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> HandlerPathResolution<'_> {
        self.handlers
            .iter()
            .find(|handler| {
                handler.name == name
                    && handler.module_name.as_deref() == current_module
                    && handler.package_name.as_deref() == current_package
            })
            .map_or(HandlerPathResolution::Missing, HandlerPathResolution::Found)
    }

    fn qualified_handler_path(
        &self,
        segments: &[String],
        name: &str,
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> HandlerPathResolution<'_> {
        let use_decl = imported_use_for_path_in_owner(
            &self.uses,
            &segments[..segments.len() - 1],
            current_module,
            current_package,
        );
        let Some(use_decl) = use_decl else {
            return self.missing_or_quarantined_handler_path(
                segments,
                current_module,
                current_package,
            );
        };
        let Some(handler) = self.handlers.iter().find(|handler| {
            handler.name == name
                && handler.module_name.as_deref() == Some(use_decl.name.as_str())
                && handler.package_name == use_decl.resolved_target().package
        }) else {
            return HandlerPathResolution::Missing;
        };
        if imported_handler_is_visible(
            handler,
            use_decl,
            current_module,
            &self.companion_effect_access_targets,
        ) {
            return HandlerPathResolution::Found(handler);
        }
        self.private_handler_path_mismatch(handler, use_decl, current_module)
    }

    fn missing_or_quarantined_handler_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> HandlerPathResolution<'_> {
        if self.quarantined_import_handler_recovery_candidate_count(
            segments,
            current_module,
            current_package,
        ) == 1
        {
            HandlerPathResolution::QuarantinedImportTarget
        } else {
            HandlerPathResolution::Missing
        }
    }

    fn private_handler_path_mismatch<'a>(
        &'a self,
        handler: &'a HandlerSignature,
        use_decl: &UseDecl,
        current_module: Option<&str>,
    ) -> HandlerPathResolution<'a> {
        if handler.visibility != Visibility::Public
            && use_decl.package.is_none()
            && let Some(access) =
                current_module.and_then(|module| self.companion_effect_access_targets.get(module))
            && access.target_module != use_decl.name
        {
            return HandlerPathResolution::PrivateCompanionTargetMismatch { handler, access };
        }
        HandlerPathResolution::Missing
    }
}
