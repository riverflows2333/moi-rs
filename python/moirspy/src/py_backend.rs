use std::ops::Deref;

use moi_core::attributes::*;
use moi_core::errors::MoiError;
use moi_core::functions::{AffineTerm, ScalarAffineFn, ScalarFunctionType};
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
use moi_solver_api::*;
use pyo3::prelude::*;
use pyo3::types::{PyDict,PyAny,PyBool,PyInt,PyFloat,PyString};

pub struct PyBackend {
    pub backend: Py<PyAny>,
}

unsafe impl Send for PyBackend {}
unsafe impl Sync for PyBackend {}

impl PyBackend {
    pub fn new(backend: Py<PyAny>) -> Self {
        Self { backend }
    }
}

impl ModelLike for PyBackend {
    fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> VarId {
        Python::attach(|py| {
            let res = self
                .backend
                .call_method1(py, "add_variable", (name, vtype, lb, ub))
                .unwrap();
            VarId(res.extract::<usize>(py).unwrap())
        })
    }

    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Vec<VarId> {
        Python::attach(|py| {
            let (name_val, name_list) = match name {
                Some(NameType::Single(s)) => (Some(s), None),
                Some(NameType::Vector(l)) => (None, Some(l)),
                None => (None, None),
            };
            let (lb_val, lb_list) = match lb {
                Some(BoundType::Single(s)) => (Some(s), None),
                Some(BoundType::Vector(l)) => (None, Some(l)),
                None => (None, None),
            };
            let (ub_val, ub_list) = match ub {
                Some(BoundType::Single(s)) => (Some(s), None),
                Some(BoundType::Vector(l)) => (None, Some(l)),
                None => (None, None),
            };
            let res = self
                .backend
                .call_method1(py, "add_variables", (n, name_val, vtype, lb_val, ub_val))
                .unwrap();
            res.extract::<Vec<usize>>(py)
                .unwrap()
                .into_iter()
                .map(VarId)
                .collect()
        })
    }

    fn add_constraint(
        &mut self,
        f: ScalarFunctionType,
        s: ScalarSetType,
        name: Option<String>,
    ) -> ConstrId {
        Python::attach(|py| {
            let (vars, coeffs, constant) = match f {
                ScalarFunctionType::Affine(af) => (
                    af.terms.iter().map(|t| t.var.0).collect::<Vec<_>>(),
                    af.terms.iter().map(|t| t.coeff).collect::<Vec<_>>(),
                    af.constant,
                ),
                ScalarFunctionType::Variable(v) => (vec![v.0], vec![1.0], 0.0),
            };
            let (sense, rhs) = match s {
                ScalarSetType::LessThan(v) => ("<", v),
                ScalarSetType::GreaterThan(v) => (">", v),
                ScalarSetType::EqualTo(v) => ("=", v),
                ScalarSetType::Interval(l, u) => {
                    panic!("Interval constraints not supported via PyBackend yet")
                }
            };
            let res = self
                .backend
                .call_method1(
                    py,
                    "add_constraint",
                    (vars, coeffs, sense, rhs, name, constant),
                )
                .unwrap();
            ConstrId(res.extract::<usize>(py).unwrap())
        })
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Vec<ConstrId> {
        Python::attach(|py| {
            let mut vars_list = Vec::new();
            let mut coeffs_list = Vec::new();
            let mut constants = Vec::new();
            for f in fs {
                match f {
                    ScalarFunctionType::Affine(af) => {
                        vars_list.push(af.terms.iter().map(|t| t.var.0).collect::<Vec<_>>());
                        coeffs_list.push(af.terms.iter().map(|t| t.coeff).collect::<Vec<_>>());
                        constants.push(af.constant);
                    }
                    ScalarFunctionType::Variable(v) => {
                        vars_list.push(vec![v.0]);
                        coeffs_list.push(vec![1.0]);
                        constants.push(0.0);
                    }
                }
            }
            let mut senses = Vec::new();
            let mut rhs_list = Vec::new();
            for s in ss {
                match s {
                    ScalarSetType::LessThan(v) => {
                        senses.push("<");
                        rhs_list.push(v);
                    }
                    ScalarSetType::GreaterThan(v) => {
                        senses.push(">");
                        rhs_list.push(v);
                    }
                    ScalarSetType::EqualTo(v) => {
                        senses.push("=");
                        rhs_list.push(v);
                    }
                    ScalarSetType::Interval(l, u) => {
                        panic!("Interval constraints not supported via PyBackend yet")
                    }
                }
            }
            let res = self
                .backend
                .call_method1(
                    py,
                    "add_constraints",
                    (vars_list, coeffs_list, constants, senses, rhs_list, names),
                )
                .unwrap();
            res.extract::<Vec<usize>>(py)
                .unwrap()
                .into_iter()
                .map(ConstrId)
                .collect()
        })
    }

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError> {
        Python::attach(|py| {
            let (vars, coeffs, constant) = match f {
                ScalarFunctionType::Affine(af) => (
                    af.terms.iter().map(|t| t.var.0).collect::<Vec<_>>(),
                    af.terms.iter().map(|t| t.coeff).collect::<Vec<_>>(),
                    af.constant,
                ),
                ScalarFunctionType::Variable(v) => (vec![v.0], vec![1.0], 0.0),
            };
            let s = match sense {
                ModelSense::Maximize => 1,
                ModelSense::Minimize => -1,
            };
            self.backend
                .call_method1(py, "set_objective", (vars, coeffs, constant, s))
                .unwrap();
            Ok(())
        })
    }

    fn update(&mut self) -> Result<(), MoiError> {
        Python::attach(|py| {
            self.backend.call_method0(py, "update").unwrap();
            Ok(())
        })
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        None
    }

    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError> {
        Ok(())
    }

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue> {
        None
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        Python::attach(|py| {
            let attr_name = match attr {
                OptimizerAttr::Raw(s) => s,
                _ => format!("{:?}", attr),
            };
            let val:Bound<'_, PyAny> = match value {
                AttrValue::Int(i) => PyInt::new(py, i).into_any(),
                AttrValue::Float(f) => PyFloat::new(py, f).into_any(),
                AttrValue::Bool(b) => PyBool::new(py, b).as_any().clone(),
                AttrValue::String(s) => PyString::new(py, &s).into_any(),
                _ => panic!("Unsupported attribute value type for PyBackend"),
            };
            self.backend
                .call_method1(py, "set_optimizer_attr", (attr_name, val))
                .unwrap();
            Ok(())
        })
    }
}

impl Optimizer for PyBackend {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        Python::attach(|py| {
            let res = self.backend.call_method0(py, "optimize").unwrap();
            let status_code = res.extract::<u32>(py).unwrap();
            let status = match status_code {
                2 => SolveStatus::Optimal,
                3 => SolveStatus::Infeasible,
                _ => SolveStatus::Unknown,
            };
            Ok(status)
        })
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        Ok(())
    }

    fn get_var_value(&self, var_id: VarId) -> Option<f64> {
        Python::attach(|py| {
            let res = self
                .backend
                .call_method1(py, "get_var_value", (var_id.0,))
                .unwrap();
            res.extract::<Option<f64>>(py).unwrap()
        })
    }

    fn get_objective_value(&self) -> Option<f64> {
        Python::attach(|py| {
            let res = self
                .backend
                .call_method0(py, "get_objective_value")
                .unwrap();
            res.extract::<Option<f64>>(py).unwrap()
        })
    }
}
