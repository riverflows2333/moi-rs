use moi_core::attributes::*;
use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
use moi_solver_api::*;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyBool, PyFloat, PyInt, PyString};

pub struct PyBackend {
    pub backend: Py<PyAny>,
}

unsafe impl Send for PyBackend {}

impl PyBackend {
    pub fn new(backend: Py<PyAny>) -> Self {
        Self { backend }
    }

    fn protocol_error(context: &str, error: PyErr) -> MoiError {
        MoiError::BackendProtocol(format!("{context}: {error}"))
    }

    fn constraint_parts(set: ScalarSetType) -> Result<(&'static str, f64), MoiError> {
        let bounds = scalar_set_to_bounds(&set);
        match (bounds.lower, bounds.upper) {
            (None, Some(value)) => Ok(("<", value)),
            (Some(value), None) => Ok((">", value)),
            (Some(lower), Some(upper)) if lower == upper => Ok(("=", lower)),
            _ => Err(MoiError::UnsupportedConstraint {
                func: "scalar linear",
                set: "interval",
            }),
        }
    }
}

impl ModelLike for PyBackend {
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError> {
        let names = match name {
            Some(NameType::Single(base)) => {
                Some((0..n).map(|i| format!("{base}_{i}")).collect::<Vec<_>>())
            }
            Some(NameType::Vector(values)) => {
                ensure_len(values.len(), n, "variable names")?;
                Some(values)
            }
            None => None,
        };
        if let Some(ref values) = vtype {
            ensure_len(values.len(), n, "variable types")?;
        }
        let lbs = match lb {
            Some(BoundType::Single(value)) => Some(vec![value; n]),
            Some(BoundType::Vector(values)) => {
                ensure_len(values.len(), n, "lower bounds")?;
                Some(values)
            }
            None => None,
        };
        let ubs = match ub {
            Some(BoundType::Single(value)) => Some(vec![value; n]),
            Some(BoundType::Vector(values)) => {
                ensure_len(values.len(), n, "upper bounds")?;
                Some(values)
            }
            None => None,
        };

        Python::attach(|py| {
            let result = self
                .backend
                .call_method1(py, "add_variables", (n, names, vtype, lbs, ubs))
                .map_err(|error| Self::protocol_error("add_variables failed", error))?;
            let ids = result
                .extract::<Vec<usize>>(py)
                .map_err(|error| Self::protocol_error("invalid add_variables result", error))?;
            ensure_len(ids.len(), n, "returned variable IDs")?;
            Ok(ids.into_iter().map(VarId).collect())
        })
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError> {
        ensure_len(ss.len(), fs.len(), "constraint sets")?;
        if let Some(ref values) = names {
            ensure_len(values.len(), fs.len(), "constraint names")?;
        }

        let mut variables = Vec::with_capacity(fs.len());
        let mut coefficients = Vec::with_capacity(fs.len());
        let mut constants = Vec::with_capacity(fs.len());
        for function in &fs {
            let linear = scalar_function_to_linear(function)?;
            variables.push(
                linear
                    .variables
                    .into_iter()
                    .map(|variable| variable.0)
                    .collect::<Vec<_>>(),
            );
            coefficients.push(linear.coefficients);
            constants.push(linear.constant);
        }

        let mut senses = Vec::with_capacity(ss.len());
        let mut rhs = Vec::with_capacity(ss.len());
        for set in ss {
            let (sense, value) = Self::constraint_parts(set)?;
            senses.push(sense);
            rhs.push(value);
        }

        Python::attach(|py| {
            let result = self
                .backend
                .call_method1(
                    py,
                    "add_constraints",
                    (variables, coefficients, constants, senses, rhs, names),
                )
                .map_err(|error| Self::protocol_error("add_constraints failed", error))?;
            let ids = result
                .extract::<Vec<usize>>(py)
                .map_err(|error| Self::protocol_error("invalid add_constraints result", error))?;
            ensure_len(ids.len(), fs.len(), "returned constraint IDs")?;
            Ok(ids.into_iter().map(ConstrId).collect())
        })
    }

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError> {
        let linear = scalar_function_to_linear(&f)?;
        let variables = linear
            .variables
            .into_iter()
            .map(|variable| variable.0)
            .collect::<Vec<_>>();
        let sense = match sense {
            ModelSense::Maximize => 1,
            ModelSense::Minimize => -1,
        };
        Python::attach(|py| {
            self.backend
                .call_method1(
                    py,
                    "set_objective",
                    (variables, linear.coefficients, linear.constant, sense),
                )
                .map_err(|error| Self::protocol_error("set_objective failed", error))?;
            Ok(())
        })
    }

    fn update(&mut self) -> Result<(), MoiError> {
        Python::attach(|py| {
            self.backend
                .call_method0(py, "update")
                .map_err(|error| Self::protocol_error("update failed", error))?;
            Ok(())
        })
    }

    fn get_model_attr(&self, _attr: ModelAttr) -> Option<AttrValue> {
        None
    }

    fn set_model_attr(&mut self, _attr: ModelAttr, _value: AttrValue) -> Result<(), MoiError> {
        Err(MoiError::UnsupportedAttribute)
    }

    fn get_optimizer_attr(&self, _attr: OptimizerAttr) -> Option<AttrValue> {
        None
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        Python::attach(|py| {
            let attr_name = match attr {
                OptimizerAttr::Raw(name) => name,
                _ => format!("{attr:?}"),
            };
            let value: Bound<'_, PyAny> = match value {
                AttrValue::Int(value) => PyInt::new(py, value).into_any(),
                AttrValue::Float(value) => PyFloat::new(py, value).into_any(),
                AttrValue::Bool(value) => PyBool::new(py, value).as_any().clone(),
                AttrValue::String(value) => PyString::new(py, &value).into_any(),
                _ => {
                    return Err(MoiError::InvalidInput(
                        "optimizer attribute requires bool, int, float, or string".to_string(),
                    ));
                }
            };
            self.backend
                .call_method1(py, "set_optimizer_attr", (attr_name, value))
                .map_err(|error| Self::protocol_error("set_optimizer_attr failed", error))?;
            Ok(())
        })
    }
}

impl Optimizer for PyBackend {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        Python::attach(|py| {
            let result = self
                .backend
                .call_method0(py, "optimize")
                .map_err(|error| Self::protocol_error("optimize failed", error))?;
            let code = result
                .extract::<u32>(py)
                .map_err(|error| Self::protocol_error("invalid optimize result", error))?;
            Ok(SolveStatus::from_code(code))
        })
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        Err(MoiError::Msg(
            "conflict computation is not implemented by the Python backend protocol".to_string(),
        ))
    }

    fn get_var_value(&self, var_id: VarId) -> Result<Option<f64>, MoiError> {
        Python::attach(|py| {
            self.backend
                .call_method1(py, "get_var_value", (var_id.0,))
                .map_err(|error| Self::protocol_error("get_var_value failed", error))?
                .extract::<Option<f64>>(py)
                .map_err(|error| Self::protocol_error("invalid get_var_value result", error))
        })
    }

    fn get_objective_value(&self) -> Result<Option<f64>, MoiError> {
        Python::attach(|py| {
            self.backend
                .call_method0(py, "get_objective_value")
                .map_err(|error| Self::protocol_error("get_objective_value failed", error))?
                .extract::<Option<f64>>(py)
                .map_err(|error| Self::protocol_error("invalid get_objective_value result", error))
        })
    }
}
