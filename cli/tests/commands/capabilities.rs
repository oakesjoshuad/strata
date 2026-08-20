use super::*;

#[test]
fn capabilities_json_describes_all_kind_purposes() {
    let output = run_without_database(["capabilities", "--json"]);
    assert!(output.status.success(), "{}", output_text(&output));
    let capabilities: Value = serde_json::from_slice(&output.stdout).expect("capabilities JSON");
    let kinds = capabilities["kinds"].as_array().expect("kind descriptions");
    assert_eq!(kinds.len(), 4);
    for kind in kinds {
        assert!(kind["kind"].is_string());
        assert!(!kind["purpose"].as_str().expect("purpose").is_empty());
    }

    let commands = capabilities["commands"]
        .as_array()
        .expect("command descriptions");
    let json_output = capabilities["json_output"]
        .as_array()
        .expect("JSON-capable command descriptions");
    assert!(commands.iter().any(|name| name == "capabilities"));
    assert!(json_output.iter().any(|name| name == "capabilities"));
    assert!(json_output
        .iter()
        .all(|name| commands.iter().any(|command| command == name)));
}
