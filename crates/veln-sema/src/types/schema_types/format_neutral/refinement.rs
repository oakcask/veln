use super::*;

type VisibilityContext<'a> = (
    &'a SurfaceModule,
    Option<&'a str>,
    &'a AdtRegistry,
    FormatNeutralSchemaTraversal,
);

pub(super) fn format_neutral_schema_written_constructors_are_visible(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    traversal: FormatNeutralSchemaTraversal,
) -> bool {
    let context = (module, current_module, adts, traversal);
    match ty {
        Type::Named { name, args, .. } => named_constructors_are_visible(context, name, args),
        Type::VariantRefinement {
            name,
            args,
            variants,
            unresolved_alternatives,
            ..
        } => refinement_constructors_are_visible(
            context,
            name,
            args,
            variants,
            unresolved_alternatives,
        ),
        Type::Record(fields) => {
            types_are_visible(context, fields.iter().map(|(_, field_ty)| field_ty))
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => function_constructors_are_visible(context, params, variadic.as_deref(), return_type),
        Type::Unknown => true,
    }
}

fn named_constructors_are_visible(
    context: VisibilityContext<'_>,
    name: &str,
    args: &[Type],
) -> bool {
    let selection_is_visible = name.rsplit_once("::").is_none_or(|(base, selected)| {
        selected_constructors_are_visible(
            context,
            &Type::named(base, args.to_vec()),
            std::slice::from_ref(&selected.to_string()),
        )
    });
    selection_is_visible && types_are_visible(context, args.iter())
}

fn refinement_constructors_are_visible(
    context: VisibilityContext<'_>,
    name: &str,
    args: &[Type],
    variants: &[String],
    unresolved_alternatives: &[(String, Vec<Type>, String)],
) -> bool {
    selected_constructors_are_visible(context, &Type::named(name, args.to_vec()), variants)
        && types_are_visible(context, args.iter())
        && unresolved_alternatives
            .iter()
            .all(|(name, args, selected)| {
                selected_constructors_are_visible(
                    context,
                    &Type::named(name, args.clone()),
                    std::slice::from_ref(selected),
                ) && types_are_visible(context, args.iter())
            })
}

fn function_constructors_are_visible(
    context: VisibilityContext<'_>,
    params: &[Type],
    variadic: Option<&Type>,
    return_type: &Type,
) -> bool {
    let (module, current_module, adts, traversal) = context;
    types_are_visible(context, params.iter())
        && variadic.is_none_or(|variadic| {
            format_neutral_schema_written_constructors_are_visible(
                module,
                current_module,
                adts,
                variadic,
                traversal,
            )
        })
        && format_neutral_schema_written_constructors_are_visible(
            module,
            current_module,
            adts,
            return_type,
            traversal,
        )
}

fn types_are_visible<'a>(
    context: VisibilityContext<'_>,
    types: impl Iterator<Item = &'a Type>,
) -> bool {
    let (module, current_module, adts, traversal) = context;
    types.into_iter().all(|ty| {
        format_neutral_schema_written_constructors_are_visible(
            module,
            current_module,
            adts,
            ty,
            traversal,
        )
    })
}

fn selected_constructors_are_visible(
    context: VisibilityContext<'_>,
    base: &Type,
    variants: &[String],
) -> bool {
    let (module, current_module, adts, traversal) = context;
    format_neutral_schema_source_adt_descriptor(module, current_module, adts, base).is_none_or(
        |descriptor| {
            variants.iter().all(|selected| {
                descriptor.variants.iter().any(|variant| {
                    variant.name == *selected
                        && (variant.visibility == Visibility::Public
                            || traversal.permits_private_variants()
                                && descriptor.module_name.as_deref() == current_module)
                })
            })
        },
    )
}

pub(crate) fn format_neutral_schema_encode_field_is_source_adt_candidate(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> bool {
    let Ok(parsed) = parse_type_annotation(text) else {
        return false;
    };
    if written_refinement_base_is_adt_candidate(adts, schema.module_name.as_deref(), &parsed) {
        return true;
    }
    let ty = crate::types::canonicalize_type_effects(
        parsed,
        &normal_use_decls(module),
        &module
            .uses
            .iter()
            .filter(|use_decl| crate::name_recovery::use_decl_has_invalid_casing(module, use_decl))
            .cloned()
            .collect::<Vec<_>>(),
        schema.module_name.as_deref(),
        &[],
        adts,
        &BTreeMap::new(),
    );
    encode_type_is_source_adt_candidate(adts, schema.module_name.as_deref(), &ty)
}

fn written_refinement_base_is_adt_candidate(
    adts: &AdtRegistry,
    current_module: Option<&str>,
    ty: &Type,
) -> bool {
    match ty {
        Type::Named { name, args, .. } => {
            let written_base = name.rsplit_once("::").map(|(base, _)| {
                let base = Type::named(base, args.clone());
                adts.descriptor_for_type(&base)
                    .or_else(|| adts.descriptor_for_type_in_module(&base, current_module))
            });
            written_base.flatten().is_some_and(|descriptor| {
                descriptor_has_source_declaration(adts, descriptor)
                    && (descriptor.module_name.as_deref() == current_module
                        || descriptor.visibility == Visibility::Public)
            }) || args
                .iter()
                .any(|arg| written_refinement_base_is_adt_candidate(adts, current_module, arg))
        }
        Type::Record(fields) => fields.iter().any(|(_, field_ty)| {
            written_refinement_base_is_adt_candidate(adts, current_module, field_ty)
        }),
        _ => false,
    }
}

pub(crate) fn format_neutral_schema_encode_field_contains_refinement(
    adts: &AdtRegistry,
    current_module: Option<&str>,
    text: &str,
) -> bool {
    parse_type_annotation(text).is_ok_and(|ty| {
        encode_type_contains_refinement(&ty)
            || written_type_contains_adt_refinement(adts, current_module, &ty)
    })
}

fn written_type_contains_adt_refinement(
    adts: &AdtRegistry,
    current_module: Option<&str>,
    ty: &Type,
) -> bool {
    match ty {
        Type::Named { name, args, .. } => {
            name.rsplit_once("::").is_some_and(|(base, _)| {
                let base = Type::named(base, args.clone());
                adts.descriptor_for_type(&base)
                    .or_else(|| adts.descriptor_for_type_in_module(&base, current_module))
                    .is_some_and(|descriptor| {
                        descriptor.module_name.as_deref() == current_module
                            || descriptor.visibility == Visibility::Public
                    })
            }) || args
                .iter()
                .any(|arg| written_type_contains_adt_refinement(adts, current_module, arg))
        }
        Type::Record(fields) => fields.iter().any(|(_, field_ty)| {
            written_type_contains_adt_refinement(adts, current_module, field_ty)
        }),
        Type::VariantRefinement { .. } => true,
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params
                .iter()
                .any(|param| written_type_contains_adt_refinement(adts, current_module, param))
                || variadic.as_deref().is_some_and(|variadic| {
                    written_type_contains_adt_refinement(adts, current_module, variadic)
                })
                || written_type_contains_adt_refinement(adts, current_module, return_type)
        }
        Type::Unknown => false,
    }
}

fn encode_type_contains_refinement(ty: &Type) -> bool {
    match ty {
        Type::VariantRefinement { .. } => true,
        Type::Named { args, .. } => args.iter().any(encode_type_contains_refinement),
        Type::Record(fields) => fields
            .iter()
            .any(|(_, field_ty)| encode_type_contains_refinement(field_ty)),
        _ => false,
    }
}

fn encode_type_is_source_adt_candidate(
    adts: &AdtRegistry,
    current_module: Option<&str>,
    ty: &Type,
) -> bool {
    match ty {
        Type::VariantRefinement { .. } => adts.descriptor_for_type(ty).is_some_and(|descriptor| {
            descriptor_has_source_declaration(adts, descriptor)
                && (descriptor.module_name.as_deref() == current_module
                    || descriptor.visibility == Visibility::Public)
        }),
        Type::Named { name, args, .. }
            if matches!(
                name.as_str(),
                "Int" | "Bool" | "Float" | "String" | "Option" | "List" | "Vec" | "Dict" | "Result"
            ) =>
        {
            args.iter()
                .any(|arg| encode_type_is_source_adt_candidate(adts, current_module, arg))
        }
        Type::Named { .. } => adts.descriptor_for_type(ty).is_some_and(|descriptor| {
            descriptor_has_source_declaration(adts, descriptor)
                && (descriptor.module_name.as_deref() == current_module
                    || descriptor.visibility == Visibility::Public)
        }),
        Type::Record(fields) => fields.iter().any(|(_, field_ty)| {
            encode_type_is_source_adt_candidate(adts, current_module, field_ty)
        }),
        _ => false,
    }
}
