use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

struct Fixture {
    directory: PathBuf,
    database: PathBuf,
    record_id: String,
}

impl Fixture {
    fn new() -> Self {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("strata-cli-test-{suffix}"));
        fs::create_dir_all(&directory).expect("temporary directory");
        let database = directory.join("strata.db");
        let output = run(
            &database,
            [
                "new",
                "adr",
                "CLI test",
                "--document",
                r#"{"schema":"adr/v1","context":"test","decision":"test","alternatives":"test","consequences":"test","evidence":"test"}"#,
                "--json",
            ],
        );
        assert!(output.status.success(), "{}", output_text(&output));
        let record: Value = serde_json::from_slice(&output.stdout).expect("record JSON");
        let number = record["id"]["number"].as_u64().expect("record number");
        let record_id = format!("ADR-{number:04}");
        Self {
            directory,
            database,
            record_id,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.directory).expect("temporary directory cleanup");
    }
}

fn run<I, S>(database: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_strata"))
        .arg("--database")
        .arg(database)
        .args(args)
        .output()
        .expect("run strata")
}

fn run_in_directory<I, S>(directory: &Path, database: &Path, args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_strata"))
        .current_dir(directory)
        .arg("--database")
        .arg(database)
        .args(args)
        .output()
        .expect("run strata")
}

fn run_without_database<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    Command::new(env!("CARGO_BIN_EXE_strata"))
        .args(args)
        .output()
        .expect("run strata")
}

fn output_text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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
}
