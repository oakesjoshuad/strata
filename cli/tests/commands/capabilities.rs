use super::*;

#[test]
fn capabilities_json_describes_all_kind_purposes() {
    let output = run_without_database(["capabilities", "--json"]);
    assert!(output.status.success(), "{}", output_text(&output));
    let capabilities: Value = serde_json::from_slice(&output.stdout).expect("capabilities JSON");
    let kinds = capabilities["kinds"].as_array().expect("kind descriptions");
    assert_eq!(kinds.len(), 9);
    for kind in kinds {
        assert!(kind["kind"].is_string());
        assert!(!kind["purpose"].as_str().expect("purpose").is_empty());
    }
    assert!(kinds.iter().any(|kind| kind["kind"] == "specification"));
    assert!(kinds.iter().any(|kind| kind["kind"] == "risk"));

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

    let command_details = capabilities["command_details"]
        .as_array()
        .expect("command details");
    assert_eq!(command_details.len(), commands.len());
    for detail in command_details {
        let name = detail["name"].as_str().expect("command name");
        assert!(commands.iter().any(|command| command == name));
        assert!(!detail["purpose"]
            .as_str()
            .expect("command purpose")
            .is_empty());
    }

    let configuration = capabilities["configuration"]
        .as_object()
        .expect("configuration");
    for name in [
        "database",
        "export_target",
        "dump_target",
        "publish_target",
        "publish_renderer",
        "code_ref_locator",
    ] {
        let setting = match configuration.get(name) {
            Some(setting) => setting,
            None => panic!("missing configuration setting: {name}"),
        };
        assert!(setting["value"].is_string(), "{name} value");
        assert!(setting["source"].is_string(), "{name} source");
    }
}
