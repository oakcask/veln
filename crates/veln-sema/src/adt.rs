use veln_ast::{PublicAliasKind, SurfaceModule, TypeDecl, UseDecl, Visibility};
use veln_core::CoreType;

use crate::prelude::PRELUDE_MODULE;
use crate::types::{Type, parse_type_or_unknown};

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtDescriptor {
    pub(crate) type_name: String,
    pub(crate) module_name: Option<String>,
    pub(crate) type_parameters: Vec<String>,
    pub(crate) variants: Vec<AdtVariantDescriptor>,
    pub(crate) diagnostic_name: String,
    pub(crate) propagation: Option<ResultPropagationDescriptor>,
    pub(crate) visibility: Visibility,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtVariantDescriptor {
    pub(crate) name: String,
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

#[derive(Clone, Debug)]
pub(crate) struct AdtRegistry {
    descriptors: Vec<AdtDescriptor>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructorLookup<'a> {
    Found(AdtConstructor<'a>),
    Ambiguous,
    Missing,
}

impl AdtRegistry {
    pub(crate) fn from_module(module: &SurfaceModule) -> Self {
        let mut descriptors = builtin_descriptors();
        let source_descriptors = module
            .types
            .iter()
            .filter_map(source_descriptor)
            .collect::<Vec<_>>();
        let aliases = type_alias_descriptors(module, &source_descriptors);
        descriptors.extend(aliases);
        descriptors.extend(source_descriptors);
        Self { descriptors }
    }

    pub(crate) fn descriptor_for_type(&self, ty: &Type) -> Option<&AdtDescriptor> {
        let Type::Named { name, args } = ty else {
            return None;
        };
        self.descriptors.iter().find(|descriptor| {
            descriptor.type_name == *name && descriptor.type_parameters.len() == args.len()
        })
    }

    pub(crate) fn descriptor_for_core_type(&self, ty: &CoreType) -> Option<&AdtDescriptor> {
        let CoreType::Named { name, args } = ty else {
            return None;
        };
        self.descriptors.iter().find(|descriptor| {
            descriptor.type_name == *name && descriptor.type_parameters.len() == args.len()
        })
    }

    pub(crate) fn constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> ConstructorLookup<'_> {
        self.lookup_constructor(segments, current_module, uses, true)
    }

    pub(crate) fn nullary_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> ConstructorLookup<'_> {
        match self.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor)
                if constructor.variant.payload_fields.is_empty() =>
            {
                ConstructorLookup::Found(constructor)
            }
            ConstructorLookup::Found(_) => ConstructorLookup::Missing,
            other => other,
        }
    }

    pub(crate) fn constructor_for_descriptor(
        &self,
        segments: &[String],
        descriptor: &AdtDescriptor,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<AdtConstructor<'_>> {
        match self.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor)
                if same_descriptor(constructor.descriptor, descriptor) =>
            {
                return Some(constructor);
            }
            ConstructorLookup::Ambiguous
                if descriptor_allows_expected_constructor_disambiguation(descriptor) => {}
            _ => return None,
        }

        let mut matches = Vec::new();
        for candidate in &self.descriptors {
            if !same_descriptor(candidate, descriptor)
                || !descriptor_visible(candidate, segments, current_module, uses, true)
            {
                continue;
            }
            for variant in &candidate.variants {
                if constructor_matches_visible_path(
                    candidate,
                    variant,
                    segments,
                    uses,
                    current_module,
                ) && variant_visible(candidate, variant, current_module, segments)
                {
                    matches.push(AdtConstructor {
                        descriptor: candidate,
                        variant,
                    });
                }
            }
        }
        match matches.as_slice() {
            [constructor] => Some(*constructor),
            _ => None,
        }
    }

    fn lookup_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        include_imports: bool,
    ) -> ConstructorLookup<'_> {
        let mut matches = Vec::new();
        for descriptor in &self.descriptors {
            if !descriptor_visible(descriptor, segments, current_module, uses, include_imports) {
                continue;
            }
            for variant in &descriptor.variants {
                if constructor_matches_visible_path(
                    descriptor,
                    variant,
                    segments,
                    uses,
                    current_module,
                ) && variant_visible(descriptor, variant, current_module, segments)
                {
                    matches.push(AdtConstructor {
                        descriptor,
                        variant,
                    });
                }
            }
        }
        match matches.as_slice() {
            [] => ConstructorLookup::Missing,
            [constructor] => ConstructorLookup::Found(*constructor),
            _ => ConstructorLookup::Ambiguous,
        }
    }
}

fn descriptor_allows_expected_constructor_disambiguation(descriptor: &AdtDescriptor) -> bool {
    descriptor.module_name.is_none()
        && matches!(descriptor.type_name.as_str(), "DecodeStep" | "EncodeStep")
        && descriptor.visibility == Visibility::Public
}

fn type_alias_descriptors(
    module: &SurfaceModule,
    descriptors: &[AdtDescriptor],
) -> Vec<AdtDescriptor> {
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Type)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            let target = descriptor_for_alias_target(
                &alias.target,
                &module.uses,
                descriptors,
                alias.module_name.as_deref(),
            )?;
            let mut descriptor = target.clone();
            descriptor.type_name = name;
            descriptor.module_name = alias.module_name.clone();
            descriptor.visibility = Visibility::Public;
            Some(descriptor)
        })
        .collect()
}

fn descriptor_for_alias_target<'a>(
    segments: &[String],
    uses: &[UseDecl],
    descriptors: &'a [AdtDescriptor],
    current_module: Option<&str>,
) -> Option<&'a AdtDescriptor> {
    match segments {
        [name] => descriptors
            .iter()
            .find(|descriptor| descriptor.type_name == *name),
        [alias, name] => {
            let module_name = uses
                .iter()
                .find(|use_decl| {
                    use_decl.module_name.as_deref() == current_module && use_decl.alias == *alias
                })
                .map(|use_decl| use_decl.name.as_str())?;
            descriptors.iter().find(|descriptor| {
                descriptor.type_name == *name
                    && descriptor.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

pub(crate) fn adt_args<'a>(ty: &'a Type, descriptor: &AdtDescriptor) -> Option<&'a [Type]> {
    match ty {
        Type::Named { name, args }
            if name == &descriptor.type_name && args.len() == descriptor.type_parameters.len() =>
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
            if name == &descriptor.type_name && args.len() == descriptor.type_parameters.len() =>
        {
            Some(args)
        }
        _ => None,
    }
}

pub(crate) fn constructed_type(constructor: AdtConstructor<'_>, payloads: &[Type]) -> Type {
    let mut args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
    for (index, field) in constructor.variant.payload_fields.iter().enumerate() {
        if let Some(payload) = payloads.get(index) {
            fill_type_parameters(&mut args, &field.ty, payload);
        }
    }
    Type::named(&constructor.descriptor.type_name, args)
}

pub(crate) fn core_constructed_type(
    constructor: AdtConstructor<'_>,
    payloads: &[CoreType],
) -> CoreType {
    let mut args = vec![CoreType::Unknown; constructor.descriptor.type_parameters.len()];
    for (index, field) in constructor.variant.payload_fields.iter().enumerate() {
        if let Some(payload) = payloads.get(index) {
            fill_core_type_parameters(&mut args, &field.ty, payload);
        }
    }
    CoreType::named(&constructor.descriptor.type_name, args)
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
    core_named_part(ty, "Option", 1)
}

pub(crate) fn result_parts(ty: &Type) -> Option<(&Type, &Type)> {
    named_parts2(ty, "Result")
}

pub(crate) fn core_result_parts(ty: &CoreType) -> Option<(&CoreType, &CoreType)> {
    core_named_parts2(ty, "Result")
}

pub(crate) fn list_part(ty: &Type) -> Option<&Type> {
    named_part(ty, "List", 1)
}

pub(crate) fn core_list_part(ty: &CoreType) -> Option<&CoreType> {
    core_named_part(ty, "List", 1)
}

fn builtin_descriptors() -> Vec<AdtDescriptor> {
    vec![
        AdtDescriptor {
            type_name: "Option".to_string(),
            module_name: None,
            type_parameters: vec!["T".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Some".to_string(),
                    kind: AdtVariantKind::OptionSome,
                    payload_fields: vec![AdtPayloadField {
                        name: "value".to_string(),
                        ty: AdtPayloadType::TypeParameter(0),
                    }],
                    coverage_case: "Some(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "None".to_string(),
                    kind: AdtVariantKind::OptionNone,
                    payload_fields: Vec::new(),
                    coverage_case: "None".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "option".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "Result".to_string(),
            module_name: None,
            type_parameters: vec!["T".to_string(), "E".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Ok".to_string(),
                    kind: AdtVariantKind::ResultOk,
                    payload_fields: vec![AdtPayloadField {
                        name: "value".to_string(),
                        ty: AdtPayloadType::TypeParameter(0),
                    }],
                    coverage_case: "Ok(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Err".to_string(),
                    kind: AdtVariantKind::ResultErr,
                    payload_fields: vec![AdtPayloadField {
                        name: "error".to_string(),
                        ty: AdtPayloadType::TypeParameter(1),
                    }],
                    coverage_case: "Err(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "result".to_string(),
            propagation: Some(ResultPropagationDescriptor {
                value_parameter_index: 0,
                error_parameter_index: 1,
            }),
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "List".to_string(),
            module_name: None,
            type_parameters: vec!["A".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Nil".to_string(),
                    kind: AdtVariantKind::ListNil,
                    payload_fields: Vec::new(),
                    coverage_case: "Nil".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Cons".to_string(),
                    kind: AdtVariantKind::ListCons,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "head".to_string(),
                            ty: AdtPayloadType::TypeParameter(0),
                        },
                        AdtPayloadField {
                            name: "tail".to_string(),
                            ty: AdtPayloadType::SelfType,
                        },
                    ],
                    coverage_case: "Cons(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "list".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "StreamInput".to_string(),
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![
                AdtVariantDescriptor {
                    name: "Chunk".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "bytes".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("ByteChunk", Vec::new())),
                    }],
                    coverage_case: "Chunk(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "End".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: Vec::new(),
                    coverage_case: "End".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "streaminput".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "DecodeError".to_string(),
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![AdtVariantDescriptor {
                name: "DecodeError".to_string(),
                kind: AdtVariantKind::Source,
                payload_fields: vec![
                    AdtPayloadField {
                        name: "id".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                    AdtPayloadField {
                        name: "offset".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("ByteOffset", Vec::new())),
                    },
                    AdtPayloadField {
                        name: "field_path".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                ],
                coverage_case: "DecodeError(_)".to_string(),
                visibility: Visibility::Public,
            }],
            diagnostic_name: "decodeerror".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "DecodeReadiness".to_string(),
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![
                AdtVariantDescriptor {
                    name: "NeedBytes".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "count".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                    }],
                    coverage_case: "NeedBytes(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "NeedEnd".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: Vec::new(),
                    coverage_case: "NeedEnd".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "decodereadiness".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "DecodeStep".to_string(),
            module_name: None,
            type_parameters: vec!["T".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Decoded".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "value".to_string(),
                            ty: AdtPayloadType::TypeParameter(0),
                        },
                        AdtPayloadField {
                            name: "consumed".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                    ],
                    coverage_case: "Decoded(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "NeedMore".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "readiness".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("DecodeReadiness", Vec::new())),
                    }],
                    coverage_case: "NeedMore(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Invalid".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "error".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("DecodeError", Vec::new())),
                    }],
                    coverage_case: "Invalid(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "decodestep".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "EncodeError".to_string(),
            module_name: None,
            type_parameters: Vec::new(),
            variants: vec![AdtVariantDescriptor {
                name: "EncodeError".to_string(),
                kind: AdtVariantKind::Source,
                payload_fields: vec![
                    AdtPayloadField {
                        name: "id".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                    AdtPayloadField {
                        name: "field_path".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                    AdtPayloadField {
                        name: "reason".to_string(),
                        ty: AdtPayloadType::Concrete(Type::string()),
                    },
                ],
                coverage_case: "EncodeError(_)".to_string(),
                visibility: Visibility::Public,
            }],
            diagnostic_name: "encodeerror".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
        AdtDescriptor {
            type_name: "EncodeStep".to_string(),
            module_name: None,
            type_parameters: vec!["TState".to_string()],
            variants: vec![
                AdtVariantDescriptor {
                    name: "Encoded".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "chunks".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named(
                            "List",
                            vec![Type::named("ByteChunk", Vec::new())],
                        )),
                    }],
                    coverage_case: "Encoded(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Partial".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![
                        AdtPayloadField {
                            name: "chunks".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named(
                                "List",
                                vec![Type::named("ByteChunk", Vec::new())],
                            )),
                        },
                        AdtPayloadField {
                            name: "produced".to_string(),
                            ty: AdtPayloadType::Concrete(Type::named("ByteCount", Vec::new())),
                        },
                        AdtPayloadField {
                            name: "state".to_string(),
                            ty: AdtPayloadType::TypeParameter(0),
                        },
                    ],
                    coverage_case: "Partial(_)".to_string(),
                    visibility: Visibility::Public,
                },
                AdtVariantDescriptor {
                    name: "Invalid".to_string(),
                    kind: AdtVariantKind::Source,
                    payload_fields: vec![AdtPayloadField {
                        name: "error".to_string(),
                        ty: AdtPayloadType::Concrete(Type::named("EncodeError", Vec::new())),
                    }],
                    coverage_case: "Invalid(_)".to_string(),
                    visibility: Visibility::Public,
                },
            ],
            diagnostic_name: "encodestep".to_string(),
            propagation: None,
            visibility: Visibility::Public,
        },
    ]
}

fn source_descriptor(decl: &TypeDecl) -> Option<AdtDescriptor> {
    let name = decl.name.clone()?;
    if matches!(name.as_str(), "Option" | "Result" | "List") {
        return None;
    }
    let variants = decl
        .variants
        .iter()
        .filter_map(|variant| {
            let name = variant.name.clone()?;
            let payload_fields = variant
                .fields
                .iter()
                .map(|field| AdtPayloadField {
                    name: field.name.clone(),
                    ty: payload_descriptor_type(&field.ty, decl),
                })
                .collect::<Vec<_>>();
            let coverage_case = if payload_fields.is_empty() {
                name.clone()
            } else {
                format!("{name}(_)")
            };
            Some(AdtVariantDescriptor {
                name,
                kind: AdtVariantKind::Source,
                payload_fields,
                coverage_case,
                visibility: variant.visibility,
            })
        })
        .collect::<Vec<_>>();
    Some(AdtDescriptor {
        type_name: name.clone(),
        module_name: decl.module_name.clone(),
        type_parameters: decl.params.clone(),
        variants,
        diagnostic_name: name.to_lowercase(),
        propagation: None,
        visibility: decl.visibility,
    })
}

fn payload_descriptor_type(text: &str, decl: &TypeDecl) -> AdtPayloadType {
    if let Some(index) = decl.params.iter().position(|param| param == text) {
        return AdtPayloadType::TypeParameter(index);
    }
    let ty = parse_type_or_unknown(Some(text));
    if is_self_type(&ty, decl) {
        AdtPayloadType::SelfType
    } else {
        AdtPayloadType::Concrete(type_parameters_to_placeholders(ty, &decl.params))
    }
}

fn is_self_type(ty: &Type, decl: &TypeDecl) -> bool {
    let Some(name) = &decl.name else {
        return false;
    };
    let Type::Named {
        name: ty_name,
        args,
    } = ty
    else {
        return false;
    };
    ty_name == name
        && args.len() == decl.params.len()
        && args.iter().zip(&decl.params).all(|(arg, param)| {
            matches!(arg, Type::Named { name, args } if name == param && args.is_empty())
        })
}

fn type_parameters_to_placeholders(ty: Type, params: &[String]) -> Type {
    match ty {
        Type::Named { name, args } if args.is_empty() => params
            .iter()
            .position(|param| param == &name)
            .map_or(Type::Named { name, args }, |index| {
                Type::named(format!("$param{index}"), Vec::new())
            }),
        Type::Named { name, args } => Type::Named {
            name,
            args: args
                .into_iter()
                .map(|arg| type_parameters_to_placeholders(arg, params))
                .collect(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name, type_parameters_to_placeholders(ty, params)))
                .collect(),
        ),
        Type::Function {
            params: fn_params,
            return_type,
            effects,
        } => Type::Function {
            params: fn_params
                .into_iter()
                .map(|ty| type_parameters_to_placeholders(ty, params))
                .collect(),
            return_type: Box::new(type_parameters_to_placeholders(*return_type, params)),
            effects,
        },
        Type::Unknown => Type::Unknown,
    }
}

fn descriptor_visible(
    descriptor: &AdtDescriptor,
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    include_imports: bool,
) -> bool {
    if descriptor.module_name.is_none() {
        return true;
    }
    let same_module = descriptor.module_name.as_deref() == current_module;
    if same_module {
        return true;
    }
    if descriptor.visibility != Visibility::Public || !include_imports {
        return false;
    }
    let Some(first) = segments.first() else {
        return false;
    };
    if let Some(module_name) = uses
        .iter()
        .find(|use_decl| {
            use_decl.module_name.as_deref() == current_module && use_decl.alias == *first
        })
        .map(|use_decl| use_decl.name.as_str())
    {
        return descriptor.module_name.as_deref() == Some(module_name);
    }
    segments.len() <= 2
        && uses.iter().any(|use_decl| {
            use_decl.module_name.as_deref() == current_module
                && descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
        })
}

fn variant_visible(
    descriptor: &AdtDescriptor,
    variant: &AdtVariantDescriptor,
    current_module: Option<&str>,
    segments: &[String],
) -> bool {
    if descriptor.module_name.is_none() || descriptor.module_name.as_deref() == current_module {
        return true;
    }
    if segments.len() > 2 {
        return variant.visibility == Visibility::Public
            && descriptor.visibility == Visibility::Public;
    }
    variant.visibility == Visibility::Public
}

fn constructor_matches_visible_path(
    descriptor: &AdtDescriptor,
    variant: &AdtVariantDescriptor,
    segments: &[String],
    uses: &[UseDecl],
    current_module: Option<&str>,
) -> bool {
    match segments {
        [name] => name == &variant.name,
        [qualifier, name] if name == &variant.name => {
            qualifier == &descriptor.type_name
                || standard_prelude_alias_matches(descriptor, qualifier)
                || import_alias_matches(descriptor, qualifier, uses, current_module)
        }
        [alias, type_name, name] => {
            name == &variant.name
                && type_name == &descriptor.type_name
                && (standard_prelude_alias_matches(descriptor, alias)
                    || import_alias_matches(descriptor, alias, uses, current_module))
        }
        _ => false,
    }
}

fn standard_prelude_alias_matches(descriptor: &AdtDescriptor, alias: &str) -> bool {
    descriptor.module_name.is_none()
        && matches!(
            descriptor.type_name.as_str(),
            "StreamInput"
                | "DecodeError"
                | "DecodeReadiness"
                | "DecodeStep"
                | "EncodeError"
                | "EncodeStep"
        )
        && descriptor.visibility == Visibility::Public
        && alias == PRELUDE_MODULE
}

fn import_alias_matches(
    descriptor: &AdtDescriptor,
    alias: &str,
    uses: &[UseDecl],
    current_module: Option<&str>,
) -> bool {
    let Some(module_name) = descriptor.module_name.as_deref() else {
        return false;
    };
    uses.iter().any(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && use_decl.alias == alias
            && use_decl.name == module_name
    })
}

fn same_descriptor(left: &AdtDescriptor, right: &AdtDescriptor) -> bool {
    left.type_name == right.type_name
        && left.module_name == right.module_name
        && left.type_parameters.len() == right.type_parameters.len()
}

fn payload_type_from_args(
    ty: &Type,
    descriptor: &AdtDescriptor,
    payload: &AdtPayloadType,
) -> Option<Type> {
    match payload {
        AdtPayloadType::TypeParameter(index) => adt_args(ty, descriptor)?.get(*index).cloned(),
        AdtPayloadType::SelfType => Some(ty.clone()),
        AdtPayloadType::Concrete(template) => {
            let args = adt_args(ty, descriptor)?;
            Some(substitute_type_parameters(template, args))
        }
    }
}

fn core_payload_type_from_args(
    ty: &CoreType,
    descriptor: &AdtDescriptor,
    payload: &AdtPayloadType,
) -> Option<CoreType> {
    match payload {
        AdtPayloadType::TypeParameter(index) => core_adt_args(ty, descriptor)?.get(*index).cloned(),
        AdtPayloadType::SelfType => Some(ty.clone()),
        AdtPayloadType::Concrete(template) => {
            let args = core_adt_args(ty, descriptor)?;
            Some(substitute_core_type_parameters(
                &core_type_template(template),
                args,
            ))
        }
    }
}

fn fill_type_parameters(args: &mut [Type], payload: &AdtPayloadType, actual: &Type) {
    match payload {
        AdtPayloadType::TypeParameter(index) => args[*index] = actual.clone(),
        AdtPayloadType::Concrete(template) => unify_template(args, template, actual),
        AdtPayloadType::SelfType => {}
    }
}

fn fill_core_type_parameters(args: &mut [CoreType], payload: &AdtPayloadType, actual: &CoreType) {
    match payload {
        AdtPayloadType::TypeParameter(index) => args[*index] = actual.clone(),
        AdtPayloadType::Concrete(template) => {
            unify_core_template(args, &core_type_template(template), actual);
        }
        AdtPayloadType::SelfType => {}
    }
}

fn unify_template(args: &mut [Type], template: &Type, actual: &Type) {
    match (template, actual) {
        (
            Type::Named { name, args: nested },
            Type::Named {
                name: _actual_name,
                args: _actual_args,
            },
        ) if name.starts_with("$param") && nested.is_empty() => {
            if let Ok(index) = name.trim_start_matches("$param").parse::<usize>()
                && let Some(slot) = args.get_mut(index)
            {
                *slot = actual.clone();
            }
        }
        (
            Type::Named { name, args: nested },
            Type::Named {
                name: actual_name,
                args: actual_args,
            },
        ) if name == actual_name && nested.len() == actual_args.len() => {
            for (nested, actual) in nested.iter().zip(actual_args) {
                unify_template(args, nested, actual);
            }
        }
        (Type::Record(fields), Type::Record(actual_fields)) => {
            for (name, field) in fields {
                if let Some((_, actual_field)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == name)
                {
                    unify_template(args, field, actual_field);
                }
            }
        }
        _ => {}
    }
}

fn unify_core_template(args: &mut [CoreType], template: &CoreType, actual: &CoreType) {
    match (template, actual) {
        (
            CoreType::Named { name, args: nested },
            CoreType::Named {
                name: _actual_name,
                args: _actual_args,
            },
        ) if name.starts_with("$param") && nested.is_empty() => {
            if let Ok(index) = name.trim_start_matches("$param").parse::<usize>()
                && let Some(slot) = args.get_mut(index)
            {
                *slot = actual.clone();
            }
        }
        (
            CoreType::Named { name, args: nested },
            CoreType::Named {
                name: actual_name,
                args: actual_args,
            },
        ) if name == actual_name && nested.len() == actual_args.len() => {
            for (nested, actual) in nested.iter().zip(actual_args) {
                unify_core_template(args, nested, actual);
            }
        }
        (CoreType::Record(fields), CoreType::Record(actual_fields)) => {
            for (name, field) in fields {
                if let Some((_, actual_field)) = actual_fields
                    .iter()
                    .find(|(actual_name, _)| actual_name == name)
                {
                    unify_core_template(args, field, actual_field);
                }
            }
        }
        _ => {}
    }
}

fn substitute_type_parameters(template: &Type, args: &[Type]) -> Type {
    match template {
        Type::Named { name, args: nested } if name.starts_with("$param") && nested.is_empty() => {
            name.trim_start_matches("$param")
                .parse::<usize>()
                .ok()
                .and_then(|index| args.get(index).cloned())
                .unwrap_or(Type::Unknown)
        }
        Type::Named { name, args: nested } => Type::Named {
            name: name.clone(),
            args: nested
                .iter()
                .map(|arg| substitute_type_parameters(arg, args))
                .collect(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), substitute_type_parameters(ty, args)))
                .collect(),
        ),
        Type::Function {
            params,
            return_type,
            effects,
        } => Type::Function {
            params: params
                .iter()
                .map(|ty| substitute_type_parameters(ty, args))
                .collect(),
            return_type: Box::new(substitute_type_parameters(return_type, args)),
            effects: effects.clone(),
        },
        Type::Unknown => Type::Unknown,
    }
}

fn substitute_core_type_parameters(template: &CoreType, args: &[CoreType]) -> CoreType {
    match template {
        CoreType::Named { name, args: nested }
            if name.starts_with("$param") && nested.is_empty() =>
        {
            name.trim_start_matches("$param")
                .parse::<usize>()
                .ok()
                .and_then(|index| args.get(index).cloned())
                .unwrap_or(CoreType::Unknown)
        }
        CoreType::Named { name, args: nested } => CoreType::Named {
            name: name.clone(),
            args: nested
                .iter()
                .map(|arg| substitute_core_type_parameters(arg, args))
                .collect(),
        },
        CoreType::Record(fields) => CoreType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), substitute_core_type_parameters(ty, args)))
                .collect(),
        ),
        CoreType::Function {
            params,
            return_type,
            effects,
        } => CoreType::Function {
            params: params
                .iter()
                .map(|ty| substitute_core_type_parameters(ty, args))
                .collect(),
            return_type: Box::new(substitute_core_type_parameters(return_type, args)),
            effects: effects.clone(),
        },
        CoreType::Unknown => CoreType::Unknown,
    }
}

fn core_type_template(ty: &Type) -> CoreType {
    match ty {
        Type::Unknown => CoreType::Unknown,
        Type::Named { name, args } => CoreType::Named {
            name: name.clone(),
            args: args.iter().map(core_type_template).collect(),
        },
        Type::Record(fields) => CoreType::Record(
            fields
                .iter()
                .map(|(name, ty)| (name.clone(), core_type_template(ty)))
                .collect(),
        ),
        Type::Function {
            params,
            return_type,
            effects,
        } => CoreType::Function {
            params: params.iter().map(core_type_template).collect(),
            return_type: Box::new(core_type_template(return_type)),
            effects: effects.clone(),
        },
    }
}

fn named_part<'a>(ty: &'a Type, name: &str, arity: usize) -> Option<&'a Type> {
    let Type::Named {
        name: ty_name,
        args,
    } = ty
    else {
        return None;
    };
    (ty_name == name && args.len() == arity)
        .then(|| args.first())
        .flatten()
}

fn core_named_part<'a>(ty: &'a CoreType, name: &str, arity: usize) -> Option<&'a CoreType> {
    let CoreType::Named {
        name: ty_name,
        args,
    } = ty
    else {
        return None;
    };
    (ty_name == name && args.len() == arity)
        .then(|| args.first())
        .flatten()
}

fn named_parts2<'a>(ty: &'a Type, name: &str) -> Option<(&'a Type, &'a Type)> {
    let Type::Named {
        name: ty_name,
        args,
    } = ty
    else {
        return None;
    };
    (ty_name == name && args.len() == 2).then(|| (&args[0], &args[1]))
}

fn core_named_parts2<'a>(ty: &'a CoreType, name: &str) -> Option<(&'a CoreType, &'a CoreType)> {
    let CoreType::Named {
        name: ty_name,
        args,
    } = ty
    else {
        return None;
    };
    (ty_name == name && args.len() == 2).then(|| (&args[0], &args[1]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> AdtRegistry {
        AdtRegistry {
            descriptors: builtin_descriptors(),
        }
    }

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|part| (*part).to_string()).collect()
    }

    #[test]
    fn constructors_match_qualified_and_unqualified_builtin_names() {
        let registry = registry();
        let ConstructorLookup::Found(some) = registry.constructor(&path(&["Some"]), None, &[])
        else {
            panic!("Some should resolve");
        };
        assert_eq!(some.descriptor.type_name, "Option");
        assert_eq!(some.variant.name, "Some");
        assert_eq!(some.variant.coverage_case, "Some(_)");
        assert_eq!(some.variant.payload_fields[0].name, "value");

        let ConstructorLookup::Found(nil) =
            registry.nullary_constructor(&path(&["List", "Nil"]), None, &[])
        else {
            panic!("List::Nil should resolve");
        };
        assert_eq!(nil.descriptor.type_name, "List");
        assert_eq!(nil.variant.name, "Nil");
        assert_eq!(nil.variant.coverage_case, "Nil");
        assert!(nil.variant.payload_fields.is_empty());
    }
}
