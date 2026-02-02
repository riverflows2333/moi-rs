use pyo3::prelude::*;
use moi_core::*;
use moi_solver_api::*;

#[pyclass]
pub struct Var {
    name: String,
    id: VarId,
}
#[pyclass]
pub struct Vars {
    name: String,
    shape: Vec<usize>,
    ids: Vec<VarId>,
}

#[pymethods]
impl Var {

}