use crate::{Store, StoreError};
use rusqlite::types::ValueRef;
use std::fmt::Write;

const TABLES: [Table; 5] = [
    Table {
        name: "engineering_record",
        order_by: "id",
    },
    Table {
        name: "record_relation",
        order_by: "source_id, relation, target_id",
    },
    Table {
        name: "record_revision",
        order_by: "record_id, revision",
    },
    Table {
        name: "evidence",
        order_by: "id",
    },
    Table {
        name: "code_reference",
        order_by: "record_id, relation, path, symbol, line_start, line_end",
    },
];

struct Table {
    name: &'static str,
    order_by: &'static str,
}

impl Store {
    pub fn dump(&self) -> Result<String, StoreError> {
        let mut output = String::from("-- strata native SQL dump v1\n\n");
        output.push_str("-- schema\n");
        self.append_schema(&mut output)?;
        output.push_str("\n-- data\n");
        for table in TABLES {
            self.append_table(&mut output, table)?;
        }
        Ok(output)
    }

    fn append_schema(&self, output: &mut String) -> Result<(), StoreError> {
        let mut statement = self.conn.prepare(
            "SELECT name, sql FROM sqlite_master WHERE type = 'table' AND name IN (:table1, :table2, :table3, :table4, :table5) ORDER BY CASE name WHEN :table1 THEN 1 WHEN :table2 THEN 2 WHEN :table3 THEN 3 WHEN :table4 THEN 4 WHEN :table5 THEN 5 END",
        )?;
        let rows = statement.query_map(
            rusqlite::named_params! {
                ":table1": TABLES[0].name,
                ":table2": TABLES[1].name,
                ":table3": TABLES[2].name,
                ":table4": TABLES[3].name,
                ":table5": TABLES[4].name,
            },
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )?;
        let definitions = rows.collect::<Result<Vec<_>, _>>()?;
        if definitions.len() != TABLES.len() {
            return Err(StoreError::Argument(
                "database is missing a dump table definition".into(),
            ));
        }
        for (_, definition) in definitions {
            output.push_str(definition.trim_end());
            output.push_str(";\n\n");
        }
        Ok(())
    }

    fn append_table(&self, output: &mut String, table: Table) -> Result<(), StoreError> {
        let sql = format!("SELECT * FROM {} ORDER BY {}", table.name, table.order_by);
        let mut statement = self.conn.prepare(&sql)?;
        let column_count = statement.column_count();
        let columns = (0..column_count)
            .map(|index| statement.column_name(index).map(str::to_owned))
            .collect::<Result<Vec<_>, _>>()?;
        let rows = statement.query_map(rusqlite::named_params! {}, |row| {
            let values = (0..column_count)
                .map(|index| format_value(row.get_ref(index)?))
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(values)
        })?;
        for row in rows {
            let values = row?;
            writeln!(
                output,
                "INSERT INTO {} ({}) VALUES ({});",
                table.name,
                columns.join(", "),
                values.join(", ")
            )
            .map_err(|error| StoreError::Argument(error.to_string()))?;
        }
        output.push('\n');
        Ok(())
    }
}

fn format_value(value: ValueRef<'_>) -> rusqlite::Result<String> {
    Ok(match value {
        ValueRef::Null => "NULL".into(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(value) => {
            format!("'{}'", String::from_utf8_lossy(value).replace('\'', "''"))
        }
        ValueRef::Blob(value) => {
            let mut hex = String::with_capacity(value.len() * 2);
            for byte in value {
                write!(hex, "{byte:02X}")
                    .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            }
            format!("X'{hex}'")
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::RecordKind;
    use serde_json::json;

    #[test]
    fn dump_is_deterministic_and_includes_domain_rows() {
        let mut store = Store::open_memory().expect("store");
        let first = store
            .create(
                RecordKind::Adr,
                "Quote ' and newline\nrecord",
                json!({
                    "schema": "adr/v1",
                    "context": "context with ' quote",
                    "decision": "keep newline\ninside JSON",
                    "alternatives": "alternative",
                    "consequences": "consequence",
                    "evidence": "evidence"
                }),
            )
            .expect("first record");
        let second = store
            .create(
                RecordKind::Edr,
                "Second",
                RecordKind::Edr.default_document("Second"),
            )
            .expect("second record");
        store
            .revise(
                &first.id,
                RecordKind::Adr.default_document("revised"),
                Some("revision"),
            )
            .expect("revision");
        store
            .link(&first.id, "constrains", &second.id)
            .expect("relationship");
        store
            .add_evidence(
                &first.id,
                "benchmark",
                "Evidence",
                Some("https://example.com"),
                Some("content"),
                Some(&json!({"quoted": "'"})),
            )
            .expect("evidence");
        store
            .add_code_reference(
                &first.id,
                "constrains",
                "src/lib.rs",
                Some("main"),
                Some(1),
                Some(2),
            )
            .expect("code reference");

        let first_dump = store.dump().expect("dump");
        let second_dump = store.dump().expect("second dump");
        assert_eq!(first_dump, second_dump);
        assert!(first_dump.contains("INSERT INTO engineering_record"));
        assert!(first_dump.contains("''"));
        assert!(!first_dump.contains("engineering_record_fts"));
        assert!(!first_dump.contains("refinery_schema_history"));
    }
}
