use super::super::*;
use super::affine::ScalarAffineFn;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub enum ScalarFunctionType {
    Affine(ScalarAffineFn),
    Variable(VarId),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub enum OperationType {
    Add,
    Sub,
    Mul,
    Div,
}

impl ScalarFunctionType {
    pub fn output_dim(&self) -> usize {
        match self {
            ScalarFunctionType::Affine(_f) => 1,
            ScalarFunctionType::Variable(_v) => 1,
        }
    }
}
