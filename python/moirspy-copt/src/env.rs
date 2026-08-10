use pyo3::prelude::*;

/// Python owner for a native COPT environment.
#[pyclass(unsendable)]
pub struct Env;

#[pymethods]
impl Env {
    #[new]
    #[pyo3(signature = (dll_path=None, license_dir=None))]
    fn new(dll_path: Option<String>, license_dir: Option<String>) -> PyResult<Self> {
        let _ = (dll_path, license_dir);
        Err(pyo3::exceptions::PyNotImplementedError::new_err(
            "COPT bindings have not been generated yet",
        ))
    }
}
