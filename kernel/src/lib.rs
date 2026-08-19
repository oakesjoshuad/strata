use rusqlite::types::Value;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct ParseValueError(pub String);

impl fmt::Display for ParseValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ParseValueError {}

#[macro_export]
macro_rules! sql_enum {
    (
        $(#[$meta:meta])*
        case_insensitive: $case_insensitive:literal;
        $vis:vis enum $name:ident {
            $(
                $variant:ident => $display:literal $(| $input:literal)*
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $($variant),+
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.write_str(match self {
                    $(Self::$variant => $display),+
                })
            }
        }

        impl ::std::str::FromStr for $name {
            type Err = String;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                let normalized = if $case_insensitive {
                    value.to_ascii_lowercase()
                } else {
                    value.to_string()
                };
                match normalized.as_str() {
                    $($display $(| $input)* => Ok(Self::$variant),)+
                    _ => Err(format!("unknown {}: {value}", stringify!($name))),
                }
            }
        }

        impl ::rusqlite::types::ToSql for $name {
            fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                Ok(::rusqlite::types::ToSqlOutput::Owned(
                    ::rusqlite::types::Value::Text(self.to_string()),
                ))
            }
        }

        impl ::rusqlite::types::FromSql for $name {
            fn column_result(
                value: ::rusqlite::types::ValueRef<'_>,
            ) -> Result<Self, ::rusqlite::types::FromSqlError> {
                Self::from_str(value.as_str()?).map_err(|error| {
                    ::rusqlite::types::FromSqlError::Other(Box::new(
                        $crate::ParseValueError(error),
                    ))
                })
            }
        }
    };
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WhereBuilder {
    clauses: Vec<String>,
    bindings: Vec<(String, Value)>,
}

impl WhereBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, clause: impl Into<String>, name: impl Into<String>, value: Value) {
        self.clauses.push(clause.into());
        self.push_binding(name, value);
    }

    pub fn push_binding(&mut self, name: impl Into<String>, value: Value) {
        self.bindings.push((name.into(), value));
    }

    pub fn sql(&self) -> String {
        self.clauses.join(" AND ")
    }

    pub fn bindings(&self) -> &[(String, Value)] {
        &self.bindings
    }
}

pub mod build {
    use std::process::Command;

    pub fn git_build_id() -> String {
        let commit = match Command::new("git")
            .args(["rev-parse", "--short=12", "HEAD"])
            .output()
        {
            Ok(output) if output.status.success() => {
                let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if value.is_empty() {
                    "unknown".to_string()
                } else {
                    value
                }
            }
            _ => "unknown".to_string(),
        };
        let dirty = match Command::new("git").args(["status", "--porcelain"]).output() {
            Ok(output) => output.status.success() && !output.stdout.is_empty(),
            Err(_) => false,
        };
        if dirty {
            format!("{commit}-dirty")
        } else {
            commit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn where_builder_keeps_typed_bindings() {
        let mut builder = WhereBuilder::new();
        builder.push("name = :name", ":name", Value::Text("Ada".into()));
        builder.push_binding(":limit", Value::Integer(10));
        assert_eq!(builder.sql(), "name = :name");
        assert_eq!(builder.bindings()[1].1, Value::Integer(10));
    }
}
