mod explicit_import_alias_navigation_performance_tests {
    use super::*;

    #[test]
    fn explicit_alias_index_work_and_retention_are_adjacent_linear() {
        fn measure(size: usize) -> ExplicitImportAliasIndexWork {
            let mut sources = Vec::new();
            for index in 0..size {
                sources.push(source(
                    &format!("target_{index}.veln"),
                    &format!("pub fn value_{index}() -> Int\n  {index}\nend\n"),
                ));
                sources.push(source(
                    &format!("part_{index}.veln"),
                    &format!(
                        "mod shared\nuse target_{index} as alias_{index}\n\nfn call_{index}() -> Int\n  alias_{index}::value_{index}()\nend\n"
                    ),
                ));
            }
            reset_explicit_import_alias_index_work();
            assert!(query(sources, "part_0.veln", 2, 17).is_some());
            explicit_import_alias_index_work()
        }

        let small = measure(24);
        let large = measure(48);
        eprintln!("explicit alias index adjacent sizes: 24={small:?}, 48={large:?}");
        assert!(
            large.construction <= small.construction * 2 + 64,
            "construction work: {small:?} -> {large:?}"
        );
        assert!(
            large.retained_entries <= small.retained_entries * 2 + 8,
            "retained entries: {small:?} -> {large:?}"
        );
    }

    #[test]
    fn explicit_alias_occurrences_on_one_line_have_adjacent_linear_token_work() {
        fn measure(size: usize) -> ExplicitImportAliasIndexWork {
            let expression = std::iter::repeat_n("calc::value()", size)
                .collect::<Vec<_>>()
                .join(" + ");
            let sources = vec![
                source("alpha.veln", "pub fn value() -> Int\n  1\nend\n"),
                source(
                    "app.veln",
                    &format!(
                        "use alpha as calc\n\nfn main() -> Int\n  {expression}\nend\n"
                    ),
                ),
            ];
            reset_explicit_import_alias_index_work();
            assert!(query(sources, "app.veln", 1, 14).is_some());
            explicit_import_alias_index_work()
        }

        let small = measure(32);
        let large = measure(64);
        eprintln!("same-line alias occurrences: 32={small:?}, 64={large:?}");
        assert!(
            large.token_visits <= small.token_visits * 2 + 16,
            "token visits: {small:?} -> {large:?}"
        );
        assert!(
            large.path_segments <= small.path_segments * 2 + 2,
            "path segments: {small:?} -> {large:?}"
        );
        assert!(
            large.path_bytes <= small.path_bytes * 2 + 8,
            "path bytes: {small:?} -> {large:?}"
        );
    }

    #[test]
    fn explicit_alias_deep_paths_have_adjacent_linear_resolution_and_storage_work() {
        fn measure(depth: usize) -> ExplicitImportAliasIndexWork {
            let suffix = std::iter::repeat_n("part", depth)
                .chain(std::iter::once("value"))
                .collect::<Vec<_>>()
                .join("::");
            let sources = vec![
                source("alpha.veln", "pub fn value() -> Int\n  1\nend\n"),
                source(
                    "app.veln",
                    &format!(
                        "use alpha as calc\n\nfn main() -> Int\n  calc::{suffix}()\nend\n"
                    ),
                ),
            ];
            reset_explicit_import_alias_index_work();
            let _ = query(sources, "app.veln", 4, 9 + depth * 6);
            let work = explicit_import_alias_index_work();
            assert!(
                work.path_segments >= depth * 2,
                "deep prefix attachment and lookup must both be measured: {work:?}"
            );
            work
        }

        fn old_prefix_key_bytes(depth: usize) -> usize {
            (depth + 1) * 4 + 3 * depth * (depth + 1)
        }

        let small = measure(32);
        let large = measure(64);
        eprintln!("deep alias paths: 32={small:?}, 64={large:?}");
        let old_small = old_prefix_key_bytes(32);
        let old_large = old_prefix_key_bytes(64);
        assert!(
            old_large > old_small * 2 + 8,
            "the replaced per-prefix keys must exceed the linear bound: {old_small} -> {old_large}"
        );
        assert!(
            large.path_segments <= small.path_segments * 2 + 2,
            "resolution segment visits: {small:?} -> {large:?}"
        );
        assert!(
            large.path_bytes <= small.path_bytes * 2 + 8,
            "retained key and constructed target bytes: {small:?} -> {large:?}"
        );
    }

    #[test]
    fn explicit_alias_target_storage_is_shared_across_occurrences_and_module_files() {
        fn measure(size: usize, distinct_suffixes: bool) -> (ExplicitImportAliasIndexWork, usize) {
            let target = "a".repeat(size);
            let package = "external";
            let occurrence = |index: usize| {
                if distinct_suffixes {
                    format!("calc::s{index}::value()")
                } else {
                    "calc::value()".to_owned()
                }
            };
            let midpoint = size / 2;
            let first_expression = (0..midpoint)
                .map(&occurrence)
                .collect::<Vec<_>>()
                .join(" + ");
            let second_expression = (midpoint..size)
                .map(occurrence)
                .collect::<Vec<_>>()
                .join(" + ");
            let first = format!(
                "mod app\nuse {target} as calc from \"{package}\"\n\nfn first() -> Int\n  {first_expression}\nend\n"
            );
            let second = format!(
                "mod app\n\nfn second() -> Int\n  {second_expression}\nend\n"
            );
            let source_bytes = first.len() + second.len();
            reset_explicit_import_alias_index_work();
            assert!(query(
                vec![source("first.veln", &first), source("second.veln", &second)],
                "first.veln",
                2,
                target.len() + 9,
            )
            .is_some());
            let work = explicit_import_alias_index_work();
            assert_eq!(work.target_bytes, target.len() + package.len());
            assert_eq!(work.retained_target_bytes, target.len() + package.len());
            assert!(work.target_bytes <= source_bytes);
            assert!(work.retained_target_bytes <= source_bytes);
            (work, source_bytes)
        }

        for distinct_suffixes in [false, true] {
            let (small, small_source_bytes) = measure(32, distinct_suffixes);
            let (large, large_source_bytes) = measure(64, distinct_suffixes);
            eprintln!(
                "shared explicit alias targets (distinct={distinct_suffixes}): \
                 32 bytes={small_source_bytes} {small:?}, \
                 64 bytes={large_source_bytes} {large:?}"
            );
            assert!(
                large.target_bytes <= small.target_bytes * 2 + 8,
                "materialized target bytes: {small:?} -> {large:?}"
            );
            assert!(
                large.retained_target_bytes <= small.retained_target_bytes * 2 + 8,
                "retained unique target bytes: {small:?} -> {large:?}"
            );
            assert!(
                large.path_bytes <= small.path_bytes * 2 + 256,
                "path storage work: {small:?} -> {large:?}"
            );
        }
    }

    #[test]
    fn explicit_alias_member_queries_do_not_copy_the_target_per_occurrence() {
        let small = measure_explicit_alias_member_queries(32);
        let large = measure_explicit_alias_member_queries(64);
        eprintln!("explicit alias member query adjacent sizes: 32={small:?}, 64={large:?}");
        assert!(small.member_identity_bytes > 0);
        assert!(
            large.path_bytes <= small.path_bytes * 2 + 256,
            "query-time target identity bytes: {small:?} -> {large:?}"
        );
        assert!(
            large.member_identity_bytes <= small.member_identity_bytes * 2 + 256,
            "copied or materialized member identity bytes: {small:?} -> {large:?}"
        );
    }

    fn measure_explicit_alias_member_queries(size: usize) -> ExplicitImportAliasIndexWork {
        let (snapshot, target_file) = explicit_alias_member_query_snapshot(size);
        reset_explicit_import_alias_index_work();
        assert_function_member_query(&snapshot, target_file, size);
        assert_constructor_member_query(&snapshot, target_file, size);
        assert_type_member_query(&snapshot, target_file, size);
        explicit_import_alias_index_work()
    }

    fn explicit_alias_member_query_snapshot(
        size: usize,
    ) -> (EffectiveProjectSnapshot, &'static str) {
        let (target, target_file) = explicit_alias_member_target(size);
        let expression = std::iter::repeat_n("calc::value()", size / 2)
            .collect::<Vec<_>>()
            .join(" + ");
        let constructors = std::iter::repeat_n("calc::Item::Ready(1)", size / 2)
            .collect::<Vec<_>>()
            .join(" + ");
        let type_functions = (0..size / 4)
            .map(|index| {
                format!("fn type_{index}(value: calc::Item) -> calc::Item\n  value\nend\n")
            })
            .collect::<Vec<_>>()
            .join("\n");
        let first = format!(
            "mod app\nuse {target} as calc from \"external\"\n\nfn first() -> Int\n  {expression}\nend\n\nfn first_items() -> calc::Item\n  {constructors}\nend\n\n{type_functions}\n"
        );
        let second = format!(
            "mod app\n\nfn second() -> Int\n  {expression}\nend\n\nfn second_items() -> calc::Item\n  {constructors}\nend\n\n{type_functions}\n"
        );
        let dependency = dependency_snapshot(
            "external",
            &[(
                target_file,
                concat!(
                    "pub fn value() -> Int\n  1\nend\n\n",
                    "pub type Item\n  pub Ready(Int)\nend\n",
                ),
            )],
            [target_file],
        );
        (
            EffectiveProjectSnapshot::with_direct_dependencies(
                vec![source("first.veln", &first), source("second.veln", &second)],
                vec![dependency],
            ),
            target_file,
        )
    }

    fn explicit_alias_member_target(size: usize) -> (&'static str, &'static str) {
        match size {
            32 => (
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.veln",
            ),
            64 => (
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.veln",
            ),
            _ => unreachable!(),
        }
    }

    fn assert_function_member_query(
        snapshot: &EffectiveProjectSnapshot,
        target_file: &str,
        size: usize,
    ) {
        let result = query_snapshot(snapshot, "second.veln", 4, 9).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::Function);
        assert_eq!(result.definition.span.file.as_str(), target_file);
        assert_eq!(result.definition.span.start.line, 1);
        assert_eq!(result.definition.span.start.column, 8);
        assert_eq!(result.references.len(), size);
        assert_eq!(references_in_file(&result, "first.veln"), size / 2);
        assert_eq!(references_in_file(&result, "second.veln"), size / 2);
        assert!(validate_rename_in_snapshot(snapshot, &result, "renamed").is_ok());
        assert_eq!(affected_spans(&result).len(), size + 1);
    }

    fn assert_constructor_member_query(
        snapshot: &EffectiveProjectSnapshot,
        target_file: &str,
        size: usize,
    ) {
        let result = query_snapshot(snapshot, "second.veln", 8, 15).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::Constructor);
        assert_eq!(result.definition.span.file.as_str(), target_file);
        assert_eq!(result.definition.span.start.line, 6);
        assert_eq!(result.definition.span.start.column, 7);
        assert_eq!(result.references.len(), size);
        assert!(validate_rename_in_snapshot(snapshot, &result, "Built").is_ok());
        assert_eq!(affected_spans(&result).len(), size + 1);
    }

    fn assert_type_member_query(
        snapshot: &EffectiveProjectSnapshot,
        target_file: &str,
        size: usize,
    ) {
        let result = query_snapshot(snapshot, "second.veln", 11, 24).unwrap();
        assert_eq!(result.selected_symbol.kind, SymbolKind::Type);
        assert_eq!(result.definition.span.file.as_str(), target_file);
        assert_eq!(result.definition.span.start.line, 5);
        assert_eq!(result.definition.span.start.column, 10);
        assert_eq!(result.references.len(), size * 2 + 2);
        assert!(validate_rename_in_snapshot(snapshot, &result, "Renamed").is_ok());
        assert_eq!(affected_spans(&result).len(), size * 2 + 3);
    }

    fn references_in_file(result: &NavigationResult, file: &str) -> usize {
        result
            .references
            .iter()
            .filter(|span| span.file.as_str() == file)
            .count()
    }

    fn affected_spans(result: &NavigationResult) -> Vec<SourceSpan> {
        let mut spans = vec![result.definition.span.clone()];
        spans.extend(result.references.clone());
        spans.sort_by(|left, right| {
            left.file
                .cmp(&right.file)
                .then_with(|| left.start.offset.cmp(&right.start.offset))
        });
        spans
    }
}
