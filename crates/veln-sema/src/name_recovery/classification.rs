use std::collections::HashMap;

use veln_ast::{SurfaceModule, UseDecl};

use super::{
    ImportBindingKey, ImportTargetKey, prefix_bucket_offsets, record_import_classification_work,
};

pub(super) fn source_ordered_import_indices(module: &SurfaceModule) -> Vec<usize> {
    let mut indices = module
        .uses
        .iter()
        .enumerate()
        .filter_map(|(index, use_decl)| {
            (use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.recovered).then_some(index)
        })
        .collect::<Vec<_>>();
    if indices.len() < 2 {
        return indices;
    }
    let mut scratch = vec![0; indices.len()];
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            radix_sort_import_offset_byte(module, &indices, &mut scratch, shift, use_end);
            std::mem::swap(&mut indices, &mut scratch);
        }
    }
    radix_sort_import_file_bytes(module, &mut indices, &mut scratch);
    indices
}

fn radix_sort_import_offset_byte(
    module: &SurfaceModule,
    indices: &[usize],
    output: &mut [usize],
    shift: u32,
    use_end: bool,
) {
    let mut counts = [0usize; 256];
    for &index in indices {
        record_import_classification_work();
        let offset = selected_offset(&module.uses[index], use_end);
        counts[(offset >> shift) as u8 as usize] += 1;
    }
    prefix_bucket_offsets(&mut counts);
    for &index in indices {
        record_import_classification_work();
        let offset = selected_offset(&module.uses[index], use_end);
        let bucket = (offset >> shift) as u8 as usize;
        output[counts[bucket]] = index;
        counts[bucket] += 1;
    }
}

fn selected_offset(use_decl: &UseDecl, use_end: bool) -> usize {
    if use_end {
        use_decl.span.end.offset
    } else {
        use_decl.span.start.offset
    }
}

fn radix_sort_import_file_bytes(
    module: &SurfaceModule,
    indices: &mut [usize],
    scratch: &mut [usize],
) {
    let Some(&first) = indices.first() else {
        return;
    };
    let first_file = &module.uses[first].span.file;
    if indices
        .iter()
        .all(|index| module.uses[*index].span.file == *first_file)
    {
        return;
    }
    let mut pending = vec![(0usize, indices.len(), 0usize)];
    while let Some((start, end, byte_index)) = pending.pop() {
        if end - start < 2 {
            continue;
        }
        let sizes = file_byte_bucket_sizes(module, &indices[start..end], byte_index);
        let mut positions = bucket_positions(&sizes, start);
        copy_file_byte_buckets(
            module,
            &indices[start..end],
            scratch,
            &mut positions,
            byte_index,
        );
        indices[start..end].copy_from_slice(&scratch[start..end]);
        enqueue_nontrivial_buckets(&mut pending, &sizes, end, byte_index);
    }
}

fn file_byte_bucket_sizes(
    module: &SurfaceModule,
    indices: &[usize],
    byte_index: usize,
) -> [usize; 257] {
    let mut sizes = [0usize; 257];
    for &index in indices {
        record_import_classification_work();
        sizes[import_file_byte_bucket(&module.uses[index], byte_index)] += 1;
    }
    sizes
}

fn bucket_positions(sizes: &[usize; 257], start: usize) -> [usize; 257] {
    let mut positions = *sizes;
    let mut next = start;
    for position in &mut positions {
        let bucket_start = next;
        next += *position;
        *position = bucket_start;
    }
    positions
}

fn copy_file_byte_buckets(
    module: &SurfaceModule,
    indices: &[usize],
    scratch: &mut [usize],
    positions: &mut [usize; 257],
    byte_index: usize,
) {
    for &index in indices {
        record_import_classification_work();
        let bucket = import_file_byte_bucket(&module.uses[index], byte_index);
        scratch[positions[bucket]] = index;
        positions[bucket] += 1;
    }
}

fn enqueue_nontrivial_buckets(
    pending: &mut Vec<(usize, usize, usize)>,
    sizes: &[usize; 257],
    end: usize,
    byte_index: usize,
) {
    let mut bucket_end = end;
    for bucket in (1..sizes.len()).rev() {
        let bucket_start = bucket_end - sizes[bucket];
        if bucket_end - bucket_start > 1 {
            pending.push((bucket_start, bucket_end, byte_index + 1));
        }
        bucket_end = bucket_start;
    }
}

fn import_file_byte_bucket(use_decl: &UseDecl, byte_index: usize) -> usize {
    use_decl
        .span
        .file
        .as_str()
        .as_bytes()
        .get(byte_index)
        .map_or(0, |byte| usize::from(*byte) + 1)
}

#[derive(Clone, Copy, Default)]
struct FirstTwo {
    first: Option<usize>,
    second: Option<usize>,
}

impl FirstTwo {
    fn push(&mut self, index: usize) {
        if self.first.is_none() {
            self.first = Some(index);
        } else if self.second.is_none() {
            self.second = Some(index);
        }
    }

    fn first_except(self, excluded: Option<usize>) -> Option<usize> {
        self.first
            .filter(|candidate| Some(*candidate) != excluded)
            .or_else(|| self.second.filter(|candidate| Some(*candidate) != excluded))
    }
}

struct DiagnosticOrigins {
    target_origins: Vec<Option<usize>>,
    qualifier_origins: Vec<Option<usize>>,
    first_targets: HashMap<ImportTargetKey, usize>,
    first_qualifiers: HashMap<ImportBindingKey, FirstTwo>,
    first_explicit_qualifiers: HashMap<ImportBindingKey, FirstTwo>,
}

impl DiagnosticOrigins {
    fn new(use_count: usize) -> Self {
        Self {
            target_origins: vec![None; use_count],
            qualifier_origins: vec![None; use_count],
            first_targets: HashMap::new(),
            first_qualifiers: HashMap::new(),
            first_explicit_qualifiers: HashMap::new(),
        }
    }

    fn record(&mut self, index: usize, use_decl: &UseDecl) {
        let target = import_target_key(use_decl);
        let duplicate_target = self.first_targets.get(&target).copied();
        self.target_origins[index] = duplicate_target;

        let qualifier = import_binding_key(use_decl);
        self.qualifier_origins[index] =
            self.qualifier_origin(&qualifier, use_decl.has_explicit_alias(), duplicate_target);
        self.remember(index, target, qualifier, use_decl.has_explicit_alias());
    }

    fn qualifier_origin(
        &self,
        qualifier: &ImportBindingKey,
        explicit: bool,
        duplicate_target: Option<usize>,
    ) -> Option<usize> {
        let candidates = if explicit {
            self.first_qualifiers.get(qualifier)
        } else {
            self.first_explicit_qualifiers.get(qualifier)
        };
        candidates.and_then(|candidates| candidates.first_except(duplicate_target))
    }

    fn remember(
        &mut self,
        index: usize,
        target: ImportTargetKey,
        qualifier: ImportBindingKey,
        explicit: bool,
    ) {
        self.first_targets.entry(target).or_insert(index);
        self.first_qualifiers
            .entry(qualifier.clone())
            .or_default()
            .push(index);
        if explicit {
            self.first_explicit_qualifiers
                .entry(qualifier)
                .or_default()
                .push(index);
        }
    }

    fn finish(self) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
        (self.target_origins, self.qualifier_origins)
    }
}

pub(super) fn duplicate_diagnostic_origins(
    module: &SurfaceModule,
    source_order: &[usize],
) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let mut origins = DiagnosticOrigins::new(module.uses.len());
    for &index in source_order {
        record_import_classification_work();
        origins.record(index, &module.uses[index]);
    }
    origins.finish()
}

fn import_target_key(use_decl: &UseDecl) -> ImportTargetKey {
    (
        use_decl.module_name.clone(),
        use_decl.written_name.clone(),
        use_decl.package.clone(),
    )
}

fn import_binding_key(use_decl: &UseDecl) -> ImportBindingKey {
    (use_decl.module_name.clone(), use_decl.alias.clone())
}
