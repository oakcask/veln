type PackageSchemaIdentity<'a> = (&'a str, &'a str, &'a str);
type PackageSchemaAliasIdentity<'a> = PackageSchemaIdentity<'a>;

struct PackageSchemaDeclarations<'a> {
    alias_counts: BTreeMap<PackageSchemaIdentity<'a>, usize>,
    exported_aliases: BTreeSet<PackageSchemaIdentity<'a>>,
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
            alias_counts: direct_dependency_alias_counts(aliases),
            exported_aliases: direct_dependency_exported_aliases(aliases),
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
    eligible_aliases: BTreeSet<PackageSchemaAliasIdentity<'a>>,
}

impl<'a> PackageSchemaAliasEligibility<'a> {
    fn new(
        declarations: &'a PackageSchemaDeclarations<'a>,
        resolved_aliases: &'a [ResolvedPackageSchemaAlias],
    ) -> Self {
        let resolved_aliases = resolved_package_schema_alias_index(resolved_aliases);
        let eligible_aliases = eligible_package_schema_aliases(declarations, &resolved_aliases);
        Self { eligible_aliases }
    }

    fn contains(&self, alias: &NeutralSymbol) -> bool {
        let Some(package) = alias.package.as_deref() else {
            return false;
        };
        self.eligible_aliases
            .contains(&(package, alias.module.as_str(), alias.name.as_str()))
    }
}

fn eligible_package_schema_aliases<'a>(
    declarations: &'a PackageSchemaDeclarations<'a>,
    resolved_aliases: &BTreeMap<PackageSchemaAliasIdentity<'a>, &'a ResolvedPackageSchemaAlias>,
) -> BTreeSet<PackageSchemaAliasIdentity<'a>> {
    let mut eligibility = BTreeMap::<PackageSchemaAliasIdentity<'a>, bool>::new();
    for &identity in resolved_aliases.keys() {
        if !eligibility.contains_key(&identity) {
            resolve_package_schema_alias_eligibility(
                identity,
                declarations,
                resolved_aliases,
                &mut eligibility,
            );
        }
    }
    eligibility
        .into_iter()
        .filter_map(|(identity, eligible)| eligible.then_some(identity))
        .collect()
}

enum PackageAliasStep<'a> {
    Alias(PackageSchemaAliasIdentity<'a>),
    Target(bool),
    Invalid,
}

fn resolve_package_schema_alias_eligibility<'a>(
    mut identity: PackageSchemaAliasIdentity<'a>,
    declarations: &PackageSchemaDeclarations<'a>,
    resolved_aliases: &BTreeMap<PackageSchemaAliasIdentity<'a>, &'a ResolvedPackageSchemaAlias>,
    eligibility: &mut BTreeMap<PackageSchemaAliasIdentity<'a>, bool>,
) {
    let mut path = Vec::new();
    let mut visited = BTreeSet::new();
    let outcome = loop {
        #[cfg(test)]
        record_schema_alias_eligibility_visit();
        if let Some(&known) = eligibility.get(&identity) {
            break known;
        }
        if !visited.insert(identity) {
            break false;
        }
        path.push(identity);
        match package_schema_alias_step(identity, declarations, resolved_aliases) {
            PackageAliasStep::Alias(next) => {
                if let Some(&known) = eligibility.get(&next) {
                    break known;
                }
                identity = next;
            }
            PackageAliasStep::Target(eligible) => break eligible,
            PackageAliasStep::Invalid => break false,
        }
    };
    for path_identity in path {
        eligibility.insert(path_identity, outcome);
    }
}

fn package_schema_alias_step<'a>(
    identity: PackageSchemaAliasIdentity<'a>,
    declarations: &PackageSchemaDeclarations<'a>,
    resolved_aliases: &BTreeMap<PackageSchemaAliasIdentity<'a>, &'a ResolvedPackageSchemaAlias>,
) -> PackageAliasStep<'a> {
    let Some(&current) = resolved_aliases.get(&identity) else {
        return PackageAliasStep::Invalid;
    };
    if !declarations.contains_alias(&identity)
        || !declarations.exported_aliases.contains(&identity)
    {
        return PackageAliasStep::Invalid;
    }
    let Some(target_module) = current.direct_target_module.as_deref() else {
        return PackageAliasStep::Invalid;
    };
    let target = (identity.0, target_module, current.direct_target_name.as_str());
    if current.direct_target_is_alias {
        PackageAliasStep::Alias(target)
    } else {
        PackageAliasStep::Target(
            current.target_exported && declarations.contains_schema(&target),
        )
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

fn direct_dependency_exported_aliases(
    aliases: &[PackageSchemaAliasDeclaration],
) -> BTreeSet<PackageSchemaIdentity<'_>> {
    let mut exported = BTreeSet::new();
    for alias in aliases
        .iter()
        .filter(|alias| alias.package_origin == PackageOrigin::DirectDependency)
    {
        #[cfg(test)]
        record_schema_alias_declaration_visit();
        if alias.exported {
            exported.insert((
                alias.package.as_str(),
                alias.module.as_str(),
                alias.name.as_str(),
            ));
        }
    }
    exported
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
