use crate::loader::{EnvLoader, load_gurobi, loader_to_dll_path};
use moi_core::AttrValue;
use moi_solver_gurobi::{GurobiApi, GurobiEnv, SharedGurobiEnv};
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[pyclass(unsendable)]
pub struct Env {
    pub(crate) inner: SharedGurobiEnv,
}

#[pymethods]
impl Env {
    #[new]
    #[pyo3(signature = (dll_path=None, empty=false))]
    fn new(dll_path: Option<String>, empty: bool) -> PyResult<Self> {
        let api = load_api(dll_path)?;
        let env = if empty {
            GurobiEnv::empty(api)
        } else {
            GurobiEnv::new(api)
        }
        .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(env)),
        })
    }

    #[pyo3(name = "setParam")]
    fn set_param(&self, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = attr_value_from_py(value)?;
        self.inner
            .lock()
            .map_err(|_| environment_lock_error())?
            .set_param(name, value)
            .map_err(to_py_error)
    }

    fn start(&self) -> PyResult<()> {
        self.inner
            .lock()
            .map_err(|_| environment_lock_error())?
            .start()
            .map_err(to_py_error)
    }

    #[getter]
    fn started(&self) -> PyResult<bool> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| environment_lock_error())?
            .is_started())
    }
}

pub(crate) fn load_api(dll_path: Option<String>) -> PyResult<Arc<GurobiApi>> {
    let loader = if let Some(path) = dll_path {
        EnvLoader::LibPath(path)
    } else {
        load_gurobi(None).map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?
    };
    let path =
        loader_to_dll_path(&loader).map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;
    GurobiApi::new(PathBuf::from(path))
        .map(Arc::new)
        .map_err(|error| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Failed to load Gurobi library: {error}"
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
    } else if let Ok(value) = value.extract::<String>() {
        Ok(AttrValue::String(value))
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "parameter value must be bool, int, float, or string",
        ))
    }
}

fn environment_lock_error() -> PyErr {
    PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Gurobi environment lock is poisoned")
}

pub(crate) fn to_py_error(error: moi_core::MoiError) -> PyErr {
    let message = error.to_string();
    match error {
        moi_core::MoiError::InvalidInput(_)
        | moi_core::MoiError::InvalidVariableIndex(_)
        | moi_core::MoiError::InvalidName(_) => {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(message)
        }
        moi_core::MoiError::UnsupportedConstraint { .. }
        | moi_core::MoiError::AddConstraintNotAllowed
        | moi_core::MoiError::UnsupportedAttribute
        | moi_core::MoiError::SetAttributeNotAllowed
        | moi_core::MoiError::ScalarFunctionConstantNotZero { .. } => {
            PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(message)
        }
        moi_core::MoiError::BackendState(_)
        | moi_core::MoiError::BackendProtocol(_)
        | moi_core::MoiError::NativeSolver { .. }
        | moi_core::MoiError::Msg(_) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(message),
    }
}
