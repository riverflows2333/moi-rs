use super::affine::ScalarAffineFn;
use super::super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug,PartialEq,Serialize, Deserialize)]
pub enum ScalarFunctionType {
    Affine(ScalarAffineFn),
    Variable(VarId),
}

#[derive(Clone, Debug,PartialEq,Serialize,Deserialize)]
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
