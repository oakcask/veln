type PackageSchemaIdentity<'a> = (&'a str, &'a str, &'a str);
type PackageSchemaAliasIdentity<'a> = PackageSchemaIdentity<'a>;

struct PackageSchemaDeclarations<'a> {
    aliases: &'a [PackageSchemaAliasDeclaration],
    alias_counts: BTreeMap<PackageSchemaIdentity<'a>, usize>,
    recovered_targets: BTreeSet<PackageSchemaIdentity<'a>>,
    target_counts: BTreeMap<PackageSchemaIdentity<'a>, (usize, bool)>,
}

impl<'a> PackageSchemaDeclarations<'a> {
    fn new(
        aliases: &'a [PackageSchemaAliasDeclaration],
        targets: &'a [PackageSchemaTarget],
        recovered_targets: &'a [PackageSchemaTarget],
    ) -> Self {
        Self {
            aliases,
            alias_counts: direct_dependency_alias_counts(aliases),
            recovered_targets: direct_dependency_recovered_targets(recovered_targets),
            target_counts: direct_dependency_target_counts(targets),
        }
    }

    fn contains_schema(&self, identity: &PackageSchemaIdentity<'_>) -> bool {
        self.target_counts.get(identity) == Some(&(1, true))
            && !self.recovered_targets.contains(identity)
            && !self.alias_counts.contains_key(identity)
    }

    fn contains_alias(&self, identity: &PackageSchemaIdentity<'_>) -> bool {
        self.alias_counts.get(identity) == Some(&1)
            && !self.recovered_targets.contains(identity)
            && !self.target_counts.contains_key(identity)
    }
}

struct PackageSchemaAliasEligibility<'a> {
    declarations: &'a PackageSchemaDeclarations<'a>,
    resolved_aliases:
        BTreeMap<PackageSchemaAliasIdentity<'a>, &'a ResolvedPackageSchemaAlias>,
}

impl<'a> PackageSchemaAliasEligibility<'a> {
    fn new(
        declarations: &'a PackageSchemaDeclarations<'a>,
        resolved_aliases: &'a [ResolvedPackageSchemaAlias],
    ) -> Self {
        Self {
            declarations,
            resolved_aliases: resolved_package_schema_alias_index(resolved_aliases),
        }
    }

    fn contains(&self, alias: &NeutralSymbol) -> bool {
        #[cfg(test)]
        record_schema_alias_declaration_visit();
        let Some(package) = alias.package.as_deref() else {
            return false;
        };
        let Some(resolved_alias) = self.resolved_aliases.get(&(
            package,
            alias.module.as_str(),
            alias.name.as_str(),
        )) else {
            return false;
        };
        let Some(target_module) = resolved_alias.target_module.as_deref() else {
            return false;
        };
        if !resolved_alias.target_exported {
            return false;
        }
        let alias_identity = (package, alias.module.as_str(), alias.name.as_str());
        let target_identity = (package, target_module, resolved_alias.target_name.as_str());
        self.chain_is_eligible(resolved_alias, alias_identity, target_identity)
    }

    fn chain_is_eligible(
        &self,
        initial: &ResolvedPackageSchemaAlias,
        initial_identity: PackageSchemaIdentity<'_>,
        final_target: PackageSchemaIdentity<'_>,
    ) -> bool {
        let mut current = initial;
        let mut identity = initial_identity;
        let mut visited = BTreeSet::new();
        loop {
            if !visited.insert(identity)
                || !self.declarations.contains_alias(&identity)
                || !self.alias_is_exported(identity)
            {
                return false;
            }
            let next_identity = (
                identity.0,
                current.direct_target_module.as_deref(),
                current.direct_target_name.as_str(),
            );
            let Some(next_module) = next_identity.1 else {
                return false;
            };
            let next_identity = (next_identity.0, next_module, next_identity.2);
            if !current.direct_target_is_alias {
                return next_identity == final_target
                    && self.declarations.contains_schema(&next_identity);
            }
            let Some(next) = self
                .resolved_aliases
                .get(&(next_identity.0, next_module, next_identity.2))
            else {
                return false;
            };
            identity = next_identity;
            current = next;
        }
    }

    fn alias_is_exported(&self, identity: PackageSchemaIdentity<'_>) -> bool {
        self.declarations
            .alias_counts
            .get(&identity)
            .is_some_and(|count| *count == 1)
            && self
                .declarations
                .aliases
                .iter()
                .filter(|alias| {
                    alias.package == identity.0
                        && alias.module == identity.1
                        && alias.name == identity.2
                })
                .all(|alias| alias.exported)
    }
}

fn resolved_package_schema_alias_index(
    aliases: &[ResolvedPackageSchemaAlias],
) -> BTreeMap<PackageSchemaAliasIdentity<'_>, &ResolvedPackageSchemaAlias> {
    aliases
        .iter()
        .filter(|candidate| candidate.package_origin == PackageOrigin::DirectDependency)
        .filter_map(|candidate| {
            #[cfg(test)]
            record_schema_alias_declaration_visit();
            Some((
                (
                    candidate.package.as_str(),
                    candidate.alias_module.as_deref()?,
                    candidate.alias_name.as_str(),
                ),
                candidate,
            ))
        })
        .collect()
}

fn direct_dependency_alias_counts(
    aliases: &[PackageSchemaAliasDeclaration],
) -> BTreeMap<PackageSchemaIdentity<'_>, usize> {
    let mut counts = BTreeMap::new();
    for alias in aliases
        .iter()
        .filter(|alias| alias.package_origin == PackageOrigin::DirectDependency)
    {
        #[cfg(test)]
        record_schema_alias_declaration_visit();
        *counts
            .entry((
                alias.package.as_str(),
                alias.module.as_str(),
                alias.name.as_str(),
            ))
            .or_insert(0usize) += 1;
    }
    counts
}

fn direct_dependency_recovered_targets(
    targets: &[PackageSchemaTarget],
) -> BTreeSet<PackageSchemaIdentity<'_>> {
    targets
        .iter()
        .filter(|target| target.package_origin == PackageOrigin::DirectDependency)
        .map(|target| {
            #[cfg(test)]
            record_schema_alias_declaration_visit();
            (
                target.package.as_str(),
                target.module.as_str(),
                target.name.as_str(),
            )
        })
        .collect()
}

fn direct_dependency_target_counts(
    targets: &[PackageSchemaTarget],
) -> BTreeMap<PackageSchemaIdentity<'_>, (usize, bool)> {
    let mut counts = BTreeMap::new();
    for target in targets
        .iter()
        .filter(|target| target.package_origin == PackageOrigin::DirectDependency)
    {
        #[cfg(test)]
        record_schema_alias_declaration_visit();
        let entry = counts
            .entry((
                target.package.as_str(),
                target.module.as_str(),
                target.name.as_str(),
            ))
            .or_insert((0usize, false));
        entry.0 += 1;
        entry.1 |= target.public && target.exported;
    }
    counts
}
