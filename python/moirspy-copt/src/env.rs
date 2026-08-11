use crate::loader::resolve_library;
use moi_core::AttrValue;
use moi_solver_copt::{CoptApi, CoptEnv, SharedCoptEnv};
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

/// Python owner for a reusable native COPT environment.
#[pyclass(unsendable)]
pub struct Env {
    pub(crate) inner: SharedCoptEnv,
}

#[pymethods]
impl Env {
    #[new]
    #[pyo3(signature = (dll_path=None, license_dir=None))]
    fn new(dll_path: Option<String>, license_dir: Option<String>) -> PyResult<Self> {
        let api = load_api(dll_path)?;
        let env = match license_dir {
            Some(path) => CoptEnv::with_license_dir(api, &path),
            None => CoptEnv::new(api),
        }
        .map_err(to_py_runtime_error)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(env)),
        })
    }
}

pub(crate) fn load_api(dll_path: Option<String>) -> PyResult<Arc<CoptApi>> {
    let path = resolve_library(dll_path)
        .map_err(|error| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string()))?;
    CoptApi::new(path).map(Arc::new).map_err(|error| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
            "failed to load COPT library: {error}"
        ))
    })
}

pub(crate) fn attr_value_from_py(value: &Bound<'_, PyAny>) -> PyResult<AttrValue> {
    if let Ok(value) = value.extract::<bool>() {
        Ok(AttrValue::Bool(value))
    } else if let Ok(value) = value.extract::<i64>() {
        Ok(AttrValue::Int(value))
    } else if let Ok(value) = value.extract::<f64>() {
        Ok(AttrValue::Float(value))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "COPT parameter value must be bool, int, or float",
        ))
    }
}

pub(crate) fn to_py_runtime_error(error: moi_core::MoiError) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(error.to_string())
}
