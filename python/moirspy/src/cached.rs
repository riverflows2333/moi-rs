#[cfg(feature = "legacy-python-backend")]
use crate::py_backend::PyBackend;
use crate::utils::{SharedBridge, lock_bridge, to_py_runtime_error};
use moi_bridge::BridgeOptimizer;
use moi_solver_api::Optimizer;
use pyo3::prelude::*;
#[cfg(feature = "legacy-python-backend")]
use pyo3::types::PyAny;
use std::sync::{Arc, Mutex};

/// Existing replayable model storage used by the public API during D1.
pub struct CachedModel {
    bridge: SharedBridge,
    #[cfg(feature = "legacy-python-backend")]
    // PyBackend owns the same object. This second handle preserves the legacy
    // package lifetime contract for one compatibility cycle.
    _backend: Option<Py<PyAny>>,
}

impl CachedModel {
    pub fn new() -> Self {
        Self {
            bridge: Arc::new(Mutex::new(BridgeOptimizer::new())),
            #[cfg(feature = "legacy-python-backend")]
            _backend: None,
        }
    }

    pub fn bridge(&self) -> &SharedBridge {
        &self.bridge
    }

    pub fn ensure_backend_attachable(&self) -> PyResult<()> {
        lock_bridge(&self.bridge)?
            .ensure_backend_attachable()
            .map_err(to_py_runtime_error)
    }

    #[cfg(feature = "legacy-python-backend")]
    pub fn attach_legacy_python_backend(
        &mut self,
        model_instance: Bound<'_, PyAny>,
        keep_cache: bool,
    ) -> PyResult<()> {
        let py_backend = Box::new(PyBackend::new(model_instance.clone().into()));
        let mut bridge = lock_bridge(&self.bridge)?;
        bridge
            .attach_backend(py_backend)
            .map_err(to_py_runtime_error)?;
        if !keep_cache {
            bridge.release_model_cache().map_err(to_py_runtime_error)?;
        }
        drop(bridge);
        self._backend = Some(model_instance.into());
        Ok(())
    }

    pub fn attach_native_backend(
        &mut self,
        backend: Box<dyn Optimizer + Send>,
        keep_cache: bool,
    ) -> PyResult<()> {
        let mut bridge = lock_bridge(&self.bridge)?;
        bridge
            .attach_backend(backend)
            .map_err(to_py_runtime_error)?;
        if !keep_cache {
            bridge.release_model_cache().map_err(to_py_runtime_error)?;
        }
        drop(bridge);
        #[cfg(feature = "legacy-python-backend")]
        {
            self._backend = None;
        }
        Ok(())
    }
}

impl Default for CachedModel {
    fn default() -> Self {
        Self::new()
    }
}
