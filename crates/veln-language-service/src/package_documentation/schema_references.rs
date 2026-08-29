use super::*;

impl SchemaDocResolver<'_> {
    pub(super) fn resolve(
        &self,
        target: &str,
        current_module: &str,
    ) -> Option<PublicSchemaDocTarget> {
        let segments = target.split("::").map(str::to_string).collect::<Vec<_>>();
        self.resolve_segments(&segments, current_module, &mut Vec::new())
    }

    pub(super) fn resolve_segments(
        &self,
        segments: &[String],
        current_module: &str,
        visited_aliases: &mut Vec<(String, String)>,
    ) -> Option<PublicSchemaDocTarget> {
        match segments {
            [name] => self.resolve_in_module(current_module, name, visited_aliases),
            [module @ .., name] => {
                let module_name = module.join("::");
                let source = self.sources.get(current_module)?;
                if !source
                    .tree
                    .uses
                    .iter()
                    .any(|use_decl| use_decl.package.is_none() && use_decl.name == module_name)
                {
                    return None;
                }
                self.resolve_in_module(&module_name, name, visited_aliases)
            }
            _ => None,
        }
    }

    pub(super) fn resolve_in_module(
        &self,
        module_name: &str,
        name: &str,
        visited_aliases: &mut Vec<(String, String)>,
    ) -> Option<PublicSchemaDocTarget> {
        let key = (module_name.to_string(), name.to_string());
        if let Some(target) = self.schemas.get(&key) {
            return Some(target.clone());
        }
        let target = self.aliases.get(&key)?;
        if visited_aliases.contains(&key) {
            return None;
        }
        visited_aliases.push(key);
        let resolved = self.resolve_segments(target, module_name, visited_aliases);
        visited_aliases.pop();
        resolved
    }
}
