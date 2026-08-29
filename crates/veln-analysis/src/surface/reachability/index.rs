use super::*;

#[derive(Default)]
pub(crate) struct ReachabilityCache {
    #[cfg(test)]
    pub(super) function_targets: OnceCell<ReachabilityIndex>,
    pub(super) separated_function_targets: OnceCell<ReachabilityIndex>,
    pub(super) direct_callees: RefCell<HashMap<ReachableFunction, Vec<ReachableFunction>>>,
}

pub(super) struct ReachabilityIndex {
    pub(super) function_targets: FunctionTargetIndex,
    functions_by_name: HashMap<(FunctionKind, String), Vec<FunctionRef>>,
    functions_by_qualified_name: HashMap<(FunctionKind, String, String), Vec<FunctionRef>>,
}

impl ReachabilityIndex {
    pub(super) fn new(
        inputs: &ReachabilityInputs<'_>,
        function_targets: Vec<FunctionTarget>,
    ) -> Self {
        let mut functions_by_name = HashMap::<(FunctionKind, String), Vec<FunctionRef>>::new();
        let mut functions_by_qualified_name =
            HashMap::<(FunctionKind, String, String), Vec<FunctionRef>>::new();
        for function_ref in inputs.function_refs() {
            let function = inputs.function(function_ref);
            let Some(name) = &function.name else {
                continue;
            };
            functions_by_name
                .entry((function.kind, name.clone()))
                .or_default()
                .push(function_ref);
            if let Some(module_name) = &function.module_name {
                functions_by_qualified_name
                    .entry((function.kind, module_name.clone(), name.clone()))
                    .or_default()
                    .push(function_ref);
            }
        }
        Self {
            function_targets: FunctionTargetIndex::new(function_targets),
            functions_by_name,
            functions_by_qualified_name,
        }
    }

    pub(super) fn function_refs(&self, key: &ReachableFunction) -> &[FunctionRef] {
        if let Some(module_name) = &key.module_name {
            self.functions_by_qualified_name
                .get(&(key.kind, module_name.clone(), key.name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default()
        } else {
            self.functions_by_name
                .get(&(key.kind, key.name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default()
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct ReachabilityInputs<'a> {
    pub(super) standard: Option<&'a SurfaceModule>,
    pub(super) application: &'a SurfaceModule,
}

impl<'a> ReachabilityInputs<'a> {
    #[cfg(test)]
    pub(super) fn combined(module: &'a SurfaceModule) -> Self {
        Self {
            standard: None,
            application: module,
        }
    }

    pub(super) fn separated(standard: &'a SurfaceModule, application: &'a SurfaceModule) -> Self {
        Self {
            standard: Some(standard),
            application,
        }
    }

    pub(super) fn module_header(&self) -> Option<veln_ast::ModuleHeader> {
        self.application
            .module
            .clone()
            .or_else(|| self.standard.and_then(|module| module.module.clone()))
    }

    pub(super) fn cloned_declarations<T: Clone + 'a>(
        &self,
        select: impl Fn(&'a SurfaceModule) -> &'a [T],
    ) -> Vec<T> {
        self.standard
            .into_iter()
            .flat_map(|module| select(module).iter())
            .chain(select(self.application).iter())
            .cloned()
            .collect()
    }

    pub(super) fn function_refs(&self) -> impl Iterator<Item = FunctionRef> + '_ {
        let standard_len = self.standard.map_or(0, |module| module.functions.len());
        (0..standard_len)
            .map(|index| FunctionRef {
                input: ReachabilityInput::Standard,
                index,
            })
            .chain(
                (0..self.application.functions.len()).map(|index| FunctionRef {
                    input: ReachabilityInput::Application,
                    index,
                }),
            )
    }

    pub(super) fn functions(&self) -> impl Iterator<Item = &'a Function> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.functions.iter())
            .chain(self.application.functions.iter())
    }

    pub(super) fn function(&self, function_ref: FunctionRef) -> &'a Function {
        match function_ref.input {
            ReachabilityInput::Standard => {
                &self
                    .standard
                    .expect("standard function ref should have standard input")
                    .functions[function_ref.index]
            }
            ReachabilityInput::Application => &self.application.functions[function_ref.index],
        }
    }

    pub(super) fn all_uses(&self) -> Vec<&'a UseDecl> {
        self.standard
            .into_iter()
            .flat_map(|module| module.uses.iter())
            .chain(self.application.uses.iter())
            .collect()
    }

    pub(super) fn uses(&self) -> Vec<&'a UseDecl> {
        let invalid_names = self.invalid_names().collect::<Vec<_>>();
        self.all_uses()
            .into_iter()
            .filter(|use_decl| !use_decl_has_invalid_module_segment(use_decl, &invalid_names))
            .collect()
    }

    pub(super) fn aliases(&self) -> impl Iterator<Item = &'a veln_ast::PublicAlias> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.aliases.iter())
            .chain(self.application.aliases.iter())
    }

    pub(super) fn handlers(&self) -> Vec<&'a veln_ast::HandlerDecl> {
        self.standard
            .into_iter()
            .flat_map(|module| module.handlers.iter())
            .chain(self.application.handlers.iter())
            .collect()
    }

    pub(super) fn types(&self) -> impl Iterator<Item = &'a veln_ast::TypeDecl> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.types.iter())
            .chain(self.application.types.iter())
    }

    pub(super) fn invalid_names(&self) -> impl Iterator<Item = &'a veln_ast::InvalidName> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.invalid_names.iter())
            .chain(self.application.invalid_names.iter())
    }

    pub(super) fn codecs(&self) -> impl Iterator<Item = &'a veln_ast::CodecDecl> + '_ {
        self.standard
            .into_iter()
            .flat_map(|module| module.codecs.iter())
            .chain(self.application.codecs.iter())
    }
}

#[derive(Clone, Copy)]
pub(super) struct FunctionRef {
    pub(super) input: ReachabilityInput,
    pub(super) index: usize,
}

#[derive(Clone, Copy)]
pub(super) enum ReachabilityInput {
    Standard,
    Application,
}

pub(super) struct FunctionTargetIndex {
    pub(super) all: Vec<FunctionTarget>,
    by_name: HashMap<String, Vec<usize>>,
    by_qualified_name: HashMap<(String, String), Vec<usize>>,
    by_shape: HashMap<FunctionShape, Vec<usize>>,
}

impl FunctionTargetIndex {
    pub(super) fn new(all: Vec<FunctionTarget>) -> Self {
        let mut by_name = HashMap::<String, Vec<usize>>::new();
        let mut by_qualified_name = HashMap::<(String, String), Vec<usize>>::new();
        let mut by_shape = HashMap::<FunctionShape, Vec<usize>>::new();
        for (index, target) in all.iter().enumerate() {
            by_name.entry(target.name.clone()).or_default().push(index);
            if let Some(module_name) = &target.module_name {
                by_qualified_name
                    .entry((module_name.clone(), target.name.clone()))
                    .or_default()
                    .push(index);
            }
            by_shape
                .entry(target.shape.clone())
                .or_default()
                .push(index);
        }
        Self {
            all,
            by_name,
            by_qualified_name,
            by_shape,
        }
    }

    pub(super) fn named(&self, name: &str) -> impl Iterator<Item = &FunctionTarget> {
        self.by_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    pub(super) fn qualified(
        &self,
        module_name: &str,
        name: &str,
    ) -> impl Iterator<Item = &FunctionTarget> {
        self.by_qualified_name
            .get(&(module_name.to_string(), name.to_string()))
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }

    pub(super) fn shaped(&self, shape: &FunctionShape) -> impl Iterator<Item = &FunctionTarget> {
        self.by_shape
            .get(shape)
            .into_iter()
            .flatten()
            .map(|index| &self.all[*index])
    }
}
