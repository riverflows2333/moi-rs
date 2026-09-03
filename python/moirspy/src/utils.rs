use moi_bridge::BridgeOptimizer;
use moi_core::MoiError;
use moi_core::VarId;
use moi_solver_api::Optimizer;
use pyo3::prelude::*;
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex};
pub type SharedBridge = Arc<Mutex<BridgeOptimizer>>;

#[derive(Clone)]
pub enum ResultSource {
    Cached(SharedBridge),
    Direct(crate::direct::DirectModel),
}

impl ResultSource {
    pub fn get_var_value(&self, id: VarId) -> PyResult<Option<f64>> {
        match self {
            Self::Cached(bridge) => lock_bridge(bridge)?
                .get_var_value(id)
                .map_err(to_py_runtime_error),
            Self::Direct(model) => model.get_var_value(id).map_err(to_py_runtime_error),
        }
    }
}

pub fn lock_bridge(bridge: &SharedBridge) -> PyResult<MutexGuard<'_, BridgeOptimizer>> {
    bridge.lock().map_err(|_| {
        PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("model state lock is poisoned")
    })
}

pub fn to_py_runtime_error(error: MoiError) -> PyErr {
    let message = error.to_string();
    match error {
        MoiError::InvalidInput(_)
        | MoiError::InvalidVariableIndex(_)
        | MoiError::InvalidName(_) => PyErr::new::<pyo3::exceptions::PyValueError, _>(message),
        MoiError::UnsupportedConstraint { .. }
        | MoiError::AddConstraintNotAllowed
        | MoiError::UnsupportedAttribute
        | MoiError::SetAttributeNotAllowed
        | MoiError::ScalarFunctionConstantNotZero { .. } => {
            PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(message)
        }
        MoiError::BackendState(_)
        | MoiError::BackendProtocol(_)
        | MoiError::NativeSolver { .. }
        | MoiError::Msg(_) => PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(message),
    }
}

pub fn ensure_finite(value: f64, field: &str) -> PyResult<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "{field} must be finite"
        )))
    }
}

pub fn operand_type_error(operation: &str, expected: &str) -> PyErr {
    PyErr::new::<pyo3::exceptions::PyTypeError, _>(format!(
        "unsupported operand for {operation}; expected {expected}"
    ))
}
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

pub fn generate_names(base: &str, shape: &[usize]) -> Vec<String> {
    let total = shape.iter().product();
    (0..total)
        .map(|i| {
            let indices = num2index(i, shape);
            format!(
                "{}[{}]",
                base,
                indices
                    .iter()
                    .map(|idx| idx.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::num2index;
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
