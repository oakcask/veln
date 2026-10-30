use std::collections::{BTreeMap, HashMap};

use veln_ast::{SchemaDecl, SchemaField, SurfaceModule};
use veln_core::{
    CoreSchemaRefinementField, CoreSchemaRefinementPayload, CoreSchemaRefinementPlan,
    CoreSchemaRefinementPlanNode, CoreSchemaRefinementValidationSpec, CoreSchemaRefinementVariant,
};
use veln_ir::{
    IrSchemaDecodeDispatch, IrSchemaDecodeDispatchCase, IrSchemaDecodeField, IrSchemaDecodeSpec,
    IrSchemaRefinementField, IrSchemaRefinementPayload, IrSchemaRefinementPlan,
    IrSchemaRefinementPlanNode, IrSchemaRefinementVariant, IrSchemaRepeat, IrSchemaReservedBits,
    IrSchemaVariantRefinement,
};

use crate::adt::descriptors::{AdtConstructor, AdtDescriptor};
use crate::adt::registry::AdtRegistry;
use crate::adt::type_operations as adt;
use crate::schema::dispatch::{
    SchemaDispatchCase, SchemaDispatchCasePayload, SchemaDispatchSpec,
    closed_dispatch_schema_primitive, extension_dispatch_schema_primitive,
};
use crate::schema::primitives::{
    SchemaRepeatPayload, SchemaRepeatSpec, byte_view_multiple_constraint,
    byte_view_schema_primitive, exact_width_schema_primitive,
    exact_width_schema_primitive_little_endian, exact_width_schema_primitive_max_value,
    repeat_schema_primitive, reserved_bits_schema_primitive, schema_field_reference_type,
    schema_length_expression_references,
};
use crate::schema::reserved_layout::supported_encode_reserved_bits;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::schema_types::{
    StandardRuntimeTypeKind, format_neutral_schema_encode_field_type_for_schema,
    format_neutral_schema_field_type_for_schema, format_neutral_schema_source_adt_descriptor,
    recursive_dispatch_decode_only_payload_case_is_eligible,
    recursive_dispatch_payload_case_is_eligible, resolved_standard_runtime_type_kind,
    schema_decode_function_name, schema_decode_value_type, schema_dispatch_has_recursive_payload,
    schema_dispatch_payload_schema, schema_field_target,
    schema_recursive_dispatch_helper_payload_type, schema_recursive_dispatch_payload_type,
};

pub(crate) fn schema_decode_specs(module: &SurfaceModule) -> Vec<IrSchemaDecodeSpec> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_decode_spec(module, schema))
        .collect()
}

pub(crate) fn schema_refinement_validation_specs(
    module: &SurfaceModule,
) -> Vec<CoreSchemaRefinementValidationSpec> {
    module
        .schemas
        .iter()
        .filter(|schema| schema.format.is_none())
        .filter_map(|schema| {
            let decoded = schema_decode_spec(module, schema)?;
            let plan =
                ir_schema_refinement_record_plan(decoded.fields.iter().filter_map(|field| {
                    ir_schema_refinement_field_plan(field).map(|plan| (field.name.clone(), plan))
                }))?;
            Some(CoreSchemaRefinementValidationSpec {
                schema_name: decoded.schema_name,
                declaration_name: decoded.declaration_name,
                plan: core_schema_refinement_plan(&plan),
            })
        })
        .collect()
}

fn ir_schema_refinement_field_plan(field: &IrSchemaDecodeField) -> Option<IrSchemaRefinementPlan> {
    field.refinement_plan.as_deref().cloned().or_else(|| {
        let child = field.payload_schema.as_deref()?;
        ir_schema_refinement_record_plan(child.fields.iter().filter_map(|field| {
            ir_schema_refinement_field_plan(field).map(|plan| (field.name.clone(), plan))
        }))
    })
}

fn core_schema_refinement_plan(plan: &IrSchemaRefinementPlan) -> CoreSchemaRefinementPlan {
    CoreSchemaRefinementPlan {
        root: plan.root,
        nodes: plan
            .nodes
            .iter()
            .map(|node| match node {
                IrSchemaRefinementPlanNode::Variant(refinement) => {
                    CoreSchemaRefinementPlanNode::Variant(refinement.expected_variants.clone())
                }
                IrSchemaRefinementPlanNode::Adt {
                    expected_variants,
                    variants,
                } => CoreSchemaRefinementPlanNode::Adt {
                    expected_variants: expected_variants.clone(),
                    variants: variants
                        .iter()
                        .map(|variant| CoreSchemaRefinementVariant {
                            name: variant.name.clone(),
                            payloads: variant
                                .payloads
                                .iter()
                                .map(|payload| CoreSchemaRefinementPayload {
                                    index: payload.index,
                                    path_segment: payload.path_segment.clone(),
                                    plan: payload.plan,
                                })
                                .collect(),
                        })
                        .collect(),
                },
                IrSchemaRefinementPlanNode::Record(fields) => CoreSchemaRefinementPlanNode::Record(
                    fields
                        .iter()
                        .map(|field| CoreSchemaRefinementField {
                            name: field.name.clone(),
                            plan: field.plan,
                        })
                        .collect(),
                ),
                IrSchemaRefinementPlanNode::Option(payload) => {
                    CoreSchemaRefinementPlanNode::Option(*payload)
                }
                IrSchemaRefinementPlanNode::Result { ok, err } => {
                    CoreSchemaRefinementPlanNode::Result { ok: *ok, err: *err }
                }
                IrSchemaRefinementPlanNode::List(payload) => {
                    CoreSchemaRefinementPlanNode::List(*payload)
                }
                IrSchemaRefinementPlanNode::Vec(payload) => {
                    CoreSchemaRefinementPlanNode::Vec(*payload)
                }
                IrSchemaRefinementPlanNode::Dict(payload) => {
                    CoreSchemaRefinementPlanNode::Dict(*payload)
                }
            })
            .collect(),
    }
}

fn ir_schema_refinement_record_plan(
    fields: impl IntoIterator<Item = (String, IrSchemaRefinementPlan)>,
) -> Option<IrSchemaRefinementPlan> {
    let mut nodes = Vec::new();
    let mut record_fields = Vec::new();
    for (name, plan) in fields {
        let start = nodes.len();
        let remapped = (start..start + plan.nodes.len()).collect::<Vec<_>>();
        for node in plan.nodes {
            let node = remap_schema_refinement_node(node, &remapped);
            nodes.push(node);
        }
        record_fields.push(IrSchemaRefinementField {
            name,
            plan: remapped[plan.root],
        });
    }
    if record_fields.is_empty() {
        return None;
    }
    let root = nodes.len();
    nodes.push(IrSchemaRefinementPlanNode::Record(record_fields));
    Some(IrSchemaRefinementPlan { root, nodes })
}

fn remap_schema_refinement_node(
    node: IrSchemaRefinementPlanNode,
    remapped: &[usize],
) -> IrSchemaRefinementPlanNode {
    match node {
        IrSchemaRefinementPlanNode::Variant(refinement) => {
            IrSchemaRefinementPlanNode::Variant(refinement)
        }
        IrSchemaRefinementPlanNode::Adt {
            expected_variants,
            variants,
        } => IrSchemaRefinementPlanNode::Adt {
            expected_variants,
            variants: variants
                .into_iter()
                .map(|variant| IrSchemaRefinementVariant {
                    name: variant.name,
                    payloads: variant
                        .payloads
                        .into_iter()
                        .map(|payload| IrSchemaRefinementPayload {
                            index: payload.index,
                            path_segment: payload.path_segment,
                            plan: remapped[payload.plan],
                        })
                        .collect(),
                })
                .collect(),
        },
        IrSchemaRefinementPlanNode::Record(fields) => IrSchemaRefinementPlanNode::Record(
            fields
                .into_iter()
                .map(|field| IrSchemaRefinementField {
                    name: field.name,
                    plan: remapped[field.plan],
                })
                .collect(),
        ),
        IrSchemaRefinementPlanNode::Option(payload) => {
            IrSchemaRefinementPlanNode::Option(remapped[payload])
        }
        IrSchemaRefinementPlanNode::Result { ok, err } => IrSchemaRefinementPlanNode::Result {
            ok: ok.map(|index| remapped[index]),
            err: err.map(|index| remapped[index]),
        },
        IrSchemaRefinementPlanNode::List(payload) => {
            IrSchemaRefinementPlanNode::List(remapped[payload])
        }
        IrSchemaRefinementPlanNode::Vec(payload) => {
            IrSchemaRefinementPlanNode::Vec(remapped[payload])
        }
        IrSchemaRefinementPlanNode::Dict(payload) => {
            IrSchemaRefinementPlanNode::Dict(remapped[payload])
        }
    }
}

fn schema_decode_spec(module: &SurfaceModule, schema: &SchemaDecl) -> Option<IrSchemaDecodeSpec> {
    schema_decode_spec_inner(module, schema, &mut Vec::new())
}

fn schema_decode_spec_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeSpec> {
    let schema_name = crate::schema::schema_link_name(module, schema);
    schema.name.as_ref()?;
    if schema
        .format
        .as_ref()
        .is_some_and(|format| format.name != "binary")
    {
        return None;
    }
    if stack.iter().any(|name| name == &schema_name) {
        return None;
    }
    stack.push(schema_name.clone());
    let spec = if schema.format.is_none() {
        format_neutral_schema_decode_spec(module, schema, stack)
    } else {
        schema_decode_spec_inner_after_push(module, schema, stack)
    };
    stack.pop();
    spec
}

fn format_neutral_schema_decode_spec(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeSpec> {
    let schema_name = schema.name.as_ref()?;
    let adts = AdtRegistry::from_module(module);
    let fields = schema
        .fields
        .iter()
        .map(|field| format_neutral_schema_decode_field(module, schema, field, &adts, stack))
        .collect::<Option<Vec<_>>>()?;
    Some(IrSchemaDecodeSpec {
        schema_name: crate::schema::schema_link_name(module, schema),
        declaration_name: schema_name.clone(),
        function_name: schema_decode_function_name(schema_name),
        fields,
        validation: ir_schema_validation(schema),
    })
}

fn format_neutral_schema_decode_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    adts: &AdtRegistry,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeField> {
    let field_ty =
        format_neutral_schema_encode_field_type_for_schema(module, schema, adts, &field.ty);
    if format_neutral_schema_field_type_for_schema(module, schema, adts, &field.ty).is_none()
        && field_ty.is_none()
    {
        return None;
    }
    let payload_schema =
        format_neutral_schema_payload(module, schema, field, adts, field_ty.as_ref(), stack);
    Some(IrSchemaDecodeField {
        predicate: field
            .where_clause
            .as_ref()
            .map(|where_clause| where_clause.predicate.clone()),
        payload_schema,
        variant_refinement: field_ty
            .as_ref()
            .and_then(|ty| ir_schema_variant_refinement(ty, adts)),
        refinement_plan: field_ty
            .as_ref()
            .and_then(|ty| ir_schema_refinement_plan(module, schema, ty, adts))
            .map(Box::new),
        ..IrSchemaDecodeField::new(field.name.clone())
    })
}

fn format_neutral_schema_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    adts: &AdtRegistry,
    field_ty: Option<&Type>,
    stack: &mut Vec<String>,
) -> Option<Box<IrSchemaDecodeSpec>> {
    (!matches!(field.ty.as_str(), "Int" | "Bool" | "Float" | "String"))
        .then(|| schema_field_target(module, schema, &field.ty))
        .flatten()
        .filter(|target| target.format.is_none())
        .and_then(|target| schema_decode_spec_inner(module, target, stack))
        .map(Box::new)
        .or_else(|| {
            field_ty
                .and_then(|ty| ir_schema_refinement_record_spec(module, schema, ty, adts))
                .map(Box::new)
        })
}

fn ir_schema_refinement_record_spec(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    ty: &Type,
    adts: &AdtRegistry,
) -> Option<IrSchemaDecodeSpec> {
    let Type::Record(fields) = ty else {
        return None;
    };
    let mut has_refinement = false;
    let fields = fields
        .iter()
        .map(|(name, field_ty)| {
            let variant_refinement = ir_schema_variant_refinement(field_ty, adts);
            let payload_schema =
                ir_schema_refinement_record_spec(module, schema, field_ty, adts).map(Box::new);
            has_refinement |= variant_refinement.is_some() || payload_schema.is_some();
            IrSchemaDecodeField {
                variant_refinement,
                payload_schema,
                refinement_plan: ir_schema_refinement_plan(module, schema, field_ty, adts)
                    .map(Box::new),
                ..IrSchemaDecodeField::new(name.clone())
            }
        })
        .collect();
    has_refinement.then_some(IrSchemaDecodeSpec {
        schema_name: String::new(),
        declaration_name: String::new(),
        function_name: String::new(),
        fields,
        validation: None,
    })
}

fn ir_schema_refinement_plan(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    ty: &Type,
    adts: &AdtRegistry,
) -> Option<IrSchemaRefinementPlan> {
    let mut builder = SchemaRefinementPlanBuilder {
        module,
        adts,
        nodes: Vec::new(),
        completed_adts: HashMap::new(),
        adt_stack: Vec::new(),
    };
    let root = builder.build(
        schema.module_name.as_deref(),
        schema.package_name.as_deref(),
        ty,
    )?;
    retain_refinement_reachable_edges(&mut builder.nodes, root)?;
    Some(IrSchemaRefinementPlan {
        root,
        nodes: builder.nodes,
    })
}

fn retain_refinement_reachable_edges(
    nodes: &mut [IrSchemaRefinementPlanNode],
    root: usize,
) -> Option<()> {
    let mut reachable = nodes
        .iter()
        .map(|node| match node {
            IrSchemaRefinementPlanNode::Variant(_) => true,
            IrSchemaRefinementPlanNode::Adt {
                expected_variants, ..
            } => !expected_variants.is_empty(),
            _ => false,
        })
        .collect::<Vec<_>>();
    loop {
        let previous = reachable.clone();
        for (index, node) in nodes.iter().enumerate() {
            reachable[index] |= schema_refinement_child_nodes(node)
                .into_iter()
                .any(|child| previous[child]);
        }
        if reachable == previous {
            break;
        }
    }
    reachable[root].then_some(())?;
    for node in nodes {
        match node {
            IrSchemaRefinementPlanNode::Adt { variants, .. } => {
                for variant in variants {
                    variant.payloads.retain(|payload| reachable[payload.plan]);
                }
            }
            IrSchemaRefinementPlanNode::Record(fields) => {
                fields.retain(|field| reachable[field.plan]);
            }
            _ => {}
        }
    }
    Some(())
}

fn schema_refinement_child_nodes(node: &IrSchemaRefinementPlanNode) -> Vec<usize> {
    match node {
        IrSchemaRefinementPlanNode::Variant(_) => Vec::new(),
        IrSchemaRefinementPlanNode::Adt { variants, .. } => variants
            .iter()
            .flat_map(|variant| variant.payloads.iter().map(|payload| payload.plan))
            .collect(),
        IrSchemaRefinementPlanNode::Record(fields) => {
            fields.iter().map(|field| field.plan).collect()
        }
        IrSchemaRefinementPlanNode::Option(payload)
        | IrSchemaRefinementPlanNode::List(payload)
        | IrSchemaRefinementPlanNode::Vec(payload)
        | IrSchemaRefinementPlanNode::Dict(payload) => vec![*payload],
        IrSchemaRefinementPlanNode::Result { ok, err } => ok.iter().chain(err).copied().collect(),
    }
}

type SchemaRefinementMemoKey = (String, Type);

struct SourceAdtVariantPayloads {
    name: String,
    payloads: Vec<SourceAdtPayload>,
}

struct SourceAdtPayload {
    index: usize,
    path_segment: String,
    ty: Type,
}

struct SchemaRefinementPlanBuilder<'a> {
    module: &'a SurfaceModule,
    adts: &'a AdtRegistry,
    nodes: Vec<IrSchemaRefinementPlanNode>,
    completed_adts: HashMap<SchemaRefinementMemoKey, Option<usize>>,
    adt_stack: Vec<(String, Vec<Type>, usize)>,
}

impl SchemaRefinementPlanBuilder<'_> {
    fn push(&mut self, node: IrSchemaRefinementPlanNode) -> usize {
        let index = self.nodes.len();
        self.nodes.push(node);
        index
    }

    fn build(
        &mut self,
        current_module: Option<&str>,
        current_package: Option<&str>,
        ty: &Type,
    ) -> Option<usize> {
        if let Some(plan) = self.source_adt(current_module, current_package, ty) {
            return Some(plan);
        }
        if let Some(refinement) = ir_schema_variant_refinement(ty, self.adts) {
            return Some(self.push(IrSchemaRefinementPlanNode::Variant(refinement)));
        }
        match ty {
            Type::Record(fields) => {
                let fields = fields
                    .iter()
                    .filter_map(|(name, ty)| {
                        self.build(current_module, current_package, ty).map(|plan| {
                            IrSchemaRefinementField {
                                name: name.clone(),
                                plan,
                            }
                        })
                    })
                    .collect::<Vec<_>>();
                (!fields.is_empty()).then(|| self.push(IrSchemaRefinementPlanNode::Record(fields)))
            }
            Type::Named { args, .. } => self.named(current_module, current_package, ty, args),
            _ => None,
        }
    }

    fn named(
        &mut self,
        current_module: Option<&str>,
        current_package: Option<&str>,
        ty: &Type,
        args: &[Type],
    ) -> Option<usize> {
        let kind = resolved_standard_runtime_type_kind(
            self.module,
            current_module,
            current_package,
            self.adts,
            ty,
        )?;
        match kind {
            StandardRuntimeTypeKind::Option => {
                let payload = self.build(current_module, current_package, &args[0])?;
                Some(self.push(IrSchemaRefinementPlanNode::Option(payload)))
            }
            StandardRuntimeTypeKind::Result => {
                let ok = self.build(current_module, current_package, &args[0]);
                let err = self.build(current_module, current_package, &args[1]);
                (ok.is_some() || err.is_some())
                    .then(|| self.push(IrSchemaRefinementPlanNode::Result { ok, err }))
            }
            StandardRuntimeTypeKind::List => {
                let payload = self.build(current_module, current_package, &args[0])?;
                Some(self.push(IrSchemaRefinementPlanNode::List(payload)))
            }
            StandardRuntimeTypeKind::Vec => {
                let payload = self.build(current_module, current_package, &args[0])?;
                Some(self.push(IrSchemaRefinementPlanNode::Vec(payload)))
            }
            StandardRuntimeTypeKind::Dict
                if resolved_standard_runtime_type_kind(
                    self.module,
                    current_module,
                    current_package,
                    self.adts,
                    &args[0],
                ) == Some(StandardRuntimeTypeKind::String) =>
            {
                let payload = self.build(current_module, current_package, &args[1])?;
                Some(self.push(IrSchemaRefinementPlanNode::Dict(payload)))
            }
            _ => None,
        }
    }

    fn source_adt(
        &mut self,
        current_module: Option<&str>,
        current_package: Option<&str>,
        ty: &Type,
    ) -> Option<usize> {
        let descriptor = eligible_source_adt_descriptor(
            self.module,
            self.adts,
            current_module,
            current_package,
            ty,
        )?;
        let memo_key = (descriptor.identity(), ty.clone());
        let identity = descriptor.identity();
        let args = match ty {
            Type::Named { args, .. } | Type::VariantRefinement { args, .. } => args,
            _ => return None,
        };
        if let Some((_, active_args, node)) = self
            .adt_stack
            .iter()
            .rev()
            .find(|(active_identity, _, _)| active_identity == &identity)
        {
            return (active_args == args).then_some(*node);
        }
        if let Some(completed) = self.completed_adts.get(&memo_key) {
            return *completed;
        }
        let descriptor_module = descriptor.module_name.clone();
        let descriptor_package = descriptor.package_name.clone();
        let variant_payloads = source_adt_variant_payloads(self.module, descriptor, ty);
        let node = self.push(IrSchemaRefinementPlanNode::Adt {
            expected_variants: Vec::new(),
            variants: Vec::new(),
        });
        self.adt_stack.push((identity, args.to_vec(), node));
        let variants = self.build_source_adt_variants(
            descriptor_module.as_deref(),
            descriptor_package.as_deref(),
            variant_payloads,
        );
        self.adt_stack.pop();
        let plan = self.fill_source_adt_plan(node, ty, variants);
        self.completed_adts.insert(memo_key, plan);
        plan
    }

    fn fill_source_adt_plan(
        &mut self,
        node: usize,
        ty: &Type,
        variants: Vec<IrSchemaRefinementVariant>,
    ) -> Option<usize> {
        let expected_variants = match ty {
            Type::VariantRefinement { variants, .. } => variants.to_vec(),
            _ => Vec::new(),
        };
        let has_payload_plan = variants.iter().any(|variant| !variant.payloads.is_empty());
        if has_payload_plan {
            self.nodes[node] = IrSchemaRefinementPlanNode::Adt {
                expected_variants,
                variants,
            };
            Some(node)
        } else if !expected_variants.is_empty() {
            self.nodes[node] = IrSchemaRefinementPlanNode::Variant(IrSchemaVariantRefinement {
                expected_variants,
            });
            Some(node)
        } else {
            None
        }
    }

    fn build_source_adt_variants(
        &mut self,
        current_module: Option<&str>,
        current_package: Option<&str>,
        variants: Vec<SourceAdtVariantPayloads>,
    ) -> Vec<IrSchemaRefinementVariant> {
        variants
            .into_iter()
            .map(|variant| IrSchemaRefinementVariant {
                name: variant.name,
                payloads: variant
                    .payloads
                    .into_iter()
                    .filter_map(|payload| {
                        self.build(current_module, current_package, &payload.ty)
                            .map(|plan| IrSchemaRefinementPayload {
                                index: payload.index,
                                path_segment: payload.path_segment,
                                plan,
                            })
                    })
                    .collect(),
            })
            .collect()
    }
}

fn eligible_source_adt_descriptor<'a>(
    module: &SurfaceModule,
    adts: &'a AdtRegistry,
    current_module: Option<&str>,
    current_package: Option<&str>,
    ty: &Type,
) -> Option<&'a AdtDescriptor> {
    let descriptor = format_neutral_schema_source_adt_descriptor(
        module,
        current_module,
        current_package,
        adts,
        ty,
    )?;
    (resolved_standard_runtime_type_kind(module, current_module, current_package, adts, ty)
        .is_none()
        && crate::types::schema_types::descriptor_has_source_declaration(adts, descriptor))
    .then_some(descriptor)
}

fn source_adt_variant_payloads(
    module: &SurfaceModule,
    descriptor: &AdtDescriptor,
    ty: &Type,
) -> Vec<SourceAdtVariantPayloads> {
    descriptor
        .variants
        .iter()
        .map(|variant| SourceAdtVariantPayloads {
            name: variant.name.clone(),
            payloads: variant
                .payload_fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    Some(SourceAdtPayload {
                        index,
                        path_segment: source_adt_payload_path_segment(
                            module,
                            descriptor,
                            &variant.name,
                            index,
                            &field.name,
                        ),
                        ty: adt::payload_type(
                            ty,
                            AdtConstructor {
                                descriptor,
                                variant,
                            },
                            index,
                        )?,
                    })
                })
                .collect(),
        })
        .collect()
}

fn source_adt_payload_path_segment(
    module: &SurfaceModule,
    descriptor: &AdtDescriptor,
    variant_name: &str,
    index: usize,
    fallback: &str,
) -> String {
    module
        .types
        .iter()
        .find(|declaration| {
            declaration.name.as_deref() == Some(descriptor.type_name.as_str())
                && declaration.module_name == descriptor.module_name
                && declaration.package_name == descriptor.package_name
        })
        .and_then(|declaration| {
            declaration
                .variants
                .iter()
                .find(|variant| variant.name.as_deref() == Some(variant_name))
        })
        .and_then(|variant| variant.fields.get(index))
        .map(|field| {
            if field.positional {
                index.to_string()
            } else {
                field.name.clone()
            }
        })
        .unwrap_or_else(|| fallback.to_string())
}

fn schema_decode_spec_inner_after_push(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeSpec> {
    let schema_name = schema.name.as_ref()?;
    let mut decoded_field_types = BTreeMap::<String, Type>::new();
    let mut fields = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        let decoded = ir_schema_field(module, schema, field, index, &decoded_field_types, stack)?;
        if let Some((ty, ir_field)) = decoded {
            if let Some(ty) = ty {
                decoded_field_types.insert(field.name.clone(), ty);
            }
            fields.push(ir_field);
        }
    }
    Some(IrSchemaDecodeSpec {
        schema_name: crate::schema::schema_link_name(module, schema),
        declaration_name: schema_name.clone(),
        function_name: schema_decode_function_name(schema_name),
        fields,
        validation: ir_schema_validation(schema),
    })
}

fn ir_schema_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    index: usize,
    decoded_field_types: &BTreeMap<String, Type>,
    stack: &mut Vec<String>,
) -> Option<Option<(Option<Type>, IrSchemaDecodeField)>> {
    if let Some(field) = ir_schema_reserved_bits_field(schema, field, index) {
        return Some(Some((None, field?)));
    }
    if let Some(field) = ir_schema_exact_width_field(field) {
        return Some(Some((Some(Type::int()), field?)));
    }
    if let Some(field) = ir_schema_byte_view_field(field, decoded_field_types) {
        return Some(Some((Some(Type::named("ByteView", Vec::new())), field?)));
    }
    if let Some(decoded) = ir_schema_repeat_field(module, schema, field, decoded_field_types, stack)
    {
        return Some(Some(decoded?));
    }
    if let Some(decoded) = ir_schema_nested_schema_field(module, schema, field, stack) {
        return Some(Some(decoded?));
    }
    if let Some(decoded) = ir_schema_anonymous_record_field(field) {
        return Some(Some(decoded?));
    }
    ir_schema_dispatch_field(module, schema, field, decoded_field_types, stack).map(Some)
}

fn ir_schema_reserved_bits_field(
    schema: &SchemaDecl,
    field: &SchemaField,
    index: usize,
) -> Option<Option<IrSchemaDecodeField>> {
    let reserved = reserved_bits_schema_primitive(&field.ty)?;
    let Some((bit_width, expected_value)) =
        supported_encode_reserved_bits(&schema.fields, index, reserved)
    else {
        return Some(None);
    };
    Some(Some(IrSchemaDecodeField {
        reserved_bits: Some(IrSchemaReservedBits {
            bit_width,
            expected_value,
        }),
        ..IrSchemaDecodeField::new(field.name.clone())
    }))
}

fn ir_schema_exact_width_field(field: &SchemaField) -> Option<Option<IrSchemaDecodeField>> {
    let width = exact_width_schema_primitive(&field.ty)?;
    Some(Some(IrSchemaDecodeField {
        width,
        max_value: exact_width_schema_primitive_max_value(&field.ty)?,
        little_endian: exact_width_schema_primitive_little_endian(&field.ty),
        predicate: field
            .where_clause
            .as_ref()
            .map(|where_clause| where_clause.predicate.clone()),
        ..IrSchemaDecodeField::new(field.name.clone())
    }))
}

fn ir_schema_byte_view_field(
    field: &SchemaField,
    decoded_field_types: &BTreeMap<String, Type>,
) -> Option<Option<IrSchemaDecodeField>> {
    let length_expr = byte_view_schema_primitive(&field.ty)?;
    if length_expr.references().into_iter().any(|reference| {
        schema_field_reference_type(decoded_field_types, reference) != Some(&Type::int())
    }) {
        return Some(None);
    }
    Some(Some(IrSchemaDecodeField {
        length_field: Some(length_expr.render()),
        length_multiple: field
            .where_clause
            .as_ref()
            .and_then(|where_clause| byte_view_multiple_constraint(&where_clause.predicate))
            .map(|constraint| constraint.render()),
        ..IrSchemaDecodeField::new(field.name.clone())
    }))
}

fn ir_schema_repeat_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_field_types: &BTreeMap<String, Type>,
    stack: &mut Vec<String>,
) -> Option<Option<(Option<Type>, IrSchemaDecodeField)>> {
    let repeat = repeat_schema_primitive(&field.ty)?;
    if schema_length_expression_references(&repeat.count_field)?
        .into_iter()
        .any(|reference| {
            schema_field_reference_type(decoded_field_types, reference) != Some(&Type::int())
        })
    {
        return Some(None);
    }
    if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload
        && schema_length_expression_references(length_field)?
            .into_iter()
            .any(|reference| {
                schema_field_reference_type(decoded_field_types, reference) != Some(&Type::int())
            })
    {
        return Some(None);
    }
    let (field_ty, ir_repeat) = ir_schema_repeat(module, schema, repeat, stack)?;
    Some(Some((
        field_ty,
        IrSchemaDecodeField {
            repeat: Some(ir_repeat),
            ..IrSchemaDecodeField::new(field.name.clone())
        },
    )))
}

fn ir_schema_nested_schema_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    stack: &mut Vec<String>,
) -> Option<Option<(Option<Type>, IrSchemaDecodeField)>> {
    let nested_schema = schema_field_target(module, schema, &field.ty)?;
    if nested_schema
        .format
        .as_ref()
        .map(|format| format.name.as_str())
        != Some("binary")
    {
        return Some(None);
    }
    let field_ty = schema_decode_value_type(module, nested_schema)?;
    let payload_schema = schema_decode_spec_inner(module, nested_schema, stack)?;
    Some(ir_schema_payload_field(field, field_ty, payload_schema))
}

fn ir_schema_payload_field(
    field: &SchemaField,
    field_ty: Type,
    payload_schema: IrSchemaDecodeSpec,
) -> Option<(Option<Type>, IrSchemaDecodeField)> {
    Some((
        Some(field_ty),
        IrSchemaDecodeField {
            payload_schema: Some(Box::new(payload_schema)),
            ..IrSchemaDecodeField::new(field.name.clone())
        },
    ))
}

fn ir_schema_anonymous_record_field(
    field: &SchemaField,
) -> Option<Option<(Option<Type>, IrSchemaDecodeField)>> {
    let Type::Record(fields) = parse_type_annotation(&field.ty).ok()? else {
        return None;
    };
    let (field_ty, payload_schema) = ir_schema_anonymous_record_spec(fields)?;
    Some(ir_schema_payload_field(field, field_ty, payload_schema))
}

fn ir_schema_anonymous_record_spec(
    fields: Vec<(String, Type)>,
) -> Option<(Type, IrSchemaDecodeSpec)> {
    ir_schema_anonymous_record_spec_inner(fields)
}

fn ir_schema_anonymous_record_spec_inner(
    fields: Vec<(String, Type)>,
) -> Option<(Type, IrSchemaDecodeSpec)> {
    let mut visible_fields = Vec::new();
    let mut ir_fields = Vec::new();
    for (name, ty) in fields {
        match ty {
            Type::Named {
                name: ty_name,
                args,
                ..
            } => {
                if !args.is_empty() {
                    return None;
                }
                let width = exact_width_schema_primitive(&ty_name)?;
                visible_fields.push((name.clone(), Type::int()));
                ir_fields.push(IrSchemaDecodeField {
                    width,
                    max_value: exact_width_schema_primitive_max_value(&ty_name)?,
                    little_endian: exact_width_schema_primitive_little_endian(&ty_name),
                    ..IrSchemaDecodeField::new(name)
                });
            }
            Type::Record(fields) => {
                let (field_ty, payload_schema) = ir_schema_anonymous_record_spec_inner(fields)?;
                visible_fields.push((name.clone(), field_ty));
                ir_fields.push(IrSchemaDecodeField {
                    payload_schema: Some(Box::new(payload_schema)),
                    ..IrSchemaDecodeField::new(name)
                });
            }
            _ => return None,
        }
    }
    Some((
        Type::Record(visible_fields),
        IrSchemaDecodeSpec {
            schema_name: String::new(),
            declaration_name: String::new(),
            function_name: String::new(),
            fields: ir_fields,
            validation: None,
        },
    ))
}

fn ir_schema_variant_refinement(
    ty: &Type,
    adts: &AdtRegistry,
) -> Option<IrSchemaVariantRefinement> {
    let Type::VariantRefinement { variants, .. } = ty else {
        return None;
    };
    let descriptor = adts.descriptor_for_type(ty)?;
    let expected_variants = descriptor
        .variants
        .iter()
        .filter(|variant| variants.iter().any(|selected| selected == &variant.name))
        .map(|variant| variant.name.clone())
        .collect::<Vec<_>>();
    (!expected_variants.is_empty()).then_some(IrSchemaVariantRefinement { expected_variants })
}

fn ir_schema_dispatch_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_field_types: &BTreeMap<String, Type>,
    stack: &mut Vec<String>,
) -> Option<(Option<Type>, IrSchemaDecodeField)> {
    let dispatch = closed_dispatch_schema_primitive(&field.ty)
        .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
    if schema_field_reference_type(decoded_field_types, &dispatch.tag_field) != Some(&Type::int())
        || dispatch.length_field.as_ref().is_some_and(|length_field| {
            schema_field_reference_type(decoded_field_types, length_field) != Some(&Type::int())
        })
    {
        return None;
    }
    let field_ty = schema_dispatch_field_type(module, schema, field, &dispatch)?;
    Some((
        Some(field_ty),
        IrSchemaDecodeField {
            dispatch: Some(IrSchemaDecodeDispatch {
                tag_field: dispatch.tag_field,
                length_field: dispatch.length_field,
                preserves_unknown: dispatch.preserves_unknown,
                cases: dispatch
                    .cases
                    .into_iter()
                    .map(|case| ir_schema_dispatch_case(module, schema, case, stack))
                    .collect::<Option<Vec<_>>>()?,
            }),
            ..IrSchemaDecodeField::new(field.name.clone())
        },
    ))
}

fn ir_schema_validation(schema: &SchemaDecl) -> Option<String> {
    schema
        .validations
        .first()
        .map(|validation| validation.predicate.clone())
}

fn ir_schema_dispatch_case(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: SchemaDispatchCase,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeDispatchCase> {
    let width = match &case.payload {
        SchemaDispatchCasePayload::Primitive { width, .. } => *width,
        SchemaDispatchCasePayload::ReservedBits { bit_width, .. } => *bit_width,
        SchemaDispatchCasePayload::Schema { .. } => 0,
    };
    let little_endian = match &case.payload {
        SchemaDispatchCasePayload::Primitive { little_endian, .. } => *little_endian,
        SchemaDispatchCasePayload::ReservedBits { .. } => false,
        SchemaDispatchCasePayload::Schema { .. } => false,
    };
    let reserved_bits = match &case.payload {
        SchemaDispatchCasePayload::ReservedBits {
            bit_width,
            expected_value,
        } => Some(IrSchemaReservedBits {
            bit_width: *bit_width,
            expected_value: *expected_value,
        }),
        SchemaDispatchCasePayload::Primitive { .. } | SchemaDispatchCasePayload::Schema { .. } => {
            None
        }
    };
    let payload_schema = match case.payload {
        SchemaDispatchCasePayload::Primitive { .. }
        | SchemaDispatchCasePayload::ReservedBits { .. } => None,
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if schema.name.as_deref() == Some(schema_name.as_str()) {
                return Some(IrSchemaDecodeDispatchCase {
                    tag: case.tag,
                    width,
                    little_endian,
                    reserved_bits,
                    payload_schema: None,
                    payload_schema_name: Some(crate::schema::schema_link_name(module, schema)),
                });
            }
            let nested_schema = schema_dispatch_payload_schema(module, schema, &schema_name)?;
            Some(Box::new(schema_decode_spec_inner(
                module,
                nested_schema,
                stack,
            )?))
        }
    };
    Some(IrSchemaDecodeDispatchCase {
        tag: case.tag,
        width,
        little_endian,
        reserved_bits,
        payload_schema,
        payload_schema_name: None,
    })
}

fn ir_schema_repeat(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    repeat: SchemaRepeatSpec,
    stack: &mut Vec<String>,
) -> Option<(Option<Type>, IrSchemaRepeat)> {
    let (
        field_ty,
        width,
        max_value,
        little_endian,
        reserved_bits,
        byte_view_length_field,
        payload_schema,
    ) = match repeat.payload {
        SchemaRepeatPayload::Primitive {
            width,
            max_value,
            little_endian,
        } => (
            Some(Type::named("List", vec![Type::int()])),
            width,
            max_value,
            little_endian,
            None,
            None,
            None,
        ),
        SchemaRepeatPayload::ReservedBits {
            bit_width,
            expected_value,
        } => (
            None,
            bit_width,
            0,
            false,
            Some(IrSchemaReservedBits {
                bit_width,
                expected_value,
            }),
            None,
            None,
        ),
        SchemaRepeatPayload::ByteView { length_field } => (
            Some(Type::named(
                "List",
                vec![Type::named("ByteView", Vec::new())],
            )),
            0,
            0,
            false,
            None,
            Some(length_field),
            None,
        ),
        SchemaRepeatPayload::Schema { schema_name } => {
            let nested_schema = schema_dispatch_payload_schema(module, schema, &schema_name)?;
            let element_ty = schema_decode_value_type(module, nested_schema)?;
            let payload_schema = schema_decode_spec_inner(module, nested_schema, stack)?;
            (
                Some(Type::named("List", vec![element_ty])),
                0,
                0,
                false,
                None,
                None,
                Some(Box::new(payload_schema)),
            )
        }
    };
    Some((
        field_ty,
        IrSchemaRepeat {
            count_field: repeat.count_field,
            width,
            max_value,
            little_endian,
            reserved_bits,
            byte_view_length_field,
            payload_schema,
        },
    ))
}

fn schema_dispatch_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    let mut payload_types = dispatch
        .cases
        .iter()
        .map(|case| match &case.payload {
            SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
            SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
            SchemaDispatchCasePayload::Schema { schema_name } => {
                if recursive_dispatch_payload_case_is_eligible(
                    module,
                    schema,
                    field,
                    dispatch,
                    schema_name,
                ) {
                    schema_recursive_dispatch_payload_type(module, schema)
                } else if recursive_dispatch_decode_only_payload_case_is_eligible(
                    module,
                    schema,
                    dispatch,
                    schema_name,
                ) {
                    schema_recursive_dispatch_helper_payload_type(module, schema, dispatch)
                } else {
                    let payload_schema =
                        schema_dispatch_payload_schema(module, schema, schema_name)?;
                    schema_decode_value_type(module, payload_schema)
                }
            }
        })
        .collect::<Option<Vec<_>>>()?;
    let payload_ty = payload_types.pop()?;
    let recursive_payload = schema_dispatch_has_recursive_payload(module, schema, field, dispatch);
    let payload_ty = if recursive_payload {
        schema_recursive_dispatch_helper_payload_type(module, schema, dispatch)?
    } else if payload_types.iter().any(|ty| ty != &payload_ty) {
        return None;
    } else {
        payload_ty
    };
    if dispatch.preserves_unknown {
        Some(Type::named("SchemaDispatchPayload", vec![payload_ty]))
    } else {
        Some(payload_ty)
    }
}
