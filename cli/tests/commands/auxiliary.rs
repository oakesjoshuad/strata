use super::*;

#[test]
fn context_json_combines_search_and_graph_for_each_hit() {
    let fixture = Fixture::new();
    let first = create_context_record(&fixture.database, "Context alpha");
    let second = create_context_record(&fixture.database, "Context beta");
    let linked = run(&fixture.database, ["link", &first, "relates-to", &second]);
    assert!(linked.status.success(), "{}", output_text(&linked));
    for id in [&first, &second] {
        let evidence = run(
            &fixture.database,
            ["evidence-add", id, "issue", "Context evidence"],
        );
        assert!(evidence.status.success(), "{}", output_text(&evidence));
    }

    let output = run(&fixture.database, ["context", "retrievalneedle", "--json"]);
    assert!(output.status.success(), "{}", output_text(&output));
    let context: Value = serde_json::from_slice(&output.stdout).expect("context JSON");
    assert_eq!(context["query"], "retrievalneedle");
    let results = context["results"].as_array().expect("context results");
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|result| {
        result["record"]["document"]["context"] == "retrievalneedle"
            && result["evidence"]
                .as_array()
                .is_some_and(|items| !items.is_empty())
    }));
    assert!(results.iter().any(|result| !result["relationships"]
        .as_array()
        .expect("relationships")
        .is_empty()));
}

#[test]
fn context_limit_bounds_results_not_each_graph() {
    let fixture = Fixture::new();
    let first = create_context_record(&fixture.database, "Context alpha");
    let second = create_context_record(&fixture.database, "Context beta");
    let linked = run(&fixture.database, ["link", &first, "relates-to", &second]);
    assert!(linked.status.success(), "{}", output_text(&linked));
    let evidence = run(
        &fixture.database,
        ["evidence-add", &first, "issue", "Context evidence"],
    );
    assert!(evidence.status.success(), "{}", output_text(&evidence));

    let output = run(
        &fixture.database,
        ["context", "retrievalneedle", "--limit", "1", "--json"],
    );
    assert!(output.status.success(), "{}", output_text(&output));
    let context: Value = serde_json::from_slice(&output.stdout).expect("context JSON");
    let results = context["results"].as_array().expect("context results");
    assert_eq!(results.len(), 1);
    assert!(!results[0]["relationships"]
        .as_array()
        .expect("relationships")
        .is_empty());
    assert!(!results[0]["evidence"]
        .as_array()
        .expect("evidence")
        .is_empty());
}

#[test]
fn link_emits_json_when_requested() {
    let fixture = Fixture::new();
    let target = create_context_record(&fixture.database, "Link target");
    let output = run(
        &fixture.database,
        ["link", &fixture.record_id, "relates-to", &target, "--json"],
    );
    assert!(output.status.success(), "{}", output_text(&output));
    let relationships: Value = serde_json::from_slice(&output.stdout).expect("relationship JSON");
    let relationship = &relationships.as_array().expect("relationship array")[0];
    assert_eq!(relationship["source_id"]["number"], 1);
    assert_eq!(relationship["relation"], "relates-to");
    assert_eq!(relationship["target_id"]["number"], 2);
}

#[test]
fn link_accepts_multiple_targets_and_emits_an_array() {
    let fixture = Fixture::new();
    let first = create_context_record(&fixture.database, "First link target");
    let second = create_context_record(&fixture.database, "Second link target");
    let output = run(
        &fixture.database,
        [
            "link",
            &fixture.record_id,
            "relates-to",
            &first,
            &second,
            "--json",
        ],
    );
    assert!(output.status.success(), "{}", output_text(&output));
    let relationships: Value = serde_json::from_slice(&output.stdout).expect("relationship JSON");
    let relationships = relationships.as_array().expect("relationship array");
    assert_eq!(relationships.len(), 2);
    assert_eq!(relationships[0]["target_id"]["number"], 2);
    assert_eq!(relationships[1]["target_id"]["number"], 3);
}

#[test]
fn evidence_add_emits_json_and_graph_includes_it() {
    let fixture = Fixture::new();
    let output = run(
        &fixture.database,
        [
            "evidence-add",
            &fixture.record_id,
            "benchmark",
            "Latency benchmark",
            "--uri",
            "https://example.com/benchmark",
            "--metadata",
            r#"{"runner":"criterion"}"#,
            "--json",
        ],
    );
    assert!(output.status.success(), "{}", output_text(&output));
    let evidence: Value = serde_json::from_slice(&output.stdout).expect("evidence JSON");
    assert_eq!(evidence["id"], format!("{}-EV-001", fixture.record_id));
    assert_eq!(evidence["kind"], "benchmark");

    let graph = run(&fixture.database, ["graph", &fixture.record_id, "--json"]);
    assert!(graph.status.success(), "{}", output_text(&graph));
    let graph: Value = serde_json::from_slice(&graph.stdout).expect("graph JSON");
    assert_eq!(graph["evidence"][0]["title"], "Latency benchmark");
}

#[test]
fn code_ref_add_emits_json() {
    let fixture = Fixture::new();
    let output = run(
        &fixture.database,
        [
            "code-ref-add",
            &fixture.record_id,
            "constrains",
            "store/src/lib.rs",
            "--symbol",
            "Store::graph",
            "--line-start",
            "42",
            "--line-end",
            "60",
            "--json",
        ],
    );
    assert!(output.status.success(), "{}", output_text(&output));
    let reference: Value = serde_json::from_slice(&output.stdout).expect("code reference JSON");
    assert_eq!(reference["relation"], "constrains");
    assert_eq!(reference["line_start"], 42);
    assert_eq!(reference["line_end"], 60);
}

#[test]
fn auxiliary_commands_report_validation_errors() {
    let fixture = Fixture::new();
    let evidence = run(
        &fixture.database,
        [
            "evidence-add",
            &fixture.record_id,
            "not-valid",
            "Bad evidence",
        ],
    );
    assert!(!evidence.status.success());
    assert!(output_text(&evidence).contains("unsupported evidence kind"));

    let code_reference = run(
        &fixture.database,
        [
            "code-ref-add",
            &fixture.record_id,
            "not-valid",
            "src/lib.rs",
        ],
    );
    assert!(!code_reference.status.success());
    assert!(output_text(&code_reference).contains("unsupported relationship"));
}

#[test]
fn dump_and_check_report_clean_snapshot() {
    let fixture = Fixture::new();
    let dumped = run_in_directory(&fixture.directory, &fixture.database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));
    assert!(fixture.directory.join("docs/db-snapshot.sql").is_file());

    let checked = run_in_directory(&fixture.directory, &fixture.database, ["dump", "--check"]);
    assert!(checked.status.success(), "{}", output_text(&checked));
    assert!(String::from_utf8_lossy(&checked.stdout).contains("dump check clean"));
}

#[test]
fn dump_check_reports_stale_file() {
    let fixture = Fixture::new();
    let dumped = run_in_directory(&fixture.directory, &fixture.database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));
    let revised = run(
        &fixture.database,
        [
            "new",
            "edr",
            "New record",
            "--document",
            r#"{"schema":"edr/v1","context":"test","decision":"test","alternatives":"test","consequences":"test","evidence":"test"}"#,
        ],
    );
    assert!(revised.status.success(), "{}", output_text(&revised));
    let checked = run_in_directory(&fixture.directory, &fixture.database, ["dump", "--check"]);
    assert!(!checked.status.success());
    assert!(output_text(&checked).contains("different docs/db-snapshot.sql"));
}

#[test]
fn dump_check_reports_different_file_and_missing_file() {
    let fixture = Fixture::new();
    let dumped = run_in_directory(&fixture.directory, &fixture.database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));
    let snapshot = fixture.directory.join("docs/db-snapshot.sql");
    fs::write(&snapshot, "different").expect("overwrite snapshot");
    let different = run_in_directory(&fixture.directory, &fixture.database, ["dump", "--check"]);
    assert!(!different.status.success());
    assert!(output_text(&different).contains("different docs/db-snapshot.sql"));

    fs::remove_file(&snapshot).expect("remove snapshot");
    let missing = run_in_directory(&fixture.directory, &fixture.database, ["dump", "--check"]);
    assert!(!missing.status.success());
    assert!(output_text(&missing).contains("missing docs/db-snapshot.sql"));
}

#[test]
fn dump_then_restore_round_trips_through_the_binary() {
    let fixture = Fixture::new();
    let dumped = run_in_directory(&fixture.directory, &fixture.database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));
    let restored_database = fixture.directory.join("restored.db");
    let restored = run_in_directory(
        &fixture.directory,
        &restored_database,
        ["restore", "docs/db-snapshot.sql"],
    );
    assert!(restored.status.success(), "{}", output_text(&restored));
    let searched = run(&restored_database, ["search", "CLI", "--json"]);
    assert!(searched.status.success(), "{}", output_text(&searched));
    let records: Value = serde_json::from_slice(&searched.stdout).expect("search JSON");
    assert_eq!(records.as_array().expect("records").len(), 1);
}
