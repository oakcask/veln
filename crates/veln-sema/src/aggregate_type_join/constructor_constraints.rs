impl ConstructorTypeArgConstraints {
    pub(crate) fn new(constructor: AdtConstructor<'_>) -> Self {
        let parameter_count = constructor.descriptor.type_parameters.len();
        record_work(parameter_count);
        Self {
            parameters: (0..parameter_count)
                .map(|_| TypeParameterConstraints::default())
                .collect(),
        }
    }

    pub(crate) fn payload_expected(
        &self,
        expected: Option<&Type>,
        constructor: AdtConstructor<'_>,
        payload_index: usize,
    ) -> Type {
        expected
            .and_then(|expected| {
                let expected_args = unification::adt_args(expected, constructor.descriptor)?;
                adt::payload_type_with_resolved_args(constructor, payload_index, |type_index| {
                    expected_args[type_index].clone()
                })
            })
            .or_else(|| {
                adt::payload_type_with_resolved_args(constructor, payload_index, |type_index| {
                    self.result(type_index, None)
                })
            })
            .unwrap_or(Type::Unknown)
    }

    pub(crate) fn payload_can_infer_without_context(
        &self,
        constructor: AdtConstructor<'_>,
        payload_index: usize,
        adts: &AdtRegistry,
    ) -> bool {
        let Some(field) = constructor.variant.payload_fields.get(payload_index) else {
            return false;
        };
        match &field.ty {
            crate::adt::descriptors::AdtPayloadType::TypeParameter(_) => true,
            crate::adt::descriptors::AdtPayloadType::SelfType => checked_parameter_variances(
                &constructor.descriptor.identity(),
                constructor.descriptor.type_parameters.len(),
                adts,
            )
            .iter()
            .any(|variance| *variance != Variance::Invariant),
            crate::adt::descriptors::AdtPayloadType::Concrete(template) => {
                template_has_non_invariant_parameter(template, Variance::Covariant, adts)
            }
        }
    }

    pub(crate) fn contribute_payload(
        &mut self,
        constructor: AdtConstructor<'_>,
        payload_index: usize,
        actual: &Type,
        expected: Option<&Type>,
        adts: &AdtRegistry,
    ) -> Result<(), unification::TypeParameterContributionConflict> {
        if let Some(conflict) = self.payload_shape_conflict(
            constructor,
            payload_index,
            actual,
            adts,
        ) {
            return Err(conflict);
        }
        let mut contributions = Vec::new();
        collect_constructor_payload_constraints(
            constructor,
            payload_index,
            actual,
            adts,
            &mut contributions,
        );
        if self.can_admit_fast_lower(expected, &contributions) {
            for contribution in contributions {
                self.parameters[contribution.parameter].admit_fast_lower(contribution.ty, adts);
            }
            return Ok(());
        }
        let mut trial = self.trial_constraints(contributions, adts)?;
        let expected_args =
            expected.and_then(|expected| unification::adt_args(expected, constructor.descriptor));
        for (index, parameter) in &mut trial {
            let fallback = expected_args.and_then(|args| args.get(*index));
            parameter.resolved = solve_type_parameter_constraints(parameter, fallback, adts)?;
        }
        for (index, parameter) in trial {
            self.parameters[index] = parameter;
        }
        Ok(())
    }

    fn payload_shape_conflict(
        &self,
        constructor: AdtConstructor<'_>,
        payload_index: usize,
        actual: &Type,
        adts: &AdtRegistry,
    ) -> Option<unification::TypeParameterContributionConflict> {
        let field = constructor.variant.payload_fields.get(payload_index);
        if field.is_some_and(|field| {
            matches!(
                field.ty,
                crate::adt::descriptors::AdtPayloadType::TypeParameter(_)
            )
        }) {
            return None;
        }
        let expected_shape =
            adt::payload_type_with_resolved_args(constructor, payload_index, |_| Type::Unknown)
                .unwrap_or(Type::Unknown);
        (!is_assignable_nested_with_registry(&expected_shape, actual, adts)).then(|| {
            let expected = self.payload_expected(None, constructor, payload_index);
            type_parameter_constraint_conflict(&expected, actual)
        })
    }

    fn can_admit_fast_lower(
        &self,
        expected: Option<&Type>,
        contributions: &[ConstructorConstraintContribution],
    ) -> bool {
        expected.is_none()
            && contributions
                .iter()
                .all(|contribution| contribution.polarity == Variance::Covariant)
            && contributions.iter().all(|contribution| {
                self.parameters
                    .get(contribution.parameter)
                    .is_some_and(TypeParameterConstraints::can_admit_fast_lower)
            })
    }

    fn trial_constraints(
        &self,
        contributions: Vec<ConstructorConstraintContribution>,
        adts: &AdtRegistry,
    ) -> Result<HashMap<usize, TypeParameterConstraints>, unification::TypeParameterContributionConflict>
    {
        let mut trial = HashMap::<usize, TypeParameterConstraints>::new();
        for contribution in contributions {
            let Some(current) = self.parameters.get(contribution.parameter) else {
                continue;
            };
            let parameter = trial
                .entry(contribution.parameter)
                .or_insert_with(|| current.clone_for_trial());
            match contribution.polarity {
                Variance::Covariant => {
                    parameter
                        .lower
                        .admit(contribution.ty, BoundDirection::Join, adts)
                }
                Variance::Contravariant => {
                    parameter
                        .upper
                        .admit(contribution.ty, BoundDirection::Meet, adts)
                }
                Variance::Invariant => parameter.admit_equality(contribution.ty)?,
                Variance::Irrelevant => {}
            }
        }
        Ok(trial)
    }

    pub(crate) fn finish(
        &self,
        expected: Option<&Type>,
        constructor: AdtConstructor<'_>,
    ) -> Vec<Type> {
        let expected_args =
            expected.and_then(|expected| unification::adt_args(expected, constructor.descriptor));
        (0..self.parameters.len())
            .map(|index| self.result(index, expected_args))
            .collect()
    }

    fn result(&self, index: usize, expected_args: Option<&[Type]>) -> Type {
        self.parameters
            .get(index)
            .and_then(TypeParameterConstraints::inferred_result)
            .or_else(|| expected_args.and_then(|args| args.get(index).cloned()))
            .unwrap_or(Type::Unknown)
    }
}

impl TypeParameterConstraints {
    fn clone_for_trial(&self) -> Self {
        let lower = self.lower_join.as_ref().map_or_else(
            || self.lower.clone_for_trial(),
            CanonicalConstraintBound::from_join,
        );
        Self {
            lower,
            upper: self.upper.clone_for_trial(),
            equality: self.equality.clone(),
            resolved: self.resolved.clone(),
            lower_join: None,
        }
    }

    fn can_admit_fast_lower(&self) -> bool {
        !self.upper.has_constraints()
            && self.equality.is_none()
            && (self.lower_join.is_some() || !self.lower.has_constraints())
    }

    fn admit_fast_lower(&mut self, contribution: Type, adts: &AdtRegistry) {
        if self.lower_join.is_none() && !self.lower.has_constraints() {
            self.lower_join = AggregateTypeJoin::new(adts, &contribution);
            if self.lower_join.is_none() {
                self.resolved = Some(contribution.clone());
                self.lower.selected = Some(CanonicalConstraint::new(contribution));
            } else {
                self.resolved = None;
            }
            return;
        }
        let Some(joined) = self.lower_join.as_mut() else {
            self.lower.admit(contribution, BoundDirection::Join, adts);
            self.resolved = self.lower.candidate().cloned();
            return;
        };
        if joined.try_join(adts, &contribution) {
            self.resolved = None;
            return;
        }
        let mut contributions = joined.constraint_contributions();
        contributions.push(contribution);
        self.lower = CanonicalConstraintBound::from_lower_contributions(contributions, adts);
        self.lower_join = None;
        self.resolved = self.lower.candidate().cloned();
    }

    fn admit_equality(
        &mut self,
        contribution: Type,
    ) -> Result<(), unification::TypeParameterContributionConflict> {
        let Some(current) = self.equality.as_mut() else {
            self.equality = Some(CanonicalConstraint::new(contribution));
            return Ok(());
        };
        record_work(1);
        if !invariant_args_match(
            std::slice::from_ref(&current.ty),
            std::slice::from_ref(&contribution),
        ) {
            return Err(type_parameter_constraint_conflict(
                &current.ty,
                &contribution,
            ));
        }
        record_work(1);
        current.merge(CanonicalConstraint::new(contribution));
        Ok(())
    }

    fn inferred_result(&self) -> Option<Type> {
        self.lower_join
            .as_ref()
            .and_then(AggregateTypeJoin::resolved_result_type)
            .or_else(|| self.resolved.clone())
    }
}

impl CanonicalConstraint {
    fn new(ty: Type) -> Self {
        Self {
            ty,
            presentation: TypePresentationJoin::default(),
        }
    }

    fn merge(&mut self, other: Self) {
        self.presentation.merge(&mut self.ty, &other.ty);
        self.presentation.absorb(&mut self.ty, &other.presentation);
    }
}

impl CanonicalConstraintBound {
    fn from_join(joined: &AggregateTypeJoin) -> Self {
        if let Some(selected) = joined.resolved_result_type() {
            let mut selected = CanonicalConstraint::new(selected);
            for contribution in joined.constraint_contributions() {
                if invariant_args_match(
                    std::slice::from_ref(&selected.ty),
                    std::slice::from_ref(&contribution),
                ) {
                    selected.merge(CanonicalConstraint::new(contribution));
                }
            }
            return Self {
                selected: Some(selected),
                unresolved: None,
                retry_witness: None,
            };
        }
        let mut bound = Self::default();
        for contribution in joined.constraint_contributions() {
            bound.push_unresolved(CanonicalConstraint::new(contribution));
        }
        bound
    }

    fn from_lower_contributions(contributions: Vec<Type>, adts: &AdtRegistry) -> Self {
        let mut bound = Self::default();
        for contribution in contributions {
            bound.admit(contribution, BoundDirection::Join, adts);
        }
        bound
    }
    fn clone_for_trial(&self) -> Self {
        self.clone()
    }

    fn has_constraints(&self) -> bool {
        self.selected.is_some() || self.unresolved.is_some()
    }

    fn refs(&self) -> Vec<&Type> {
        self.constraints()
            .into_iter()
            .map(|constraint| &constraint.ty)
            .collect()
    }

    fn candidate(&self) -> Option<&Type> {
        self.selected.as_ref().map(|selected| &selected.ty)
    }

    fn admit(&mut self, contribution: Type, direction: BoundDirection, adts: &AdtRegistry) {
        if !self.has_constraints() {
            self.selected = Some(CanonicalConstraint::new(contribution));
            return;
        }
        let contribution = CanonicalConstraint::new(contribution);
        if self.merge_equivalent(contribution.clone()) {
            return;
        }
        let Ok(contribution) = self.prepare_unresolved_retry(contribution, direction, adts) else {
            return;
        };
        self.resolve_with(contribution, direction, adts);
    }

    fn prepare_unresolved_retry(
        &mut self,
        contribution: CanonicalConstraint,
        direction: BoundDirection,
        adts: &AdtRegistry,
    ) -> Result<CanonicalConstraint, ()> {
        let Some(unresolved) = &self.unresolved else {
            return Ok(contribution);
        };
        let Some(pair_bound) =
            constraint_pair_bound(&contribution.ty, &unresolved.constraint.ty, direction, adts)
        else {
            self.push_unresolved(contribution);
            return Err(());
        };
        if self.retry_witness.as_ref().is_some_and(|witness| {
            invariant_args_match(
                std::slice::from_ref(witness),
                std::slice::from_ref(&pair_bound),
            )
        }) {
            self.push_unresolved(contribution);
            return Err(());
        }
        self.retry_witness = Some(pair_bound);
        Ok(contribution)
    }

    fn resolve_with(
        &mut self,
        contribution: CanonicalConstraint,
        direction: BoundDirection,
        adts: &AdtRegistry,
    ) {
        let mut constraints = self.constraints();
        constraints.push(&contribution);
        let candidates = constraints
            .iter()
            .map(|constraint| &constraint.ty)
            .collect::<Vec<_>>();
        record_work(candidates.len());
        let resolved = match direction {
            BoundDirection::Join => bounded_join_refs(adts, &candidates),
            BoundDirection::Meet => bounded_meet_refs(adts, &candidates),
        };
        if let Some(resolved) = resolved {
            let mut selected = CanonicalConstraint::new(resolved);
            for constraint in constraints {
                if invariant_args_match(
                    std::slice::from_ref(&selected.ty),
                    std::slice::from_ref(&constraint.ty),
                ) {
                    selected.merge(constraint.clone());
                }
            }
            self.selected = Some(selected);
            self.unresolved = None;
            self.retry_witness = None;
        } else {
            if let Some(selected) = self.selected.take() {
                self.push_unresolved(selected);
            }
            self.push_unresolved(contribution);
        }
    }

    fn merge_equivalent(&mut self, contribution: CanonicalConstraint) -> bool {
        record_work(1);
        if let Some(selected) = self.selected.as_mut() {
            if invariant_args_match(
                std::slice::from_ref(&selected.ty),
                std::slice::from_ref(&contribution.ty),
            ) {
                selected.merge(contribution);
                return true;
            }
            return false;
        }
        let Some(head) = self.unresolved.as_ref() else {
            return false;
        };
        if !invariant_args_match(
            std::slice::from_ref(&head.constraint.ty),
            std::slice::from_ref(&contribution.ty),
        ) {
            return false;
        }
        let mut constraint = head.constraint.clone();
        constraint.merge(contribution);
        self.unresolved = Some(Arc::new(ConstraintListNode {
            constraint,
            previous: head.previous.clone(),
        }));
        true
    }

    fn push_unresolved(&mut self, constraint: CanonicalConstraint) {
        self.unresolved = Some(Arc::new(ConstraintListNode {
            constraint,
            previous: self.unresolved.clone(),
        }));
    }

    fn constraints(&self) -> Vec<&CanonicalConstraint> {
        if let Some(selected) = &self.selected {
            return vec![selected];
        }
        let mut constraints = Vec::new();
        let mut current = self.unresolved.as_deref();
        while let Some(node) = current {
            constraints.push(&node.constraint);
            current = node.previous.as_deref();
        }
        constraints
    }
}
