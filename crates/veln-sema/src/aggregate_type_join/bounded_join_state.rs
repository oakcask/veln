struct BoundedJoin {
    contributions: Vec<Type>,
    resolved: Option<Type>,
    fallback: Option<ContributedCandidateState>,
    function_candidates: Option<FunctionCandidateStates>,
    admission_witness: Option<usize>,
}

struct ContributedCandidateState {
    direction: BoundDirection,
    valid: Vec<bool>,
    selected: Option<usize>,
    resolved: Option<Type>,
    presentation: TypePresentationJoin,
}

struct FunctionCandidateStates {
    inputs: Vec<ContributedCandidateState>,
    has_variadic: bool,
    returns: FunctionReturnCandidateState,
}

struct FunctionReturnCandidateState {
    return_keys: HashSet<String>,
    return_join: Option<Box<AggregateTypeJoin>>,
    return_type: Option<Type>,
    return_presentation: TypePresentationJoin,
}

impl BoundedJoin {
    fn from_contributions(adts: &AdtRegistry, contributions: Vec<Type>) -> Self {
        let admission_witness = admission_witness(adts, &contributions);
        let function_candidates = FunctionCandidateStates::new(adts, &contributions);
        let fallback = uses_outer_contributed_fallback(&contributions)
            .then(|| ContributedCandidateState::new(adts, &contributions, BoundDirection::Join));
        let resolved = if let Some(state) = &function_candidates {
            state.result(&contributions)
        } else if let Some(state) = &fallback {
            state.result(&contributions)
        } else {
            bounded_join(adts, &contributions)
        };
        Self {
            contributions,
            resolved,
            fallback,
            function_candidates,
            admission_witness,
        }
    }

    fn admit(&mut self, adts: &AdtRegistry, right: &Type) {
        self.update_admission_witness(adts, right);
        self.contributions.push(right.clone());
        if let Some(function_candidates) = &mut self.function_candidates {
            function_candidates.admit(adts, &self.contributions);
            let resolved = function_candidates.result(&self.contributions);
            self.replace_resolved(resolved);
            return;
        }
        if let Some(fallback) = &mut self.fallback {
            fallback.admit(adts, &self.contributions);
            let resolved = fallback.result(&self.contributions);
            self.replace_resolved(resolved);
            return;
        }
        self.replace_resolved(bounded_join(adts, &self.contributions));
        if uses_outer_contributed_fallback(&self.contributions) {
            self.fallback = Some(ContributedCandidateState::new(
                adts,
                &self.contributions,
                BoundDirection::Join,
            ));
        }
    }

    fn replace_resolved(&mut self, resolved: Option<Type>) {
        let previous = std::mem::replace(&mut self.resolved, resolved);
        if let Some(previous) = previous {
            crate::semantic_model::drop_type_iteratively(previous);
        }
    }

    fn shapes_are_admissible(&self, adts: &AdtRegistry, right: &Type) -> bool {
        if let Some(witness) = self.admission_witness {
            return structurally_compatible_function_shapes(
                adts,
                &self.contributions[witness],
                right,
            );
        }
        bounded_shapes_are_admissible(adts, &self.contributions, right)
    }

    fn update_admission_witness(&mut self, adts: &AdtRegistry, right: &Type) {
        self.admission_witness = if let Some(witness) = self.admission_witness {
            let witness = &self.contributions[witness];
            if structural_paths_cover(adts, right, witness) {
                Some(self.contributions.len())
            } else if structural_paths_cover(adts, witness, right) {
                self.admission_witness
            } else {
                None
            }
        } else if self
            .contributions
            .iter()
            .all(|contribution| structural_paths_cover(adts, right, contribution))
        {
            Some(self.contributions.len())
        } else {
            None
        };
    }
}

impl Drop for BoundedJoin {
    fn drop(&mut self) {
        drop_types_iteratively(&mut self.contributions);
        if let Some(resolved) = self.resolved.take() {
            crate::semantic_model::drop_type_iteratively(resolved);
        }
    }
}

impl ContributedCandidateState {
    fn new(adts: &AdtRegistry, contributions: &[Type], direction: BoundDirection) -> Self {
        let contributions = contributions.iter().collect::<Vec<_>>();
        Self::new_refs(adts, &contributions, direction)
    }

    fn new_refs(adts: &AdtRegistry, contributions: &[&Type], direction: BoundDirection) -> Self {
        let mut state = Self {
            direction,
            valid: Vec::new(),
            selected: None,
            resolved: None,
            presentation: TypePresentationJoin::default(),
        };
        for end in 1..=contributions.len() {
            state.admit_refs(adts, &contributions[..end]);
        }
        state
    }

    fn admit(&mut self, adts: &AdtRegistry, contributions: &[Type]) {
        let contributions = contributions.iter().collect::<Vec<_>>();
        self.admit_refs(adts, &contributions);
    }

    fn admit_refs(&mut self, adts: &AdtRegistry, contributions: &[&Type]) {
        let new_index = contributions.len() - 1;
        debug_assert_eq!(self.valid.len(), new_index);
        let representative = self.valid.iter().position(|valid| *valid);
        let prior_candidates_remain_valid = representative.is_some_and(|index| {
            candidate_relation(
                contributions[index],
                contributions[new_index],
                self.direction,
                adts,
            )
        });
        if representative.is_some() && !prior_candidates_remain_valid {
            self.valid.fill(false);
        }
        let new_valid = if let Some(index) = representative {
            candidate_relation(
                contributions[new_index],
                contributions[index],
                self.direction,
                adts,
            )
        } else {
            contributions[..new_index].iter().all(|contribution| {
                candidate_relation(contributions[new_index], contribution, self.direction, adts)
            })
        };
        self.valid.push(new_valid);

        let selected = if prior_candidates_remain_valid {
            let previous = self
                .selected
                .expect("a valid candidate has a selected representative");
            if new_valid
                && candidate_order_key(contributions[new_index])
                    < candidate_order_key(contributions[previous])
            {
                Some(new_index)
            } else {
                Some(previous)
            }
        } else {
            new_valid.then_some(new_index)
        };
        self.update_resolved(contributions, selected);
    }

    fn result(&self, contributions: &[Type]) -> Option<Type> {
        let contributions = contributions.iter().collect::<Vec<_>>();
        self.result_refs(&contributions)
    }

    fn result_refs(&self, contributions: &[&Type]) -> Option<Type> {
        debug_assert_eq!(self.valid.len(), contributions.len());
        if let Some(resolved) = &self.resolved {
            record_bounded_result_materialization(resolved);
        }
        self.resolved.clone()
    }

    fn update_resolved(&mut self, contributions: &[&Type], selected: Option<usize>) {
        let previous_selected = self.selected;
        if selected == previous_selected {
            if let Some(resolved) = &mut self.resolved {
                self.presentation.merge(
                    resolved,
                    contributions
                        .last()
                        .expect("candidate admission has a contribution"),
                );
            }
            return;
        }

        let previous = self.resolved.take();
        let Some(selected) = selected else {
            if let Some(previous) = previous {
                crate::semantic_model::drop_type_iteratively(previous);
            }
            self.presentation = TypePresentationJoin::default();
            self.selected = None;
            return;
        };

        let candidate = contributions[selected];
        let mut resolved = candidate.clone();
        let can_reuse = previous
            .as_ref()
            .is_some_and(|previous| presentation_layout_is_reusable(previous, candidate));
        if can_reuse {
            self.presentation.merge(
                &mut resolved,
                previous
                    .as_ref()
                    .expect("reusable presentation has a previous result"),
            );
        } else {
            self.presentation = TypePresentationJoin::default();
            for contribution in contributions {
                self.presentation.merge(&mut resolved, contribution);
            }
        }
        if let Some(previous) = previous {
            crate::semantic_model::drop_type_iteratively(previous);
        }
        self.selected = Some(selected);
        self.resolved = Some(resolved);
    }

    #[cfg(test)]
    fn retained_units(&self) -> usize {
        self.valid.len()
            + self.resolved.as_ref().map_or(0, type_retained_units)
            + self.presentation.retained_nodes()
    }
}

impl Drop for ContributedCandidateState {
    fn drop(&mut self) {
        if let Some(resolved) = self.resolved.take() {
            crate::semantic_model::drop_type_iteratively(resolved);
        }
    }
}

impl FunctionCandidateStates {
    fn new(adts: &AdtRegistry, contributions: &[Type]) -> Option<Self> {
        let functions = root_function_parts(contributions)?;
        let first = functions.first()?;
        let mut inputs = (0..first.params.len())
            .map(|index| {
                let candidates = functions
                    .iter()
                    .map(|function| &function.params[index])
                    .collect::<Vec<_>>();
                ContributedCandidateState::new_refs(adts, &candidates, BoundDirection::Meet)
            })
            .collect::<Vec<_>>();
        if first.variadic.is_some() {
            let candidates = functions
                .iter()
                .map(|function| function.variadic.expect("variadic shape was checked"))
                .collect::<Vec<_>>();
            inputs.push(ContributedCandidateState::new_refs(
                adts,
                &candidates,
                BoundDirection::Meet,
            ));
        }
        Some(Self {
            inputs,
            has_variadic: first.variadic.is_some(),
            returns: FunctionReturnCandidateState::new(adts, &functions),
        })
    }

    fn admit(&mut self, adts: &AdtRegistry, contributions: &[Type]) {
        let functions = root_function_parts(contributions)
            .expect("admitted bounded function contributions keep their shape");
        self.admit_inputs(adts, &functions);
        self.returns.admit(adts, &functions);
    }

    fn admit_inputs(&mut self, adts: &AdtRegistry, functions: &[FunctionBoundParts<'_>]) {
        let parameter_count = functions[0].params.len();
        for (index, state) in self.inputs.iter_mut().enumerate() {
            let candidates = if index < parameter_count {
                functions
                    .iter()
                    .map(|function| &function.params[index])
                    .collect::<Vec<_>>()
            } else {
                functions
                    .iter()
                    .map(|function| function.variadic.expect("variadic shape was checked"))
                    .collect::<Vec<_>>()
            };
            state.admit_refs(adts, &candidates);
        }
    }

    fn result(&self, contributions: &[Type]) -> Option<Type> {
        record_bounded_frame();
        let functions = root_function_parts(contributions)?;
        let contribution_refs = contributions.iter().collect::<Vec<_>>();
        let effects = collect_function_bound_parts(&contribution_refs)?.effects;
        let parameter_count = functions.first()?.params.len();
        let mut params = self.result_params(&functions, parameter_count)?;
        let variadic = if self.has_variadic {
            let candidates = functions
                .iter()
                .map(|function| function.variadic.expect("variadic shape was checked"))
                .collect::<Vec<_>>();
            let Some(variadic) = self.inputs[parameter_count].result_refs(&candidates) else {
                drop_types_iteratively(&mut params);
                return None;
            };
            Some(Box::new(variadic))
        } else {
            None
        };
        let Some(return_type) = self.returns.return_type.clone() else {
            drop_types_iteratively(&mut params);
            if let Some(variadic) = variadic {
                crate::semantic_model::drop_type_iteratively(*variadic);
            }
            return None;
        };
        Some(Type::Function {
            params,
            variadic,
            return_type: Box::new(return_type),
            effects,
        })
    }

    fn result_params(
        &self,
        functions: &[FunctionBoundParts<'_>],
        parameter_count: usize,
    ) -> Option<Vec<Type>> {
        let mut params = Vec::with_capacity(parameter_count);
        for (index, state) in self.inputs.iter().take(parameter_count).enumerate() {
            let candidates = functions
                .iter()
                .map(|function| &function.params[index])
                .collect::<Vec<_>>();
            let Some(param) = state.result_refs(&candidates) else {
                drop_types_iteratively(&mut params);
                return None;
            };
            params.push(param);
        }
        Some(params)
    }
}

impl FunctionReturnCandidateState {
    fn new(adts: &AdtRegistry, functions: &[FunctionBoundParts<'_>]) -> Self {
        let returns = function_return_types(functions);
        let return_keys = returns
            .iter()
            .map(|return_type| canonical_relation_key(return_type))
            .collect();
        let return_join = incremental_function_return_join(adts, &returns);
        let (return_type, return_presentation) = return_join.as_ref().map_or_else(
            || resolve_function_returns(adts, &returns),
            |joined| {
                let mut resolved = joined.resolved_result_type();
                let mut presentation = TypePresentationJoin::default();
                if let Some(resolved) = &mut resolved {
                    for return_type in &returns {
                        presentation.merge(resolved, return_type);
                    }
                }
                (resolved, presentation)
            },
        );
        Self {
            return_keys,
            return_join,
            return_type,
            return_presentation,
        }
    }

    fn admit(&mut self, adts: &AdtRegistry, functions: &[FunctionBoundParts<'_>]) {
        let return_type = functions
            .last()
            .expect("admitted bounded function contributions are nonempty")
            .return_type;
        if !self.return_keys.insert(canonical_relation_key(return_type)) {
            if let Some(resolved) = &mut self.return_type {
                self.return_presentation.merge(resolved, return_type);
            }
            return;
        }

        let returns = function_return_types(functions);
        let incrementally_admitted = self
            .return_join
            .as_mut()
            .is_some_and(|joined| joined.try_join(adts, return_type));
        if !incrementally_admitted {
            self.return_join = None;
            let (resolved, presentation) = resolve_function_returns(adts, &returns);
            self.replace_return(resolved, presentation);
            return;
        }

        let mut resolved = self
            .return_join
            .as_ref()
            .and_then(|joined| joined.resolved_result_type());
        if self.return_type.is_some() {
            if let Some(resolved) = &mut resolved {
                self.return_presentation.merge(resolved, return_type);
            } else {
                self.return_presentation = TypePresentationJoin::default();
            }
            self.replace_return_type(resolved);
            return;
        }

        let mut presentation = TypePresentationJoin::default();
        if let Some(resolved) = &mut resolved {
            for return_type in &returns {
                presentation.merge(resolved, return_type);
            }
        }
        self.replace_return(resolved, presentation);
    }

    fn replace_return(&mut self, resolved: Option<Type>, presentation: TypePresentationJoin) {
        self.return_presentation = presentation;
        self.replace_return_type(resolved);
    }

    fn replace_return_type(&mut self, resolved: Option<Type>) {
        let previous = std::mem::replace(&mut self.return_type, resolved);
        if let Some(previous) = previous {
            crate::semantic_model::drop_type_iteratively(previous);
        }
    }

    #[cfg(test)]
    fn retained_units(&self) -> usize {
        self.return_keys.iter().map(String::len).sum::<usize>()
            + self
                .return_join
                .as_ref()
                .map_or(0, |joined| joined.bounded_retained_state())
            + self.return_type.as_ref().map_or(0, type_retained_units)
            + self.return_presentation.retained_nodes()
    }
}

fn function_return_types<'a>(functions: &[FunctionBoundParts<'a>]) -> Vec<&'a Type> {
    functions
        .iter()
        .map(|function| function.return_type)
        .collect()
}

fn incremental_function_return_join(
    adts: &AdtRegistry,
    returns: &[&Type],
) -> Option<Box<AggregateTypeJoin>> {
    // A nested aggregate at every direct function return would retain every
    // remaining suffix. The bounded worklist resolves that spine iteratively.
    if matches!(returns.first(), Some(Type::Function { .. })) {
        return None;
    }
    let mut joined = Box::new(AggregateTypeJoin::new(adts, returns.first()?)?);
    for return_type in &returns[1..] {
        if !joined.try_join(adts, return_type) {
            return None;
        }
    }
    Some(joined)
}

impl Drop for FunctionReturnCandidateState {
    fn drop(&mut self) {
        if let Some(return_type) = self.return_type.take() {
            crate::semantic_model::drop_type_iteratively(return_type);
        }
    }
}

fn resolve_function_returns(
    adts: &AdtRegistry,
    returns: &[&Type],
) -> (Option<Type>, TypePresentationJoin) {
    let mut resolved = bounded_join_refs(adts, returns);
    let mut presentation = TypePresentationJoin::default();
    if let Some(resolved) = &mut resolved {
        for return_type in returns {
            presentation.merge(resolved, return_type);
        }
    }
    (resolved, presentation)
}

fn root_function_parts(contributions: &[Type]) -> Option<Vec<FunctionBoundParts<'_>>> {
    let Type::Function {
        params: first_params,
        variadic: first_variadic,
        ..
    } = contributions.first()?
    else {
        return None;
    };
    contributions
        .iter()
        .map(|contribution| {
            let Type::Function {
                params,
                variadic,
                return_type,
                ..
            } = contribution
            else {
                return None;
            };
            (params.len() == first_params.len() && variadic.is_some() == first_variadic.is_some())
                .then_some(FunctionBoundParts {
                    params,
                    variadic: variadic.as_deref(),
                    return_type,
                })
        })
        .collect()
}
