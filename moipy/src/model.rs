use pyo3::prelude::*;
use moi_core::*;
use moi_solver_api::*;
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
}