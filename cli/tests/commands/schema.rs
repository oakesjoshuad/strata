use super::*;

#[test]
fn schema_json_includes_kind_purpose() {
    let output = run_without_database(["schema", "adr", "--json"]);
    assert!(output.status.success(), "{}", output_text(&output));
    let schema: Value = serde_json::from_slice(&output.stdout).expect("schema JSON");
    assert_eq!(
        schema["purpose"],
        "What architecturally significant choice was made and why?"
    );
}

#[test]
fn specification_schema_exposes_machine_readable_requirement_tables() {
    let output = run_without_database(["schema", "spec", "--json"]);
    assert!(output.status.success(), "{}", output_text(&output));
    let schema: Value = serde_json::from_slice(&output.stdout).expect("schema JSON");
    assert_eq!(schema["kind"], "specification");
    assert!(schema["required_fields"]
        .as_array()
        .expect("required fields")
        .iter()
        .any(|field| field[0] == "requirements" && field[1] == "table"));
}
