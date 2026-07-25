use binary_reader::BinaryReader;

use super::{
    binary_reader::{AutoReadable, BinaryReadable},
    error::ParserError,
};

pub struct TriggerCategory {
    pub id: i32,
    pub name: String,
    pub is_comment: Option<i32>,
}

pub struct Variable {
    pub name: String,
    pub variable_type: String,
    pub u1: i32,
    pub is_array: i32,
    pub array_size: Option<i32>,
    pub is_initialized: i32,
    pub initial_value: String,
}

pub struct War3MapWtg {
    pub version: i32,
    pub categories: Vec<TriggerCategory>,
    pub u1: i32,
    pub variables: Vec<Variable>,
    pub triggers: Vec<String>,
}

impl BinaryReadable for TriggerCategory {
    fn load(stream: &mut BinaryReader, version: u32) -> Result<Self, ParserError> {
        Ok(TriggerCategory {
            id: AutoReadable::read(stream)?,
            name: AutoReadable::read(stream)?,
            is_comment: if version == 7 {
                Some(AutoReadable::read(stream)?)
            } else {
                None
            },
        })
    }
}

impl BinaryReadable for Variable {
    fn load(stream: &mut BinaryReader, version: u32) -> Result<Self, ParserError> {
        Ok(Variable {
            name: AutoReadable::read(stream)?,
            variable_type: AutoReadable::read(stream)?,
            u1: AutoReadable::read(stream)?,
            is_array: AutoReadable::read(stream)?,
            array_size: if version == 7 {
                Some(AutoReadable::read(stream)?)
            } else {
                None
            },
            is_initialized: AutoReadable::read(stream)?,
            initial_value: AutoReadable::read(stream)?,
        })
    }
}
