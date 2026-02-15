use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString, PyTuple};
#[derive(Clone, Debug)]
pub enum Param<T> {
    Scalar(T),
    Vector(Vec<T>),
}

impl<T: Clone> Param<T> {
    pub fn to_vec(&self, n: Option<usize>) -> Vec<T> {
        match self {
            Param::Scalar(val) => vec![val.clone(); n.unwrap_or(1)],
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

pub fn num2index(num: usize, shape: &[usize]) -> Vec<usize> {
    let mut index = vec![0; shape.len()];
    let mut remainder = num;
    for i in (0..shape.len()).rev() {
        index[i] = remainder % shape[i];
        remainder /= shape[i];
    }
    index
}

mod tests {
    use super::*;
    #[test]
    fn test_num2index() {
        let shape = vec![2, 3, 4];
        let indices = num2index(21, &shape);
        assert_eq!(indices, vec![1, 2, 1]);
        let s = format!(
            "a[{}]",
            indices
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );
        assert_eq!(s, "a[1,2,1]");
    }
}
