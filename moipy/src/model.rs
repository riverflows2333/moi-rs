use crate::moi::*;
use crate::utils::*;
use crate::var::*;
use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyTuple};
use std::collections::HashMap;
#[pyclass]
pub struct Model {
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
            backend: None,
        }
    }
    fn _set_backend(&mut self, backend: Py<PyAny>) {
        self.backend = Some(backend);
    }
    #[pyo3(signature = (lb=0., ub=std::f64::INFINITY, obj=0.0, vtype=None, name=""),name="addVar")]
    fn add_var(
        &mut self,
        lb: f64,
        ub: f64,
        obj: f64,
        vtype: Option<VarType>,
        name: &str,
    ) -> PyResult<Var> {
        let var_id = self.model.add_variable(
            Some(&name),
            vtype.map(|t| match t {
                VarType::CONTINUOUS => 'C',
                VarType::BINARY => 'B',
                VarType::INTEGER => 'I',
            }),
            Some(lb),
            Some(ub),
        );
        Ok(Var::new(var_id.0))
    }
    #[pyo3(signature = (*indices, lb=None, ub=None, obj=None, vtype=None, name=None),name="addVars")]
    fn add_vars<'py>(
        &mut self,
        indices: &Bound<'py, PyTuple>,
        lb: Option<&Bound<'py, PyAny>>,
        ub: Option<&Bound<'py, PyAny>>,
        obj: Option<&Bound<'py, PyAny>>,
        vtype: Option<&Bound<'py, PyAny>>,
        name: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        todo!()
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("Model(name={})", self.name))
    }
}
