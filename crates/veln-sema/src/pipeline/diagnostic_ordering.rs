use std::collections::HashMap;

use veln_diagnostics::Diagnostic;
use veln_source::SourcePath;

use crate::analysis::VariantRefinementBaseDiagnostic;

use super::record_variant_refinement_base_work;

pub(super) fn order_variant_refinement_visibility_diagnostics(diagnostics: &mut [Diagnostic]) {
    let slots = diagnostics
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| {
            (diagnostic.id == "type.variant_refinement_private").then_some(index)
        })
        .collect::<Vec<_>>();
    if slots.len() < 2 {
        return;
    }

    let (mut ordered_indices, mut indices_by_file, file_order) =
        group_visibility_diagnostic_indices(&slots, diagnostics);
    append_sorted_visibility_diagnostic_indices(
        &mut ordered_indices,
        &mut indices_by_file,
        file_order,
        diagnostics,
    );
    replace_visibility_diagnostics(&slots, ordered_indices, diagnostics);
}

fn group_visibility_diagnostic_indices(
    slots: &[usize],
    diagnostics: &[Diagnostic],
) -> (Vec<usize>, HashMap<SourcePath, Vec<usize>>, Vec<SourcePath>) {
    let mut ordered_indices = Vec::with_capacity(slots.len());
    let mut indices_by_file = HashMap::<SourcePath, Vec<usize>>::new();
    let mut file_order = Vec::new();
    for &index in slots {
        let Some(span) = diagnostics[index].span.as_ref() else {
            ordered_indices.push(index);
            continue;
        };
        match indices_by_file.entry(span.file.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().push(index);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                file_order.push(span.file.clone());
                entry.insert(vec![index]);
            }
        }
    }
    (ordered_indices, indices_by_file, file_order)
}

fn append_sorted_visibility_diagnostic_indices(
    ordered_indices: &mut Vec<usize>,
    indices_by_file: &mut HashMap<SourcePath, Vec<usize>>,
    file_order: Vec<SourcePath>,
    diagnostics: &[Diagnostic],
) {
    for file in file_order {
        let indices = indices_by_file
            .get_mut(&file)
            .expect("visibility diagnostic file was indexed");
        radix_sort_diagnostic_indices(indices, diagnostics);
        ordered_indices.extend(indices.iter().copied());
    }
}

fn replace_visibility_diagnostics(
    slots: &[usize],
    ordered_indices: Vec<usize>,
    diagnostics: &mut [Diagnostic],
) {
    let ordered = ordered_indices
        .into_iter()
        .map(|index| diagnostics[index].clone())
        .collect::<Vec<_>>();
    for (&slot, diagnostic) in slots.iter().zip(ordered) {
        diagnostics[slot] = diagnostic;
    }
}

fn radix_sort_diagnostic_indices(indices: &mut Vec<usize>, diagnostics: &[Diagnostic]) {
    let mut scratch = vec![0; indices.len()];
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            let mut counts = [0usize; 256];
            for &index in indices.iter() {
                let span = diagnostics[index]
                    .span
                    .as_ref()
                    .expect("indexed diagnostic has a source span");
                let key = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                counts[(key >> shift) as u8 as usize] += 1;
            }
            let mut next = 0;
            for count in &mut counts {
                let start = next;
                next += *count;
                *count = start;
            }
            for &index in indices.iter() {
                let span = diagnostics[index]
                    .span
                    .as_ref()
                    .expect("indexed diagnostic has a source span");
                let key = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                let bucket = (key >> shift) as u8 as usize;
                scratch[counts[bucket]] = index;
                counts[bucket] += 1;
            }
            std::mem::swap(indices, &mut scratch);
        }
    }
}

pub(super) fn merge_variant_refinement_base_diagnostics(
    diagnostics: &mut Vec<Diagnostic>,
    base_diagnostics: Vec<VariantRefinementBaseDiagnostic>,
) {
    let (mut bases_by_file, ordered_indices) = index_base_diagnostics(&base_diagnostics);
    let mut bases = base_diagnostics.into_iter().map(Some).collect::<Vec<_>>();
    merge_existing_base_diagnostics(diagnostics, &mut bases_by_file, &mut bases);
    for index in ordered_indices {
        record_variant_refinement_base_work(1);
        if let Some(base) = bases[index].take() {
            diagnostics.push(base.diagnostic);
        }
    }
}

type BaseDiagnosticsByFile = HashMap<SourcePath, (Vec<usize>, usize)>;
type DiagnosticSpansByFile = HashMap<SourcePath, HashMap<(usize, usize), usize>>;

fn index_base_diagnostics(
    base_diagnostics: &[VariantRefinementBaseDiagnostic],
) -> (BaseDiagnosticsByFile, Vec<usize>) {
    let mut bases_by_file = BaseDiagnosticsByFile::new();
    let mut file_order = Vec::new();
    let mut ordered_indices = Vec::with_capacity(base_diagnostics.len());
    for (index, diagnostic) in base_diagnostics.iter().enumerate() {
        record_variant_refinement_base_work(1);
        if let Some(span) = diagnostic.diagnostic.span.as_ref() {
            match bases_by_file.entry(span.file.clone()) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().0.push(index);
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    file_order.push(span.file.clone());
                    entry.insert((vec![index], 0));
                }
            }
        } else {
            ordered_indices.push(index);
        }
    }
    for file in &file_order {
        let (indices, _) = bases_by_file
            .get_mut(file)
            .expect("base diagnostic file was indexed");
        radix_sort_base_diagnostic_indices(indices, base_diagnostics);
        ordered_indices.extend(indices.iter().copied());
    }
    (bases_by_file, ordered_indices)
}

fn merge_existing_base_diagnostics(
    diagnostics: &mut Vec<Diagnostic>,
    bases_by_file: &mut BaseDiagnosticsByFile,
    bases: &mut [Option<VariantRefinementBaseDiagnostic>],
) {
    let existing = std::mem::take(diagnostics);
    let (future_min_end, last_same_span) = index_existing_diagnostic_order(&existing);
    diagnostics.reserve(existing.len() + bases.len());
    for (existing_index, existing_diagnostic) in existing.into_iter().enumerate() {
        record_variant_refinement_base_work(1);
        let mut matching_bases = existing_diagnostic
            .span
            .as_ref()
            .and_then(|span| bases_by_file.get_mut(&span.file));
        while matching_bases.as_ref().is_some_and(|(indices, cursor)| {
            let Some(index) = indices.get(*cursor) else {
                return false;
            };
            record_variant_refinement_base_work(1);
            base_precedes_existing(
                bases[*index].as_ref().expect("pending base diagnostic"),
                &existing_diagnostic,
            ) && !future_existing_precedes_base(
                bases[*index].as_ref().expect("pending base diagnostic"),
                &existing_diagnostic,
                existing_index,
                future_min_end[existing_index],
                &last_same_span,
            )
        }) {
            let (indices, cursor) = matching_bases.as_mut().expect("matched base file");
            let index = indices[*cursor];
            *cursor += 1;
            diagnostics.push(
                bases[index]
                    .take()
                    .expect("pending base diagnostic")
                    .diagnostic,
            );
        }
        diagnostics.push(existing_diagnostic);
    }
}

fn index_existing_diagnostic_order(
    existing: &[Diagnostic],
) -> (Vec<Option<usize>>, DiagnosticSpansByFile) {
    let mut future_min_end = vec![None; existing.len()];
    let mut min_end_by_file = HashMap::<SourcePath, usize>::new();
    let mut last_same_span = DiagnosticSpansByFile::new();
    for (index, diagnostic) in existing.iter().enumerate().rev() {
        record_variant_refinement_base_work(1);
        let Some(span) = diagnostic.span.as_ref() else {
            continue;
        };
        future_min_end[index] = min_end_by_file.get(&span.file).copied();
        min_end_by_file
            .entry(span.file.clone())
            .and_modify(|end| *end = (*end).min(span.end.offset))
            .or_insert(span.end.offset);
        if diagnostic.id != "type.invalid_annotation" {
            last_same_span
                .entry(span.file.clone())
                .or_default()
                .entry((span.start.offset, span.end.offset))
                .or_insert(index);
        }
    }
    (future_min_end, last_same_span)
}

fn future_existing_precedes_base(
    base: &VariantRefinementBaseDiagnostic,
    existing: &Diagnostic,
    existing_index: usize,
    future_min_end: Option<usize>,
    last_same_span: &DiagnosticSpansByFile,
) -> bool {
    let Some(span) = base.diagnostic.span.as_ref() else {
        return false;
    };
    let later_alternative_casing = existing.id == "name.invalid_case"
        && existing.span.as_ref().is_some_and(|existing_span| {
            base.union_scope.as_ref().is_some_and(|union_scope| {
                existing_span.file == union_scope.file
                    && existing_span.start.offset >= base.ordering_scope.end.offset
                    && existing_span.start.offset >= union_scope.start.offset
                    && existing_span.end.offset <= union_scope.end.offset
            })
        });
    (future_min_end.is_some_and(|end| end <= span.start.offset) && !later_alternative_casing)
        || last_same_span
            .get(&span.file)
            .and_then(|positions| positions.get(&(span.start.offset, span.end.offset)))
            .is_some_and(|index| *index > existing_index)
}

fn radix_sort_base_diagnostic_indices(
    indices: &mut Vec<usize>,
    diagnostics: &[VariantRefinementBaseDiagnostic],
) {
    record_variant_refinement_base_work(indices.len());
    let mut scratch = vec![0; indices.len()];
    radix_sort_base_diagnostic_phase(indices, &mut scratch, diagnostics);
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            let mut counts = [0usize; 256];
            record_variant_refinement_base_work(counts.len());
            for &index in indices.iter() {
                record_variant_refinement_base_work(1);
                let span = base_diagnostic_ordering_span(&diagnostics[index]);
                let value = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                counts[(value >> shift) as u8 as usize] += 1;
            }
            let mut next = 0;
            for count in &mut counts {
                record_variant_refinement_base_work(1);
                let start = next;
                next += *count;
                *count = start;
            }
            for &index in indices.iter() {
                record_variant_refinement_base_work(1);
                let span = base_diagnostic_ordering_span(&diagnostics[index]);
                let value = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                let bucket = (value >> shift) as u8 as usize;
                scratch[counts[bucket]] = index;
                counts[bucket] += 1;
            }
            std::mem::swap(indices, &mut scratch);
        }
    }
}

fn base_diagnostic_ordering_span(
    diagnostic: &VariantRefinementBaseDiagnostic,
) -> &veln_source::SourceSpan {
    if diagnostic.diagnostic.id == "type.variant_refinement_unknown" {
        &diagnostic.ordering_scope
    } else {
        diagnostic
            .diagnostic
            .span
            .as_ref()
            .expect("indexed base diagnostic has a source span")
    }
}

fn radix_sort_base_diagnostic_phase(
    indices: &mut Vec<usize>,
    scratch: &mut Vec<usize>,
    diagnostics: &[VariantRefinementBaseDiagnostic],
) {
    let mut counts = [0usize; 2];
    record_variant_refinement_base_work(counts.len());
    for &index in indices.iter() {
        record_variant_refinement_base_work(1);
        let phase = usize::from(diagnostics[index].diagnostic.id == "type.variant_union_base");
        counts[phase] += 1;
    }
    let mut next = 0;
    for count in &mut counts {
        record_variant_refinement_base_work(1);
        let start = next;
        next += *count;
        *count = start;
    }
    for &index in indices.iter() {
        record_variant_refinement_base_work(1);
        let phase = usize::from(diagnostics[index].diagnostic.id == "type.variant_union_base");
        scratch[counts[phase]] = index;
        counts[phase] += 1;
    }
    std::mem::swap(indices, scratch);
}

fn base_precedes_existing(base: &VariantRefinementBaseDiagnostic, existing: &Diagnostic) -> bool {
    let (Some(span), Some(existing_span)) = (base.diagnostic.span.as_ref(), existing.span.as_ref())
    else {
        return false;
    };
    if existing.id == "name.invalid_case"
        && existing_span.file == base.ordering_scope.file
        && existing_span.start.offset >= base.ordering_scope.start.offset
        && existing_span.end.offset <= base.ordering_scope.end.offset
    {
        return false;
    }
    existing_span.file == span.file
        && (existing_span.start.offset >= span.end.offset
            || (existing.id == "type.invalid_annotation"
                && existing_span.start.offset == span.start.offset
                && existing_span.end.offset >= span.end.offset))
}
