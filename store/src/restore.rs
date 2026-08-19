use crate::{index_record, Store, StoreError};
use records::RecordId;
use serde_json::Value as JsonValue;

const TABLES: [&str; 5] = [
    "engineering_record",
    "record_relation",
    "record_revision",
    "evidence",
    "code_reference",
];

impl Store {
    pub fn restore(&mut self, dump: &str) -> Result<(), StoreError> {
        self.ensure_empty()?;
        let statements = statements(dump)?;
        let tx = self.conn.transaction()?;
        for statement in statements {
            if statement.starts_with("CREATE TABLE") {
                continue;
            }
            if !statement.starts_with("INSERT INTO ") {
                return Err(StoreError::Argument(format!(
                    "unsupported statement in dump: {}",
                    first_line(&statement)
                )));
            }
            let table = statement["INSERT INTO ".len()..]
                .split_once(' ')
                .map(|(table, _)| table)
                .ok_or_else(|| StoreError::Argument("malformed INSERT in dump".into()))?;
            if !TABLES.contains(&table) {
                return Err(StoreError::Argument(format!(
                    "unsupported INSERT table in dump: {table}"
                )));
            }
            tx.execute(&statement, rusqlite::named_params! {})?;
        }
        rebuild_index(&tx)?;
        tx.commit()?;
        Ok(())
    }

    fn ensure_empty(&self) -> Result<(), StoreError> {
        for table in TABLES {
            let sql = format!("SELECT EXISTS (SELECT 1 FROM {table} LIMIT 1)");
            let populated: bool = self
                .conn
                .query_row(&sql, rusqlite::named_params! {}, |row| row.get(0))?;
            if populated {
                return Err(StoreError::Argument(
                    "restore target database is not empty".into(),
                ));
            }
        }
        Ok(())
    }
}

fn rebuild_index(tx: &rusqlite::Transaction<'_>) -> Result<(), StoreError> {
    let mut statement =
        tx.prepare("SELECT id, title, document FROM engineering_record ORDER BY id")?;
    let records = statement
        .query_map(rusqlite::named_params! {}, |row| {
            let id: RecordId = row.get(0)?;
            let title: String = row.get(1)?;
            let document: JsonValue =
                serde_json::from_str(&row.get::<_, String>(2)?).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })?;
            Ok((id, title, document))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    for (id, title, document) in records {
        index_record(tx, &id, &title, &document)?;
    }
    Ok(())
}

fn statements(dump: &str) -> Result<Vec<String>, StoreError> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut comment = false;
    let mut chars = dump.chars().peekable();
    while let Some(character) = chars.next() {
        if comment {
            if character == '\n' {
                comment = false;
            }
            continue;
        }
        if !quoted && character == '-' && chars.peek() == Some(&'-') {
            chars.next();
            comment = true;
            continue;
        }
        if character == '\'' {
            current.push(character);
            if quoted && chars.peek() == Some(&'\'') {
                current.push(chars.next().ok_or_else(|| {
                    StoreError::Argument("unterminated quoted value in dump".into())
                })?);
            } else {
                quoted = !quoted;
            }
        } else if character == ';' && !quoted {
            let statement = current.trim().to_string();
            if !statement.is_empty() {
                statements.push(statement);
            }
            current.clear();
        } else {
            current.push(character);
        }
    }
    if quoted {
        return Err(StoreError::Argument(
            "unterminated quoted value in dump".into(),
        ));
    }
    if !current.trim().is_empty() {
        return Err(StoreError::Argument(
            "dump ends without a statement terminator".into(),
        ));
    }
    Ok(statements)
}

fn first_line(statement: &str) -> &str {
    match statement.lines().next() {
        Some(line) => line,
        None => statement,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use records::RecordKind;

    #[test]
    fn restore_round_trips_records_and_rebuilds_search_index() {
        let mut source = Store::open_memory().expect("source store");
        let rfc = source
            .create(
                RecordKind::Rfc,
                "Restored RFC",
                RecordKind::Rfc.default_document("Restored RFC"),
            )
            .expect("rfc");
        let pdr = source
            .create(
                RecordKind::Pdr,
                "Restored PDR",
                RecordKind::Pdr.default_document("Restored PDR"),
            )
            .expect("pdr");
        let adr = source
            .create(
                RecordKind::Adr,
                "Restored searchable record",
                RecordKind::Adr.default_document("Restored searchable record"),
            )
            .expect("adr");
        let edr = source
            .create(
                RecordKind::Edr,
                "Restored EDR",
                RecordKind::Edr.default_document("Restored EDR"),
            )
            .expect("edr");
        source
            .revise(
                &adr.id,
                RecordKind::Adr.default_document("revised searchable record"),
                Some("changed"),
            )
            .expect("revision");
        source
            .link(&rfc.id, "explored-by", &pdr.id)
            .expect("rfc relationship");
        source
            .link(&pdr.id, "produces", &adr.id)
            .expect("adr relationship");
        source
            .link(&adr.id, "constrains", &edr.id)
            .expect("edr relationship");
        source
            .add_evidence(&adr.id, "benchmark", "Restored evidence", None, None, None)
            .expect("evidence");
        source
            .add_code_reference(
                &edr.id,
                "implements",
                "store/src/restore.rs",
                Some("Store::restore"),
                Some(1),
                Some(2),
            )
            .expect("code reference");
        let dump = source.dump().expect("dump");

        let mut restored = Store::open_memory().expect("restored store");
        restored.restore(&dump).expect("restore");
        assert_eq!(
            format!("{:?}", restored.all_records().expect("restored records")),
            format!("{:?}", source.all_records().expect("source records"))
        );
        assert_eq!(
            format!("{:?}", restored.history(&adr.id).expect("restored history")),
            format!("{:?}", source.history(&adr.id).expect("source history"))
        );
        for id in [&rfc.id, &pdr.id, &adr.id, &edr.id] {
            assert_eq!(
                format!("{:?}", restored.graph(id).expect("restored graph")),
                format!("{:?}", source.graph(id).expect("source graph"))
            );
        }
        assert_eq!(
            restored.search("searchable", 10, 0).expect("search").len(),
            1
        );
    }

    #[test]
    fn restore_rejects_non_empty_target_without_changing_it() {
        let mut source = Store::open_memory().expect("source store");
        source
            .create(
                RecordKind::Adr,
                "Source",
                RecordKind::Adr.default_document("Source"),
            )
            .expect("source record");
        let dump = source.dump().expect("dump");
        let mut target = Store::open_memory().expect("target store");
        target
            .create(
                RecordKind::Edr,
                "Existing",
                RecordKind::Edr.default_document("Existing"),
            )
            .expect("target record");
        assert!(matches!(
            target.restore(&dump),
            Err(StoreError::Argument(message)) if message == "restore target database is not empty"
        ));
        assert_eq!(target.all_records().expect("records").len(), 1);
    }
}
