mod context_projection;
mod metrics;

#[cfg(test)]
pub(crate) use context_projection::{
    ContextProjectionWork, reset_context_classification_work, take_context_classification_work,
};
pub(crate) use context_projection::{
    LocalContext, expression_admitted_context, expression_has_compiler_covariant_root,
    expression_is_immediately_context_free,
};
pub(crate) use metrics::record_work;
#[cfg(test)]
pub(crate) use metrics::{
    BoundedCandidateWork, DeepJoinWork, reset_bounded_candidate_work, reset_bounded_frames,
    reset_deep_work, reset_fallback_comparisons, reset_function_scan_nodes, reset_work,
    take_bounded_candidate_work, take_bounded_frames, take_deep_work, take_fallback_comparisons,
    take_function_scan_nodes, take_work,
};
use metrics::{
    record_bounded_candidate_iteration, record_bounded_frame,
    record_bounded_result_materialization, record_candidate_order_node, record_fallback_comparison,
    record_function_scan_node, record_materialization, record_node_visit, record_presentation_work,
    record_relation_cache_insert, record_relation_key_node, record_retained_nodes,
    record_setup_type_node, reset_relation_cache_current_key_bytes,
};

struct JoinNode {
    name: String,
    canonical_name: String,
    presentation_conflicted: bool,
    identity: String,
    args: Vec<JoinArg>,
    declaration_order: Option<Arc<VariantDeclarationOrder>>,
    variants: Option<HashSet<usize>>,
    record_field_names: Option<Vec<String>>,
}

type JoinNodeSetup<'a> = (JoinNode, Vec<(usize, &'a Type, bool)>);

enum JoinArg {
    Exact {
        ty: Type,
        presentation: TypePresentationJoin,
        covariant: bool,
    },
    Covariant(usize),
}

struct PreparedContribution<'a> {
    node: usize,
    right: PreparedTypeParts<'a>,
    variant_ranks: Option<Vec<usize>>,
}

enum PreparedTypeParts<'a> {
    Adt(AdtTypeParts<'a>),
    Record(Vec<&'a Type>),
}

struct InitialVariantState {
    declaration_order: Option<Arc<VariantDeclarationOrder>>,
    variants: Option<HashSet<usize>>,
}

impl AggregateTypeJoin {
    pub(crate) fn new(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        Self::new_optimized(adts, initial).or_else(|| {
            matches!(initial, Type::Function { .. }).then(|| Self {
                nodes: Vec::new(),
                root: 0,
                optimized_contributions: None,
                bounded: Some(BoundedJoin::from_contributions(adts, vec![initial.clone()])),
                projection_cache: RefCell::new(context_projection::ProjectionCache::default()),
            })
        })
    }

    fn new_optimized(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        let (root, pending) = join_node(adts, initial, false)?;
        let mut nodes = vec![root];
        let mut pending = pending
            .into_iter()
            .map(|(argument, ty, record_subtree_is_complete)| {
                (0, argument, ty, record_subtree_is_complete)
            })
            .collect::<Vec<_>>();
        while let Some((parent, argument, ty, record_subtree_is_complete)) = pending.pop() {
            let Some((node, children)) = join_node(adts, ty, record_subtree_is_complete) else {
                drop_join_nodes_iteratively(&mut nodes);
                return None;
            };
            let child = nodes.len();
            nodes.push(node);
            nodes[parent].args[argument] = JoinArg::Covariant(child);
            pending.extend(children.into_iter().map(
                |(argument, ty, record_subtree_is_complete)| {
                    (child, argument, ty, record_subtree_is_complete)
                },
            ));
        }
        record_retained_nodes(nodes.len());
        Some(Self {
            nodes,
            root: 0,
            optimized_contributions: optimized_join_retains_contributions(adts, initial)
                .then(|| vec![initial.clone()]),
            bounded: None,
            projection_cache: RefCell::new(context_projection::ProjectionCache::default()),
        })
    }

    pub(crate) fn new_resolved_refinement(adts: &AdtRegistry, initial: &Type) -> Option<Self> {
        if matches!(initial, Type::Function { .. }) {
            return Self::new(adts, initial);
        }
        if matches!(initial, Type::Record(_)) {
            return Self::new(adts, initial);
        }
        let (identity, args) = match initial {
            Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
                (identity, args)
            }
            _ => return None,
        };
        let compiler_covariant = Type::is_compiler_covariant_identity(identity);
        if (!compiler_covariant && args.iter().any(|argument| type_shape(argument).1))
            || (adts.descriptor_for_type(initial).is_none() && !compiler_covariant)
        {
            return None;
        }
        Self::new(adts, initial)
    }

    pub(crate) fn try_join(&mut self, adts: &AdtRegistry, right: &Type) -> bool {
        if right == &Type::Unknown {
            return true;
        }
        if let Some(bounded) = &mut self.bounded {
            if !bounded.shapes_are_admissible(adts, right) {
                return false;
            }
            bounded.admit(adts, right);
            self.projection_cache.get_mut().clear();
            return true;
        }
        let Some(prepared) = self.prepare(adts, right) else {
            let Some(retained) = self.optimized_contributions.as_deref() else {
                return false;
            };
            if !bounded_shapes_are_admissible(adts, retained, right) {
                return false;
            }
            let mut contributions = retained.to_vec();
            contributions.push(right.clone());
            let bounded = BoundedJoin::from_contributions(adts, contributions);
            if bounded.resolved.is_none()
                && !can_retain_unresolved_contributions(adts, &bounded.contributions)
            {
                return false;
            }
            if let Some(mut optimized) = self.optimized_contributions.take() {
                drop_types_iteratively(&mut optimized);
            }
            self.bounded = Some(bounded);
            self.projection_cache.get_mut().clear();
            return true;
        };
        if self.commit(prepared, adts) {
            self.projection_cache.get_mut().clear();
        }
        if let Some(contributions) = &mut self.optimized_contributions {
            contributions.push(right.clone());
        }
        true
    }

    fn constraint_contributions(&self) -> Vec<Type> {
        if let Some(bounded) = &self.bounded {
            return bounded.contributions.clone();
        }
        self.optimized_contributions
            .clone()
            .unwrap_or_else(|| self.resolved_result_type().into_iter().collect())
    }

    #[cfg(test)]
    pub(crate) fn bounded_retained_state(&self) -> usize {
        let Some(bounded) = &self.bounded else {
            return 0;
        };
        let contribution_state = bounded
            .contributions
            .iter()
            .map(type_retained_units)
            .sum::<usize>();
        let resolved_state = bounded.resolved.as_ref().map_or(0, type_retained_units);
        let fallback_state = bounded
            .fallback
            .as_ref()
            .map_or(0, ContributedCandidateState::retained_units);
        let function_candidate_state = bounded.function_candidates.as_ref().map_or(0, |states| {
            states
                .inputs
                .iter()
                .map(ContributedCandidateState::retained_units)
                .sum::<usize>()
                + states.returns.retained_units()
        });
        bounded.contributions.len()
            + contribution_state
            + resolved_state
            + fallback_state
            + function_candidate_state
    }

    fn prepare<'a>(
        &self,
        _adts: &AdtRegistry,
        right: &'a Type,
    ) -> Option<Vec<PreparedContribution<'a>>> {
        let mut pending = vec![(self.root, right)];
        let mut prepared = Vec::new();
        while let Some((node_index, right)) = pending.pop() {
            record_node_visit();
            prepared.push(self.prepare_contribution(node_index, right, &mut pending)?);
        }
        Some(prepared)
    }

    fn prepare_contribution<'a>(
        &self,
        node_index: usize,
        right: &'a Type,
        pending: &mut Vec<(usize, &'a Type)>,
    ) -> Option<PreparedContribution<'a>> {
        if let Some(field_names) = &self.nodes[node_index].record_field_names {
            let Type::Record(fields) = right else {
                return None;
            };
            if fields.len() != field_names.len() {
                return None;
            }
            let mut fields = fields.iter().collect::<Vec<_>>();
            fields.sort_unstable_by(|left, right| left.0.cmp(&right.0));
            if fields
                .iter()
                .zip(field_names)
                .any(|((actual, _), expected)| actual != expected)
            {
                return None;
            }
            let right_fields = fields.into_iter().map(|(_, ty)| ty).collect::<Vec<_>>();
            queue_covariant_contributions(
                &self.nodes[node_index],
                right_fields.iter().copied(),
                pending,
            )?;
            return Some(PreparedContribution {
                node: node_index,
                right: PreparedTypeParts::Record(right_fields),
                variant_ranks: None,
            });
        }
        let right = adt_type_parts(right)?;
        let node = &self.nodes[node_index];
        (same_type_identity(&node.name, &node.identity, right.name, right.identity)
            && node.args.len() == right.args.len())
        .then_some(())?;
        queue_covariant_contributions(node, right.args, pending)?;
        Some(PreparedContribution {
            node: node_index,
            right: PreparedTypeParts::Adt(right),
            variant_ranks: joined_variant_ranks(node, right.variants)?,
        })
    }

    fn commit(&mut self, prepared: Vec<PreparedContribution<'_>>, adts: &AdtRegistry) -> bool {
        let mut changed = false;
        for contribution in prepared {
            let node = &mut self.nodes[contribution.node];
            changed |= commit_prepared_contribution(node, contribution);
        }
        changed | self.expand_covariant_nodes(adts)
    }

    fn expand_covariant_nodes(&mut self, adts: &AdtRegistry) -> bool {
        let original_nodes = self.nodes.len();
        for parent in 0..original_nodes {
            for argument in 0..self.nodes[parent].args.len() {
                let should_expand = matches!(
                    &self.nodes[parent].args[argument],
                    JoinArg::Exact {
                        ty,
                        covariant: true,
                        ..
                    } if join_node(adts, ty, false).is_some()
                );
                if !should_expand {
                    continue;
                }
                let JoinArg::Exact { ty, .. } = &mut self.nodes[parent].args[argument] else {
                    unreachable!();
                };
                let source = std::mem::replace(ty, Type::Unknown);
                let (root, children) = join_node(adts, &source, false)
                    .expect("a concrete covariant source argument has join state");
                let child = self.nodes.len();
                self.nodes.push(root);
                self.nodes[parent].args[argument] = JoinArg::Covariant(child);
                let mut pending = children
                    .into_iter()
                    .map(|(argument, ty, record_subtree_is_complete)| {
                        (child, argument, ty, record_subtree_is_complete)
                    })
                    .collect::<Vec<_>>();
                while let Some((parent, argument, ty, record_subtree_is_complete)) = pending.pop() {
                    let (node, children) = join_node(adts, ty, record_subtree_is_complete)
                        .expect("a nested covariant source argument has join state");
                    let child = self.nodes.len();
                    self.nodes.push(node);
                    self.nodes[parent].args[argument] = JoinArg::Covariant(child);
                    pending.extend(children.into_iter().map(
                        |(argument, ty, record_subtree_is_complete)| {
                            (child, argument, ty, record_subtree_is_complete)
                        },
                    ));
                }
                crate::semantic_model::drop_type_iteratively(source);
            }
        }
        record_retained_nodes(self.nodes.len());
        self.nodes.len() != original_nodes
    }

    pub(crate) fn try_join_resolved(&mut self, adts: &AdtRegistry, right: &Type) -> bool {
        self.try_join(adts, right)
    }

    pub(crate) fn has_compiler_covariant_root(&self) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        self.nodes[self.root].record_field_names.is_none()
            && Type::is_compiler_covariant_identity(&self.nodes[self.root].identity)
    }

    pub(crate) fn preserves_failed_control_flow_contribution(&self, right: &Type) -> bool {
        if self.bounded.is_some() {
            return true;
        }
        (self.has_compiler_covariant_root() && self.has_same_root(right))
            || (self.nodes[self.root].record_field_names.is_some()
                && matches!(right, Type::Record(_)))
    }

    pub(crate) fn has_complete_domain(&self) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        self.nodes[self.root].variants.is_none()
    }

    pub(crate) fn has_incomplete_nested_refinement_domain(&self) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        self.nodes.iter().enumerate().any(|(index, node)| {
            index != self.root
                && node.variants.as_ref().is_some_and(|variants| {
                    variants.len()
                        != node
                            .declaration_order
                            .as_ref()
                            .expect("a refinement join retains declaration order")
                            .len()
                })
        })
    }

    pub(crate) fn has_same_root(&self, right: &Type) -> bool {
        if self.bounded.is_some() {
            return false;
        }
        let Some(right) = adt_type_parts(right) else {
            return false;
        };
        let root = &self.nodes[self.root];
        (Type::is_compiler_covariant_identity(&root.identity)
            || right.args.iter().all(|argument| !type_shape(argument).1))
            && same_type_identity(&root.name, &root.identity, right.name, right.identity)
            && root.args.len() == right.args.len()
    }

    pub(crate) fn inference_type(&self) -> Type {
        self.materialize(false)
    }

    pub(crate) fn has_resolved_result(&self) -> bool {
        self.bounded
            .as_ref()
            .is_none_or(|bounded| bounded.resolved.is_some())
    }

    pub(crate) fn resolved_result_type(&self) -> Option<Type> {
        self.has_resolved_result().then(|| self.result_type())
    }

    pub(crate) fn result_type(&self) -> Type {
        self.materialize(true)
    }

    fn materialize(&self, include_root_refinement: bool) -> Type {
        if let Some(bounded) = &self.bounded {
            return bounded.resolved.clone().unwrap_or(Type::Unknown);
        }
        let mut materialized = (0..self.nodes.len()).map(|_| None).collect::<Vec<_>>();
        for (index, node) in self.nodes.iter().enumerate().rev() {
            record_materialization();
            let args = materialize_args(node, &mut materialized);
            materialized[index] =
                Some(self.materialize_node(index, node, args, include_root_refinement));
        }
        materialized[self.root]
            .take()
            .expect("aggregate root is materialized")
    }

    fn materialize_node(
        &self,
        index: usize,
        node: &JoinNode,
        args: Vec<Type>,
        include_root_refinement: bool,
    ) -> Type {
        if let Some(field_names) = &node.record_field_names {
            return Type::Record(field_names.iter().cloned().zip(args).collect());
        }
        let variants = (index != self.root || include_root_refinement)
            .then_some(node.variants.as_ref())
            .flatten();
        let Some(variants) = variants else {
            return materialize_named_node(node, args);
        };
        if variants.len()
            == node
                .declaration_order
                .as_ref()
                .expect("a refinement join retains declaration order")
                .len()
        {
            return materialize_named_node(node, args);
        }
        materialize_refined_node(node, args, variants)
    }
}
