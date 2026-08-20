use crate::CliError;
use std::fs;
use std::path::{Path, PathBuf};

pub(crate) const TEMPLATE: &str = include_str!("../assets/site/template.html");
pub(crate) const STYLESHEET: &str = include_str!("../assets/site/style.css");
pub(crate) const LUA_FILTER: &str = include_str!("../assets/site/xref.lua");
pub(crate) const ASSETS_DIR_NAME: &str = "_assets";
pub(crate) const STYLESHEET_FILE_NAME: &str = "style.css";

pub(crate) fn write_template(directory: &Path) -> Result<PathBuf, CliError> {
    let path = directory.join("template.html");
    fs::write(&path, TEMPLATE)?;
    Ok(path)
}

pub(crate) fn write_lua_filter(directory: &Path) -> Result<PathBuf, CliError> {
    let path = directory.join("xref.lua");
    fs::write(&path, LUA_FILTER)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        directory: PathBuf,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).expect("remove temporary fixture");
        }
    }

    fn fixture() -> Fixture {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("strata-assets-test-{suffix}"));
        fs::create_dir_all(&directory).expect("temp directory");
        Fixture { directory }
    }

    #[test]
    fn constants_describe_the_published_asset_convention() {
        assert_eq!(ASSETS_DIR_NAME, "_assets");
        assert_eq!(STYLESHEET_FILE_NAME, "style.css");
        assert!(TEMPLATE.contains("theme-toggle"));
        assert!(STYLESHEET.contains("data-theme"));
        assert!(LUA_FILTER.contains("site_index"));
    }

    #[test]
    fn writes_embedded_template_and_returns_its_path() {
        let fixture = fixture();
        let path = write_template(&fixture.directory).expect("template");
        assert_eq!(path, fixture.directory.join("template.html"));
        assert_eq!(fs::read_to_string(path).expect("template bytes"), TEMPLATE);
    }

    #[test]
    fn writes_embedded_lua_filter_and_returns_its_path() {
        let fixture = fixture();
        let path = write_lua_filter(&fixture.directory).expect("Lua filter");
        assert_eq!(path, fixture.directory.join("xref.lua"));
        assert_eq!(
            fs::read_to_string(path).expect("Lua filter bytes"),
            LUA_FILTER
        );
    }
}
