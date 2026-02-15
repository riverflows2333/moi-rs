use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};

pub enum Param<T> {
    Scalar(T),
    Vector(Vec<T>),
}

impl<T: Clone> Param<T> {
    pub fn to_vec(&self) -> Vec<T> {
        match self {
            Param::Scalar(val) => vec![val.clone()],
            Param::Vector(vec) => vec.clone(),
        }
    }

    pub fn from_py<'py>(obj: &Bound<'py, PyAny>) -> PyResult<Self>
    where
        T: for<'a> FromPyObject<'a, 'py>,
    {
        if let Ok(val) = obj.extract::<T>() {
            Ok(Param::Scalar(val))
        } else if let Ok(vec) = obj.extract::<Vec<T>>() {
            Ok(Param::Vector(vec))
        } else {
            Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected a scalar or a list",
            ))
        }
    }
}
