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
fn validate_reports_stale_projections() {
    let fixture = Fixture::new();
    let exported = run_in_directory(&fixture.directory, &fixture.database, ["export"]);
    assert!(exported.status.success(), "{}", output_text(&exported));
    let dumped = run_in_directory(&fixture.directory, &fixture.database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));

    let clean = run_in_directory(
        &fixture.directory,
        &fixture.database,
        ["validate", "--json"],
    );
    assert!(clean.status.success(), "{}", output_text(&clean));
    let clean: Value = serde_json::from_slice(&clean.stdout).expect("clean validation JSON");
    assert_eq!(clean["valid"], true);
    assert_eq!(clean["issues"].as_array().expect("clean issues").len(), 0);

    let retitled = run(
        &fixture.database,
        ["retitle", &fixture.record_id, "Retitled CLI test"],
    );
    assert!(retitled.status.success(), "{}", output_text(&retitled));

    let stale = run_in_directory(
        &fixture.directory,
        &fixture.database,
        ["validate", "--json"],
    );
    assert!(stale.status.success(), "{}", output_text(&stale));
    let stale_json: Value = serde_json::from_slice(&stale.stdout).expect("stale validation JSON");
    assert_eq!(stale_json["valid"], false);
    let stale_issues = stale_json["issues"].as_array().expect("stale issues");
    assert!(stale_issues.iter().any(|issue| {
        issue
            .as_str()
            .is_some_and(|issue| issue.contains("docs/records/adr/0001-cli-test.md"))
    }));

    let plain = run_in_directory(&fixture.directory, &fixture.database, ["validate"]);
    assert!(!plain.status.success());
    let plain_stdout = String::from_utf8_lossy(&plain.stdout);
    assert!(plain_stdout.contains("ERROR different docs/records/adr/0001-cli-test.md"));
    assert!(plain_stdout.contains("ERROR different docs/db-snapshot.sql"));
}

#[test]
fn validate_from_nested_directory_uses_repository_default_targets() {
    let directory = temporary_directory("validate-default-targets");
    let root = directory.join("repo");
    let nested = root.join("nested");
    let database = root.join("selected.db");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").expect("git HEAD");
    fs::create_dir_all(&nested).expect("nested directory");

    let created = run_in_directory(
        &root,
        &database,
        [
            "new",
            "adr",
            "Nested validation test",
            "--document",
            r#"{"schema":"adr/v1","context":"test","decision":"test","alternatives":"test","consequences":"test","evidence":"test"}"#,
            "--json",
        ],
    );
    assert!(created.status.success(), "{}", output_text(&created));
    let record: Value = serde_json::from_slice(&created.stdout).expect("record JSON");
    let record_id = format!(
        "ADR-{:04}",
        record["id"]["number"].as_u64().expect("record number")
    );

    let exported = run_in_directory(&root, &database, ["export"]);
    assert!(exported.status.success(), "{}", output_text(&exported));
    let dumped = run_in_directory(&root, &database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));

    let retitled = run(&database, ["retitle", &record_id, "Retitled nested test"]);
    assert!(retitled.status.success(), "{}", output_text(&retitled));

    let validated = Command::new(env!("CARGO_BIN_EXE_strata"))
        .current_dir(&nested)
        .env_remove("STRATA_DATABASE")
        .env_remove("STRATA_EXPORT_TARGET")
        .env_remove("STRATA_DUMP_TARGET")
        .args([
            "--database",
            database.to_str().expect("database path"),
            "validate",
            "--json",
        ])
        .output()
        .expect("run strata");
    assert!(validated.status.success(), "{}", output_text(&validated));
    let validation: Value = serde_json::from_slice(&validated.stdout).expect("validation JSON");
    assert_eq!(validation["valid"], false);
    let issues = validation["issues"].as_array().expect("validation issues");
    let export_path = root.join("docs/records/adr/0001-nested-validation-test.md");
    let dump_path = root.join("docs/db-snapshot.sql");
    assert!(issues.iter().any(|issue| {
        issue
            .as_str()
            .is_some_and(|issue| issue.contains(&format!("different {}", export_path.display())))
    }));
    assert!(issues.iter().any(|issue| {
        issue
            .as_str()
            .is_some_and(|issue| issue.contains(&format!("different {}", dump_path.display())))
    }));

    cleanup_temporary_directory(directory);
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
