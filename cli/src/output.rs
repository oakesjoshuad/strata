use crate::{build_version, CliError};
use records::RecordKind;
use serde::Serialize;
use serde_json::json;
use std::fmt::Debug;
use store::Graph;

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

pub(crate) fn output_schema(kind: RecordKind, json_flag: bool) -> Result<(), CliError> {
    let value = json!({"kind": kind, "schema": format!("{}/v1", kind.slug()), "required_fields": kind.required_fields(), "statuses": kind.statuses()});
    emit_json(value, json_flag)
}

pub(crate) fn output_capabilities(json_flag: bool) -> Result<(), CliError> {
    let value = json!({"version": build_version(), "kinds": RecordKind::ALL, "commands": ["init", "new", "show", "search", "graph", "render", "export", "template", "history", "link", "status", "retitle", "revise", "validate", "schema", "capabilities", "relationships"], "json_output": ["new", "show", "search", "graph", "status", "retitle", "revise", "validate", "schema", "capabilities", "relationships"]});
    emit_json(value, json_flag)
}

pub(crate) fn output_relationships(json_flag: bool) -> Result<(), CliError> {
    let value = json!({"relationships": records::RELATIONSHIPS});
    emit_json(value, json_flag)
}
