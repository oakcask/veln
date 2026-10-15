use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalTypeAnnotationIdentity {
    canonical: Type,
    type_parameter_occurrences: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CanonicalTypeAnnotationWork {
    pub type_parameter_contexts: usize,
    pub type_parameter_names: usize,
    pub type_parameter_lookups: usize,
}

pub fn canonical_type_annotation_identities_with_context(
    project: &SurfaceModule,
    annotations: &[(&str, Option<&str>, &[String])],
) -> Vec<Option<CanonicalTypeAnnotationIdentity>> {
    canonical_type_annotation_identities_with_context_impl(project, annotations, false).0
}

pub fn canonical_type_annotation_identities_with_context_and_work(
    project: &SurfaceModule,
    annotations: &[(&str, Option<&str>, &[String])],
) -> (
    Vec<Option<CanonicalTypeAnnotationIdentity>>,
    CanonicalTypeAnnotationWork,
) {
    canonical_type_annotation_identities_with_context_impl(project, annotations, true)
}

fn canonical_type_annotation_identities_with_context_impl(
    project: &SurfaceModule,
    annotations: &[(&str, Option<&str>, &[String])],
    record_work: bool,
) -> (
    Vec<Option<CanonicalTypeAnnotationIdentity>>,
    CanonicalTypeAnnotationWork,
) {
    let environment = TypeEnvironment::for_path_classification(project);
    let mut type_parameter_contexts = BTreeMap::new();
    for (_, _, type_parameters) in annotations {
        let key = (type_parameters.as_ptr() as usize, type_parameters.len());
        type_parameter_contexts.entry(key).or_insert_with(|| {
            if record_work {
                crate::types::TypeParameterIdentityContext::recording(type_parameters)
            } else {
                crate::types::TypeParameterIdentityContext::new(type_parameters)
            }
        });
    }
    let identities = annotations
        .iter()
        .map(|(annotation, current_module, type_parameters)| {
            let key = (type_parameters.as_ptr() as usize, type_parameters.len());
            let type_parameters = &type_parameter_contexts[&key];
            crate::type_syntax::parse_type_annotation(annotation)
                .ok()
                .and_then(|ty| {
                    environment
                        .type_annotation_is_fully_resolved_with_context(
                            &ty,
                            *current_module,
                            type_parameters,
                        )
                        .then(|| CanonicalTypeAnnotationIdentity {
                            type_parameter_occurrences: type_parameter_occurrences(
                                &ty,
                                type_parameters,
                            ),
                            canonical: canonical_identity_type(
                                environment.canonicalize_type_annotation(ty, *current_module),
                            ),
                        })
                })
        })
        .collect();
    let work = CanonicalTypeAnnotationWork {
        type_parameter_contexts: type_parameter_contexts.len(),
        type_parameter_names: type_parameter_contexts
            .values()
            .map(crate::types::TypeParameterIdentityContext::name_count)
            .sum(),
        type_parameter_lookups: type_parameter_contexts
            .values()
            .map(crate::types::TypeParameterIdentityContext::lookup_count)
            .sum(),
    };
    (identities, work)
}

fn type_parameter_occurrences(
    ty: &Type,
    type_parameters: &crate::types::TypeParameterIdentityContext<'_>,
) -> Vec<usize> {
    let mut occurrences = Vec::new();
    collect_type_parameter_occurrences(ty, type_parameters, &mut occurrences);
    occurrences
}

fn collect_type_parameter_occurrences(
    ty: &Type,
    type_parameters: &crate::types::TypeParameterIdentityContext<'_>,
    occurrences: &mut Vec<usize>,
) {
    match ty {
        Type::Named { name, args, .. } => {
            if args.is_empty()
                && let Some(index) = type_parameters.ordinal(name)
            {
                occurrences.push(index);
            }
            for argument in args {
                collect_type_parameter_occurrences(argument, type_parameters, occurrences);
            }
        }
        Type::VariantRefinement {
            args,
            unresolved_alternatives,
            ..
        } => {
            for argument in args {
                collect_type_parameter_occurrences(argument, type_parameters, occurrences);
            }
            for (_, arguments, _) in unresolved_alternatives {
                for argument in arguments {
                    collect_type_parameter_occurrences(argument, type_parameters, occurrences);
                }
            }
        }
        Type::Record(fields) => {
            for (_, field) in fields {
                collect_type_parameter_occurrences(field, type_parameters, occurrences);
            }
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            for parameter in params {
                collect_type_parameter_occurrences(parameter, type_parameters, occurrences);
            }
            if let Some(variadic) = variadic {
                collect_type_parameter_occurrences(variadic, type_parameters, occurrences);
            }
            collect_type_parameter_occurrences(return_type, type_parameters, occurrences);
        }
        Type::Unknown => {}
    }
}

fn canonical_identity_type(ty: Type) -> Type {
    match ty {
        Type::Named { identity, args, .. } => Type::Named {
            name: identity.clone(),
            identity,
            args: args.into_iter().map(canonical_identity_type).collect(),
        },
        Type::VariantRefinement {
            identity,
            args,
            variants,
            unresolved_alternatives,
            ..
        } => Type::VariantRefinement {
            name: identity.clone(),
            identity,
            args: args.into_iter().map(canonical_identity_type).collect(),
            variants,
            unresolved_alternatives: unresolved_alternatives
                .into_iter()
                .map(|(name, args, variant)| {
                    (
                        name,
                        args.into_iter().map(canonical_identity_type).collect(),
                        variant,
                    )
                })
                .collect(),
        },
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name, canonical_identity_type(ty)))
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => Type::Function {
            params: params.into_iter().map(canonical_identity_type).collect(),
            variadic: variadic.map(|ty| Box::new(canonical_identity_type(*ty))),
            return_type: Box::new(canonical_identity_type(*return_type)),
            effects,
        },
        Type::Unknown => Type::Unknown,
    }
}
