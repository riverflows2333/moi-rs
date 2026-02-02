use pyo3::prelude::*;

use moi_core::*;
use moi_solver_api::*;

#[pyclass]
pub struct LinExpr {
    expr: ScalarAffineFn
}

#[pymethods]
impl LinExpr {
    
}