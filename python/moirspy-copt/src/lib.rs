use pyo3::prelude::*;

pub mod env;
pub mod loader;
pub mod model;

#[pymodule]
mod moirspy_copt {
    #[pymodule_export]
    use crate::env::Env;
    #[pymodule_export]
    use crate::model::Model;
}
