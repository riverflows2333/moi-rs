use pyo3::prelude::*;
use moi_core::*;
use moi_solver_api::*;
use moi_bridge::BridgeOptimizer;
use std::collections::HashMap;
#[pyclass]
struct Model {
    backend: Option<Py<PyAny>>,
}

#[pymethods]
impl Model {
    #[new]
    fn new(name: String) -> Self {
        Model {
            backend: None
        }
    }
    fn _set_backend(&mut self, backend: Py<PyAny>) {
        self.backend = Some(backend);
    }

    fn add_var(&self, name: String, lb: f64, ub: f64) -> PyResult<()> {
        todo!()
    }
    fn add_vars(&self, names: Vec<String>, lbs: Vec<f64>, ubs: Vec<f64>) -> PyResult<()> {
        todo!()
    }

}