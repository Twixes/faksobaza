use std::{collections::HashSet, str::FromStr};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DataTypeRaw {
    UInt8,
    UInt16,
    UInt32,
    UInt64,
    UInt128,
    Bool,
    Timestamp,
    Uuid,
    String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataType {
    pub raw_type: DataTypeRaw,
    pub is_nullable: bool,
}

impl FromStr for DataTypeRaw {
    type Err = String;

    fn from_str(candidate: &str) -> std::result::Result<Self, Self::Err> {
        match candidate.to_lowercase().as_str() {
            "uint8" => Ok(Self::UInt8),
            "uint16" => Ok(Self::UInt16),
            "uint32" => Ok(Self::UInt32),
            "uint64" => Ok(Self::UInt64),
            "uint128" => Ok(Self::UInt128),
            "bool" => Ok(Self::Bool),
            "timestamp" => Ok(Self::Timestamp),
            "uuid" => Ok(Self::Uuid),
            "string" => Ok(Self::String),
            _ => Err(format!(
                "`{}` does not refer to a supported type",
                candidate
            )),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataInstanceRaw {
    UInt8(u8),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    UInt128(u128),
    Bool(bool),
    Timestamp(i64),
    Uuid(u128),
    String(String),
}

#[derive(Debug, PartialEq, Eq)]
pub enum DataInstance {
    Direct(DataInstanceRaw),
    Nullable(DataInstanceRaw),
    Null,
}

trait Validatable {
    /// Make sure that this definition (self) actually makes sense.
    fn validate(&self) -> Result<(), String>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: DataType,
    pub primary_key: bool,
}

impl Validatable for ColumnDefinition {
    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("A column must have a name".into());
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct TableDefinition {
    // Table name.
    pub name: String,
    // Column definitions.
    pub columns: Vec<ColumnDefinition>,
    // Index of the primary key within column definitions.
    pub primary_key_index: usize,
}

impl TableDefinition {
    pub fn new(name: String, columns: Vec<ColumnDefinition>) -> Self {
        let primary_key_index = columns
            .iter()
            .position(|column| column.primary_key)
            .expect("A table must have a PRIMARY KEY column");
        TableDefinition {
            name,
            columns,
            primary_key_index,
        }
    }
}

impl Validatable for TableDefinition {
    fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("A table must have a name".into());
        }
        if self.columns.is_empty() {
            return Err("A table must have at least one column".into());
        }
        let mut primary_key_count = 0;
        let mut column_names: HashSet<String> = HashSet::new();
        for (column_index, column) in self.columns.iter().enumerate() {
            if column_names.contains(&column.name) {
                return Err(format!(
                    "There is more than one column with name `{}` in table definition",
                    column.name
                ));
            }
            column_names.insert(column.name.clone());
            if column.primary_key {
                primary_key_count += 1;
            }
            if let Err(column_error) = column.validate() {
                return Err(format!(
                    "Problem at column {}: {}",
                    column_index + 1,
                    column_error
                ));
            }
        }
        if primary_key_count != 1 {
            return Err(format!(
                "A table must have exactly 1 PRIMARY KEY column, not {}",
                primary_key_count
            ));
        }
        Ok(())
    }
}