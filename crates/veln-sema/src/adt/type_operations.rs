use super::unification::{
    NamedTypeArguments, core_payload_type_from_args, fill_core_type_parameters,
    fill_type_parameters, named_part, named_parts2, payload_type_from_args,
};
use super::{AdtConstructor, AdtDescriptor, CoreType, Type};
#[cfg(test)]
use super::{
    InvalidStandardSymbolCase, build_builtin_descriptors, validate_adt_lookup_descriptors,
};

pub(crate) fn adt_args<'a, T: NamedTypeArguments>(
    ty: &'a T,
    descriptor: &AdtDescriptor,
) -> Option<&'a [T]> {
    let (name, args) = ty.named_type_arguments()?;
    (name == descriptor.type_name && args.len() == descriptor.type_parameters.len()).then_some(args)
}

pub(crate) fn constructed_type(constructor: AdtConstructor<'_>, payloads: &[Type]) -> Type {
    let mut args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
    for (index, field) in constructor.variant.payload_fields.iter().enumerate() {
        if let Some(payload) = payloads.get(index) {
            fill_type_parameters(&mut args, constructor.descriptor, &field.ty, payload);
        }
    }
    constructed_type_from_args(constructor, &args)
}

pub(crate) fn core_constructed_type(
    constructor: AdtConstructor<'_>,
    payloads: &[CoreType],
) -> CoreType {
    let mut args = vec![CoreType::Unknown; constructor.descriptor.type_parameters.len()];
    for (index, field) in constructor.variant.payload_fields.iter().enumerate() {
        if let Some(payload) = payloads.get(index) {
            fill_core_type_parameters(&mut args, constructor.descriptor, &field.ty, payload);
        }
    }
    core_constructed_type_from_args(constructor, &args)
}

pub(crate) fn constructed_type_from_args(constructor: AdtConstructor<'_>, args: &[Type]) -> Type {
    Type::named(&constructor.descriptor.type_name, args.to_vec())
}

pub(crate) fn core_constructed_type_from_args(
    constructor: AdtConstructor<'_>,
    args: &[CoreType],
) -> CoreType {
    CoreType::named(&constructor.descriptor.type_name, args.to_vec())
}

pub(crate) fn payload_type_with_args(
    constructor: AdtConstructor<'_>,
    args: &[Type],
    payload_index: usize,
) -> Option<Type> {
    let ty = constructed_type_from_args(constructor, args);
    payload_type(&ty, constructor, payload_index)
}

pub(crate) fn core_payload_type_with_args(
    constructor: AdtConstructor<'_>,
    args: &[CoreType],
    payload_index: usize,
) -> Option<CoreType> {
    let ty = core_constructed_type_from_args(constructor, args);
    core_payload_type(&ty, constructor, payload_index)
}

pub(crate) fn merge_type_args_from_payload(
    args: &mut [Type],
    constructor: AdtConstructor<'_>,
    payload_index: usize,
    actual: &Type,
) {
    if let Some(field) = constructor.variant.payload_fields.get(payload_index) {
        fill_type_parameters(args, constructor.descriptor, &field.ty, actual);
    }
}

pub(crate) fn merge_core_type_args_from_payload(
    args: &mut [CoreType],
    constructor: AdtConstructor<'_>,
    payload_index: usize,
    actual: &CoreType,
) {
    if let Some(field) = constructor.variant.payload_fields.get(payload_index) {
        fill_core_type_parameters(args, constructor.descriptor, &field.ty, actual);
    }
}

pub(crate) fn payload_type(
    ty: &Type,
    constructor: AdtConstructor<'_>,
    payload_index: usize,
) -> Option<Type> {
    let field = constructor.variant.payload_fields.get(payload_index)?;
    payload_type_from_args(ty, constructor.descriptor, &field.ty)
}

pub(crate) fn core_payload_type(
    ty: &CoreType,
    constructor: AdtConstructor<'_>,
    payload_index: usize,
) -> Option<CoreType> {
    let field = constructor.variant.payload_fields.get(payload_index)?;
    core_payload_type_from_args(ty, constructor.descriptor, &field.ty)
}

pub(crate) fn option_type(value: Type) -> Type {
    Type::named("Option", vec![value])
}

pub(crate) fn core_option_type(value: CoreType) -> CoreType {
    CoreType::named("Option", vec![value])
}

pub(crate) fn result_type(value: Type, error: Type) -> Type {
    Type::named("Result", vec![value, error])
}

pub(crate) fn core_result_type(value: CoreType, error: CoreType) -> CoreType {
    CoreType::named("Result", vec![value, error])
}

pub(crate) fn list_type(item: Type) -> Type {
    Type::named("List", vec![item])
}

pub(crate) fn core_list_type(item: CoreType) -> CoreType {
    CoreType::named("List", vec![item])
}

pub(crate) fn option_part(ty: &Type) -> Option<&Type> {
    named_part(ty, "Option", 1)
}

pub(crate) fn core_option_part(ty: &CoreType) -> Option<&CoreType> {
    named_part(ty, "Option", 1)
}

pub(crate) fn result_parts(ty: &Type) -> Option<(&Type, &Type)> {
    named_parts2(ty, "Result")
}

pub(crate) fn core_result_parts(ty: &CoreType) -> Option<(&CoreType, &CoreType)> {
    named_parts2(ty, "Result")
}

pub(crate) fn list_part(ty: &Type) -> Option<&Type> {
    named_part(ty, "List", 1)
}

pub(crate) fn core_list_part(ty: &CoreType) -> Option<&CoreType> {
    named_part(ty, "List", 1)
}

#[cfg(test)]
pub(crate) fn validate_builtin_adt_descriptors() -> Result<(), InvalidStandardSymbolCase> {
    validate_adt_lookup_descriptors("adt", &build_builtin_descriptors())
}

#[cfg(test)]
pub(super) fn raw_builtin_descriptors_for_test() -> Vec<AdtDescriptor> {
    build_builtin_descriptors()
}
