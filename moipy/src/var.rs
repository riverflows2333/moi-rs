use crate::expr::LinExpr;
use crate::constr::Constr;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;
#[pyclass]
#[derive(Clone, Debug)]
pub struct Var {
    id: VarId,
}

#[pyclass]
#[derive(Clone, Debug)]
pub struct Vars {
    shape: Vec<usize>,
    ids: Vec<VarId>,
}

impl Var {
    pub fn get_id(&self) -> VarId {
        self.id
    }
}

#[pymethods]
impl Var {
    #[new]
    fn new(id: usize) -> Self {
        //NOTE: 用于Model当中添加变量方法，一般不会单独实例化变量
        Var { id: VarId(id) }
    }
    fn __add__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += value;
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, 1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            for term in expr.get_fn().terms.iter() {
                afn.push_term(term.var, term.coeff);
            }
        } else {
            panic!("Unsupported type for addition with Var");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __sub__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= value;
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, -1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            for term in expr.get_fn().terms.iter() {
                afn.push_term(term.var, -term.coeff);
            }
            afn.constant -= expr.get_fn().constant;
        } else {
            panic!("Unsupported type for subtraction with Var");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __le__(&self, _other: &Bound<'_, PyAny>) -> Constr {
        let mut afn = ScalarAffineFn::new();
        let s :ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::LessThan(value);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            for term in expr.get_fn().terms.iter() {
                afn.push_term(term.var, -term.coeff);
            }
            afn.constant -= expr.get_fn().constant;
            s = ScalarSetType::LessThan(0.0);
        } else {
            panic!("Unsupported type for comparison with Var");
        }
        afn.simplify();
        let constr_f = ScalarFunctionType::Affine(afn);
        Constr::new(constr_f, s)
    }

    fn __ge__(&self, _other: &Bound<'_, PyAny>) -> Constr {
        let mut afn = ScalarAffineFn::new();
        let s :ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::GreaterThan(value);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            for term in expr.get_fn().terms.iter() {
                afn.push_term(term.var, -term.coeff);
            }
            afn.constant -= expr.get_fn().constant;
            s = ScalarSetType::GreaterThan(0.0);
        } else {
            panic!("Unsupported type for comparison with Var");
        }
        afn.simplify();
        let constr_f = ScalarFunctionType::Affine(afn);
        Constr::new(constr_f, s)
    }
}

#[pymethods]
impl Vars {
    #[new]
    fn new(shape: Vec<usize>, ids: Vec<usize>) -> Self {
        let var_ids = ids.into_iter().map(|id| VarId(id)).collect();
        Vars {
            shape,
            ids: var_ids,
        }
    }
}
