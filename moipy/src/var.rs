use crate::constr::Constr;
use crate::expr::LinExpr;
use crate::model::Model;
use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;
use crate::utils::*;

#[pyclass]
#[derive(Debug, Clone)]
pub struct Var {
    id: VarId,
    bridge: Option<SharedBridge>,
}

#[pyclass]
#[derive(Clone, Debug)]
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
    pub fn get_x(&self) -> Option<f64> {
        //NOTE: 目前仅作为占位，后续会通过桥接优化器获取变量的值
        if let Some(bridge) = &self.bridge {
            let bridge = bridge.read().unwrap();
            bridge.get_value_by_var_id(self.id)
        } else {
            None
        }
    }


    fn __add__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Add);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, 1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Add);
        } else {
            panic!("Unsupported type for addition with Var");
        }
        afn.simplify();

        LinExpr::new(afn)
    }

    fn __radd__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, 1.0);
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Add);
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
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Sub);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, -1.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
        } else {
            panic!("Unsupported type for subtraction with Var");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __rsub__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = ScalarAffineFn::new();
        afn.push_term(self.id, -1.0);
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn = afn.calculate(&ScalarAffineFn::with_constant(value), OperationType::Add);
        } else {
            panic!("Unsupported type for subtraction with Var");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __mul__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = ScalarAffineFn::new();
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            afn.push_term(self.id, value);
        } else if let Ok(var) = _other.extract::<Var>() {
            // TODO: 后续会考虑非线性实现
            panic!("Multiplication of two variables is not supported in linear expressions");
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            panic!(
                "Multiplication of a variable with a linear expression is not supported in linear expressions"
            );
        } else {
            panic!("Unsupported type for multiplication with Var");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __rmul__(&self, _other: &Bound<'_, PyAny>) -> LinExpr {
        let mut afn = ScalarAffineFn::new();
        // 判断左侧项类型，为浮点数
        if let Ok(value) = _other.extract::<f64>() {
            afn.push_term(self.id, value);
        } else {
            panic!("Unsupported type for multiplication with Var");
        }
        afn.simplify();
        LinExpr::new(afn)
    }

    fn __le__(&self, _other: &Bound<'_, PyAny>) -> Constr {
        let mut afn = ScalarAffineFn::new();
        let s: ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::LessThan(value);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::LessThan(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
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
        let s: ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::GreaterThan(value);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::GreaterThan(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
            s = ScalarSetType::GreaterThan(0.0);
        } else {
            panic!("Unsupported type for comparison with Var");
        }
        afn.simplify();
        let constr_f = ScalarFunctionType::Affine(afn);
        Constr::new(constr_f, s)
    }

    fn __eq__(&self, _other: &Bound<'_, PyAny>) -> Constr {
        let mut afn = ScalarAffineFn::new();
        let s: ScalarSetType;
        afn.push_term(self.id, 1.0);
        // 判断右侧项类型，为浮点数、变量或线性表达式
        if let Ok(value) = _other.extract::<f64>() {
            s = ScalarSetType::EqualTo(value);
        } else if let Ok(var) = _other.extract::<Var>() {
            afn.push_term(var.id, -1.0);
            s = ScalarSetType::EqualTo(0.0);
        } else if let Ok(expr) = _other.extract::<LinExpr>() {
            afn = afn.calculate(&expr.get_fn(), OperationType::Sub);
            s = ScalarSetType::EqualTo(0.0);
        } else {
            panic!("Unsupported type for comparison with Var");
        }
        afn.simplify();
        let constr_f = ScalarFunctionType::Affine(afn);
        Constr::new(constr_f, s)
    }

    fn __str__(&self) -> String {
        format!("Var({})", self.id.0)
    }
}

#[pymethods]
impl Vars {
    #[new]
    fn new_py(shape: Vec<usize>, ids: Vec<usize>) -> Self {
        let var_ids: Vec<VarId> = ids.into_iter().map(|id| VarId(id)).collect();
        Vars { shape, var_ids, bridge: None }
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
        let mut flat_index = 0;
        let mut multiplier = 1;
        for (i, &idx) in idx_vec.iter().rev().enumerate() {
            if idx >= self.shape[self.shape.len() - 1 - i] {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "Index out of bounds",
                ));
            }
            flat_index += idx * multiplier;
            multiplier *= self.shape[self.shape.len() - 1 - i];
        }
        if flat_index >= self.var_ids.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "Flat index out of bounds",
            ));
        }
        let var_id = self.var_ids[flat_index];
        Ok(Var { id: var_id, bridge: self.bridge.clone() })
    }
    fn __str__(&self) -> String {
        format!("Vars(shape={:?}, ids={:?})", self.shape, self.var_ids)
    }
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
