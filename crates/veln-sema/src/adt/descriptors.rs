use veln_ast::Visibility;

use crate::semantic_model::Type;
use crate::source_less_names::SourceLessNameClass;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdtVariantKind {
    OptionSome,
    OptionNone,
    ResultOk,
    ResultErr,
    ListNil,
    ListCons,
    Source,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VariantRefinementAvailability {
    Finite,
    Opaque,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtDescriptor {
    pub(crate) type_name: String,
    pub(crate) name_class: SourceLessNameClass,
    pub(crate) package_name: Option<String>,
    pub(crate) module_name: Option<String>,
    pub(crate) nominal_identity: Option<String>,
    pub(crate) type_parameters: Vec<String>,
    pub(crate) variants: Vec<AdtVariantDescriptor>,
    pub(crate) diagnostic_name: String,
    pub(crate) refinement_availability: VariantRefinementAvailability,
    pub(crate) propagation: Option<ResultPropagationDescriptor>,
    pub(crate) visibility: Visibility,
}

impl AdtDescriptor {
    pub(crate) fn has_standard_prelude_origin(&self) -> bool {
        self.package_name.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
            && self.module_name.as_deref() == Some("std::prelude")
    }

    pub(crate) fn identity(&self) -> String {
        if let Some(identity) = &self.nominal_identity {
            return identity.clone();
        }
        self.declaration_identity()
    }

    pub(crate) fn declaration_identity(&self) -> String {
        match (self.package_name.as_deref(), self.module_name.as_deref()) {
            (None, None) => self.type_name.clone(),
            (Some(package), Some("std::prelude")) if package == veln_stdlib::PACKAGE_NAME => {
                self.type_name.clone()
            }
            (Some(package), Some(module))
                if module == package || module.starts_with(&format!("{package}::")) =>
            {
                format!("{module}::{}", self.type_name)
            }
            (Some(package), Some(module)) => format!("{package}::{module}::{}", self.type_name),
            (None, Some(module)) if module.starts_with("std::") => {
                format!("workspace::{module}::{}", self.type_name)
            }
            (None, Some(module)) => format!("{module}::{}", self.type_name),
            (Some(package), None) => format!("{package}::{}", self.type_name),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtVariantDescriptor {
    pub(crate) name: String,
    pub(crate) name_class: SourceLessNameClass,
    pub(crate) kind: AdtVariantKind,
    pub(crate) payload_fields: Vec<AdtPayloadField>,
    pub(crate) coverage_case: String,
    pub(crate) visibility: Visibility,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtPayloadField {
    pub(crate) name: String,
    pub(crate) ty: AdtPayloadType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AdtPayloadType {
    TypeParameter(usize),
    SelfType,
    Concrete(Type),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultPropagationDescriptor {
    pub(crate) value_parameter_index: usize,
    pub(crate) error_parameter_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdtConstructor<'a> {
    pub(crate) descriptor: &'a AdtDescriptor,
    pub(crate) variant: &'a AdtVariantDescriptor,
}
