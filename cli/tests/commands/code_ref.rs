use super::*;

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
    assert_eq!(reference["id"], format!("{}-CR-001", fixture.record_id));
}

#[test]
fn code_ref_update_changes_fields() {
    let fixture = Fixture::new();
    let added = run(
        &fixture.database,
        [
            "code-ref-add",
            &fixture.record_id,
            "constrains",
            "store/src/lib.rs",
            "--json",
        ],
    );
    assert!(added.status.success(), "{}", output_text(&added));
    let added: Value = serde_json::from_slice(&added.stdout).expect("code reference JSON");
    let id = added["id"].as_str().expect("id").to_owned();

    let updated = run(
        &fixture.database,
        [
            "code-ref-update",
            &id,
            "implements",
            "store/src/other.rs",
            "--symbol",
            "Store::other",
            "--json",
        ],
    );
    assert!(updated.status.success(), "{}", output_text(&updated));
    let updated: Value = serde_json::from_slice(&updated.stdout).expect("code reference JSON");
    assert_eq!(updated["id"], id);
    assert_eq!(updated["relation"], "implements");
    assert_eq!(updated["path"], "store/src/other.rs");
    assert_eq!(updated["symbol"], "Store::other");

    let graph = run(&fixture.database, ["graph", &fixture.record_id, "--json"]);
    assert!(graph.status.success(), "{}", output_text(&graph));
    let graph: Value = serde_json::from_slice(&graph.stdout).expect("graph JSON");
    assert_eq!(graph["code_references"][0]["path"], "store/src/other.rs");
}

#[test]
fn code_ref_update_rejects_unknown_id() {
    let fixture = Fixture::new();
    let output = run(
        &fixture.database,
        [
            "code-ref-update",
            &format!("{}-CR-999", fixture.record_id),
            "constrains",
            "store/src/lib.rs",
        ],
    );
    assert!(!output.status.success());
    assert!(output_text(&output).contains("code reference not found"));
}

#[test]
fn code_ref_remove_deletes_and_is_idempotent_failure() {
    let fixture = Fixture::new();
    let added = run(
        &fixture.database,
        [
            "code-ref-add",
            &fixture.record_id,
            "constrains",
            "store/src/lib.rs",
            "--json",
        ],
    );
    assert!(added.status.success(), "{}", output_text(&added));
    let added: Value = serde_json::from_slice(&added.stdout).expect("code reference JSON");
    let id = added["id"].as_str().expect("id").to_owned();

    let removed = run(&fixture.database, ["code-ref-remove", &id, "--json"]);
    assert!(removed.status.success(), "{}", output_text(&removed));
    let removed: Value = serde_json::from_slice(&removed.stdout).expect("removal JSON");
    assert_eq!(removed["removed"], id);

    let graph = run(&fixture.database, ["graph", &fixture.record_id, "--json"]);
    assert!(graph.status.success(), "{}", output_text(&graph));
    let graph: Value = serde_json::from_slice(&graph.stdout).expect("graph JSON");
    assert!(graph["code_references"]
        .as_array()
        .expect("code_references")
        .is_empty());

    let remove_again = run(&fixture.database, ["code-ref-remove", &id]);
    assert!(!remove_again.status.success());
    assert!(output_text(&remove_again).contains("code reference not found"));
}
