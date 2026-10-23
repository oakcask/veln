use super::*;

pub(super) struct SourceMetrics {
    pub(super) abc_subjects: Vec<AbcSubjectMetric>,
    pub(super) abc_contract_subject_count: usize,
    pub(super) similarities: Vec<SimilarityInstanceMetric>,
    pub(super) similarity_fingerprint_count: usize,
}

pub(super) fn collect_source_metrics(
    project: &Project,
    selected_paths: &BTreeSet<String>,
    similarity_min_tokens: usize,
) -> SourceMetrics {
    let mut abc_subjects = Vec::new();
    let mut abc_contract_subject_count = 0;
    let mut similarity_candidates = Vec::new();

    for source in &project.files {
        let path = source.path().as_str().to_string();
        if !selected_paths.contains(&path) || is_generated_or_doctest_path(&path) {
            continue;
        }
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        let lexed = lex(source);
        for item in parsed.tree.items {
            let SyntaxItem::Function(function) = item else {
                continue;
            };
            abc_contract_subject_count += usize::from(!function.contracts.is_empty());
            abc_subjects.push(abc_subject(source, &path, &function));

            let Some(body_range) = function_body_range(&function) else {
                continue;
            };
            let tokens = normalized_body_tokens(&lexed.tokens, body_range);
            if tokens.len() < similarity_min_tokens {
                continue;
            }
            let declaration = similarity_declaration(source, &path, &function, body_range);
            let fingerprint = similarity_fingerprint(&tokens);
            similarity_candidates.push(SimilarityCandidate {
                declaration,
                tokens,
                fingerprint,
            });
        }
    }

    abc_subjects.sort_by(compare_abc_subjects);
    let similarity_fingerprint_count = similarity_candidates.len();
    let similarities = similarity_instances_from_candidates(similarity_candidates);
    SourceMetrics {
        abc_subjects,
        abc_contract_subject_count,
        similarities,
        similarity_fingerprint_count,
    }
}
