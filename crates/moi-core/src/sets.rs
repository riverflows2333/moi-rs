// Rework: introduce a unified SetType enum while keeping concrete structs
use serde::{Deserialize, Serialize};
use bincode::{Encode, Decode};
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub enum ScalarSetType {
    GreaterThan(f64),
    LessThan(f64),
    EqualTo(f64),
    Interval(f64, f64),
}

impl ScalarSetType {
    pub fn dimension(&self) -> usize {
        match self {
            ScalarSetType::GreaterThan(_) => 1,
            ScalarSetType::LessThan(_) => 1,
            ScalarSetType::EqualTo(_) => 1,
            ScalarSetType::Interval(_, _) => 1,
        }
    }
}
