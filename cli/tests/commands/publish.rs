use super::*;

const STUB_RENDERER: &str = "sh -c 'cat \"$1\" > \"$2\"' sh {input} {output}";
const FAILING_RENDERER: &str = "sh -c 'printf renderer-failed >&2; exit 1' sh";

fn publish_args(target: &Path, renderer: &str) -> Vec<String> {
    vec![
        "--publish-target".into(),
        target.display().to_string(),
        "--renderer-command".into(),
        renderer.into(),
        "publish".into(),
    ]
}

fn prepare_projections(fixture: &Fixture) {
    fs::create_dir_all(fixture.directory.join(".git")).expect("repository marker");
    fs::write(
        fixture.directory.join(".git/HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("repository HEAD");
    let exported = run_in_directory(&fixture.directory, &fixture.database, ["export"]);
    assert!(exported.status.success(), "{}", output_text(&exported));
    let dumped = run_in_directory(&fixture.directory, &fixture.database, ["dump"]);
    assert!(dumped.status.success(), "{}", output_text(&dumped));
}

#[test]
fn publish_subprocess_writes_site_files() {
    let fixture = Fixture::new();
    prepare_projections(&fixture);
    let target = fixture.directory.join("site");
    let published = run(&fixture.database, publish_args(&target, STUB_RENDERER));
    assert!(published.status.success(), "{}", output_text(&published));
    assert!(target.join("adr/0001-cli-test.html").is_file());
    assert!(target.join("manifest.json").is_file());
    assert!(target.join("_assets/style.css").is_file());
}

#[test]
fn publish_check_subprocess_reports_clean_after_publish() {
    let fixture = Fixture::new();
    prepare_projections(&fixture);
    let target = fixture.directory.join("site");
    let published = run(&fixture.database, publish_args(&target, STUB_RENDERER));
    assert!(published.status.success(), "{}", output_text(&published));

    let mut check_args = publish_args(&target, STUB_RENDERER);
    check_args.push("--check".into());
    let checked = run(&fixture.database, check_args);
    assert!(checked.status.success(), "{}", output_text(&checked));
    assert!(String::from_utf8_lossy(&checked.stdout).contains("publish check clean"));
}

#[test]
fn publish_check_subprocess_reports_new_record_as_missing() {
    let fixture = Fixture::new();
    prepare_projections(&fixture);
    let target = fixture.directory.join("site");
    let published = run(&fixture.database, publish_args(&target, STUB_RENDERER));
    assert!(published.status.success(), "{}", output_text(&published));

    let created = run(
        &fixture.database,
        [
            "new",
            "adr",
            "Second publish record",
            "--document",
            r#"{"schema":"adr/v1","context":"test","decision":"test","alternatives":"test","consequences":"test","evidence":"test"}"#,
        ],
    );
    assert!(created.status.success(), "{}", output_text(&created));
    prepare_projections(&fixture);

    let mut check_args = publish_args(&target, STUB_RENDERER);
    check_args.push("--check".into());
    let checked = run(&fixture.database, check_args);
    assert!(!checked.status.success());
    let output = format!(
        "{}{}",
        String::from_utf8_lossy(&checked.stdout),
        output_text(&checked)
    );
    assert!(output.contains("missing adr/0002-second-publish-record.html"));
}

#[test]
fn validation_error_blocks_publish_before_target_creation() {
    let fixture = Fixture::new();
    prepare_projections(&fixture);
    let target = fixture.directory.join("site");
    let proposed = run(
        &fixture.database,
        ["status", &fixture.record_id, "proposed"],
    );
    assert!(proposed.status.success(), "{}", output_text(&proposed));
    let accepted = run(
        &fixture.database,
        ["status", &fixture.record_id, "accepted"],
    );
    assert!(accepted.status.success(), "{}", output_text(&accepted));
    let created = run(
        &fixture.database,
        [
            "new",
            "adr",
            "Superseding record",
            "--document",
            r#"{"schema":"adr/v1","context":"test","decision":"test","alternatives":"test","consequences":"test","evidence":"test"}"#,
            "--json",
        ],
    );
    assert!(created.status.success(), "{}", output_text(&created));
    let source = serde_json::from_slice::<Value>(&created.stdout).expect("record JSON");
    let source_id = format!(
        "ADR-{:04}",
        source["id"]["number"].as_u64().expect("source number")
    );
    let linked = run(
        &fixture.database,
        ["link", &source_id, "supersedes", &fixture.record_id],
    );
    assert!(linked.status.success(), "{}", output_text(&linked));

    let published = run(&fixture.database, publish_args(&target, STUB_RENDERER));
    assert!(!published.status.success());
    assert!(output_text(&published).contains("remains accepted"));
    assert!(!target.exists());
}

#[test]
fn renderer_failure_leaves_target_uncreated() {
    let fixture = Fixture::new();
    prepare_projections(&fixture);
    let target = fixture.directory.join("site");
    let published = run(&fixture.database, publish_args(&target, FAILING_RENDERER));
    assert!(!published.status.success());
    assert!(output_text(&published).contains("renderer command"));
    assert!(!target.exists());
}

#[test]
fn staging_failure_leaves_partial_sibling_and_target_untouched() {
    let fixture = Fixture::new();
    prepare_projections(&fixture);
    let target = fixture.directory.join("site");
    fs::write(&target, "old target").expect("old target");
    let published = run(&fixture.database, publish_args(&target, STUB_RENDERER));
    assert!(!published.status.success());
    assert_eq!(fs::read(&target).expect("target contents"), b"old target");
    let staging = fs::read_dir(&fixture.directory)
        .expect("staging directory parent")
        .map(|entry| entry.expect("staging directory entry"))
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with("site.staging-")
        })
        .map(|entry| entry.path())
        .expect("staging directory");
    assert!(staging.is_dir());
    assert!(staging.join("adr/0001-cli-test.html").is_file());
}
