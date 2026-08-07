use pyo3::prelude::*;
pub mod env;
pub mod loader;
pub mod model;

/// A Python module implemented in Rust.
#[pymodule]
mod moirspy_gurobi {

    /// Formats the sum of two numbers as string.
    #[pymodule_export]
    use crate::env::Env;
    #[pymodule_export]
    use crate::model::Model;
}
