use super::*;

impl FunctionChecker<'_> {
    pub(in crate::analysis::body) fn projection_local_context(
        &self,
        name: &str,
    ) -> Option<crate::aggregate_type_join::LocalContext> {
        let index = self.visible_binding_index(name)?;
        let current = self.binding_type(index);
        Some(
            if matches!(current, Type::Record(ref fields) if fields.is_empty()) {
                crate::aggregate_type_join::LocalContext::EmptyRecord
            } else if type_contains_unknown(&current) {
                crate::aggregate_type_join::LocalContext::UnknownBearing
            } else {
                crate::aggregate_type_join::LocalContext::Complete
            },
        )
    }
}
