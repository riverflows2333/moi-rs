use pyo3::prelude::*;
use moi_core::*;
use moi_solver_api::*;
use crate::var::Var;
#[pyclass]
#[derive(Clone,Debug)]
pub struct LinExpr {
    f: ScalarAffineFn
}

impl LinExpr {
    pub fn new(f: ScalarAffineFn) -> Self {
        LinExpr { f }
    }
    pub fn get_fn(&self) -> &ScalarAffineFn {
        &self.f
    }
}

#[pymethods]
impl LinExpr {
    pub fn __add__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = self.f.clone();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += value;
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.get_id(), 1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            for term in expr.get_fn().terms.iter() {
                afn.push_term(term.var, term.coeff);
            }
            afn.constant += expr.get_fn().constant;
        } else {
            panic!("Unsupported type for addition with LinExpr");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __sub__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = self.f.clone();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= value;
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.get_id(), -1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            for term in expr.get_fn().terms.iter() {
                afn.push_term(term.var, -term.coeff);
            }
            afn.constant -= expr.get_fn().constant;
        } else {
            panic!("Unsupported type for subtraction with LinExpr");
        }
        afn.simplify();
        LinExpr::new(afn)
    }
}