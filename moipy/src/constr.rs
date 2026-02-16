use moi_core::*;
use pyo3::prelude::*;

#[pyclass]
#[derive(Clone, Debug)]
pub struct Constr {
    f: ScalarFunctionType,
    s: ScalarSetType,
}

impl Constr {
    pub fn new(f: ScalarFunctionType, s: ScalarSetType) -> Self {
        Constr { f, s }
    }

    pub fn get_f(&self) -> ScalarFunctionType {
        self.f.clone()
    }

    pub fn get_s(&self) -> ScalarSetType {
        self.s.clone()
    }
}
