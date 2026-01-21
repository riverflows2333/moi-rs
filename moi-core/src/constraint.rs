use crate::functions::ScalarFunctionType;
use crate::indices::ConstrId;
use crate::sets::ScalarSetType;
#[derive(Debug, Clone)]
pub struct ScalarConstraint {
    pub id: ConstrId,
    pub func: ScalarFunctionType,
    pub set: ScalarSetType,
}

impl ScalarConstraint {
    pub fn new(id: usize, func: ScalarFunctionType, set: ScalarSetType) -> Self {
        Self {
            id: ConstrId::new(id),
            func,
            set,
        }
    }
}


