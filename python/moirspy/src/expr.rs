use crate::constr::*;
use crate::var::Var;
use moi_core::*;
use pyo3::prelude::*;
#[pyclass(from_py_object)]
#[derive(Clone, Debug)]
pub struct LinExpr {
    f: ScalarAffineFn,
}

impl LinExpr {
    pub fn new(f: ScalarAffineFn) -> Self {
        LinExpr { f }
    }
    pub fn get_fn(&self) -> ScalarAffineFn {
        self.f.clone()
    }
    pub fn get_fn_ref(&self) -> &ScalarAffineFn {
        &self.f
    }
}

impl Default for LinExpr {
    fn default() -> Self {
        LinExpr {
            f: ScalarAffineFn::new(),
        }
    }
}

#[pymethods]
impl LinExpr {
    fn __add__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = self.f.clone();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += value;
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), 1.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_assign(expr.get_fn_ref());
        } else {
            panic!("Unsupported type for addition with LinExpr");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __radd__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = self.f.clone();
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += value;
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
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
        } else {
            panic!("Unsupported type for subtraction with LinExpr");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __rsub__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            let mut afn = ScalarAffineFn::with_constant(value);
            afn.add_scaled_assign(&self.f, -1.0);
            afn.simplify();
            LinExpr::new(afn)
        } else {
            panic!("Unsupported type for subtraction with LinExpr");
        }
    }

    fn __mul__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        if let Ok(value) = _other.extract::<f64>() {
            let mut afn = ScalarAffineFn::with_capacity(self.f.terms.len());
            afn.add_scaled_assign(&self.f, value);
            LinExpr::new(afn)
        } else {
            panic!("Unsupported type for multiplication with LinExpr");
        }
    }

    fn __rmul__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        if let Ok(value) = _other.extract::<f64>() {
            let mut afn = ScalarAffineFn::with_capacity(self.f.terms.len());
            afn.add_scaled_assign(&self.f, value);
            LinExpr::new(afn)
        } else {
            panic!("Unsupported type for multiplication with LinExpr");
        }
    }

    fn __le__(&self, _other: &Bound<'_, PyAny>) -> Constr {
        let mut afn = self.f.clone();
        let s: ScalarSetType;
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= value;
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else {
            panic!("Unsupported type for comparison with LinExpr");
        }
        afn.simplify();
        Constr::new(ScalarFunctionType::Affine(afn), s)
    }

    fn __ge__(&self, _other: &Bound<'_, PyAny>) -> Constr {
        let mut afn = self.f.clone();
        let s: ScalarSetType;
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= value;
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else {
            panic!("Unsupported type for comparison with LinExpr");
        }
        afn.simplify();
        Constr::new(ScalarFunctionType::Affine(afn), s)
    }

    fn __eq__(&self, _other: &Bound<'_, PyAny>) -> Constr {
        let mut afn = self.f.clone();
        let s: ScalarSetType;
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= value;
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else {
            panic!("Unsupported type for comparison with LinExpr");
        }
        afn.simplify();
        Constr::new(ScalarFunctionType::Affine(afn), s)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.f)
    }
}
