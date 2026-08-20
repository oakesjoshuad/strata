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

fn create_context_record(database: &Path, title: &str) -> String {
    let output = run(
        database,
        [
            "new",
            "adr",
            title,
            "--document",
            r#"{"schema":"adr/v1","context":"retrievalneedle","decision":"test","alternatives":"test","consequences":"test","evidence":"test"}"#,
            "--json",
        ],
    );
    assert!(output.status.success(), "{}", output_text(&output));
    let record: Value = serde_json::from_slice(&output.stdout).expect("record JSON");
    let number = record["id"]["number"].as_u64().expect("record number");
    format!("ADR-{number:04}")
}

fn temporary_directory(name: &str) -> PathBuf {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!("strata-{name}-{suffix}"));
    fs::create_dir_all(&directory).expect("temporary directory");
    directory
}

fn cleanup_temporary_directory(directory: PathBuf) {
    fs::remove_dir_all(directory).expect("temporary directory cleanup");
}

#[path = "commands/auxiliary.rs"]
mod auxiliary;
#[path = "commands/capabilities.rs"]
mod capabilities;
#[path = "commands/configuration.rs"]
mod configuration;
#[path = "commands/publish.rs"]
mod publish;
#[path = "commands/schema.rs"]
mod schema;
#[path = "commands/validation.rs"]
mod validation;
