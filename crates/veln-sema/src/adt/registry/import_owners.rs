use super::*;

#[cfg(test)]
thread_local! {
    static OWNER_WORK: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_owner_work() {
    OWNER_WORK.set(OWNER_WORK.get() + 1);
}

#[derive(Clone, Debug, Default)]
pub(super) struct ImportOwners {
    modules: HashMap<String, Option<ResolvedImportOwner>>,
    unscoped: Option<ResolvedImportOwner>,
}

impl ImportOwners {
    fn new(uses: &[UseDecl]) -> Self {
        let mut packages_by_module = HashMap::<Option<&str>, HashSet<Option<&str>>>::new();
        for use_decl in uses {
            #[cfg(test)]
            record_owner_work();
            packages_by_module
                .entry(use_decl.module_name.as_deref())
                .or_default()
                .insert(use_decl.importing_package.as_deref());
        }
        let mut owners = Self::default();
        for (module, packages) in packages_by_module {
            let owner = import_owner_from_packages(packages, module);
            if let Some(module) = module {
                owners.modules.insert(module.to_owned(), owner);
            } else {
                owners.unscoped = owner;
            }
        }
        owners
    }

    fn get(&self, module: Option<&str>) -> Option<ResolvedImportOwner> {
        #[cfg(test)]
        record_owner_work();
        match module {
            Some(module) => self.modules.get(module).cloned().flatten(),
            None => self.unscoped.clone(),
        }
    }
}

impl AdtRegistry {
    pub(crate) fn import_scope(&self) -> Arc<[UseDecl]> {
        self.import_owner_scope.clone()
    }

    pub(crate) fn bind_import_owner_scope(&mut self, uses: Arc<[UseDecl]>) {
        self.import_owners = Arc::new(ImportOwners::new(&uses));
        self.import_owner_scope = uses;
    }

    pub(super) fn import_owner_for_module(
        &self,
        uses: &[UseDecl],
        current_module: Option<&str>,
    ) -> Option<ResolvedImportOwner> {
        // The retained Arc keeps this exact immutable allocation alive, including
        // across registry clones; unrelated equal-length slices must use fallback.
        if std::ptr::eq(uses, self.import_owner_scope.as_ref()) {
            return self.import_owners.get(current_module);
        }
        import_owner_for_module(uses, current_module)
    }
}

fn import_owner_for_module(
    uses: &[UseDecl],
    current_module: Option<&str>,
) -> Option<ResolvedImportOwner> {
    let packages = uses
        .iter()
        .inspect(|_| {
            #[cfg(test)]
            record_owner_work();
        })
        .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
        .map(|use_decl| use_decl.importing_package.as_deref())
        .collect::<HashSet<_>>();
    import_owner_from_packages(packages, current_module)
}

fn import_owner_from_packages(
    mut packages: HashSet<Option<&str>>,
    current_module: Option<&str>,
) -> Option<ResolvedImportOwner> {
    let package = if packages.remove(&None) {
        None
    } else if packages.len() == 1 {
        packages.into_iter().next().flatten().map(str::to_string)
    } else {
        return None;
    };
    Some(ResolvedImportOwner {
        package,
        module: current_module.map(str::to_string),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module() -> veln_ast::SurfaceModule {
        let source = veln_source::SourceFile::new("scope.veln", "mod scope\nuse dependency\n");
        let parsed = veln_syntax::parse(&source);
        assert!(parsed.diagnostics.is_empty());
        veln_ast::lower_surface_ast(&parsed.tree)
    }

    fn uses(packages: &[Option<&str>]) -> Arc<[UseDecl]> {
        let prototype = module().uses.remove(0);
        packages
            .iter()
            .map(|package| {
                let mut use_decl = prototype.clone();
                use_decl.importing_package = package.map(str::to_owned);
                use_decl
            })
            .collect::<Vec<_>>()
            .into()
    }

    #[test]
    fn shared_import_owners_preserve_package_precedence_and_ambiguity() {
        for (packages, expected_package) in [
            (vec![], None),
            (vec![None], Some(None)),
            (vec![Some("first")], Some(Some("first"))),
            (vec![Some("first"), Some("first")], Some(Some("first"))),
            (vec![Some("first"), Some("second")], None),
            (vec![Some("first"), Some("second"), None], Some(None)),
        ] {
            let scope = uses(&packages);
            let mut registry = AdtRegistry::from_module(&module());
            registry.bind_import_owner_scope(scope.clone());
            let expected = expected_package.map(|package| ResolvedImportOwner {
                package: package.map(str::to_owned),
                module: Some("scope".to_owned()),
            });
            assert_eq!(
                registry.import_owner_for_module(&scope, Some("scope")),
                expected
            );
            assert_eq!(
                registry.import_owner_for_module(&scope, Some("missing")),
                None
            );
            assert_eq!(registry.import_owner_for_module(&scope, None), None);
        }
    }

    #[test]
    fn equal_length_foreign_imports_do_not_reuse_another_scopes_owner() {
        let scope = uses(&[Some("first")]);
        let mut registry = AdtRegistry::from_module(&module());
        registry.bind_import_owner_scope(scope.clone());
        let cloned = registry.clone();
        drop(registry);
        let registry = cloned;
        drop(scope);
        let foreign = uses(&[Some("second")]);
        assert_eq!(
            registry.import_owner_for_module(&foreign, Some("scope")),
            Some(ResolvedImportOwner {
                package: Some("second".to_owned()),
                module: Some("scope".to_owned()),
            })
        );
        assert_eq!(
            registry.import_owner_for_module(&registry.import_owner_scope, Some("scope")),
            Some(ResolvedImportOwner {
                package: Some("first".to_owned()),
                module: Some("scope".to_owned()),
            })
        );
    }

    #[test]
    fn shared_owner_lookup_work_includes_one_build_and_constant_probes() {
        let work = [16, 32, 64].map(|count| {
            let scope = uses(&vec![Some("first"); count]);
            let mut registry = AdtRegistry::from_module(&module());
            OWNER_WORK.set(0);
            registry.bind_import_owner_scope(scope.clone());
            for _ in 0..count {
                let owner = registry
                    .import_owner_for_module(&scope, Some("scope"))
                    .unwrap();
                assert_eq!(owner.package.as_deref(), Some("first"));
            }
            let work = OWNER_WORK.get();
            assert!(work >= 2 * count);
            eprintln!("import owners: imports={count} queries={count} work={work}");
            work
        });
        assert_eq!(work[2] - work[1], 2 * (work[1] - work[0]), "{work:?}");
    }
}
