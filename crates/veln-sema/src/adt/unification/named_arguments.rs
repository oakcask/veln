use veln_core::CoreType;

use crate::semantic_model::Type;

use super::AdtDescriptor;

pub(crate) trait NamedTypeArguments: Sized {
    fn named_type_arguments(&self) -> Option<(&str, &[Self])>;

    fn matches_adt_descriptor(&self, descriptor: &AdtDescriptor) -> bool;
}

pub(crate) fn adt_args<'a, T: NamedTypeArguments>(
    ty: &'a T,
    descriptor: &AdtDescriptor,
) -> Option<&'a [T]> {
    let (_, args) = ty.named_type_arguments()?;
    (ty.matches_adt_descriptor(descriptor) && args.len() == descriptor.type_parameters.len())
        .then_some(args)
}

impl NamedTypeArguments for Type {
    fn named_type_arguments(&self) -> Option<(&str, &[Self])> {
        match self {
            Self::Named { name, args, .. } | Self::VariantRefinement { name, args, .. } => {
                Some((name, args))
            }
            _ => None,
        }
    }

    fn matches_adt_descriptor(&self, descriptor: &AdtDescriptor) -> bool {
        match self {
            Self::Named { name, identity, .. } | Self::VariantRefinement { name, identity, .. } => {
                if identity == name {
                    name == &descriptor.type_name
                } else {
                    identity == &descriptor.identity()
                }
            }
            _ => false,
        }
    }
}

impl NamedTypeArguments for CoreType {
    fn named_type_arguments(&self) -> Option<(&str, &[Self])> {
        let Self::Named { name, args } = self else {
            return None;
        };
        Some((name, args))
    }

    fn matches_adt_descriptor(&self, descriptor: &AdtDescriptor) -> bool {
        matches!(self, Self::Named { name, .. } if name == &descriptor.type_name)
    }
}

pub(crate) fn named_part<'a, T: NamedTypeArguments>(
    ty: &'a T,
    name: &str,
    arity: usize,
) -> Option<&'a T> {
    let (ty_name, args) = ty.named_type_arguments()?;
    (ty_name == name && args.len() == arity)
        .then(|| args.first())
        .flatten()
}

pub(crate) fn named_parts2<'a, T: NamedTypeArguments>(
    ty: &'a T,
    name: &str,
) -> Option<(&'a T, &'a T)> {
    let (ty_name, args) = ty.named_type_arguments()?;
    (ty_name == name && args.len() == 2).then(|| (&args[0], &args[1]))
}
