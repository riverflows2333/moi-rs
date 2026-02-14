use pyo3::prelude::*;
use moi_core::*;
use moi_solver_api::*;
use moi_bridge::BridgeOptimizer;
use std::collections::HashMap;
use crate::moi::*;
#[pyclass]
struct Model {
    name: String,
    model: BridgeOptimizer,
    backend: Option<Py<PyAny>>,
}

#[pymethods]
impl Model {
    #[new]
    fn new(name: String) -> Self {
        Model {
            name,
            model: BridgeOptimizer::new(),
            backend: None
        }
    }
    fn _set_backend(&mut self, backend: Py<PyAny>) {
        self.backend = Some(backend);
    }
    #[pyo3(signature = (lb=0., ub=std::f64::INFINITY, obj=0.0, vtype=None, name="".to_string()))]
    fn add_var(&self,lb: f64, ub: f64, obj: f64, vtype:Option<VarType>, name: String, ) -> PyResult<()> {
        todo!()
    }
    fn add_vars(&self, names: Vec<String>, lbs: Vec<f64>, ubs: Vec<f64>) -> PyResult<()> {
        todo!()
    }

}