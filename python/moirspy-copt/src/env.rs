use crate::loader::resolve_library;
use moi_core::AttrValue;
use moi_solver_copt::{CoptApi, CoptEnv, CoptEnvConfig, SharedCoptEnv};
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

/// Python owner for a reusable native COPT environment.
#[pyclass(unsendable)]
pub struct Env {
    pub(crate) inner: SharedCoptEnv,
}

/// Python wrapper matching coptpy's client configuration object.
#[pyclass(name = "EnvrConfig", unsendable)]
pub struct EnvConfig {
    pub(crate) inner: CoptEnvConfig,
}

/// COPT client configuration names exposed by COPT 8.0.6.
#[pyclass(name = "COPT")]
pub struct CoptConstants;

#[pymethods]
impl CoptConstants {
    #[classattr]
    const CLIENT_CAFILE: &'static str = "CaFile";
    #[classattr]
    const CLIENT_CERTFILE: &'static str = "CertFile";
    #[classattr]
    const CLIENT_CERTKEYFILE: &'static str = "CertKeyFile";
    #[classattr]
    const CLIENT_CLUSTER: &'static str = "Cluster";
    #[classattr]
    const CLIENT_FLOATING: &'static str = "Floating";
    #[classattr]
    const CLIENT_PASSWORD: &'static str = "PassWord";
    #[classattr]
    const CLIENT_PORT: &'static str = "Port";
    #[classattr]
    const CLIENT_PRIORITY: &'static str = "Priority";
    #[classattr]
    const CLIENT_WAITTIME: &'static str = "WaitTime";
    #[classattr]
    const CLIENT_WEBSERVER: &'static str = "WebServer";
    #[classattr]
    const CLIENT_WEBLICENSEID: &'static str = "WebLicenseId";
    #[classattr]
    const CLIENT_WEBACCESSKEY: &'static str = "WebAccessKey";
    #[classattr]
    const CLIENT_WEBTOKENDURATION: &'static str = "WebTokenDuration";
}

#[pymethods]
impl EnvConfig {
    #[new]
    #[pyo3(signature = (dll_path=None))]
    fn new(dll_path: Option<String>) -> PyResult<Self> {
        let api = load_api(dll_path)?;
        let inner = CoptEnvConfig::new(api).map_err(to_py_runtime_error)?;
        Ok(Self { inner })
    }

    /// Set a client configuration value before creating an environment.
    fn set(&mut self, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let value = config_value_from_py(value)?;
        self.inner.set(name, &value).map_err(to_py_runtime_error)
    }
}

#[pymethods]
impl Env {
    #[new]
    #[pyo3(signature = (dll_path=None, license_dir=None, config=None))]
    fn new(
        dll_path: Option<String>,
        license_dir: Option<String>,
        config: Option<PyRef<'_, EnvConfig>>,
    ) -> PyResult<Self> {
        let env = if let Some(config) = config {
            if dll_path.is_some() || license_dir.is_some() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "config cannot be provided together with dll_path or license_dir",
                ));
            }
            CoptEnv::with_config(&config.inner)
        } else {
            let api = load_api(dll_path)?;
            match license_dir {
                Some(path) => CoptEnv::with_license_dir(api, &path),
                None => CoptEnv::new(api),
            }
        }
        .map_err(to_py_runtime_error)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(env)),
        })
    }
}

fn config_value_from_py(value: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(value) = value.extract::<String>() {
        Ok(value)
    } else if let Ok(value) = value.extract::<bool>() {
        Ok(if value { "1" } else { "0" }.to_owned())
    } else if let Ok(value) = value.extract::<i64>() {
        Ok(value.to_string())
    } else if let Ok(value) = value.extract::<f64>() {
        Ok(value.to_string())
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
            "COPT environment configuration value must be str, bool, int, or float",
        ))
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
