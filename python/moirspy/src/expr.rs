use crate::constr::*;
use crate::utils::{ensure_finite, operand_type_error};
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
    fn __add__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = self.f.clone();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += ensure_finite(value, "addition operand")?;
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), 1.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_assign(expr.get_fn_ref());
        } else {
            return Err(operand_type_error(
                "LinExpr addition",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __radd__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = self.f.clone();
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += ensure_finite(value, "addition operand")?;
        } else {
            return Err(operand_type_error("LinExpr addition", "a finite number"));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __sub__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = self.f.clone();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= ensure_finite(value, "subtraction operand")?;
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
        } else {
            return Err(operand_type_error(
                "LinExpr subtraction",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __rsub__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            let mut afn =
                ScalarAffineFn::with_constant(ensure_finite(value, "subtraction operand")?);
            afn.add_scaled_assign(&self.f, -1.0);
            afn.simplify();
            Ok(LinExpr::new(afn))
        } else {
            Err(operand_type_error("LinExpr subtraction", "a finite number"))
        }
    }

    fn __mul__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        if let Ok(value) = _other.extract::<f64>() {
            let value = ensure_finite(value, "multiplication operand")?;
            let mut afn = ScalarAffineFn::with_capacity(self.f.terms.len());
            afn.add_scaled_assign(&self.f, value);
            Ok(LinExpr::new(afn))
        } else {
            Err(operand_type_error(
                "LinExpr multiplication",
                "a finite number",
            ))
        }
    }

    fn __rmul__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        if let Ok(value) = _other.extract::<f64>() {
            let value = ensure_finite(value, "multiplication operand")?;
            let mut afn = ScalarAffineFn::with_capacity(self.f.terms.len());
            afn.add_scaled_assign(&self.f, value);
            Ok(LinExpr::new(afn))
        } else {
            Err(operand_type_error(
                "LinExpr multiplication",
                "a finite number",
            ))
        }
    }

    fn __le__(&self, _other: &Bound<'_, PyAny>) -> PyResult<Constr> {
        let mut afn = self.f.clone();
        let s: ScalarSetType;
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= ensure_finite(value, "comparison operand")?;
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else {
            return Err(operand_type_error(
                "LinExpr comparison",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        Ok(Constr::new(ScalarFunctionType::Affine(afn), s))
    }

    fn __ge__(&self, _other: &Bound<'_, PyAny>) -> PyResult<Constr> {
        let mut afn = self.f.clone();
        let s: ScalarSetType;
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= ensure_finite(value, "comparison operand")?;
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else {
            return Err(operand_type_error(
                "LinExpr comparison",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        Ok(Constr::new(ScalarFunctionType::Affine(afn), s))
    }

    fn __eq__(&self, _other: &Bound<'_, PyAny>) -> PyResult<Constr> {
        let mut afn = self.f.clone();
        let s: ScalarSetType;
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= ensure_finite(value, "comparison operand")?;
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.get_id(), -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else {
            return Err(operand_type_error(
                "LinExpr comparison",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        Ok(Constr::new(ScalarFunctionType::Affine(afn), s))
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.f)
    }
}
