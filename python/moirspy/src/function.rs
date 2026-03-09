use crate::expr::LinExpr;
use crate::var::Var;
use moi_core::*;
use pyo3::prelude::*;

#[pyfunction]
pub fn quicksum(generator: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
    let mut f = LinExpr::default().get_fn();
    for item in generator.try_iter()? {
        let item = item?;
        if let Ok(expr) = item.extract::<LinExpr>() {
            f = f.calculate(&expr.get_fn(), OperationType::Add);
        } else if let Ok(var) = item.extract::<Var>() {
            f.push_term(var.get_id(), 1.0);
        } else if let Ok(value) = item.extract::<f64>() {
            f = f.calculate(&ScalarAffineFn::with_constant(value), OperationType::Add);
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Item is not a LinExpr, Var, or f64",
            ));
        }
    }
    Ok(LinExpr::new(f))
}
