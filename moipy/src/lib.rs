pub mod env;
pub mod var;
pub mod expr;
pub mod constr;
pub mod model;
pub mod moi;

use pyo3::prelude::*;

/// A Python module implemented in Rust.
#[pymodule]
mod moipy {
    use pyo3::prelude::*;

    /// Formats the sum of two numbers as string.
    #[pyfunction]
    fn sum_as_string(a: usize, b: usize) -> PyResult<String> {
        Ok((a + b).to_string())
    }
}
