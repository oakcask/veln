fn assert_duplicate_handler_declarations_are_rejected() {
    let duplicate = vec![
        source(
            "first.veln",
            "mod shared\n\nhandler run() for Work\n  go() => 1\nend\n",
        ),
        source(
            "second.veln",
            concat!(
                "mod shared\n\n",
                "handler run() for Work\n  go() => 2\nend\n\n",
                "handler keep() for Work\n  go() => 3\nend\n\n",
                "fn use() -> Int\n  handle 1 with run()\nend\n\n",
                "fn valid() -> Int\n  handle 2 with keep()\nend\n",
            ),
        ),
    ];
    let declaration = query(duplicate.clone(), "first.veln", 3, 9).unwrap();
    assert!(!declaration.reference_eligible);
    assert!(declaration.references.is_empty());
    let duplicate_snapshot = EffectiveProjectSnapshot::new(duplicate.clone());
    for source_path in ["first.veln", "second.veln"] {
        assert_eq!(
            definition_at(
                &duplicate_snapshot,
                SourcePosition {
                    source: SourcePath::new(source_path),
                    line: 3,
                    column: 9,
                },
            ),
            None,
        );
    }
    assert!(query(duplicate.clone(), "second.veln", 12, 17).is_none());
    let valid = query_snapshot(&duplicate_snapshot, "second.veln", 16, 17).unwrap();
    assert_location(&valid.definition, "second.veln", 7, 9);
    assert!(valid.reference_eligible);
    assert_eq!(locations(&valid.references), [("second.veln", 16, 17)]);
    assert_location(
        &definition_at(
            &duplicate_snapshot,
            SourcePosition {
                source: SourcePath::new("second.veln"),
                line: 16,
                column: 17,
            },
        )
        .unwrap(),
        "second.veln",
        7,
        9,
    );
}

fn assert_recovered_handler_declarations_are_rejected() {
    let recovered = vec![source(
        "main.veln",
        concat!(
            "handler run() for Work @\n  go() => 1\nend\n\n",
            "handler keep() for Work\n  go() => 2\nend\n\n",
            "fn use() -> Int\n  handle 1 with run()\nend\n\n",
            "fn valid() -> Int\n  handle 2 with keep()\nend\n",
        ),
    )];
    let recovered_snapshot = EffectiveProjectSnapshot::new(recovered.clone());
    let recovered_declaration = query_snapshot(&recovered_snapshot, "main.veln", 1, 9).unwrap();
    assert!(!recovered_declaration.reference_eligible);
    assert!(recovered_declaration.references.is_empty());
    assert_eq!(
        definition_at(
            &recovered_snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 1,
                column: 9,
            },
        ),
        None,
    );
    assert!(query(recovered.clone(), "main.veln", 10, 17).is_none());
    let valid = query_snapshot(&recovered_snapshot, "main.veln", 14, 17).unwrap();
    assert_location(&valid.definition, "main.veln", 5, 9);
    assert_eq!(locations(&valid.references), [("main.veln", 14, 17)]);
    assert_location(
        &definition_at(
            &recovered_snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 14,
                column: 17,
            },
        )
        .unwrap(),
        "main.veln",
        5,
        9,
    );
}
