use veln_core::CoreType;

use crate::types::Type;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdtVariantKind {
    OptionSome,
    OptionNone,
    ResultOk,
    ResultErr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdtDescriptor {
    pub(crate) type_name: &'static str,
    pub(crate) type_parameters: &'static [&'static str],
    pub(crate) variants: &'static [AdtVariantDescriptor],
    pub(crate) diagnostic_name: &'static str,
    pub(crate) propagation: Option<ResultPropagationDescriptor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdtVariantDescriptor {
    pub(crate) name: &'static str,
    pub(crate) kind: AdtVariantKind,
    pub(crate) payload_fields: &'static [AdtPayloadField],
    pub(crate) coverage_case: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdtPayloadField {
    pub(crate) name: &'static str,
    pub(crate) type_parameter_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultPropagationDescriptor {
    pub(crate) value_parameter_index: usize,
    pub(crate) error_parameter_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdtConstructor {
    pub(crate) descriptor: &'static AdtDescriptor,
    pub(crate) variant: &'static AdtVariantDescriptor,
}

const OPTION_VALUE_FIELD: &[AdtPayloadField] = &[AdtPayloadField {
    name: "value",
    type_parameter_index: 0,
}];

const RESULT_OK_FIELD: &[AdtPayloadField] = &[AdtPayloadField {
    name: "value",
    type_parameter_index: 0,
}];

const RESULT_ERR_FIELD: &[AdtPayloadField] = &[AdtPayloadField {
    name: "error",
    type_parameter_index: 1,
}];

const OPTION_VARIANTS: &[AdtVariantDescriptor] = &[
    AdtVariantDescriptor {
        name: "Some",
        kind: AdtVariantKind::OptionSome,
        payload_fields: OPTION_VALUE_FIELD,
        coverage_case: "Some(_)",
    },
    AdtVariantDescriptor {
        name: "None",
        kind: AdtVariantKind::OptionNone,
        payload_fields: &[],
        coverage_case: "None",
    },
];

const RESULT_VARIANTS: &[AdtVariantDescriptor] = &[
    AdtVariantDescriptor {
        name: "Ok",
        kind: AdtVariantKind::ResultOk,
        payload_fields: RESULT_OK_FIELD,
        coverage_case: "Ok(_)",
    },
    AdtVariantDescriptor {
        name: "Err",
        kind: AdtVariantKind::ResultErr,
        payload_fields: RESULT_ERR_FIELD,
        coverage_case: "Err(_)",
    },
];

const OPTION_DESCRIPTOR: AdtDescriptor = AdtDescriptor {
    type_name: "Option",
    type_parameters: &["T"],
    variants: OPTION_VARIANTS,
    diagnostic_name: "option",
    propagation: None,
};

const RESULT_DESCRIPTOR: AdtDescriptor = AdtDescriptor {
    type_name: "Result",
    type_parameters: &["T", "E"],
    variants: RESULT_VARIANTS,
    diagnostic_name: "result",
    propagation: Some(ResultPropagationDescriptor {
        value_parameter_index: 0,
        error_parameter_index: 1,
    }),
};

const DESCRIPTORS: &[AdtDescriptor] = &[OPTION_DESCRIPTOR, RESULT_DESCRIPTOR];

pub(crate) fn descriptor_for_type_name(name: &str) -> Option<&'static AdtDescriptor> {
    DESCRIPTORS
        .iter()
        .find(|descriptor| descriptor.type_name == name)
}

fn option_descriptor() -> &'static AdtDescriptor {
    descriptor_for_type_name("Option").expect("built-in Option descriptor should exist")
}

fn result_descriptor() -> &'static AdtDescriptor {
    descriptor_for_type_name("Result").expect("built-in Result descriptor should exist")
}

pub(crate) fn descriptor_for_type(ty: &Type) -> Option<&'static AdtDescriptor> {
    let Type::Named { name, args } = ty else {
        return None;
    };
    descriptor_for_type_name(name)
        .filter(|descriptor| descriptor.type_parameters.len() == args.len())
}

pub(crate) fn constructor(segments: &[String]) -> Option<AdtConstructor> {
    DESCRIPTORS.iter().find_map(|descriptor| {
        descriptor.variants.iter().find_map(|variant| {
            constructor_matches(descriptor, variant, segments).then_some(AdtConstructor {
                descriptor,
                variant,
            })
        })
    })
}

pub(crate) fn nullary_constructor(segments: &[String]) -> Option<AdtConstructor> {
    constructor(segments).filter(|constructor| constructor.variant.payload_fields.is_empty())
}

pub(crate) fn constructor_for_descriptor(
    segments: &[String],
    descriptor: &'static AdtDescriptor,
) -> Option<AdtConstructor> {
    constructor(segments).filter(|constructor| {
        constructor.descriptor.type_name == descriptor.type_name
            && constructor.descriptor.type_parameters.len() == descriptor.type_parameters.len()
    })
}

pub(crate) fn adt_args<'a>(ty: &'a Type, descriptor: &AdtDescriptor) -> Option<&'a [Type]> {
    match ty {
        Type::Named { name, args }
            if name == descriptor.type_name && args.len() == descriptor.type_parameters.len() =>
        {
            Some(args)
        }
        _ => None,
    }
}

pub(crate) fn core_adt_args<'a>(
    ty: &'a CoreType,
    descriptor: &AdtDescriptor,
) -> Option<&'a [CoreType]> {
    match ty {
        CoreType::Named { name, args }
            if name == descriptor.type_name && args.len() == descriptor.type_parameters.len() =>
        {
            Some(args)
        }
        _ => None,
    }
}

pub(crate) fn constructed_type(constructor: AdtConstructor, payload_type: Type) -> Type {
    let mut args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
    if let Some(field) = constructor.variant.payload_fields.first() {
        args[field.type_parameter_index] = payload_type;
    }
    Type::named(constructor.descriptor.type_name, args)
}

pub(crate) fn core_constructed_type(
    constructor: AdtConstructor,
    payload_type: CoreType,
) -> CoreType {
    let mut args = vec![CoreType::Unknown; constructor.descriptor.type_parameters.len()];
    if let Some(field) = constructor.variant.payload_fields.first() {
        args[field.type_parameter_index] = payload_type;
    }
    CoreType::named(constructor.descriptor.type_name, args)
}

pub(crate) fn payload_type(
    ty: &Type,
    constructor: AdtConstructor,
    payload_index: usize,
) -> Option<&Type> {
    let field = constructor.variant.payload_fields.get(payload_index)?;
    adt_args(ty, constructor.descriptor)?.get(field.type_parameter_index)
}

pub(crate) fn core_payload_type(
    ty: &CoreType,
    constructor: AdtConstructor,
    payload_index: usize,
) -> Option<&CoreType> {
    let field = constructor.variant.payload_fields.get(payload_index)?;
    core_adt_args(ty, constructor.descriptor)?.get(field.type_parameter_index)
}

pub(crate) fn option_type(value: Type) -> Type {
    Type::named(option_descriptor().type_name, vec![value])
}

pub(crate) fn core_option_type(value: CoreType) -> CoreType {
    CoreType::named(option_descriptor().type_name, vec![value])
}

pub(crate) fn result_type(value: Type, error: Type) -> Type {
    Type::named(result_descriptor().type_name, vec![value, error])
}

pub(crate) fn core_result_type(value: CoreType, error: CoreType) -> CoreType {
    CoreType::named(result_descriptor().type_name, vec![value, error])
}

pub(crate) fn option_part(ty: &Type) -> Option<&Type> {
    adt_args(ty, option_descriptor())?.first()
}

pub(crate) fn core_option_part(ty: &CoreType) -> Option<&CoreType> {
    core_adt_args(ty, option_descriptor())?.first()
}

pub(crate) fn result_parts(ty: &Type) -> Option<(&Type, &Type)> {
    let descriptor = result_descriptor();
    let propagation = descriptor.propagation?;
    let args = adt_args(ty, descriptor)?;
    Some((
        args.get(propagation.value_parameter_index)?,
        args.get(propagation.error_parameter_index)?,
    ))
}

pub(crate) fn core_result_parts(ty: &CoreType) -> Option<(&CoreType, &CoreType)> {
    let descriptor = result_descriptor();
    let propagation = descriptor.propagation?;
    let args = core_adt_args(ty, descriptor)?;
    Some((
        args.get(propagation.value_parameter_index)?,
        args.get(propagation.error_parameter_index)?,
    ))
}

fn constructor_matches(
    descriptor: &AdtDescriptor,
    variant: &AdtVariantDescriptor,
    segments: &[String],
) -> bool {
    matches!(segments, [name] if name == variant.name)
        || matches!(segments, [type_name, name] if type_name == descriptor.type_name && name == variant.name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

    #[test]
    fn descriptors_match_builtin_type_arities() {
        assert_eq!(
            descriptor_for_type(&Type::named("Option", vec![Type::int()]))
                .expect("Option(T) should have a descriptor")
                .type_name,
            "Option"
        );
        assert_eq!(
            descriptor_for_type(&Type::result(Type::int(), Type::string()))
                .expect("Result(T, E) should have a descriptor")
                .type_name,
            "Result"
        );
        assert!(descriptor_for_type(&Type::named("Option", Vec::new())).is_none());
        assert!(descriptor_for_type(&Type::named("Result", vec![Type::int()])).is_none());
    }

    #[test]
    fn constructors_match_qualified_and_unqualified_builtin_names() {
        let some = constructor(&path(&["Some"])).expect("Some should resolve");
        assert_eq!(some.descriptor.type_name, "Option");
        assert_eq!(some.variant.name, "Some");
        assert_eq!(some.variant.coverage_case, "Some(_)");
        assert_eq!(some.variant.payload_fields[0].name, "value");

        let none = nullary_constructor(&path(&["Option", "None"]))
            .expect("Option::None should resolve as nullary");
        assert_eq!(none.descriptor.type_name, "Option");
        assert_eq!(none.variant.name, "None");
        assert_eq!(none.variant.coverage_case, "None");
        assert!(none.variant.payload_fields.is_empty());

        let err = constructor(&path(&["Result", "Err"])).expect("Result::Err should resolve");
        assert_eq!(err.descriptor.type_name, "Result");
        assert_eq!(err.variant.name, "Err");
        assert_eq!(err.variant.payload_fields[0].name, "error");
        assert_eq!(err.variant.payload_fields[0].type_parameter_index, 1);
    }

    #[test]
    fn result_payload_and_propagation_use_descriptor_parameter_indices() {
        let result = Type::result(Type::int(), Type::string());
        let ok = constructor(&path(&["Ok"])).expect("Ok should resolve");
        let err = constructor(&path(&["Err"])).expect("Err should resolve");

        assert_eq!(payload_type(&result, ok, 0), Some(&Type::int()));
        assert_eq!(payload_type(&result, err, 0), Some(&Type::string()));
        assert_eq!(result_parts(&result), Some((&Type::int(), &Type::string())));
        assert_eq!(
            constructed_type(err, Type::string()),
            result_type(Type::Unknown, Type::string())
        );

        let core_result = CoreType::result(CoreType::int(), CoreType::string());
        assert_eq!(
            core_payload_type(&core_result, ok, 0),
            Some(&CoreType::int())
        );
        assert_eq!(
            core_payload_type(&core_result, err, 0),
            Some(&CoreType::string())
        );
        assert_eq!(
            core_result_parts(&core_result),
            Some((&CoreType::int(), &CoreType::string()))
        );
        assert_eq!(
            core_constructed_type(err, CoreType::string()),
            core_result_type(CoreType::Unknown, CoreType::string())
        );
    }
}
