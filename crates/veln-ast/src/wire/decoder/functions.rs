use super::*;

struct DecodedFunctionHeader {
    node_id: NodeId,
    module_name: Option<String>,
    package_name: Option<String>,
    kind: FunctionKind,
    visibility: Visibility,
    name: Option<String>,
    type_binders: Vec<TypeBinder>,
    effect_binder: Option<EffectBinder>,
    params: Vec<Param>,
}

impl Reader<'_> {
    pub(super) fn function(&mut self) -> Result<Function, String> {
        let header = self.function_header()?;
        Ok(Function {
            node_id: header.node_id,
            module_name: header.module_name,
            package_name: header.package_name,
            kind: header.kind,
            visibility: header.visibility,
            name: header.name,
            type_binders: header.type_binders,
            effect_binder: header.effect_binder,
            params: header.params,
            return_binding: self.option(Self::result_binding)?,
            return_type: self.option(Self::string)?,
            return_type_span: self.option(Self::span)?,
            return_type_paths: self.vec(Self::type_path_segments)?,
            return_type_refinements: self.vec(Self::variant_refinement)?,
            effects: self.option(|reader| reader.vec(Self::string))?,
            effect_spans: self.option(|reader| reader.vec(Self::span))?,
            callsite: self.option(Self::span)?,
            contracts: self.vec(Self::contract)?,
            body: self.vec(Self::body_line)?.into(),
            span: self.span()?,
        })
    }

    fn function_header(&mut self) -> Result<DecodedFunctionHeader, String> {
        Ok(DecodedFunctionHeader {
            node_id: self.node_id()?,
            module_name: self.option(Self::string)?,
            package_name: self.option(Self::string)?,
            kind: self.function_kind()?,
            visibility: self.visibility()?,
            name: self.option(Self::string)?,
            type_binders: self.vec(Self::type_binder)?,
            effect_binder: self.option(Self::effect_binder)?,
            params: self.vec(Self::param)?,
        })
    }

    fn type_binder(&mut self) -> Result<TypeBinder, String> {
        Ok(TypeBinder {
            name: self.string()?,
            span: self.span()?,
        })
    }
}
