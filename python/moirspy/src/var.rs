use crate::constr::Constr;
use crate::expr::LinExpr;
use crate::utils::*;
use moi_core::*;
use moi_solver_api::VariableBound;
use numpy::{PyArray1, PyArrayDyn, PyArrayMethods, ndarray::IxDyn};
use pyo3::prelude::*;
use pyo3::types::PyTuple;

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Var {
    id: VarId,
    result_source: Option<ResultSource>,
}

#[pyclass(from_py_object)]
#[derive(Clone)]
pub struct Vars {
    shape: Vec<usize>,
    var_ids: VariableIds,
    result_source: Option<ResultSource>,
}

/// Model-created IDs are contiguous. Explicit user-created Vars may be sparse.
#[derive(Clone)]
enum VariableIds {
    Range { start: usize, len: usize },
    Explicit(Vec<VarId>),
}

impl From<Vec<VarId>> for VariableIds {
    fn from(ids: Vec<VarId>) -> Self {
        let start = ids.first().map_or(0, |id| id.0);
        if ids
            .iter()
            .enumerate()
            .all(|(i, id)| start.checked_add(i) == Some(id.0))
        {
            Self::Range {
                start,
                len: ids.len(),
            }
        } else {
            Self::Explicit(ids)
        }
    }
}

impl VariableIds {
    fn len(&self) -> usize {
        match self {
            Self::Range { len, .. } => *len,
            Self::Explicit(ids) => ids.len(),
        }
    }
    fn get(&self, index: usize) -> VarId {
        match self {
            Self::Range { start, .. } => VarId(start + index),
            Self::Explicit(ids) => ids[index],
        }
    }

    fn to_vec(&self) -> Vec<VarId> {
        match self {
            Self::Range { start, len } => (*start..*start + *len).map(VarId).collect(),
            Self::Explicit(ids) => ids.clone(),
        }
    }
}

impl std::fmt::Debug for VariableIds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries((0..self.len()).map(|i| self.get(i)))
            .finish()
    }
}

impl Var {
    pub fn get_id(&self) -> VarId {
        self.id
    }

    fn model_source(&self) -> PyResult<&ResultSource> {
        self.result_source.as_ref().ok_or_else(|| {
            pyo3::exceptions::PyRuntimeError::new_err("variable is not associated with a model")
        })
    }

    pub(crate) fn result_source(&self) -> Option<&ResultSource> {
        self.result_source.as_ref()
    }
}

#[pymethods]
impl Var {
    #[getter]
    fn lb(&self) -> PyResult<f64> {
        self.model_source()?
            .get_var_bound(self.id, VariableBound::Lower)
    }

    #[setter(lb)]
    fn set_lb(&self, value: f64) -> PyResult<()> {
        self.model_source()?
            .set_var_bound(self.id, VariableBound::Lower, value)
    }

    #[getter]
    fn ub(&self) -> PyResult<f64> {
        self.model_source()?
            .get_var_bound(self.id, VariableBound::Upper)
    }

    #[setter(ub)]
    fn set_ub(&self, value: f64) -> PyResult<()> {
        self.model_source()?
            .set_var_bound(self.id, VariableBound::Upper, value)
    }

    #[new]
    pub fn new(id: usize) -> Self {
        //NOTE: 用于Model当中添加变量方法，一般不会单独实例化变量
        Var {
            id: VarId(id),
            result_source: None,
        }
    }
    #[getter]
    #[pyo3(name = "X")]
    pub fn get_x(&self) -> PyResult<Option<f64>> {
        if let Some(result_source) = &self.result_source {
            result_source.get_var_value(self.id)
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

    fn __truediv__(&self, other: &Bound<'_, PyAny>) -> PyResult<LinExpr> {
        let denominator = division_denominator(other)?;
        let coefficient = ensure_finite(1.0 / denominator, "division result coefficient")?;
        let mut afn = ScalarAffineFn::with_capacity(1);
        afn.push_term(self.id, coefficient);
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
    #[pyo3(signature = (*pattern, dim=None))]
    fn sum(
        &self,
        py: Python<'_>,
        pattern: &Bound<'_, PyTuple>,
        dim: Option<isize>,
    ) -> PyResult<Py<PyAny>> {
        if let Some(dim) = dim {
            if !pattern.is_empty() {
                return Err(pyo3::exceptions::PyTypeError::new_err(
                    "sum accepts either index patterns or dim, not both",
                ));
            }
            let ndim = self.shape.len() as isize;
            let axis = if dim < 0 { dim + ndim } else { dim };
            if axis < 0 || axis >= ndim {
                return Err(pyo3::exceptions::PyIndexError::new_err(
                    "sum dimension out of bounds",
                ));
            }
            let axis = axis as usize;
            let axis_len = self.shape[axis];
            let inner: usize = self.shape[axis + 1..].iter().product();
            let outer: usize = self.shape[..axis].iter().product();
            let mut shape = self.shape.clone();
            shape.remove(axis);
            let mut expressions = Vec::with_capacity(outer * inner);
            for i in 0..outer {
                for j in 0..inner {
                    let expr = self
                        .sum_flat_indices((0..axis_len).map(|k| (i * axis_len + k) * inner + j));
                    expressions.push(Py::new(py, expr)?.into_any());
                }
            }
            if shape.is_empty() {
                return Ok(expressions.pop().expect("positive variable dimensions"));
            }
            return Ok(PyArray1::from_vec(py, expressions)
                .reshape(IxDyn(&shape))?
                .into_any()
                .unbind());
        }
        if pattern.is_empty() {
            return Ok(Py::new(py, self.sum_flat_indices(0..self.var_ids.len()))?.into_any());
        }
        if pattern.len() != self.shape.len() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "sum requires one index or '*' per dimension",
            ));
        }
        let mut fixed = Vec::with_capacity(self.shape.len());
        let mut count = 1usize;
        let mut matched = true;
        for (item, &size) in pattern.iter().zip(&self.shape) {
            if let Ok(text) = item.extract::<String>() {
                if text != "*" {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "sum supports only '*' as a wildcard; regular expressions are not supported",
                    ));
                }
                fixed.push(None);
                count *= size;
            } else {
                let index = item.extract::<isize>().map_err(|_| {
                    pyo3::exceptions::PyTypeError::new_err("sum patterns must be integers or '*'")
                })?;
                matched &= index >= 0 && (index as usize) < size;
                fixed.push(Some(index as usize));
            }
        }
        if !matched {
            return Ok(Py::new(py, LinExpr::default())?.into_any());
        }
        // Enumerate only the matching Cartesian slice, without scanning all variables
        // or creating Python Var objects for its members.
        let indices = (0..count).map(|mut item| {
            let mut flat = 0;
            let mut stride = 1;
            for (&size, index) in self.shape.iter().zip(&fixed).rev() {
                let coordinate = match index {
                    Some(index) => *index,
                    None => {
                        let coordinate = item % size;
                        item /= size;
                        coordinate
                    }
                };
                flat += coordinate * stride;
                stride *= size;
            }
            flat
        });
        Ok(Py::new(py, self.sum_flat_indices(indices))?.into_any())
    }

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
            var_ids: var_ids.into(),
            result_source: None,
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
        let var_id = self.var_ids.get(flat_index);
        Ok(Var {
            id: var_id,
            result_source: self.result_source.clone(),
        })
    }
    #[getter]
    #[pyo3(name = "X")]
    fn get_x<'py>(&self, py: Python<'py>) -> PyResult<Option<Bound<'py, PyArrayDyn<f64>>>> {
        let Some(result_source) = &self.result_source else {
            return Ok(None);
        };
        let values = match &self.var_ids {
            VariableIds::Range { start, len } => {
                result_source.get_var_values_range(*start..*start + *len)?
            }
            VariableIds::Explicit(ids) => result_source.get_var_values(ids)?,
        };
        let Some(values) = values else {
            return Ok(None);
        };
        PyArray1::from_vec(py, values)
            .reshape(IxDyn(&self.shape))
            .map(Some)
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
        self.result_source = Some(ResultSource::Cached(bridge.clone()));
    }
    pub fn set_direct(&mut self, model: &crate::direct::DirectModel) {
        self.result_source = Some(ResultSource::Direct(model.clone()));
    }
}

impl Vars {
    fn sum_flat_indices(&self, indices: impl ExactSizeIterator<Item = usize>) -> LinExpr {
        let mut function = ScalarAffineFn::with_capacity(indices.len());
        for index in indices {
            function.push_term(self.var_ids.get(index), 1.0);
        }
        // Explicit collections may contain repeated or unordered variable IDs.
        if matches!(self.var_ids, VariableIds::Explicit(_)) {
            function.simplify();
        }
        LinExpr::new(function)
    }

    pub fn new(shape: Vec<usize>, ids: Vec<VarId>) -> Self {
        Vars {
            shape,
            var_ids: ids.into(),
            result_source: None,
        }
    }
    pub(crate) fn ids(&self) -> Vec<VarId> {
        self.var_ids.to_vec()
    }
    pub(crate) fn result_source(&self) -> Option<&ResultSource> {
        self.result_source.as_ref()
    }
    pub fn set_bridge(&mut self, bridge: &SharedBridge) {
        self.result_source = Some(ResultSource::Cached(bridge.clone()));
    }
    pub fn set_direct(&mut self, model: &crate::direct::DirectModel) {
        self.result_source = Some(ResultSource::Direct(model.clone()));
    }
}

#[cfg(test)]
mod compact_ids_tests {
    use super::*;
    #[test]
    fn contiguous_ids_use_constant_storage_and_sparse_ids_are_preserved() {
        let ids = VariableIds::from((17..100_017).map(VarId).collect::<Vec<_>>());
        assert!(matches!(ids, VariableIds::Range { .. }));
        assert_eq!(ids.get(99_999), VarId(100_016));
        let sparse = VariableIds::from(vec![VarId(9), VarId(3), VarId(9)]);
        assert!(matches!(sparse, VariableIds::Explicit(_)));
        assert_eq!(sparse.clone().get(1), VarId(3));
        let extreme = VariableIds::from(vec![VarId(usize::MAX), VarId(0)]);
        assert!(matches!(extreme, VariableIds::Explicit(_)));
    }
}
