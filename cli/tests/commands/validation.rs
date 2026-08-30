use super::*;

#[test]
fn validate_from_different_repository_uses_database_repository_targets() {
    let directory = temporary_directory("validate-different-repository");
    let selected = directory.join("selected");
    let caller = directory.join("caller");
    let database = selected.join("selected.db");
    fs::create_dir_all(selected.join(".git")).expect("selected git marker");
    fs::write(selected.join(".git/HEAD"), "ref: refs/heads/main\n").expect("selected git HEAD");
    fs::create_dir_all(caller.join(".git")).expect("caller git marker");
    fs::write(caller.join(".git/HEAD"), "ref: refs/heads/main\n").expect("caller git HEAD");

    let created = run_in_directory(
        &selected,
        &database,
        [
            "new",
            "adr",
            "Cross repository validation test",
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

    let exported = run_in_directory(&selected, &database, ["export"]);
    assert!(exported.status.success(), "{}", output_text(&exported));
    let dumped = run_in_directory(&selected, &database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));
    let retitled = run(
        &database,
        ["retitle", &record_id, "Retitled cross repository test"],
    );
    assert!(retitled.status.success(), "{}", output_text(&retitled));

    let validated = Command::new(env!("CARGO_BIN_EXE_strata"))
        .current_dir(&caller)
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
    let export_path = selected.join("docs/records/adr/0001-cross-repository-validation-test.md");
    let dump_path = selected.join("docs/db-snapshot.sql");
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
    assert!(!issues.iter().any(|issue| {
        issue
            .as_str()
            .is_some_and(|issue| issue.contains(&caller.display().to_string()))
    }));

    cleanup_temporary_directory(directory);
}
