use crate::args::Cli;
use clap::CommandFactory;
use std::io::IsTerminal;

const BOLD: &str = "\x1b[1m";
const UNDERLINE: &str = "\x1b[4m";
const RESET: &str = "\x1b[0m";

/// Styling is opt-out: disabled when stdout isn't a terminal (piped, redirected,
/// captured in a test) or when NO_COLOR is set, per https://no-color.org.
fn styling_enabled() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn heading(text: &str) -> String {
    if styling_enabled() {
        format!("{BOLD}{UNDERLINE}{text}{RESET}")
    } else {
        text.to_string()
    }
}

fn strong(text: &str) -> String {
    if styling_enabled() {
        format!("{BOLD}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Concept groups for the top-level `strata --help` overview, in display order.
/// Descriptions themselves come from each Command variant's doc comment
/// (args.rs) via clap's introspection, so this table only owns grouping and
/// group order -- it does not duplicate the descriptions themselves.
const HELP_GROUPS: &[(&str, &[&str])] = &[
    ("Get started", &["init"]),
    (
        "Create and edit records",
        &["new", "revise", "retitle", "status", "template"],
    ),
    (
        "Link records and evidence",
        &["link", "evidence-add", "code-ref-add"],
    ),
    ("Find and inspect", &["show", "search", "graph", "history"]),
    (
        "Publish and verify",
        &["render", "export", "dump", "restore", "validate"],
    ),
    (
        "Self-description (for agents and tooling)",
        &["schema", "capabilities", "relationships"],
    ),
];

/// True when argv (excluding the program name) is exactly a top-level help
/// request, so `strata <command> --help` still goes through clap unchanged.
pub(crate) fn requested(args: &[String]) -> bool {
    matches!(args, [only] if only == "-h" || only == "--help" || only == "help")
}

pub(crate) fn print() {
    let mut command = Cli::command();
    let about = command
        .get_about()
        .map(ToString::to_string)
        .unwrap_or_default();
    let usage = command.render_usage().to_string();
    let usage_label_end = usage.find(' ').unwrap_or(usage.len());
    let (usage_label, usage_rest) = usage.split_at(usage_label_end);
    println!("{about}\n\n{}{usage_rest}\n", heading(usage_label));

    let name_width = HELP_GROUPS
        .iter()
        .flat_map(|(_, names)| names.iter())
        .map(|name| name.len())
        .max()
        .unwrap_or(0);

    for (group_heading, names) in HELP_GROUPS {
        println!("{}:", heading(group_heading));
        for name in *names {
            let about = command
                .find_subcommand(name)
                .and_then(|sub| sub.get_about())
                .map(ToString::to_string)
                .unwrap_or_default();
            let padded = format!("{name:<name_width$}");
            println!("  {}  {about}", strong(&padded));
        }
        println!();
    }

    println!("{}:", heading("Options"));
    println!(
        "      {}  Path to the SQLite database (CLI, STRATA_DATABASE, config file, or default)",
        strong("--database <PATH>")
    );
    println!(
        "  {}, {}             Print help",
        strong("-h"),
        strong("--help")
    );
    println!(
        "  {}, {}          Print version",
        strong("-V"),
        strong("--version")
    );
    println!();
    println!(
        "Run '{} {} --help' for details on a specific command.",
        strong("strata"),
        strong("<command>")
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requested_matches_only_a_bare_top_level_help_request() {
        let cases: &[(&[&str], bool)] = &[
            (&["-h"], true),
            (&["--help"], true),
            (&["help"], true),
            (&[], false),
            (&["new", "--help"], false),
            (&["--help", "new"], false),
            (&["-h", "-h"], false),
        ];
        for (args, expected) in cases {
            let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            assert_eq!(requested(&args), *expected, "args: {args:?}");
        }
    }

    #[test]
    fn every_real_subcommand_is_grouped_exactly_once() {
        let command = Cli::command();
        let real_names: Vec<&str> = command
            .get_subcommands()
            .map(clap::Command::get_name)
            .filter(|name| *name != "help")
            .collect();
        let grouped_names: Vec<&str> = HELP_GROUPS
            .iter()
            .flat_map(|(_, names)| names.iter().copied())
            .collect();

        for name in real_names.iter().copied() {
            let occurrences = grouped_names
                .iter()
                .filter(|grouped| **grouped == name)
                .count();
            assert_eq!(
                occurrences, 1,
                "{name} appears in HELP_GROUPS {occurrences} times, expected exactly 1"
            );
        }
        for name in grouped_names.iter() {
            assert!(
                real_names.contains(name),
                "{name} is listed in HELP_GROUPS but is not a real subcommand"
            );
        }
    }

    #[test]
    fn every_real_subcommand_has_an_about() {
        let command = Cli::command();
        for sub in command
            .get_subcommands()
            .filter(|sub| sub.get_name() != "help")
        {
            assert!(
                sub.get_about().is_some(),
                "{} has no doc comment to use as its --help description",
                sub.get_name()
            );
        }
    }
}
