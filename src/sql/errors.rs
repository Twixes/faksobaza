use serde::{ser::SerializeMap, Serialize, Serializer};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
#[error("SyntaxError: {0}")]
pub struct SyntaxError(pub String);

impl Serialize for SyntaxError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", "syntax")?;
        map.serialize_entry("message", &self.0)?;
        map.end()
    }
}

#[derive(Error, Debug, PartialEq)]
#[error("StatementValidationError: {0}")]
pub struct StatementValidationError(pub String);

impl Serialize for StatementValidationError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("type", "statement_validation")?;
        map.serialize_entry("message", &self.0)?;
        map.end()
    }
}
