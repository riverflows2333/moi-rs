use crate::utils::to_py_runtime_error;
use moi_solver_api::Optimizer;
use moi_solver_copt::{
    CoptApi, CoptEnv as NativeCoptEnv, CoptEnvConfig as NativeCoptEnvConfig, CoptOptimizer,
    SharedCoptEnv,
};
use pyo3::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// Reusable COPT environment owned by the main moirspy extension.
#[pyclass(name = "CoptEnv", unsendable)]
pub struct CoptEnv {
    inner: SharedCoptEnv,
}

/// COPT client/OEM configuration owned by the main moirspy extension.
#[pyclass(name = "CoptEnvConfig", unsendable)]
pub struct CoptEnvConfig {
    inner: NativeCoptEnvConfig,
}

/// COPT 8 client configuration names.
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
impl CoptEnvConfig {
    #[new]
    #[pyo3(signature = (dll_path=None))]
    fn new(dll_path: Option<String>) -> PyResult<Self> {
        let inner = NativeCoptEnvConfig::new(load_api(dll_path)?).map_err(to_py_runtime_error)?;
        Ok(Self { inner })
    }

    fn set(&mut self, name: &str, value: &Bound<'_, PyAny>) -> PyResult<()> {
        self.inner
            .set(name, &config_value_from_py(value)?)
            .map_err(to_py_runtime_error)
    }
}

#[pymethods]
impl CoptEnv {
    #[new]
    #[pyo3(signature = (dll_path=None, license_dir=None, config=None))]
    fn new(
        dll_path: Option<String>,
        license_dir: Option<String>,
        config: Option<PyRef<'_, CoptEnvConfig>>,
    ) -> PyResult<Self> {
        let env = if let Some(config) = config {
            if dll_path.is_some() || license_dir.is_some() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "config cannot be provided together with dll_path or license_dir",
                ));
            }
            NativeCoptEnv::with_config(&config.inner)
        } else {
            let api = load_api(dll_path)?;
            match license_dir {
                Some(path) => NativeCoptEnv::with_license_dir(api, &path),
                None => NativeCoptEnv::new(api),
            }
        }
        .map_err(to_py_runtime_error)?;
        Ok(Self {
            inner: Arc::new(Mutex::new(env)),
        })
    }
}

pub fn create_optimizer(env: Option<&CoptEnv>) -> PyResult<Box<dyn Optimizer + Send>> {
    Ok(Box::new(native_optimizer(env)?))
}

pub fn create_direct_optimizer(env: Option<&CoptEnv>) -> PyResult<Box<dyn Optimizer + Send>> {
    let mut optimizer = native_optimizer(env)?;
    optimizer.discard_objective_storage();
    Ok(Box::new(optimizer))
}

fn native_optimizer(env: Option<&CoptEnv>) -> PyResult<CoptOptimizer> {
    let env = match env {
        Some(env) => env.inner.clone(),
        None => Arc::new(Mutex::new(
            NativeCoptEnv::new(load_api(None)?).map_err(to_py_runtime_error)?,
        )),
    };
    CoptOptimizer::new(env).map_err(to_py_runtime_error)
}

fn load_api(dll_path: Option<String>) -> PyResult<Arc<CoptApi>> {
    let path = moi_solver_copt::resolve_library(dll_path.as_deref().map(Path::new))
        .map_err(|error| pyo3::exceptions::PyRuntimeError::new_err(error.to_string()))?;
    CoptApi::new(path).map(Arc::new).map_err(|error| {
        pyo3::exceptions::PyRuntimeError::new_err(format!("failed to load COPT library: {error}"))
    })
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
        Err(pyo3::exceptions::PyTypeError::new_err(
            "COPT environment configuration value must be str, bool, int, or float",
        ))
    }
}
