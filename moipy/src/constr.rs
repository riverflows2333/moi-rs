use moi_core::*;
use pyo3::prelude::*;

#[pyclass]
pub struct Constr {
    f: ScalarFunctionType,
    s: ScalarSetType,
}

impl Constr {
    pub fn new(f: ScalarFunctionType, s: ScalarSetType) -> Self {
        Constr { f, s }
    }
}
