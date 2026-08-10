use crate::env::Env;
use pyo3::prelude::*;

/// Python adapter for the native COPT optimizer.
#[pyclass(unsendable)]
pub struct Model;

#[pymethods]
impl Model {
    #[new]
    #[pyo3(signature = (name=None, dll_path=None, env=None, license_dir=None))]
    fn new(
        name: Option<String>,
        dll_path: Option<String>,
        env: Option<PyRef<'_, Env>>,
        license_dir: Option<String>,
    ) -> PyResult<Self> {
        let _ = (name, dll_path, env, license_dir);
        Err(pyo3::exceptions::PyNotImplementedError::new_err(
            "COPT bindings have not been generated yet",
        ))
    }
}
