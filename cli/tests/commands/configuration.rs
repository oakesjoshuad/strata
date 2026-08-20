use super::*;

#[test]
fn capabilities_json_reports_file_configuration_source() {
    let directory = temporary_directory("capabilities-file");
    let root = directory.join("repo");
    let nested = root.join("nested");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::create_dir_all(&nested).expect("nested directory");
    fs::write(
        root.join("strata.config.json"),
        r#"{"database":"file.db","export_target":"file-records","dump_target":"file.sql"}"#,
    )
    .expect("configuration");

    let output = Command::new(env!("CARGO_BIN_EXE_strata"))
        .current_dir(nested)
        .env_remove("STRATA_DATABASE")
        .env_remove("STRATA_EXPORT_TARGET")
        .env_remove("STRATA_DUMP_TARGET")
        .args(["capabilities", "--json"])
        .output()
        .expect("run strata");
    assert!(output.status.success(), "{}", output_text(&output));
    let capabilities: Value = serde_json::from_slice(&output.stdout).expect("capabilities JSON");
    assert_eq!(capabilities["configuration"]["database"]["source"], "file");
    assert_eq!(
        capabilities["configuration"]["database"]["value"],
        root.join("file.db").to_string_lossy().as_ref()
    );
    assert_eq!(
        capabilities["configuration"]["export_target"]["source"],
        "file"
    );
    cleanup_temporary_directory(directory);
}

#[test]
fn capabilities_json_reports_cli_source_over_environment_and_file() {
    let directory = temporary_directory("capabilities-cli");
    let root = directory.join("repo");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::write(
        root.join("strata.config.json"),
        r#"{"database":"file.db","export_target":"file-records","dump_target":"file.sql"}"#,
    )
    .expect("configuration");

    let output = Command::new(env!("CARGO_BIN_EXE_strata"))
        .current_dir(&root)
        .env("STRATA_DATABASE", "env.db")
        .args(["--database", "cli.db", "capabilities", "--json"])
        .output()
        .expect("run strata");
    assert!(output.status.success(), "{}", output_text(&output));
    let capabilities: Value = serde_json::from_slice(&output.stdout).expect("capabilities JSON");
    assert_eq!(capabilities["configuration"]["database"]["source"], "cli");
    assert_eq!(capabilities["configuration"]["database"]["value"], "cli.db");
    cleanup_temporary_directory(directory);
}

#[test]
fn capabilities_json_reports_environment_source_over_file_when_no_cli_flag() {
    let directory = temporary_directory("capabilities-env");
    let root = directory.join("repo");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::write(
        root.join("strata.config.json"),
        r#"{"database":"file.db","export_target":"file-records","dump_target":"file.sql"}"#,
    )
    .expect("configuration");

    let output = Command::new(env!("CARGO_BIN_EXE_strata"))
        .current_dir(&root)
        .env("STRATA_DATABASE", "env.db")
        .args(["capabilities", "--json"])
        .output()
        .expect("run strata");
    assert!(output.status.success(), "{}", output_text(&output));
    let capabilities: Value = serde_json::from_slice(&output.stdout).expect("capabilities JSON");
    assert_eq!(capabilities["configuration"]["database"]["source"], "env");
    assert_eq!(capabilities["configuration"]["database"]["value"], "env.db");
    assert_eq!(
        capabilities["configuration"]["export_target"]["source"],
        "file"
    );
    cleanup_temporary_directory(directory);
}
