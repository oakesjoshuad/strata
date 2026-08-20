use crate::manifest::ManifestEntry;
use crate::CliError;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

pub(crate) struct RenderJob<'a> {
    pub(crate) input: &'a Path,
    pub(crate) output: &'a Path,
    pub(crate) metadata: &'a Path,
    pub(crate) template: &'a Path,
    pub(crate) css: &'a Path,
    pub(crate) lua_filter: &'a Path,
    pub(crate) site_index: &'a BTreeMap<String, String>,
    pub(crate) entry: &'a ManifestEntry,
}

pub(crate) fn render(command_template: &str, job: &RenderJob<'_>) -> Result<Vec<u8>, CliError> {
    let metadata = json!({
        "id": job.entry.id,
        "kind": job.entry.kind,
        "status": job.entry.status,
        "revision": job.entry.revision,
        "slug": job.entry.slug,
        "title": job.entry.title,
        "tags": job.entry.tags,
        // The Markdown input already has a canonical `relationships` map in
        // its frontmatter (relation kind -> raw target IDs). Emitting the
        // resolved publication links under that same key does not cleanly
        // override it -- Pandoc merges the two differently-shaped values
        // (verified empirically: the resulting $for(relationships)$ loop
        // runs but each item's fields resolve empty) -- so the resolved
        // list is kept under a distinct key instead.
        "publication_relationships": job.entry.relationships,
        "site_index": job.site_index,
    });
    fs::write(job.metadata, serde_json::to_vec(&metadata)?)?;

    let command = substitute(command_template, job);
    let arguments = shell_words::split(&command).map_err(|error| {
        CliError::Message(format!(
            "could not parse renderer command {command:?}: {error}"
        ))
    })?;
    let (program, arguments) = arguments
        .split_first()
        .ok_or_else(|| CliError::Message("configured renderer command is empty".into()))?;
    let output = Command::new(program)
        .args(arguments)
        .output()
        .map_err(|error| {
            CliError::Message(format!(
                "could not invoke renderer command {command:?}: {error}"
            ))
        })?;

    if !output.status.success() {
        return Err(CliError::Message(format!(
            "renderer command {command:?} failed with exit code {}: {}",
            output
                .status
                .code()
                .map_or_else(|| "unknown".to_owned(), |code| code.to_string()),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    if !job.output.exists() {
        return Err(CliError::Message(format!(
            "renderer command {command:?} succeeded but did not produce output {}",
            job.output.display()
        )));
    }

    Ok(fs::read(job.output)?)
}

fn substitute(command: &str, job: &RenderJob<'_>) -> String {
    command
        .replace("{input}", &job.input.display().to_string())
        .replace("{output}", &job.output.display().to_string())
        .replace("{metadata}", &job.metadata.display().to_string())
        .replace("{template}", &job.template.display().to_string())
        .replace("{css}", &job.css.display().to_string())
        .replace("{lua_filter}", &job.lua_filter.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets;
    use crate::config::PUBLISH_RENDERER_DEFAULT;
    use crate::manifest::ResolvedRelationship;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Fixture {
        directory: std::path::PathBuf,
        entry: ManifestEntry,
        input: std::path::PathBuf,
        output: std::path::PathBuf,
        metadata: std::path::PathBuf,
        template: std::path::PathBuf,
        css: std::path::PathBuf,
        lua_filter: std::path::PathBuf,
        site_index: BTreeMap<String, String>,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.directory).expect("remove temporary fixture");
        }
    }

    impl Fixture {
        fn job(&self) -> RenderJob<'_> {
            RenderJob {
                input: &self.input,
                output: &self.output,
                metadata: &self.metadata,
                template: &self.template,
                css: &self.css,
                lua_filter: &self.lua_filter,
                site_index: &self.site_index,
                entry: &self.entry,
            }
        }

        fn command(&self, script: &str) -> String {
            format!(
                "sh -c {} sh {{output}} {{metadata}}",
                shell_words::quote(script)
            )
        }
    }

    fn fixture() -> Fixture {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("strata-render-backend-test-{suffix}"));
        fs::create_dir_all(&directory).expect("temp directory");
        let input = directory.join("source.md");
        let output = directory.join("output.html");
        let metadata = directory.join("metadata.json");
        let template = directory.join("template.html");
        let css = directory.join("style.css");
        let lua_filter = directory.join("xref.lua");
        let site_index =
            BTreeMap::from([("ADR-0002".into(), "../adr/0002-target-record.html".into())]);
        fs::write(&input, "# Source").expect("input");
        fs::write(&lua_filter, assets::LUA_FILTER).expect("Lua filter");
        Fixture {
            directory,
            entry: ManifestEntry {
                id: "ADR-0001".into(),
                kind: "adr".into(),
                status: "accepted".into(),
                revision: 3,
                slug: "renderer-test".into(),
                publication_path: "adr/0001-renderer-test.html".into(),
                title: "Renderer test".into(),
                tags: vec!["rust".into(), "sqlite".into()],
                content_hash: "hash".into(),
                relationships: Vec::new(),
                rendered_markdown: "# Renderer test".into(),
            },
            input,
            output,
            metadata,
            template,
            css,
            lua_filter,
            site_index,
        }
    }

    #[test]
    fn invokes_renderer_and_returns_output_and_metadata() {
        let fixture = fixture();
        let command = fixture.command(r#"test -s "$2" && printf rendered > "$1""#);
        let contents = render(&command, &fixture.job()).expect("render");
        assert_eq!(contents, b"rendered");
        let metadata: serde_json::Value =
            serde_json::from_slice(&fs::read(&fixture.metadata).expect("metadata file"))
                .expect("metadata JSON");
        assert_eq!(metadata["id"], "ADR-0001");
        assert_eq!(metadata["kind"], "adr");
        assert_eq!(metadata["status"], "accepted");
        assert_eq!(metadata["slug"], "renderer-test");
        assert_eq!(metadata["title"], "Renderer test");
        assert_eq!(metadata["revision"], 3);
        assert_eq!(metadata["tags"], serde_json::json!(["rust", "sqlite"]));
        assert_eq!(metadata["publication_relationships"], serde_json::json!([]));
        assert_eq!(
            metadata["site_index"]["ADR-0002"],
            "../adr/0002-target-record.html"
        );
    }

    #[test]
    fn empty_relationships_are_not_rendered_by_the_backend_contract() {
        let fixture = fixture();
        let command = fixture.command(
            r#"if grep -q '"publication_relationships":\[\]' "$2"; then printf '<article>no relationships</article>' > "$1"; else printf '<section class="relationships">relationships</section>' > "$1"; fi"#,
        );
        let contents = render(&command, &fixture.job()).expect("render");
        let html = String::from_utf8(contents).expect("HTML");
        assert!(!html.contains("class=\"relationships\""));
    }

    // This smoke test needs a real Pandoc installation and is ignored in the
    // normal test suite so environments without Pandoc remain green.
    #[test]
    #[ignore = "requires Pandoc installed on PATH"]
    fn renders_shipped_template_with_real_pandoc() {
        if let Err(error) = Command::new("pandoc").arg("--version").output() {
            println!("skipping real Pandoc smoke test: {error}");
            return;
        }

        let fixture = fixture();
        fs::write(
            &fixture.input,
            "# ADR-0001: Published renderer test\n\n## Decision\n\nUse the shipped template.\n",
        )
        .expect("source");
        let template_directory = fixture.directory.join("template");
        fs::create_dir_all(&template_directory).expect("template directory");
        let template = assets::write_template(&template_directory).expect("write template");
        let css = std::path::PathBuf::from("../_assets/style.css");
        let job = RenderJob {
            input: &fixture.input,
            output: &fixture.output,
            metadata: &fixture.metadata,
            template: &template,
            css: &css,
            lua_filter: &fixture.lua_filter,
            site_index: &fixture.site_index,
            entry: &fixture.entry,
        };
        let command = PUBLISH_RENDERER_DEFAULT
            .replace("{css}", "../_assets/style.css")
            .replace("{lua_filter}", &fixture.lua_filter.display().to_string());
        let output = render(&command, &job).expect("Pandoc render");
        let output = String::from_utf8(output).expect("HTML output");
        for expected in [
            "Renderer test",
            "ADR-0001",
            "adr",
            "accepted",
            "<header",
            "<main",
            "<article",
            "<footer",
            "id=\"theme-toggle\"",
        ] {
            assert!(output.contains(expected), "output is missing {expected:?}");
        }
        assert!(!output.contains("class=\"relationships\""));
    }

    #[test]
    #[ignore = "requires Pandoc installed on PATH"]
    fn renders_relationship_link_with_real_pandoc() {
        if let Err(error) = Command::new("pandoc").arg("--version").output() {
            println!("skipping real Pandoc smoke test: {error}");
            return;
        }

        let mut fixture = fixture();
        fixture.entry.relationships = vec![ResolvedRelationship {
            relation: "relates-to".into(),
            target_id: "ADR-0002".into(),
            target_title: "Target record".into(),
            target_href: "../adr/0002-target-record.html".into(),
        }];
        fs::write(
            &fixture.input,
            "# ADR-0001: Published renderer test\n\nSee ADR-0002. Unknown RFC-9999.\n\nInline `ADR-0002`.\n\n```text\nADR-0002\n```\n",
        )
        .expect("source");
        let template_directory = fixture.directory.join("template");
        fs::create_dir_all(&template_directory).expect("template directory");
        let template = assets::write_template(&template_directory).expect("write template");
        let css = std::path::PathBuf::from("../_assets/style.css");
        let job = RenderJob {
            input: &fixture.input,
            output: &fixture.output,
            metadata: &fixture.metadata,
            template: &template,
            css: &css,
            lua_filter: &fixture.lua_filter,
            site_index: &fixture.site_index,
            entry: &fixture.entry,
        };
        let command = PUBLISH_RENDERER_DEFAULT
            .replace("{css}", "../_assets/style.css")
            .replace("{lua_filter}", &fixture.lua_filter.display().to_string());
        let output = render(&command, &job).expect("Pandoc render");
        let output = String::from_utf8(output).expect("HTML output");
        assert!(output.contains("class=\"relationships\""));
        assert_eq!(
            output
                .matches("href=\"../adr/0002-target-record.html\"")
                .count(),
            2
        );
        assert!(output.contains("Target record"));
        assert!(output.contains("RFC-9999"));
        assert!(!output.contains("href=\"../rfc-9999"));
        assert!(output.contains("<code>ADR-0002</code>"));
        assert!(output.contains("<pre class=\"text\"><code>ADR-0002"));
    }

    #[test]
    fn reports_non_zero_exit_and_stderr() {
        let fixture = fixture();
        let error = render(
            &fixture.command("printf failed >&2; exit 1"),
            &fixture.job(),
        )
        .expect_err("renderer failure");
        assert!(error.to_string().contains("exit code 1"));
        assert!(error.to_string().contains("failed"));
    }

    #[test]
    fn reports_missing_output_after_success() {
        let fixture = fixture();
        let error = render(&fixture.command(":"), &fixture.job()).expect_err("missing output");
        assert!(error.to_string().contains("did not produce output"));
    }

    #[test]
    fn reports_command_name_when_renderer_cannot_start() {
        let fixture = fixture();
        let error = render("strata-renderer-does-not-exist {input}", &fixture.job())
            .expect_err("missing renderer");
        assert!(error.to_string().contains("strata-renderer-does-not-exist"));
    }

    #[test]
    fn leaves_unknown_tokens_and_allows_missing_placeholders() {
        let fixture = fixture();
        let command = fixture.command("printf '%s' '{unknown}' > \"$1\"");
        let contents = render(&command, &fixture.job()).expect("render");
        assert_eq!(contents, b"{unknown}");
    }
}
