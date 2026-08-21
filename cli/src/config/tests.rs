use super::*;
use std::sync::{Mutex, OnceLock};

fn environment_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

#[test]
fn precedence_is_cli_then_environment_then_file_then_default() {
    let _guard = environment_lock().lock().expect("environment lock");
    let directory = tempfile_directory("precedence");
    let root = directory.join("repo");
    fs::create_dir_all(root.join("nested")).expect("nested directory");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::write(
        root.join(CONFIG_FILE),
        r#"{"database":"file.db","export_target":"file-export","dump_target":"file.sql","publish_target":"file-site","publish_renderer":"file-renderer"}"#,
    )
    .expect("config");
    let current = env::current_dir().expect("current directory");
    env::set_current_dir(root.join("nested")).expect("nested directory");
    env::set_var("STRATA_DATABASE", "env.db");
    let config = resolve(Some(Path::new("cli.db")), None, None, None, None, None).expect("config");
    assert_eq!(config.database.source, Source::Cli);
    assert_eq!(config.export_target.source, Source::File);
    assert_eq!(config.dump_target.source, Source::File);
    assert_eq!(config.publish_target.source, Source::File);
    assert_eq!(config.publish_renderer.source, Source::File);
    env::remove_var("STRATA_DATABASE");
    env::set_current_dir(current).expect("restore directory");
    fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn publish_values_follow_cli_environment_file_and_default_sources() {
    let _guard = environment_lock().lock().expect("environment lock");
    let directory = tempfile_directory("publish-sources");
    let root = directory.join("repo");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::write(
        root.join(CONFIG_FILE),
        r#"{"publish_target":"file-site","publish_renderer":"file-renderer"}"#,
    )
    .expect("config");
    let current = env::current_dir().expect("current directory");
    env::set_current_dir(&root).expect("repository directory");
    env::remove_var("STRATA_PUBLISH_TARGET");
    env::remove_var("STRATA_PUBLISH_RENDERER");

    let file = resolve(None, None, None, None, None, None).expect("file config");
    assert_eq!(file.publish_target.source, Source::File);
    assert_eq!(file.publish_renderer.source, Source::File);

    env::set_var("STRATA_PUBLISH_TARGET", "env-site");
    env::set_var("STRATA_PUBLISH_RENDERER", "env-renderer");
    let environment = resolve(None, None, None, None, None, None).expect("environment config");
    assert_eq!(environment.publish_target.source, Source::Env);
    assert_eq!(environment.publish_renderer.source, Source::Env);

    let cli = resolve(
        Some(Path::new("db")),
        None,
        None,
        Some(Path::new("cli-site")),
        Some("cli-renderer"),
        None,
    )
    .expect("CLI config");
    assert_eq!(cli.publish_target.source, Source::Cli);
    assert_eq!(cli.publish_renderer.source, Source::Cli);

    env::remove_var("STRATA_PUBLISH_TARGET");
    env::remove_var("STRATA_PUBLISH_RENDERER");
    env::set_current_dir(current).expect("restore directory");
    fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn missing_file_falls_through_to_defaults() {
    let _guard = environment_lock().lock().expect("environment lock");
    let directory = tempfile_directory("missing");
    let root = directory.join("repo");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    let current = env::current_dir().expect("current directory");
    env::set_current_dir(&root).expect("repository directory");
    env::remove_var("STRATA_DATABASE");
    let config = resolve(None, None, None, None, None, None).expect("config");
    assert_eq!(config.database.source, Source::Default);
    env::set_current_dir(current).expect("restore directory");
    fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn defaults_are_anchored_to_repository_root_from_nested_directory() {
    let _guard = environment_lock().lock().expect("environment lock");
    let directory = tempfile_directory("default-root");
    let root = directory.join("repo");
    let nested = root.join("nested");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::create_dir_all(&nested).expect("nested directory");
    let current = env::current_dir().expect("current directory");
    env::set_current_dir(&nested).expect("nested directory");
    env::remove_var("STRATA_DATABASE");
    env::remove_var("STRATA_EXPORT_TARGET");
    env::remove_var("STRATA_DUMP_TARGET");
    env::remove_var("STRATA_PUBLISH_TARGET");
    env::remove_var("STRATA_PUBLISH_RENDERER");

    let config = resolve(None, None, None, None, None, None).expect("config");
    assert_eq!(config.database.value, root.join(DATABASE_DEFAULT));
    assert_eq!(config.export_target.value, root.join(EXPORT_TARGET_DEFAULT));
    assert_eq!(config.dump_target.value, root.join(DUMP_TARGET_DEFAULT));
    assert_eq!(
        config.publish_target.value,
        root.join(PUBLISH_TARGET_DEFAULT)
    );
    assert_eq!(config.publish_renderer.value, PUBLISH_RENDERER_DEFAULT);
    assert_eq!(config.database.source, Source::Default);
    assert_eq!(config.export_target.source, Source::Default);
    assert_eq!(config.dump_target.source, Source::Default);

    env::set_current_dir(current).expect("restore directory");
    fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn defaults_are_anchored_to_explicit_database_repository() {
    let _guard = environment_lock().lock().expect("environment lock");
    let directory = tempfile_directory("selected-database-root");
    let selected = directory.join("selected");
    let caller = directory.join("caller");
    fs::create_dir_all(selected.join(".git")).expect("selected git marker");
    fs::create_dir_all(caller.join(".git")).expect("caller git marker");
    let database = selected.join("strata.db");
    let current = env::current_dir().expect("current directory");
    env::set_current_dir(&caller).expect("caller directory");
    env::remove_var("STRATA_DATABASE");
    env::remove_var("STRATA_EXPORT_TARGET");
    env::remove_var("STRATA_DUMP_TARGET");

    let config = resolve(Some(&database), None, None, None, None, None).expect("config");
    assert_eq!(config.database.value, database);
    assert_eq!(
        config.export_target.value,
        selected.join(EXPORT_TARGET_DEFAULT)
    );
    assert_eq!(config.dump_target.value, selected.join(DUMP_TARGET_DEFAULT));
    assert_eq!(
        config.publish_target.value,
        selected.join(PUBLISH_TARGET_DEFAULT)
    );
    assert!(!config.export_target.value.starts_with(&caller));
    assert!(!config.dump_target.value.starts_with(&caller));

    env::set_current_dir(current).expect("restore directory");
    fs::remove_dir_all(directory).expect("fixture cleanup");
}

#[test]
fn malformed_file_is_an_error() {
    let _guard = environment_lock().lock().expect("environment lock");
    let directory = tempfile_directory("malformed");
    let root = directory.join("repo");
    fs::create_dir_all(root.join(".git")).expect("git marker");
    fs::write(root.join(CONFIG_FILE), "not json").expect("config");
    let current = env::current_dir().expect("current directory");
    env::set_current_dir(&root).expect("repository directory");
    let error =
        resolve(None, None, None, None, None, None).expect_err("malformed config must fail");
    assert!(error.to_string().contains("failed to parse"));
    env::set_current_dir(current).expect("restore directory");
    fs::remove_dir_all(directory).expect("fixture cleanup");
}

fn tempfile_directory(name: &str) -> PathBuf {
    let path = env::temp_dir().join(format!("strata-config-test-{name}-{}", std::process::id()));
    fs::create_dir_all(&path).expect("temporary directory");
    path
}
