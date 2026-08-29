use super::*;

pub(crate) fn validate_adt_lookup_descriptors(
    provider: &'static str,
    descriptors: &[AdtDescriptor],
) -> Result<(), InvalidStandardSymbolCase> {
    let mut type_names = BTreeSet::new();
    let mut constructor_names = BTreeSet::new();
    for descriptor in descriptors {
        if let Some(module_name) = &descriptor.module_name {
            for segment in module_name.split("::") {
                validate_source_less_lookup_segment(
                    provider,
                    segment,
                    SourceLessNameClass::Module,
                )?;
            }
        }
        if descriptor.name_class != SourceLessNameClass::Type {
            return Err(InvalidStandardSymbolCase {
                provider,
                name: descriptor.type_name.clone(),
                name_class: SourceLessNameClass::Type,
                reason: InvalidStandardSymbolReason::InvalidLookupClass,
            });
        }
        validate_source_less_lookup_segment(
            provider,
            &descriptor.type_name,
            descriptor.name_class,
        )?;
        if !type_names.insert((
            descriptor.module_name.as_deref(),
            descriptor.type_name.as_str(),
        )) {
            return Err(InvalidStandardSymbolCase {
                provider,
                name: adt_type_lookup_key(descriptor),
                name_class: SourceLessNameClass::Type,
                reason: InvalidStandardSymbolReason::DuplicateLookupKey,
            });
        }
        for variant in &descriptor.variants {
            if variant.name_class != SourceLessNameClass::Constructor {
                return Err(InvalidStandardSymbolCase {
                    provider,
                    name: variant.name.clone(),
                    name_class: SourceLessNameClass::Constructor,
                    reason: InvalidStandardSymbolReason::InvalidLookupClass,
                });
            }
            validate_source_less_lookup_segment(provider, &variant.name, variant.name_class)?;
            if !constructor_names.insert((
                descriptor.module_name.as_deref(),
                descriptor.type_name.as_str(),
                variant.name.as_str(),
            )) {
                return Err(InvalidStandardSymbolCase {
                    provider,
                    name: adt_constructor_lookup_key(descriptor, variant),
                    name_class: SourceLessNameClass::Constructor,
                    reason: InvalidStandardSymbolReason::DuplicateLookupKey,
                });
            }
        }
    }
    Ok(())
}

pub(super) fn adt_type_lookup_key(descriptor: &AdtDescriptor) -> String {
    match descriptor.module_name.as_deref() {
        Some(module_name) => format!("{module_name}::{}", descriptor.type_name),
        None => descriptor.type_name.clone(),
    }
}

pub(super) fn adt_constructor_lookup_key(
    descriptor: &AdtDescriptor,
    variant: &AdtVariantDescriptor,
) -> String {
    match descriptor.module_name.as_deref() {
        Some(module_name) => {
            format!("{module_name}::{}::{}", descriptor.type_name, variant.name)
        }
        None => format!("{}::{}", descriptor.type_name, variant.name),
    }
}

pub(super) fn source_descriptor(decl: &TypeDecl) -> Option<AdtDescriptor> {
    let name = decl.name.clone()?;
    if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
        return None;
    }
    if matches!(name.as_str(), "Option" | "Result" | "List") {
        return None;
    }
    let variants = decl
        .variants
        .iter()
        .filter_map(|variant| {
            let name = variant.name.clone()?;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                return None;
            }
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
                name_class: SourceLessNameClass::Constructor,
                kind: AdtVariantKind::Source,
                payload_fields,
                coverage_case,
                visibility: if decl.module_name.as_deref() == Some("std::prelude")
                    && decl.visibility == Visibility::Public
                {
                    Visibility::Public
                } else {
                    variant.visibility
                },
            })
        })
        .collect::<Vec<_>>();
    Some(AdtDescriptor {
        type_name: name.clone(),
        name_class: SourceLessNameClass::Type,
        module_name: decl.module_name.clone(),
        type_parameters: decl.params.clone(),
        variants,
        diagnostic_name: name.to_lowercase(),
        propagation: None,
        visibility: decl.visibility,
    })
}

pub(super) fn payload_descriptor_type(text: &str, decl: &TypeDecl) -> AdtPayloadType {
    if let Some(index) = decl.params.iter().position(|param| param == text) {
        return AdtPayloadType::TypeParameter(index);
    }
    let ty = parse_adt_payload_type_or_unknown(text);
    if is_self_type(&ty, decl) {
        AdtPayloadType::SelfType
    } else {
        AdtPayloadType::Concrete(type_parameters_to_placeholders(ty, &decl.params))
    }
}

pub(super) fn parse_adt_payload_type_or_unknown(text: &str) -> Type {
    parse_type_annotation_with_arity(text, &adt_builtin_type_arity).unwrap_or(Type::Unknown)
}

pub(super) fn adt_builtin_type_arity(name: &str) -> Result<Option<usize>, String> {
    BuiltinTypeSyntaxRegistry::from_validated_source_less_descriptors(
        BUILTIN_TYPE_SYNTAX_DESCRIPTORS,
    )
    .map(|registry| registry.arity(name))
    .map_err(|failure| failure.diagnostic().message)
}

pub(super) fn is_self_type(ty: &Type, decl: &TypeDecl) -> bool {
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

pub(super) fn type_parameters_to_placeholders(ty: Type, params: &[String]) -> Type {
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
            variadic,
            return_type,
            effects,
        } => Type::Function {
            params: fn_params
                .into_iter()
                .map(|ty| type_parameters_to_placeholders(ty, params))
                .collect(),
            variadic: variadic.map(|ty| Box::new(type_parameters_to_placeholders(*ty, params))),
            return_type: Box::new(type_parameters_to_placeholders(*return_type, params)),
            effects,
        },
        Type::Unknown => Type::Unknown,
    }
}

pub(super) fn constructor_matches_visible_path(
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

pub(super) fn companion_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            let companion = classify_companion_source(function.span.file.as_str())?;
            let companion_module = function.module_name.clone()?;
            let target_module = companion
                .target_path
                .strip_suffix(".veln")?
                .replace('/', "::");
            Some((companion_module, target_module))
        })
        .collect()
}

pub(super) fn standard_prelude_alias_matches(descriptor: &AdtDescriptor, alias: &str) -> bool {
    descriptor.module_name.is_none()
        && matches!(
            descriptor.type_name.as_str(),
            "StreamInput"
                | "StreamAdapterAction"
                | "DecodeError"
                | "DecodeReadiness"
                | "DecodeStep"
                | "SchemaDispatchPayload"
                | "EncodeError"
                | "RuntimeDiagnostic"
                | "RuntimeDiagnosticDetail"
                | "Http2DiagnosticDetail"
                | "HpackDiagnosticDetail"
                | "RuntimeDiagnosticFieldPathSegment"
                | "RuntimeByteDiagnosticFacts"
                | "RuntimeBytePreview"
                | "EncodeStep"
        )
        && descriptor.visibility == Visibility::Public
        && alias == PRELUDE_MODULE
}

pub(super) fn import_alias_matches(
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

pub(super) fn same_descriptor(left: &AdtDescriptor, right: &AdtDescriptor) -> bool {
    left.type_name == right.type_name
        && left.module_name == right.module_name
        && left.type_parameters.len() == right.type_parameters.len()
}
