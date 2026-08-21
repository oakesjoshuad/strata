use crate::CliError;
use std::path::Path;
use std::process::Command;

#[derive(Debug, thiserror::Error)]
pub(crate) enum LocatorError {
    #[error("locator unavailable: {0}")]
    Unavailable(String),
    #[error("symbol not found")]
    Missing,
    #[error("symbol matched more than once")]
    Ambiguous,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SymbolLocation {
    pub(crate) line_start: u32,
    pub(crate) line_end: u32,
}

pub(crate) fn locate(
    command_template: &str,
    code_root: &Path,
    query: &str,
    path: &Path,
) -> Result<SymbolLocation, LocatorError> {
    let stable_ids = run_symbols(command_template, code_root, query, path)?;
    let stable_id = match stable_ids.as_slice() {
        [] => return Err(LocatorError::Missing),
        [stable_id] => stable_id,
        _ => return Err(LocatorError::Ambiguous),
    };
    let graph = run_locator(
        command_template,
        code_root,
        &["graph", &format!("sym:{stable_id}")],
    )?;
    // `graph` prints the focus node plus its neighborhood, and every neighbor
    // with a known location also carries its own `range` attribute -- so the
    // range must be matched back to this symbol's own stable_id, not just
    // "the first (or only) range-bearing tag in the document".
    let ranges = tags_with_attribute(&graph, "range")
        .into_iter()
        .filter(|attributes| {
            attributes.get("stable_id").map(String::as_str) == Some(stable_id.as_str())
        })
        .filter_map(|attributes| attributes.get("range").cloned())
        .filter_map(|range| parse_range(&range))
        .collect::<Vec<_>>();
    match ranges.as_slice() {
        [range] => Ok(*range),
        [] => Err(LocatorError::Missing),
        _ => Err(LocatorError::Ambiguous),
    }
}

fn run_symbols(
    command_template: &str,
    code_root: &Path,
    query: &str,
    path: &Path,
) -> Result<Vec<String>, LocatorError> {
    let path = path.to_string_lossy();
    let output = run_locator(
        command_template,
        code_root,
        &["symbols", query, "--file", &path],
    )?;
    Ok(tags_with_attribute(&output, "stable_id")
        .into_iter()
        .filter_map(|attributes| attributes.get("stable_id").cloned())
        .collect())
}

fn run_locator(
    command_template: &str,
    code_root: &Path,
    arguments: &[&str],
) -> Result<String, LocatorError> {
    let command = shell_words::split(command_template)
        .map_err(|error| LocatorError::Unavailable(format!("invalid command: {error}")))?;
    let (program, prefix) = command
        .split_first()
        .ok_or_else(|| LocatorError::Unavailable("empty command".into()))?;
    let output = Command::new(program)
        .args(prefix)
        .args(arguments)
        .current_dir(code_root)
        .output()
        .map_err(|error| LocatorError::Unavailable(error.to_string()))?;
    if !output.status.success() {
        return Err(LocatorError::Unavailable(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|error| LocatorError::Unavailable(format!("non-UTF-8 output: {error}")))
}

fn tags_with_attribute(xml: &str, wanted: &str) -> Vec<std::collections::BTreeMap<String, String>> {
    xml.split('<')
        .filter_map(|fragment| fragment.split_once('>'))
        .filter(|(tag, _)| !tag.starts_with('/') && !tag.starts_with('!'))
        .filter_map(|(tag, _)| parse_attributes(tag))
        .filter(|attributes| attributes.contains_key(wanted))
        .collect()
}

// A hand-rolled, quote-aware attribute scanner -- not a naive whitespace
// split, because real attribute values (a `signature` in particular) always
// contain internal spaces and newlines. Splitting on whitespace first and
// looking for `=` in each token, as an XML-agnostic parser might, silently
// drops every such tag.
fn parse_attributes(tag: &str) -> Option<std::collections::BTreeMap<String, String>> {
    let after_name = match tag.split_once(char::is_whitespace) {
        Some((_, rest)) => rest,
        None => return Some(std::collections::BTreeMap::new()),
    };
    let mut attributes = std::collections::BTreeMap::new();
    let mut remaining = after_name.trim_start();
    while !remaining.is_empty() && remaining != "/" {
        let (name, after_equals) = remaining.split_once('=')?;
        let name = name.trim();
        let after_equals = after_equals.trim_start();
        let quote = after_equals.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let value_and_rest = &after_equals[quote.len_utf8()..];
        let end = value_and_rest.find(quote)?;
        attributes.insert(name.to_owned(), value_and_rest[..end].to_owned());
        remaining = value_and_rest[end + quote.len_utf8()..].trim_start();
    }
    Some(attributes)
}

fn parse_range(value: &str) -> Option<SymbolLocation> {
    let value = value.strip_prefix('L')?;
    let (start, end) = value.split_once("-L")?;
    Some(SymbolLocation {
        line_start: start.parse().ok()?,
        line_end: end.parse().ok()?,
    })
}

impl From<LocatorError> for CliError {
    fn from(error: LocatorError) -> Self {
        Self::Message(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn resolves_symbol_with_fake_locator() {
        let root = std::env::temp_dir().join(format!("strata-locator-test-{}", std::process::id()));
        fs::create_dir_all(&root).expect("root");
        let file = root.join("src/lib.rs");
        fs::create_dir_all(file.parent().expect("parent")).expect("directory");
        fs::write(&file, "fn target() {}").expect("file");
        // Real `graph` output includes the focus node plus neighbors, and
        // neighbors carry their own `range` attribute too -- this fixture
        // reproduces that shape so a naive "any range-bearing tag" scan
        // would wrongly report SYMBOL-AMBIGUOUS on an unambiguous match.
        let script = r#"if test "$1" = symbols; then printf '<results><symbol stable_id="src/lib.rs::fn::target"/></results>'; else printf '<graph><symbol stable_id="src/lib.rs::fn::other" range="L20-L30"/><symbol stable_id="src/lib.rs::fn::target" range="L1-L1"/></graph>'; fi"#;
        let command = format!("sh -c {} sh", shell_words::quote(script));
        assert_eq!(
            locate(&command, &root, "target", Path::new("src/lib.rs")).expect("locate"),
            SymbolLocation {
                line_start: 1,
                line_end: 1
            }
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn resolves_symbol_when_attribute_values_contain_whitespace() {
        // Real graphlite output always carries a multi-line, space-filled
        // `signature` attribute on every symbol tag. A naive whitespace
        // split (looking for `=` in each space-delimited token) breaks on
        // this and drops the tag entirely -- this fixture reproduces that
        // shape as a regression test.
        let root = std::env::temp_dir().join(format!(
            "strata-locator-whitespace-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        let file = root.join("src/lib.rs");
        fs::create_dir_all(file.parent().expect("parent")).expect("directory");
        fs::write(&file, "fn target() {}").expect("file");
        let script = r#"if test "$1" = symbols; then printf '<symbols>\n  <symbol id="1" name="target" stable_id="src/lib.rs::fn::target" signature="fn target(\n    a: i32,\n) -&gt; Result&lt;(), Error&gt;"/>\n</symbols>'; else printf '<graph><symbol stable_id="src/lib.rs::fn::target" range="L1-L1" signature="fn target(\n    a: i32,\n)"/></graph>'; fi"#;
        let command = format!("sh -c {} sh", shell_words::quote(script));
        assert_eq!(
            locate(&command, &root, "target", Path::new("src/lib.rs")).expect("locate"),
            SymbolLocation {
                line_start: 1,
                line_end: 1
            }
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    #[ignore = "requires graphlite installed on PATH"]
    fn resolves_symbol_with_real_graphlite() {
        // Cargo runs test binaries with the crate directory as the working
        // directory, not the workspace root where `.graphlite` actually
        // lives, so the repository root has to be derived explicitly.
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace root")
            .to_path_buf();
        let location = locate(
            "graphlite",
            &root,
            "substitute",
            Path::new("cli/src/render_backend.rs"),
        )
        .expect("graphlite locator");
        assert!(location.line_start > 0);
    }
}
