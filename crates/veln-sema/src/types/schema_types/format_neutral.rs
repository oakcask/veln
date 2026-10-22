use super::*;

use crate::adt::descriptors::{AdtConstructor, AdtDescriptor};
use crate::adt::registry::AdtRegistry;
use crate::adt::type_operations as adt;
use crate::name_recovery::{normal_use_decls, use_decl_has_invalid_module_segment};
use crate::type_syntax::parse_type_annotation;

pub(super) fn format_neutral_schema_decode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    let decoded_type = Type::Record(format_neutral_schema_decode_record_fields_with_adts(
        module, schema, adts,
    )?);
    Some(FunctionSignature {
        name: schema_decode_function_name(schema_name),
        target_name: format!("{SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![decoded_type.clone()],
        variadic: None,
        return_type: Type::named("Result", vec![decoded_type, Type::string()]),
        effects: Vec::new(),
        callsite: false,
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

pub(crate) fn format_neutral_schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type)>> {
    let adts = AdtRegistry::from_module(module);
    format_neutral_schema_decode_record_fields_with_adts(module, schema, &adts)
}

fn format_neutral_schema_decode_record_fields_with_adts(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<Vec<(String, Type)>> {
    schema
        .fields
        .iter()
        .map(|field| {
            Some((
                field.name.clone(),
                format_neutral_schema_field_type_for_schema(module, schema, adts, &field.ty)?,
            ))
        })
        .collect()
}

pub(crate) fn format_neutral_schema_field_type_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> Option<Type> {
    format_neutral_schema_field_type_for_traversal(
        module,
        schema,
        adts,
        text,
        FormatNeutralSchemaTraversal::Decode,
    )
}

fn format_neutral_schema_field_type_for_traversal(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    format_neutral_schema_resolved_field_type(
        module,
        schema,
        adts,
        text,
        traversal,
        &mut Vec::new(),
    )
}

fn format_neutral_schema_resolved_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
    traversal: FormatNeutralSchemaTraversal,
    composition_stack: &mut Vec<(Option<String>, String)>,
) -> Option<Type> {
    let parsed = parse_type_annotation(text).ok()?;
    if traversal.is_encode()
        && !format_neutral_schema_written_constructors_are_visible(
            module,
            schema.module_name.as_deref(),
            adts,
            &parsed,
            traversal,
        )
    {
        return None;
    }
    let ty = if traversal.is_encode() {
        crate::types::canonicalize_type_effects(
            parsed,
            &normal_use_decls(module),
            &module
                .uses
                .iter()
                .filter(|use_decl| use_decl_has_invalid_module_segment(module, use_decl))
                .cloned()
                .collect::<Vec<_>>(),
            schema.module_name.as_deref(),
            &[],
            adts,
            &BTreeMap::new(),
        )
    } else {
        parsed
    };
    if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
        module,
        schema.module_name.as_deref(),
        adts,
        &ty,
        &mut FormatNeutralSchemaTraversalState::default(),
        traversal,
    ) {
        return Some(ty);
    }
    if let Some(target) = schema_field_target(module, schema, text)
        && target.format.is_none()
    {
        return format_neutral_schema_composition_value_type(
            module,
            target,
            traversal,
            composition_stack,
        );
    }
    None
}

fn format_neutral_schema_written_constructors_are_visible(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    traversal: FormatNeutralSchemaTraversal,
) -> bool {
    let selected_are_visible = |descriptor: &AdtDescriptor, variants: &[String]| {
        variants.iter().all(|selected| {
            descriptor.variants.iter().any(|variant| {
                variant.name == *selected
                    && (variant.visibility == Visibility::Public
                        || traversal.permits_private_variants()
                            && descriptor.module_name.as_deref() == current_module)
            })
        })
    };
    match ty {
        Type::Named { name, args, .. } => {
            let selection_is_visible = name.rsplit_once("::").is_none_or(|(base, selected)| {
                let base = Type::named(base, args.clone());
                format_neutral_schema_source_adt_descriptor(module, current_module, adts, &base)
                    .is_none_or(|descriptor| {
                        selected_are_visible(descriptor, &[selected.to_string()])
                    })
            });
            selection_is_visible
                && args.iter().all(|arg| {
                    format_neutral_schema_written_constructors_are_visible(
                        module,
                        current_module,
                        adts,
                        arg,
                        traversal,
                    )
                })
        }
        Type::VariantRefinement {
            name,
            args,
            variants,
            unresolved_alternatives,
            ..
        } => {
            let base = Type::named(name, args.clone());
            let selected_are_visible_here =
                format_neutral_schema_source_adt_descriptor(module, current_module, adts, &base)
                    .is_none_or(|descriptor| selected_are_visible(descriptor, variants));
            selected_are_visible_here
                && args.iter().all(|arg| {
                    format_neutral_schema_written_constructors_are_visible(
                        module,
                        current_module,
                        adts,
                        arg,
                        traversal,
                    )
                })
                && unresolved_alternatives
                    .iter()
                    .all(|(name, args, selected)| {
                        let base = Type::named(name, args.clone());
                        format_neutral_schema_source_adt_descriptor(
                            module,
                            current_module,
                            adts,
                            &base,
                        )
                        .is_none_or(|descriptor| {
                            selected_are_visible(descriptor, std::slice::from_ref(selected))
                        }) && args.iter().all(|arg| {
                            format_neutral_schema_written_constructors_are_visible(
                                module,
                                current_module,
                                adts,
                                arg,
                                traversal,
                            )
                        })
                    })
        }
        Type::Record(fields) => fields.iter().all(|(_, field_ty)| {
            format_neutral_schema_written_constructors_are_visible(
                module,
                current_module,
                adts,
                field_ty,
                traversal,
            )
        }),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().all(|param| {
                format_neutral_schema_written_constructors_are_visible(
                    module,
                    current_module,
                    adts,
                    param,
                    traversal,
                )
            }) && variadic.as_deref().is_none_or(|variadic| {
                format_neutral_schema_written_constructors_are_visible(
                    module,
                    current_module,
                    adts,
                    variadic,
                    traversal,
                )
            }) && format_neutral_schema_written_constructors_are_visible(
                module,
                current_module,
                adts,
                return_type,
                traversal,
            )
        }
        Type::Unknown => true,
    }
}

pub(crate) fn binary_schema_anonymous_record_decode_type(text: &str) -> Option<Type> {
    let Type::Record(fields) = parse_type_annotation(text).ok()? else {
        return None;
    };
    binary_schema_anonymous_record_type(fields).map(Type::Record)
}

fn binary_schema_anonymous_record_type(fields: Vec<(String, Type)>) -> Option<Vec<(String, Type)>> {
    fields
        .into_iter()
        .map(|(name, ty)| {
            if binary_schema_anonymous_record_leaf_type(&ty).is_some() {
                return Some((name, Type::int()));
            }
            let Type::Record(fields) = ty else {
                return None;
            };
            Some((
                name,
                Type::Record(binary_schema_anonymous_record_type(fields)?),
            ))
        })
        .collect()
}

fn binary_schema_anonymous_record_leaf_type(ty: &Type) -> Option<()> {
    match ty {
        Type::Named { name, args, .. }
            if args.is_empty() && exact_width_schema_primitive(name).is_some() =>
        {
            Some(())
        }
        _ => None,
    }
}

fn format_neutral_schema_scalar_type_is_supported(name: &str, args: &[Type]) -> bool {
    args.is_empty() && matches!(name, "Int" | "Bool" | "Float" | "String")
}

fn format_neutral_schema_scalar_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args, .. }
            if format_neutral_schema_scalar_type_is_supported(name, args)
    )
}

pub(crate) fn format_neutral_schema_encode_field_type_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> Option<Type> {
    let traversal = if schema.visibility == Visibility::Public {
        FormatNeutralSchemaTraversal::EncodePublic
    } else {
        FormatNeutralSchemaTraversal::EncodePrivate
    };
    format_neutral_schema_field_type_for_traversal(module, schema, adts, text, traversal)
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
    if format_neutral_schema_encode_type_contains_refinement(&parsed) {
        return true;
    }
    if format_neutral_schema_encode_written_refinement_base_is_adt_candidate(
        adts,
        schema.module_name.as_deref(),
        &parsed,
    ) {
        return true;
    }
    let ty = crate::types::canonicalize_type_effects(
        parsed,
        &normal_use_decls(module),
        &module
            .uses
            .iter()
            .filter(|use_decl| use_decl_has_invalid_module_segment(module, use_decl))
            .cloned()
            .collect::<Vec<_>>(),
        schema.module_name.as_deref(),
        &[],
        adts,
        &BTreeMap::new(),
    );
    format_neutral_schema_encode_type_is_source_adt_candidate(
        adts,
        schema.module_name.as_deref(),
        &ty,
    )
}

fn format_neutral_schema_encode_written_refinement_base_is_adt_candidate(
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
                descriptor.module_name.as_deref() == current_module
                    || descriptor.visibility == Visibility::Public
            }) || args.iter().any(|arg| {
                format_neutral_schema_encode_written_refinement_base_is_adt_candidate(
                    adts,
                    current_module,
                    arg,
                )
            })
        }
        Type::Record(fields) => fields.iter().any(|(_, field_ty)| {
            format_neutral_schema_encode_written_refinement_base_is_adt_candidate(
                adts,
                current_module,
                field_ty,
            )
        }),
        _ => false,
    }
}

fn format_neutral_schema_encode_type_contains_refinement(ty: &Type) -> bool {
    match ty {
        Type::VariantRefinement { .. } => true,
        Type::Named { args, .. } => args
            .iter()
            .any(format_neutral_schema_encode_type_contains_refinement),
        Type::Record(fields) => fields
            .iter()
            .any(|(_, field_ty)| format_neutral_schema_encode_type_contains_refinement(field_ty)),
        _ => false,
    }
}

fn format_neutral_schema_encode_type_is_source_adt_candidate(
    adts: &AdtRegistry,
    current_module: Option<&str>,
    ty: &Type,
) -> bool {
    match ty {
        Type::VariantRefinement { .. } => true,
        Type::Named { name, args, .. }
            if matches!(
                name.as_str(),
                "Int" | "Bool" | "Float" | "String" | "Option" | "List" | "Vec" | "Dict" | "Result"
            ) =>
        {
            args.iter().any(|arg| {
                format_neutral_schema_encode_type_is_source_adt_candidate(adts, current_module, arg)
            })
        }
        Type::Named { .. } => adts.descriptor_for_type(ty).is_some_and(|descriptor| {
            descriptor.module_name.as_deref() == current_module
                || descriptor.visibility == Visibility::Public
        }),
        Type::Record(fields) => fields.iter().any(|(_, field_ty)| {
            format_neutral_schema_encode_type_is_source_adt_candidate(
                adts,
                current_module,
                field_ty,
            )
        }),
        _ => false,
    }
}

pub(super) fn format_neutral_schema_encode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<Vec<(String, Type)>> {
    schema
        .fields
        .iter()
        .map(|field| {
            let ty = format_neutral_schema_encode_field_type_for_schema(
                module, schema, adts, &field.ty,
            )?;
            Some((field.name.clone(), ty))
        })
        .collect()
}

pub(crate) fn format_neutral_schema_first_unsupported_encode_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<SchemaField> {
    if schema.format.is_some() {
        return None;
    }
    schema
        .fields
        .iter()
        .find(|field| {
            let declaration_diagnostic_exists =
                format_neutral_schema_field_type_for_schema(module, schema, adts, &field.ty)
                    .is_none()
                    && format_neutral_schema_encode_field_is_source_adt_candidate(
                        module, schema, adts, &field.ty,
                    );
            !declaration_diagnostic_exists
                && format_neutral_schema_encode_field_type_for_schema(
                    module, schema, adts, &field.ty,
                )
                .is_none()
        })
        .cloned()
}

#[derive(Clone, PartialEq, Eq)]
struct FormatNeutralSchemaAdtFrame {
    module_name: Option<String>,
    type_name: String,
    type_arguments: Vec<Type>,
}

#[derive(Default)]
struct FormatNeutralSchemaTraversalState {
    stack: Vec<FormatNeutralSchemaAdtFrame>,
    stack_cacheable: Vec<bool>,
    completed: Vec<(FormatNeutralSchemaAdtFrame, bool)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatNeutralSchemaTraversal {
    Decode,
    EncodePrivate,
    EncodePublic,
}

impl FormatNeutralSchemaTraversal {
    fn is_encode(self) -> bool {
        matches!(self, Self::EncodePrivate | Self::EncodePublic)
    }

    fn permits_private_variants(self) -> bool {
        self == Self::EncodePrivate
    }
}

fn format_neutral_schema_composition_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    traversal: FormatNeutralSchemaTraversal,
    stack: &mut Vec<(Option<String>, String)>,
) -> Option<Type> {
    let key = (schema.module_name.clone(), schema.name.clone()?);
    if stack.contains(&key) {
        return None;
    }
    stack.push(key);
    let adts = AdtRegistry::from_module(module);
    let mut fields = Vec::new();
    for field in &schema.fields {
        let ty = format_neutral_schema_resolved_field_type(
            module, schema, &adts, &field.ty, traversal, stack,
        )?;
        fields.push((field.name.clone(), ty));
    }
    stack.pop();
    Some(Type::Record(fields))
}

fn format_neutral_schema_visible_shape_type_for_schema(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    match ty {
        Type::Named { name, args, .. }
            if matches!(name.as_str(), "List" | "Vec") && args.len() == 1 =>
        {
            Some(Type::named(
                name.clone(),
                vec![format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[0],
                    state,
                    traversal,
                )?],
            ))
        }
        Type::Named { name, args, .. } if name == "Option" && args.len() == 1 => Some(Type::named(
            "Option",
            vec![format_neutral_schema_visible_shape_type_for_schema(
                module,
                current_module,
                adts,
                &args[0],
                state,
                traversal,
            )?],
        )),
        Type::Named { name, args, .. } if name == "Dict" && args.len() == 2 => {
            if !matches!(&args[0], Type::Named { name, args, .. } if name == "String" && args.is_empty())
            {
                return None;
            }
            Some(Type::dict(
                Type::string(),
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[1],
                    state,
                    traversal,
                )?,
            ))
        }
        Type::Named { name, args, .. } if name == "Result" && args.len() == 2 => Some(Type::named(
            "Result",
            vec![
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[0],
                    state,
                    traversal,
                )?,
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[1],
                    state,
                    traversal,
                )?,
            ],
        )),
        Type::Named { .. } if format_neutral_schema_scalar_type(ty) => Some(ty.clone()),
        Type::Named { .. } => format_neutral_schema_source_adt_type(
            module,
            current_module,
            adts,
            ty,
            state,
            traversal,
        ),
        Type::VariantRefinement { .. } if traversal.is_encode() => {
            format_neutral_schema_source_adt_type(
                module,
                current_module,
                adts,
                ty,
                state,
                traversal,
            )
        }
        Type::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|(name, field_ty)| {
                    Some((
                        name.clone(),
                        format_neutral_schema_visible_shape_type_for_schema(
                            module,
                            current_module,
                            adts,
                            field_ty,
                            state,
                            traversal,
                        )?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

fn format_neutral_schema_source_adt_type(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    let descriptor = format_neutral_schema_source_adt_descriptor(module, current_module, adts, ty)?;
    if let Type::VariantRefinement { variants, .. } = ty
        && variants.iter().any(|selected| {
            !descriptor.variants.iter().any(|variant| {
                variant.name == *selected
                    && (variant.visibility == Visibility::Public
                        || traversal.permits_private_variants()
                            && descriptor.module_name.as_deref() == current_module)
            })
        })
    {
        return None;
    }
    let descriptor_ty = format_neutral_schema_descriptor_type(ty, descriptor);
    let type_arguments = match &descriptor_ty {
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => args,
        _ => return None,
    };
    let frame = FormatNeutralSchemaAdtFrame {
        module_name: descriptor.module_name.clone(),
        type_name: descriptor.type_name.clone(),
        type_arguments: type_arguments.clone(),
    };
    if let Some((_, supported)) = state.completed.iter().find(|(key, _)| key == &frame) {
        return (*supported).then_some(descriptor_ty);
    }
    if let Some(index) = state.stack.iter().position(|active| active == &frame) {
        state.stack_cacheable[index + 1..].fill(false);
        return Some(descriptor_ty);
    }
    if let Some(index) = state.stack.iter().position(|active| {
        active.module_name == frame.module_name && active.type_name == frame.type_name
    }) {
        state.stack_cacheable[index + 1..].fill(false);
        if traversal == FormatNeutralSchemaTraversal::Decode {
            return Some(descriptor_ty);
        }
        return type_arguments
            .iter()
            .all(|arg| {
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    descriptor.module_name.as_deref(),
                    adts,
                    arg,
                    state,
                    traversal,
                )
                .is_some()
            })
            .then_some(descriptor_ty);
    }
    state.stack.push(frame.clone());
    state.stack_cacheable.push(true);
    let supported = descriptor.variants.iter().all(|variant| {
        variant
            .payload_fields
            .iter()
            .enumerate()
            .all(|(index, _field)| {
                let Some(payload_ty) = adt::payload_type(
                    &descriptor_ty,
                    AdtConstructor {
                        descriptor,
                        variant,
                    },
                    index,
                ) else {
                    return false;
                };
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    descriptor.module_name.as_deref(),
                    adts,
                    &payload_ty,
                    state,
                    traversal,
                )
                .is_some()
            })
    });
    state.stack.pop();
    if state
        .stack_cacheable
        .pop()
        .expect("ADT traversal cacheability should match the active stack")
    {
        state.completed.push((frame, supported));
    }
    supported.then_some(descriptor_ty)
}

fn format_neutral_schema_descriptor_type(ty: &Type, descriptor: &AdtDescriptor) -> Type {
    match ty {
        Type::Named { args, .. } => Type::resolved_named(
            descriptor.type_name.clone(),
            descriptor.identity(),
            args.clone(),
        ),
        Type::VariantRefinement {
            args,
            variants,
            unresolved_alternatives,
            ..
        } if unresolved_alternatives.is_empty() => Type::resolved_variant_refinement_shared(
            descriptor.type_name.clone(),
            descriptor.identity(),
            args.clone(),
            Arc::clone(variants),
        ),
        _ => ty.clone(),
    }
}

fn format_neutral_schema_source_adt_descriptor<'a>(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &'a AdtRegistry,
    ty: &Type,
) -> Option<&'a AdtDescriptor> {
    if let Some(descriptor) = adts.descriptor_for_type(ty)
        && (descriptor.module_name.as_deref() == current_module
            || descriptor.visibility == Visibility::Public)
    {
        return Some(descriptor);
    }
    let (name, args) = match ty {
        Type::Named { name, args, .. } | Type::VariantRefinement { name, args, .. } => (name, args),
        _ => return None,
    };
    let segments = name.split("::").collect::<Vec<_>>();
    match segments.as_slice() {
        [local_name] => adts
            .descriptor_for_type_in_module(&Type::named(*local_name, args.clone()), current_module),
        [_, .., type_name] => {
            let import_path = segments[..segments.len() - 1]
                .iter()
                .map(|segment| (*segment).to_string())
                .collect::<Vec<_>>();
            let use_decl = normal_imported_use_for_path(module, &import_path, current_module)?;
            adts.descriptors().iter().rev().find(|descriptor| {
                descriptor.type_name == *type_name
                    && descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
                    && descriptor.type_parameters.len() == args.len()
                    && descriptor.visibility == Visibility::Public
            })
        }
        _ => None,
    }
}
