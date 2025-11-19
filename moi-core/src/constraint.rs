use crate::functions::FunctionType;
use crate::sets::ScalarSetType;

#[derive(Debug, Clone)]
pub struct ConstraintType {
    pub id: usize,
    pub func: FunctionType,
    pub set: ScalarSetType,
}

impl ConstraintType {
    pub fn new(id: usize, func: FunctionType, set: ScalarSetType) -> Self {
        Self { id, func, set }
    }
}


