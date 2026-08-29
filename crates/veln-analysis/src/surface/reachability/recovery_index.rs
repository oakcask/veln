use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum ReachableInvalidNameSpan {
    Declaration(SourceSpan),
    Name(SourceSpan),
}

#[derive(Clone, Debug)]
pub(super) struct ReachableRecoveryCandidate {
    pub(super) spans: Vec<ReachableInvalidNameSpan>,
}

impl ReachableRecoveryCandidate {
    pub(super) fn new(spans: Vec<ReachableInvalidNameSpan>) -> Self {
        Self { spans }
    }
}

impl ReachableInvalidNameSpan {
    pub(super) fn is_declaration(&self, span: &SourceSpan) -> bool {
        matches!(self, Self::Declaration(reachable) if reachable == span)
    }
}

pub(super) struct ReachableInvalidNameSelector<'a> {
    pub(super) uses: Vec<&'a UseDecl>,
    pub(super) invalid_uses: Vec<&'a UseDecl>,
    pub(super) handlers: Vec<&'a veln_ast::HandlerDecl>,
    pub(super) functions_by_name: HashMap<(Option<String>, String), Vec<&'a Function>>,
    pub(super) aliases_by_name: HashMap<(Option<String>, String), Vec<&'a veln_ast::PublicAlias>>,
    pub(super) types_by_name: HashMap<(Option<String>, String), Vec<&'a veln_ast::TypeDecl>>,
    pub(super) constructors_by_name: ConstructorVariantsByName<'a>,
    pub(super) invalid_names: Vec<&'a veln_ast::InvalidName>,
    pub(super) companion_access_targets: HashMap<String, String>,
}

pub(super) type ConstructorVariantRef<'a> = (&'a veln_ast::TypeDecl, &'a veln_ast::TypeVariantDecl);
pub(super) type ConstructorVariantsByName<'a> =
    HashMap<(Option<String>, String), Vec<ConstructorVariantRef<'a>>>;

pub(super) fn index_functions_by_name<'a>(
    functions: &[&'a Function],
) -> HashMap<(Option<String>, String), Vec<&'a Function>> {
    let mut index = HashMap::<(Option<String>, String), Vec<&'a Function>>::new();
    for function in functions {
        if let Some(name) = &function.name {
            index
                .entry((function.module_name.clone(), name.clone()))
                .or_default()
                .push(*function);
        }
    }
    index
}

pub(super) fn index_aliases_by_name<'a>(
    aliases: &[&'a veln_ast::PublicAlias],
) -> HashMap<(Option<String>, String), Vec<&'a veln_ast::PublicAlias>> {
    let mut index = HashMap::<(Option<String>, String), Vec<&'a veln_ast::PublicAlias>>::new();
    for alias in aliases {
        if let Some(name) = &alias.name {
            index
                .entry((alias.module_name.clone(), name.clone()))
                .or_default()
                .push(*alias);
        }
    }
    index
}

pub(super) fn index_types_by_name<'a>(
    types: &[&'a veln_ast::TypeDecl],
) -> HashMap<(Option<String>, String), Vec<&'a veln_ast::TypeDecl>> {
    let mut index = HashMap::<(Option<String>, String), Vec<&'a veln_ast::TypeDecl>>::new();
    for type_decl in types {
        if let Some(name) = &type_decl.name {
            index
                .entry((type_decl.module_name.clone(), name.clone()))
                .or_default()
                .push(*type_decl);
        }
    }
    index
}

pub(super) fn index_constructors_by_name<'a>(
    types: &[&'a veln_ast::TypeDecl],
) -> ConstructorVariantsByName<'a> {
    let mut index = ConstructorVariantsByName::new();
    for type_decl in types {
        for variant in &type_decl.variants {
            if let Some(name) = &variant.name {
                index
                    .entry((type_decl.module_name.clone(), name.clone()))
                    .or_default()
                    .push((*type_decl, variant));
            }
        }
    }
    index
}
