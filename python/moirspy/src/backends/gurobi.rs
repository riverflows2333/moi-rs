use crate::utils::to_py_runtime_error;
use moi_solver_api::Optimizer;
use moi_solver_gurobi::{
    GurobiApi, GurobiEnv as NativeGurobiEnv, GurobiOptimizer, SharedGurobiEnv, find_library,
};
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Reusable Gurobi environment owned by the main moirspy extension.
#[pyclass(name = "GurobiEnv", unsendable)]
pub struct GurobiEnv {
    inner: SharedGurobiEnv,
}

#[pymethods]
impl GurobiEnv {
    #[new]
    #[pyo3(signature = (dll_path=None, empty=false))]
    fn new(dll_path: Option<String>, empty: bool) -> PyResult<Self> {
        let api = load_api(dll_path)?;
        let env = if empty {
            NativeGurobiEnv::empty(api)
        } else {
            NativeGurobiEnv::new(api)
        }
        .map_err(pyo3::exceptions::PyRuntimeError::new_err)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(env)),
        })
    }

    #[pyo3(name = "setParam")]
    fn set_param(&self, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = if let Ok(value) = value.extract::<bool>() {
            moi_core::AttrValue::Bool(value)
        } else if let Ok(value) = value.extract::<i64>() {
            moi_core::AttrValue::Int(value)
        } else if let Ok(value) = value.extract::<f64>() {
            moi_core::AttrValue::Float(value)
        } else if let Ok(value) = value.extract::<String>() {
            moi_core::AttrValue::String(value)
        } else {
            return Err(pyo3::exceptions::PyTypeError::new_err(
                "parameter value must be bool, int, float, or string",
            ));
        };
        self.inner
            .lock()
            .map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("Gurobi environment lock is poisoned")
            })?
            .set_param(name, value)
            .map_err(to_py_runtime_error)
    }

    fn start(&self) -> PyResult<()> {
        self.inner
            .lock()
            .map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("Gurobi environment lock is poisoned")
            })?
            .start()
            .map_err(to_py_runtime_error)
    }

    #[getter]
    fn started(&self) -> PyResult<bool> {
        Ok(self
            .inner
            .lock()
            .map_err(|_| {
                pyo3::exceptions::PyRuntimeError::new_err("Gurobi environment lock is poisoned")
            })?
            .is_started())
    }
}

pub fn create_optimizer(
    env: Option<&GurobiEnv>,
    name: Option<&str>,
) -> PyResult<Box<dyn Optimizer + Send>> {
    Ok(Box::new(native_optimizer(env, name)?))
}

pub fn create_direct_optimizer(
    env: Option<&GurobiEnv>,
    name: Option<&str>,
) -> PyResult<Box<dyn Optimizer + Send>> {
    // GurobiOptimizer already retains only native state and result metadata.
    create_optimizer(env, name)
}

fn native_optimizer(env: Option<&GurobiEnv>, name: Option<&str>) -> PyResult<GurobiOptimizer> {
    let env = match env {
        Some(env) => env.inner.clone(),
        None => Arc::new(Mutex::new(
            NativeGurobiEnv::new(load_api(None)?)
                .map_err(pyo3::exceptions::PyRuntimeError::new_err)?,
        )),
    };
    GurobiOptimizer::new(env, name).map_err(pyo3::exceptions::PyRuntimeError::new_err)
}

fn load_api(dll_path: Option<String>) -> PyResult<Arc<GurobiApi>> {
    let path = match dll_path {
        Some(path) => PathBuf::from(path),
        None => find_library().map(|(path, _)| path).ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err(
                "failed to locate Gurobi native library; set GUROBI_HOME or pass dll_path",
            )
        })?,
    };
    GurobiApi::new(path).map(Arc::new).map_err(|error| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("failed to load Gurobi library: {error}"))
    })
}
