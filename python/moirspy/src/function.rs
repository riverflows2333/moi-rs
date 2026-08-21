use crate::expr::LinExpr;
use crate::utils::ensure_finite;
use crate::var::Var;
use moi_core::*;
use pyo3::prelude::*;

#[pyfunction]
pub fn quicksum(generator: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
    let mut f = ScalarAffineFn::new();
    for item in generator.try_iter()? {
        let item = item?;
        if let Ok(expr) = item.extract::<PyRef<'_, LinExpr>>() {
            f.add_assign(expr.get_fn_ref());
        } else if let Ok(var) = item.extract::<PyRef<'_, Var>>() {
            f.push_term(var.get_id(), 1.0);
        } else if let Ok(value) = item.extract::<f64>() {
            f.constant += ensure_finite(value, "quicksum item")?;
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item is not a LinExpr, Var, or f64",
            ));
        }
    }
    f.simplify();
    Ok(LinExpr::new(f))
}

#[pyfunction]
pub fn dot(coefficients: &Bound<'_, PyAny>, variables: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
    let mut coefficients = coefficients.try_iter()?;
    let mut variables = variables.try_iter()?;
    let mut f = ScalarAffineFn::new();

    loop {
        match (coefficients.next(), variables.next()) {
            (Some(coefficient), Some(variable)) => {
                let coefficient = coefficient?.extract::<f64>().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                        "dot coefficients must contain only numbers",
                    )
                })?;
                let coefficient = ensure_finite(coefficient, "dot coefficient")?;
                let variable = variable?.extract::<PyRef<'_, Var>>().map_err(|_| {
                    PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                        "dot variables must contain only Var objects",
                    )
                })?;
                f.push_term(variable.get_id(), coefficient);
            }
            (None, None) => break,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "dot coefficients and variables must have the same length",
                ));
            }
        }
    }

    f.simplify();
    Ok(LinExpr::new(f))
}
