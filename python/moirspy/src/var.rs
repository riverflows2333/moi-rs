use crate::constr::Constr;
use crate::expr::LinExpr;
use crate::utils::*;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Var {
    id: VarId,
    bridge: Option<SharedBridge>,
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Vars {
    shape: Vec<usize>,
    var_ids: Vec<VarId>,
    bridge: Option<SharedBridge>,
}

impl Var {
    pub fn get_id(&self) -> VarId {
        self.id
    }
}

#[pymethods]
impl Var {
    #[new]
    pub fn new(id: usize) -> Self {
        //NOTE: 用于Model当中添加变量方法，一般不会单独实例化变量
        Var {
            id: VarId(id),
            bridge: None,
        }
    }
    #[getter]
    #[pyo3(name = "X")]
    pub fn get_x(&self) -> PyResult<Option<f64>> {
        if let Some(bridge) = &self.bridge {
            let bridge = lock_bridge(bridge)?;
            bridge.get_var_value(self.id).map_err(to_py_runtime_error)
        } else {
            Ok(None)
        }
    }

    fn __add__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += ensure_finite(value, "addition operand")?;
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.id, 1.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_assign(expr.get_fn_ref());
        } else {
            return Err(operand_type_error(
                "Var addition",
                "a number, Var, or LinExpr",
            ));
        }
        afn.simplify();

        Ok(LinExpr::new(afn))
    }

    fn __radd__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, 1.0);
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += ensure_finite(value, "addition operand")?;
        } else {
            return Err(operand_type_error("Var addition", "a number"));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __sub__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant -= ensure_finite(value, "subtraction operand")?;
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.id, -1.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
        } else {
            return Err(operand_type_error(
                "Var subtraction",
                "a number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __rsub__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, -1.0);
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn.constant += ensure_finite(value, "subtraction operand")?;
        } else {
            return Err(operand_type_error("Var subtraction", "a number"));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __mul__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = ScalarAffineFn::new();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.push_term(self.id, ensure_finite(value, "multiplication operand")?);
        } else if _other.extract::<Var>().is_ok() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "multiplication of two variables is not supported in linear expressions",
            ));
        } else if _other.extract::<LinExpr>().is_ok() {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "multiplication of a variable with a linear expression is not supported in linear expressions",
            ));
        } else {
            return Err(operand_type_error("Var multiplication", "a finite number"));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __rmul__(&self, _other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let mut afn = ScalarAffineFn::new();
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn.push_term(self.id, ensure_finite(value, "multiplication operand")?);
        } else {
            return Err(operand_type_error("Var multiplication", "a finite number"));
        }
        afn.simplify();
        Ok(LinExpr::new(afn))
    }

    fn __le__(&self, _other: &Bound<'_, PyAny>) -> PyResult<Constr> {
        let mut afn = ScalarAffineFn::new();
        let s: ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::LessThan(ensure_finite(value, "comparison operand")?);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else {
            return Err(operand_type_error(
                "Var comparison",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        let constr_f = ScalarFunctionType::Affine(afn);
        Ok(Constr::new(constr_f, s))
    }

    fn __ge__(&self, _other: &Bound<'_, PyAny>) -> PyResult<Constr> {
        let mut afn = ScalarAffineFn::new();
        let s: ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::GreaterThan(ensure_finite(value, "comparison operand")?);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else {
            return Err(operand_type_error(
                "Var comparison",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        let constr_f = ScalarFunctionType::Affine(afn);
        Ok(Constr::new(constr_f, s))
    }

    fn __eq__(&self, _other: &Bound<'_, PyAny>) -> PyResult<Constr> {
        let mut afn = ScalarAffineFn::new();
        let s: ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::EqualTo(ensure_finite(value, "comparison operand")?);
        } else if let Ok(var) = _other.extract::<PyRef<'_, Var>>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(expr) = _other.extract::<PyRef<'_, LinExpr>>() {
            afn.add_scaled_assign(expr.get_fn_ref(), -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else {
            return Err(operand_type_error(
                "Var comparison",
                "a finite number, Var, or LinExpr",
            ));
        }
        afn.simplify();
        let constr_f = ScalarFunctionType::Affine(afn);
        Ok(Constr::new(constr_f, s))
    }

    fn __str__(&self) -> String {
        format!("Var({})", self.id.0)
    }
}

#[pymethods]
impl Vars {
    #[new]
    fn new_py(shape: Vec<usize>, ids: Vec<usize>) -> PyResult<Self> {
        let expected = validate_shape(&shape)?;
        if ids.len() != expected {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "variable IDs have length {}, expected {expected} for shape {shape:?}",
                ids.len()
            )));
        }
        let var_ids: Vec<VarId> = ids.into_iter().map(VarId).collect();
        Ok(Vars {
            shape,
            var_ids,
            bridge: None,
        })
    }
    fn __getitem__(&self, idx: &Bound<'_, PyAny>) -> PyResult<Var> {
        let idx_vec: Vec<usize>;
        if let Ok(shape) = idx.extract::<Vec<usize>>() {
            idx_vec = shape;
        } else if let Ok(shape) = idx.extract::<usize>() {
            idx_vec = vec![shape];
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Expected integers for indices",
            ));
        }
        if idx_vec.len() != self.shape.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "Index dimension does not match variable dimension",
            ));
        }
        let mut flat_index: usize = 0;
        let mut multiplier: usize = 1;
        for (i, &idx) in idx_vec.iter().rev().enumerate() {
            if idx >= self.shape[self.shape.len() - 1 - i] {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "Index out of bounds",
                ));
            }
            let contribution = idx.checked_mul(multiplier).ok_or_else(|| {
                pyo3::exceptions::PyOverflowError::new_err("variable index calculation overflow")
            })?;
            flat_index = flat_index.checked_add(contribution).ok_or_else(|| {
                pyo3::exceptions::PyOverflowError::new_err("variable index calculation overflow")
            })?;
            multiplier = multiplier
                .checked_mul(self.shape[self.shape.len() - 1 - i])
                .ok_or_else(|| {
                    pyo3::exceptions::PyOverflowError::new_err("variable shape product overflow")
                })?;
        }
        if flat_index >= self.var_ids.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "Flat index out of bounds",
            ));
        }
        let var_id = self.var_ids[flat_index];
        Ok(Var {
            id: var_id,
            bridge: self.bridge.clone(),
        })
    }
    fn __str__(&self) -> String {
        format!("Vars(shape={:?}, ids={:?})", self.shape, self.var_ids)
    }
}

fn validate_shape(shape: &[usize]) -> PyResult<usize> {
    if shape.is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "variable shape must contain at least one dimension",
        ));
    }
    if shape.contains(&0) {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "variable shape dimensions must be positive",
        ));
    }
    shape.iter().try_fold(1usize, |product, dimension| {
        product.checked_mul(*dimension).ok_or_else(|| {
            pyo3::exceptions::PyOverflowError::new_err("variable shape product exceeds usize range")
        })
    })
}

impl Var {
    pub fn set_bridge(&mut self, bridge: &SharedBridge) {
        self.bridge = Some(bridge.clone());
    }
}

impl Vars {
    pub fn new(shape: Vec<usize>, ids: Vec<VarId>) -> Self {
        Vars {
            shape,
            var_ids: ids,
            bridge: None,
        }
    }
    pub fn set_bridge(&mut self, bridge: &SharedBridge) {
        self.bridge = Some(bridge.clone());
    }
}
