use crate::{build_version, config, CliError};
use records::RecordKind;
use records::{CodeReference, Evidence};
use serde::Serialize;
use serde_json::json;
use std::fmt::Debug;
use store::Graph;

pub(crate) fn print_dump_result(
    result: crate::dump::DumpResult,
    path: &std::path::Path,
) -> Result<(), CliError> {
    match result {
        crate::dump::DumpResult::Checked => println!("dump check clean"),
        crate::dump::DumpResult::Written(bytes) => {
            println!("dumped {} bytes to {}", bytes, path.display())
        }
    }
    Ok(())
}

pub(crate) fn print_restored(path: &std::path::Path) {
    println!("restored {}", path.display());
}

pub(crate) fn emit_json<T: Serialize>(value: T, json_flag: bool) -> Result<(), CliError> {
    if json_flag {
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        println!("{}", serde_json::to_string(&value)?);
    }
    Ok(())
}

pub(crate) fn print_value<T: Serialize + Debug>(value: T, json_flag: bool) -> Result<(), CliError> {
    if json_flag {
        emit_json(value, true)?;
    } else {
        println!("{:?}", value);
    }
    Ok(())
}

pub(crate) fn print_graph(graph: Graph, json_flag: bool) -> Result<(), CliError> {
    if json_flag {
        emit_json(graph, true)?;
    } else {
        println!(
            "{} [{}] {}",
            graph.record.id, graph.record.status, graph.record.title
        );
        for relation in graph.relationships {
            println!(
                "  {} {} {}",
                relation.source_id, relation.relation, relation.target_id
            );
        }
        for evidence in graph.evidence {
            println!("  evidence [{}] {}", evidence.kind, evidence.title);
        }
        for code_ref in graph.code_references {
            println!("  {} {}", code_ref.relation, code_ref.path);
        }
    }
    Ok(())
}

pub(crate) fn print_evidence(evidence: Evidence, json_flag: bool) -> Result<(), CliError> {
    if json_flag {
        emit_json(evidence, true)?;
    } else {
        println!("{} [{}] {}", evidence.id, evidence.kind, evidence.title);
    }
    Ok(())
}

pub(crate) fn print_code_reference(
    code_reference: CodeReference,
    json_flag: bool,
) -> Result<(), CliError> {
    if json_flag {
        emit_json(code_reference, true)?;
    } else {
        match code_reference.symbol {
            Some(symbol) => println!(
                "{} {}::{symbol}",
                code_reference.relation, code_reference.path
            ),
            None => println!("{} {}", code_reference.relation, code_reference.path),
        }
    }
    Ok(())
}

pub(crate) fn output_schema(kind: RecordKind, json_flag: bool) -> Result<(), CliError> {
    let value = json!({"kind": kind, "purpose": kind.purpose(), "schema": format!("{}/v1", kind.slug()), "required_fields": kind.required_fields(), "statuses": kind.statuses()});
    emit_json(value, json_flag)
}

pub(crate) fn output_capabilities(
    json_flag: bool,
    resolved: &config::ResolvedConfig,
) -> Result<(), CliError> {
    let kinds = RecordKind::ALL.map(|kind| json!({"kind": kind, "purpose": kind.purpose()}));
    let configuration = json!({
        "database": {"value": resolved.database.value, "source": resolved.database.source.as_str()},
        "export_target": {"value": resolved.export_target.value, "source": resolved.export_target.source.as_str()},
        "dump_target": {"value": resolved.dump_target.value, "source": resolved.dump_target.source.as_str()},
    });
    let value = json!({"version": build_version(), "kinds": kinds, "commands": ["init", "new", "show", "search", "graph", "render", "export", "dump", "restore", "template", "history", "link", "evidence-add", "code-ref-add", "status", "retitle", "revise", "validate", "schema", "capabilities", "relationships"], "json_output": ["new", "show", "search", "graph", "evidence-add", "code-ref-add", "status", "retitle", "revise", "validate", "schema", "capabilities", "relationships"], "configuration": configuration});
    emit_json(value, json_flag)
}

pub(crate) fn output_relationships(json_flag: bool) -> Result<(), CliError> {
    let value =
        json!({"relationships": records::RELATIONSHIPS, "evidence_kinds": records::EVIDENCE_KINDS});
    emit_json(value, json_flag)
}
