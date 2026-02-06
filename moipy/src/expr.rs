use pyo3::prelude::*;
use moi_core::*;
use moi_solver_api::*;
use crate::var::Var;
use crate::constr::*;
#[pyclass]
#[derive(Clone,Debug)]
pub struct LinExpr {
    f: ScalarAffineFn
}

impl LinExpr {
    pub fn new(f: ScalarAffineFn) -> Self {
        LinExpr { f }
    }
    pub fn get_fn(&self) -> ScalarAffineFn {
        self.f.clone()
    }
}

#[pymethods]
impl LinExpr {
    fn __add__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = self.f.clone();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
             afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Add);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.get_id(), 1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Add);
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
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Add);
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
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Sub);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.get_id(), -1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
        } else {
            panic!("Unsupported type for subtraction with LinExpr");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __rsub__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = self.f.clone();
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn = ScalarAffineFn::with_constant(value).calculate(&afn, OperationType::Sub);
        } else {
            panic!("Unsupported type for subtraction with LinExpr");
        }
        afn.simplify();
        LinExpr::new(afn)
    }


    fn __mul__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        if let Ok(value) = _other.extract::<f64>() {
            let mut afn = self.f.clone();
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Mul);
            LinExpr::new(afn)
        } else {
            panic!("Unsupported type for multiplication with LinExpr");
        }
    }

    fn __rmul__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        if let Ok(value) = _other.extract::<f64>() {
            let mut afn = self.f.clone();
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Mul);
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
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Sub);
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
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
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Sub);
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
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
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Sub);
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
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