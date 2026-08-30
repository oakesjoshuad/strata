use crate::RecordKind;
use rusqlite::types::{FromSql, FromSqlError, ToSql, ToSqlOutput, Value, ValueRef};
use std::fmt;
use std::str::FromStr;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize)]
pub struct RecordId {
    pub kind: RecordKind,
    pub number: u32,
}

impl RecordId {
    pub fn new(kind: RecordKind, number: u32) -> Self {
        Self { kind, number }
    }
}

impl fmt::Display for RecordId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:04}", self.kind, self.number)
    }
}

impl FromStr for RecordId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (kind, number) = s
            .split_once('-')
            .ok_or_else(|| format!("invalid record id: {s}"))?;
        let kind = RecordKind::from_str(kind)?;
        let number = number
            .parse()
            .map_err(|_| format!("invalid record number: {s}"))?;
        Ok(Self::new(kind, number))
    }
}

impl ToSql for RecordId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(Value::Text(self.to_string())))
    }
}

impl FromSql for RecordId {
    fn column_result(value: ValueRef<'_>) -> Result<Self, FromSqlError> {
        Self::from_str(value.as_str()?)
            .map_err(|e| rusqlite::types::FromSqlError::Other(Box::new(kernel::ParseValueError(e))))
    }
}
