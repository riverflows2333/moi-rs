use super::affine::ScalarAffineFn;
use super::super::*;
#[derive(Clone, Debug)]
pub enum ScalarFunctionType {
    Affine(ScalarAffineFn),
    Variable(VarId),
}

impl ScalarFunctionType {
    pub fn output_dim(&self) -> usize {
        match self {
            ScalarFunctionType::Affine(_f) => 1,
            ScalarFunctionType::Variable(_v) => 1,
        }
    }
}
