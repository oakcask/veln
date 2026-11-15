pub(crate) struct AggregateTypeJoin {
    nodes: Vec<JoinNode>,
    root: usize,
    optimized_contributions: Option<Vec<Type>>,
    bounded: Option<BoundedJoin>,
    projection_cache: RefCell<context_projection::ProjectionCache>,
}

pub(crate) struct ConstructorTypeArgConstraints {
    parameters: Vec<TypeParameterConstraints>,
}

#[derive(Default)]
struct TypeParameterConstraints {
    lower: CanonicalConstraintBound,
    upper: CanonicalConstraintBound,
    equality: Option<CanonicalConstraint>,
    resolved: Option<Type>,
    lower_join: Option<AggregateTypeJoin>,
}

#[derive(Clone)]
struct CanonicalConstraint {
    ty: Type,
    presentation: TypePresentationJoin,
}

struct ConstraintListNode {
    constraint: CanonicalConstraint,
    previous: Option<Arc<ConstraintListNode>>,
}

impl Drop for ConstraintListNode {
    fn drop(&mut self) {
        let mut previous = self.previous.take();
        while let Some(node) = previous {
            let Ok(mut node) = Arc::try_unwrap(node) else {
                break;
            };
            previous = node.previous.take();
        }
    }
}

#[cfg(test)]
pub(crate) fn drop_unresolved_constraint_chain(count: usize) {
    let mut bound = CanonicalConstraintBound::default();
    for _ in 0..count {
        bound.push_unresolved(CanonicalConstraint::new(Type::Unknown));
    }
    drop(bound);
}

#[derive(Clone, Default)]
struct CanonicalConstraintBound {
    selected: Option<CanonicalConstraint>,
    unresolved: Option<Arc<ConstraintListNode>>,
    retry_witness: Option<Type>,
}

#[derive(Clone)]
struct ConstructorConstraintContribution {
    parameter: usize,
    polarity: Variance,
    ty: Type,
}
