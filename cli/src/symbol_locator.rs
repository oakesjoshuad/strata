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
    let stable_id = run_resolve(command_template, code_root, query, path)?;
    let stable_id = &stable_id;
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

// `graphlite resolve --prefer-file` replaces a raw `graphlite symbols --file`
// full-text search (EDR-0018): `symbols` matches a query string anywhere in a
// symbol's rendered signature, so a function invoked from within another
// item's body in the same file (its call site's signature text contains the
// callee's name) was wrongly reported as a second match, making an
// unambiguous query look ambiguous. `resolve` already ranks candidates and
// excludes exactly that kind of incidental substring hit -- confirmed
// directly, the caller-substring case reports `candidates="1"` under
// `resolve` where raw `symbols` reported two matches for the same query.
//
// `resolve` does not eliminate genuine ambiguity, though: two distinct items
// sharing a name in different scopes of the same file (verified directly --
// two functions both named `run` in different modules of one file) are
// still reported as separate candidates, `resolve` still silently picks a
// "top" one (`selected_id`) with no error, and -- because graphlite's
// stable_id does not encode module nesting -- both candidates can even
// carry the *same* stable_id, so a later `graph sym:<stable_id>` lookup
// cannot catch the collision either (it resolves to whichever one graphlite
// picked, not both). The `candidates` count on `<resolution>` is what
// actually distinguishes these two cases: the caller-substring false
// positive resolves to `candidates="1"` (resolve itself excluded the
// substring hit); genuine ambiguity resolves to `candidates="2"` or more.
// Trusting `selected_id` alone would silently swallow real ambiguity;
// checking `candidates == 1` restores it as an explicit error without
// reintroducing the original false-positive rate.
fn run_resolve(
    command_template: &str,
    code_root: &Path,
    query: &str,
    path: &Path,
) -> Result<String, LocatorError> {
    let expected_path = normalize_path(&path.to_string_lossy());
    let path_arg = path.to_string_lossy();
    let output = match run_locator(
        command_template,
        code_root,
        &["resolve", query, "--prefer-file", &path_arg],
    ) {
        Ok(output) => output,
        Err(LocatorError::Unavailable(message)) if message.contains("no symbol matched") => {
            return Err(LocatorError::Missing)
        }
        Err(error) => return Err(error),
    };

    let candidates: usize = tags_with_attribute(&output, "candidates")
        .into_iter()
        .find_map(|attributes| attributes.get("candidates")?.parse().ok())
        .ok_or(LocatorError::Missing)?;
    if candidates != 1 {
        return Err(LocatorError::Ambiguous);
    }

    let winner = tags_with_attribute(&output, "stable_id")
        .into_iter()
        .next()
        .ok_or(LocatorError::Missing)?;
    let (stable_id, file) = (
        winner.get("stable_id").cloned(),
        winner.get("file").map(|f| normalize_path(f)),
    );
    match (stable_id, file) {
        (Some(stable_id), Some(file)) if file == expected_path => Ok(stable_id),
        _ => Err(LocatorError::Missing),
    }
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches("./").to_owned()
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
        let script = r#"if test "$1" = resolve; then printf '<resolution query="target" candidates="1" selected_id="1"><symbol stable_id="src/lib.rs::fn::target" file="./src/lib.rs"/></resolution>'; else printf '<graph><symbol stable_id="src/lib.rs::fn::other" range="L20-L30"/><symbol stable_id="src/lib.rs::fn::target" range="L1-L1"/></graph>'; fi"#;
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
        let script = r#"if test "$1" = resolve; then printf '<resolution query="target" candidates="1" selected_id="1">\n  <symbol id="1" name="target" file="./src/lib.rs" stable_id="src/lib.rs::fn::target" signature="fn target(\n    a: i32,\n) -&gt; Result&lt;(), Error&gt;"/>\n</resolution>'; else printf '<graph><symbol stable_id="src/lib.rs::fn::target" range="L1-L1" signature="fn target(\n    a: i32,\n)"/></graph>'; fi"#;
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
    fn rejects_top_candidate_from_a_different_file() {
        // `--prefer-file` is a ranking bias, not a hard filter: if nothing in
        // the target file matches, resolve can still return a top candidate
        // from elsewhere. That must not be silently trusted as this file's
        // symbol.
        let root = std::env::temp_dir().join(format!(
            "strata-locator-wrong-file-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        let script = r#"printf '<resolution query="target" candidates="1" selected_id="1"><symbol stable_id="src/other.rs::fn::target" file="./src/other.rs"/></resolution>'"#;
        let command = format!("sh -c {} sh", shell_words::quote(script));
        assert!(matches!(
            locate(&command, &root, "target", Path::new("src/lib.rs")),
            Err(LocatorError::Missing)
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn treats_multiple_resolve_candidates_as_ambiguous() {
        // resolve always picks a `selected_id` even when it reports more
        // than one real candidate (verified directly against real graphlite:
        // two same-named functions in different modules of one file report
        // candidates="2" and a silently chosen selected_id, with no error).
        // Trusting selected_id alone would swallow that; the candidates
        // count is what actually distinguishes it from a clean match.
        let root = std::env::temp_dir().join(format!(
            "strata-locator-candidates-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        let script = r#"printf '<resolution query="run" candidates="2" selected_id="1"><symbol stable_id="src/lib.rs::fn::run" file="./src/lib.rs"/><alternatives><symbol stable_id="src/lib.rs::fn::run" file="./src/lib.rs"/></alternatives></resolution>'"#;
        let command = format!("sh -c {} sh", shell_words::quote(script));
        assert!(matches!(
            locate(&command, &root, "run", Path::new("src/lib.rs")),
            Err(LocatorError::Ambiguous)
        ));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn treats_no_symbol_matched_as_missing_not_unavailable() {
        // `graphlite resolve` exits non-zero with an error message (not
        // empty XML) when nothing matches at all -- confirmed directly.
        // That must surface as the same Missing outcome `symbols` returning
        // zero results used to produce, not as a locator failure.
        let root = std::env::temp_dir().join(format!(
            "strata-locator-not-found-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        let script = r#"echo "Error: no symbol matched query 'target'" >&2; exit 1"#;
        let command = format!("sh -c {} sh", shell_words::quote(script));
        assert!(matches!(
            locate(&command, &root, "target", Path::new("src/lib.rs")),
            Err(LocatorError::Missing)
        ));
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

    #[test]
    #[ignore = "requires graphlite installed on PATH"]
    fn real_graphlite_in_file_ambiguity_is_caught_by_the_candidates_check() {
        // EDR-0018's flagged trade-off, tested against real graphlite rather
        // than assumed: two distinct functions sharing a name in different
        // modules of one file. Confirmed directly that `resolve` alone would
        // NOT be a safe replacement for `symbols` here -- it deterministically
        // picks a `selected_id` for this case with no error, and because
        // graphlite's own stable_id does not encode module nesting, both
        // candidates even carry the *same* stable_id, so a later
        // `graph sym:<id>` lookup cannot catch the collision either (it just
        // resolves to whichever one graphlite picked). The `candidates`
        // count on `<resolution>` is what actually distinguishes this case
        // (candidates="2") from the original false-positive this record
        // fixes (candidates="1" once resolve's own ranking excludes the
        // incidental substring hit) -- this test proves that check holds
        // against a real, physically ambiguous fixture, not just the
        // fake-locator unit tests above.
        let root = std::env::temp_dir().join(format!(
            "strata-locator-real-ambiguity-test-{}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("root");
        fs::write(
            root.join("lib.rs"),
            "mod outer {\n    pub fn run() {}\n}\n\nmod inner {\n    pub fn run() {}\n}\n",
        )
        .expect("file");
        std::process::Command::new("graphlite")
            .arg("discover")
            .arg(".")
            .current_dir(&root)
            .output()
            .expect("graphlite discover");

        assert!(matches!(
            locate("graphlite", &root, "run", Path::new("lib.rs")),
            Err(LocatorError::Ambiguous)
        ));

        fs::remove_dir_all(root).expect("cleanup");
    }
}
