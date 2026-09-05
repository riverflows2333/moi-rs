use crate::backends::copt::{CoptEnv, create_optimizer as create_copt_optimizer};
use crate::constr::Constr;
use crate::direct::DirectModel;
use crate::expr::LinExpr;
use crate::moi::*;
use crate::runtime::ModelRuntime;
use crate::utils::*;
use crate::var::*;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};
#[pyclass]
pub struct Model {
    name: String,
    runtime: ModelRuntime,
}

#[pymethods]
impl Model {
    #[new]
    #[pyo3(signature = (name, backend=None, env=None))]
    fn new(
        py: Python<'_>,
        name: String,
        backend: Option<&str>,
        env: Option<Py<PyAny>>,
    ) -> PyResult<Self> {
        let runtime = match backend {
            None if env.is_none() => ModelRuntime::new_cached(),
            None => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "env requires an explicit backend",
                ));
            }
            Some("copt") => {
                let optimizer = match env.as_ref() {
                    Some(env) => {
                        let copt_env =
                            env.bind(py).extract::<PyRef<'_, CoptEnv>>().map_err(|_| {
                                PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                                    "Model(..., backend='copt', env=...) requires moirspy.CoptEnv",
                                )
                            })?;
                        create_copt_optimizer(Some(&copt_env))?
                    }
                    None => create_copt_optimizer(None)?,
                };
                ModelRuntime::Direct(DirectModel::with_optimizer(optimizer))
            }
            Some(other) => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "direct backend '{other}' is not available; currently supported: 'copt'"
                )));
            }
        };
        Ok(Model { name, runtime })
    }

    #[pyo3(signature = (lb=0., ub=f64::INFINITY, obj=0.0, vtype=None, name=""),name="addVar")]
    fn add_var(
        &mut self,
        lb: f64,
        ub: f64,
        obj: f64,
        vtype: Option<VarType>,
        name: &str,
    ) -> PyResult<Var> {
        validate_objective_coefficients(&[obj])?;
        let variable_type = vtype.map(|t| match t {
            VarType::CONTINUOUS => 'C',
            VarType::BINARY => 'B',
            VarType::INTEGER => 'I',
        });
        let (var_id, result_source) = match &self.runtime {
            ModelRuntime::Cached(cached) => {
                let shared = cached.bridge();
                let var_id = lock_bridge(shared)?
                    .add_variable(Some(name), variable_type, Some(lb), Some(ub))
                    .map_err(to_py_runtime_error)?;
                (var_id, ResultSource::Cached(shared.clone()))
            }
            ModelRuntime::Direct(direct) => {
                let var_id = direct
                    .add_variable(Some(name), variable_type, Some(lb), Some(ub))
                    .map_err(to_py_runtime_error)?;
                (var_id, ResultSource::Direct(direct.clone()))
            }
            ModelRuntime::Poisoned(state) => {
                return Err(to_py_runtime_error(MoiError::BackendState(format!(
                    "model is poisoned after '{}': {}",
                    state.operation, state.reason
                ))));
            }
        };
        let mut var = Var::new(var_id.0);
        match result_source {
            ResultSource::Cached(shared) => var.set_bridge(&shared),
            ResultSource::Direct(direct) => var.set_direct(&direct),
        }
        Ok(var)
    }
    #[pyo3(signature = (*indices, lb=None, ub=None, obj=None, vtype=None, name=None),name="addVars")]
    fn add_vars<'py>(
        &mut self,
        indices: &Bound<'py, PyTuple>,
        lb: Option<&Bound<'py, PyAny>>,
        ub: Option<&Bound<'py, PyAny>>,
        obj: Option<&Bound<'py, PyAny>>,
        vtype: Option<&Bound<'py, PyAny>>,
        name: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Vars> {
        let shape_vec: Vec<usize> = indices.extract()?;
        if shape_vec.is_empty() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "addVars requires at least one dimension",
            ));
        }
        if shape_vec.contains(&0) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "addVars dimensions must be positive",
            ));
        }
        let num_vars = shape_vec.iter().try_fold(1usize, |product, dimension| {
            product.checked_mul(*dimension).ok_or_else(|| {
                PyErr::new::<pyo3::exceptions::PyOverflowError, _>(
                    "addVars shape product exceeds usize range",
                )
            })
        })?;
        if num_vars > i32::MAX as usize {
            return Err(PyErr::new::<pyo3::exceptions::PyOverflowError, _>(
                "addVars shape product exceeds the solver index range",
            ));
        }
        let lb_param = lb
            .map(Param::from_py)
            .transpose()?
            .unwrap_or(Param::Vector(vec![0.0; num_vars]));
        let ub_param = ub
            .map(Param::from_py)
            .transpose()?
            .unwrap_or(Param::Vector(vec![f64::INFINITY; num_vars]));
        let obj_param = obj
            .map(Param::from_py)
            .transpose()?
            .unwrap_or(Param::Vector(vec![0.0; num_vars]));
        let objective_coefficients = obj_param.to_vec(Some(num_vars));
        if objective_coefficients.len() != num_vars {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "objective coefficients have length {}, expected {num_vars}",
                objective_coefficients.len()
            )));
        }
        validate_objective_coefficients(&objective_coefficients)?;
        let vtype_param = vtype
            .map(Param::from_py)
            .transpose()?
            .unwrap_or(Param::Vector(vec![VarType::CONTINUOUS; num_vars]));
        let name_param = name
            .map(Param::from_py)
            .transpose()?
            .unwrap_or(Param::Scalar("".to_string()));
        // 如果传入参数为单一字符串，则按照shape生成a[0],a[1]或a[0,0],a[0,1]等变量名称
        let name_param = if let Param::Scalar(s) = &name_param {
            let names = generate_names(s, &shape_vec);
            Param::Vector(names)
        } else {
            name_param
        };
        let names = NameType::Vector(name_param.to_vec(Some(num_vars)));
        let variable_types = vtype_param
            .to_vec(Some(num_vars))
            .iter()
            .map(|t| match t {
                VarType::CONTINUOUS => 'C',
                VarType::BINARY => 'B',
                VarType::INTEGER => 'I',
            })
            .collect::<Vec<_>>();
        let lower_bounds = BoundType::Vector(lb_param.to_vec(Some(num_vars)));
        let upper_bounds = BoundType::Vector(ub_param.to_vec(Some(num_vars)));
        let (var_ids, result_source) = match &self.runtime {
            ModelRuntime::Cached(cached) => {
                let shared = cached.bridge();
                let ids = lock_bridge(shared)?
                    .add_variables(
                        num_vars,
                        Some(names),
                        Some(variable_types),
                        Some(lower_bounds),
                        Some(upper_bounds),
                    )
                    .map_err(to_py_runtime_error)?;
                (ids, ResultSource::Cached(shared.clone()))
            }
            ModelRuntime::Direct(direct) => {
                let ids = direct
                    .add_variables(
                        num_vars,
                        Some(names),
                        Some(variable_types),
                        Some(lower_bounds),
                        Some(upper_bounds),
                    )
                    .map_err(to_py_runtime_error)?;
                (ids, ResultSource::Direct(direct.clone()))
            }
            ModelRuntime::Poisoned(state) => {
                return Err(to_py_runtime_error(MoiError::BackendState(format!(
                    "model is poisoned after '{}': {}",
                    state.operation, state.reason
                ))));
            }
        };
        let mut vars = Vars::new(shape_vec.clone(), var_ids);
        match result_source {
            ResultSource::Cached(shared) => vars.set_bridge(&shared),
            ResultSource::Direct(direct) => vars.set_direct(&direct),
        }
        Ok(vars)
    }

    #[pyo3(signature = (constr, name=None),name="addConstr")]
    fn add_constr(&mut self, constr: &Bound<'_, Constr>, name: Option<&str>) -> PyResult<()> {
        if let ModelRuntime::Direct(direct) = &self.runtime {
            let mut rows = LinearRows::new(direct.num_variables());
            constr
                .borrow()
                .append_to(&mut rows)
                .map_err(to_py_runtime_error)?;
            rows.names = name.map(|n| vec![n.to_string()]);
            return direct.add_linear_rows(rows).map_err(to_py_runtime_error);
        }
        let constr: Constr = constr.extract()?;

        match &self.runtime {
            ModelRuntime::Cached(cached) => lock_bridge(cached.bridge())?
                .add_constraint(constr.get_f(), constr.get_s(), name.map(str::to_string))
                .map(|_| ())
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Direct(direct) => direct
                .add_constraint(constr.get_f(), constr.get_s(), name.map(str::to_string))
                .map(|_| ())
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Poisoned(state) => return Err(poisoned_py_error(state)),
        }
        Ok(())
    }
    #[pyo3(signature = (generator, name=None),name="addConstrs")]
    fn add_constrs(
        &mut self,
        generator: &Bound<'_, PyAny>,
        name: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
        if let ModelRuntime::Direct(direct) = &self.runtime {
            let mut rows = LinearRows::new(direct.num_variables());
            // Do not hold the native state lock while evaluating user Python.
            for item in generator.try_iter()? {
                let item = item?;
                let constr = item.extract::<PyRef<'_, Constr>>()?;
                constr.append_to(&mut rows).map_err(to_py_runtime_error)?;
            }
            rows.names = match name.map(Param::<String>::from_py).transpose()? {
                None => None,
                Some(Param::Scalar(base)) => Some(generate_names(&base, &[rows.num_rows()])),
                Some(Param::Vector(names)) => Some(names),
            };
            return direct.add_linear_rows(rows).map_err(to_py_runtime_error);
        }
        let mut fs = Vec::new();
        let mut ss = Vec::new();
        let mut count = 0;
        let _ = generator
            .try_iter()?
            .map(|item| {
                let constr: Constr = item?.extract()?;
                fs.push(constr.get_f());
                ss.push(constr.get_s());
                count += 1;
                Ok(())
            })
            .collect::<PyResult<Vec<_>>>()?;
        let shape_vec = vec![count];
        let name_param = name
            .map(Param::from_py)
            .transpose()?
            .unwrap_or(Param::Scalar("Cons".to_string()));
        // 如果传入参数为单一字符串，则按照shape生成a[0],a[1]或a[0,0],a[0,1]等约束名称
        let name_param = if let Param::Scalar(s) = &name_param {
            let names = generate_names(s, &shape_vec);
            Param::Vector(names)
        } else {
            name_param
        };

        let names = Some(name_param.to_vec(Some(count)));
        match &self.runtime {
            ModelRuntime::Cached(cached) => lock_bridge(cached.bridge())?
                .add_constraints(fs, ss, names)
                .map(|_| ())
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Direct(direct) => direct
                .add_constraints(fs, ss, names)
                .map(|_| ())
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Poisoned(state) => return Err(poisoned_py_error(state)),
        }
        Ok(())
    }
    #[pyo3(signature = (expr, sense),name="setObjective")]
    fn set_objective(&mut self, expr: &Bound<'_, PyAny>, sense: Sense) -> PyResult<()> {
        if let ModelRuntime::Direct(direct) = &self.runtime {
            let obj_expr = expr.extract::<PyRef<'_, LinExpr>>()?;
            let objective =
                LinearObjective::from_affine(direct.num_variables(), obj_expr.get_fn_ref())
                    .map_err(to_py_runtime_error)?;
            let sense = match sense {
                Sense::MINIMIZE => ModelSense::Minimize,
                Sense::MAXIMIZE => ModelSense::Maximize,
            };
            return direct
                .set_linear_objective(objective, sense)
                .map_err(to_py_runtime_error);
        }
        let obj_expr = expr.extract::<LinExpr>()?;
        let function = ScalarFunctionType::Affine(obj_expr.get_fn());
        let sense = match sense {
            Sense::MINIMIZE => ModelSense::Minimize,
            Sense::MAXIMIZE => ModelSense::Maximize,
        };
        match &self.runtime {
            ModelRuntime::Cached(cached) => lock_bridge(cached.bridge())?
                .set_objective(function, sense)
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Direct(direct) => direct
                .set_objective(function, sense)
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Poisoned(state) => return Err(poisoned_py_error(state)),
        }
        Ok(())
    }

    #[pyo3(name = "setParam")]
    fn set_param(&mut self, paramname: &str, newvalue: &Bound<'_, PyAny>) -> PyResult<()> {
        let attr = OptimizerAttr::Raw(paramname.to_string());
        let val = if let Ok(v) = newvalue.extract::<bool>() {
            AttrValue::Bool(v)
        } else if let Ok(v) = newvalue.extract::<i64>() {
            AttrValue::Int(v)
        } else if let Ok(v) = newvalue.extract::<f64>() {
            AttrValue::Float(v)
        } else if let Ok(v) = newvalue.extract::<String>() {
            AttrValue::String(v)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Unsupported attribute value type".to_string(),
            ));
        };

        match &self.runtime {
            ModelRuntime::Cached(cached) => lock_bridge(cached.bridge())?
                .set_optimizer_attr(attr, val)
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Direct(direct) => direct
                .set_optimizer_attr(attr, val)
                .map_err(to_py_runtime_error)?,
            ModelRuntime::Poisoned(state) => return Err(poisoned_py_error(state)),
        }
        Ok(())
    }

    // 选择求解器后端
    #[pyo3(name = "setBackend")]
    #[pyo3(signature = (backend, env=None))]
    fn set_backend(&mut self, py: Python, backend: &str, env: Option<Py<PyAny>>) -> PyResult<()> {
        self.runtime
            .cached("set backend")
            .map_err(to_py_runtime_error)?;
        if backend == "copt" {
            if let Some(env_handle) = env.as_ref() {
                if let Ok(copt_env) = env_handle.bind(py).extract::<PyRef<'_, CoptEnv>>() {
                    let optimizer = create_copt_optimizer(Some(&copt_env))?;
                    return self
                        .runtime
                        .cached_mut("set backend")
                        .map_err(to_py_runtime_error)?
                        .attach_native_backend(optimizer);
                }
            } else {
                let optimizer = create_copt_optimizer(None)?;
                return self
                    .runtime
                    .cached_mut("set backend")
                    .map_err(to_py_runtime_error)?
                    .attach_native_backend(optimizer);
            }
        }

        let model_instance = py
            .import(format!("moirspy_{backend}"))
            .and_then(|module| module.getattr("Model"))
            .and_then(|model_class| match env {
                Some(env) => model_class.call1((Some(self.name.to_string()), None::<String>, env)),
                None => model_class.call1((Some(self.name.to_string()), None::<String>)),
            })
            .map_err(|error| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Failed to initialize backend '{backend}': {error}"
                ))
            })?;

        self.runtime
            .cached_mut("set backend")
            .map_err(to_py_runtime_error)?
            .attach_python_backend(model_instance)
    }
    // 调用底层求解器进行优化
    fn optimize(&mut self, _py: Python) -> PyResult<()> {
        match &self.runtime {
            ModelRuntime::Cached(cached) => lock_bridge(cached.bridge())?
                .optimize()
                .map(|_| ())
                .map_err(to_py_runtime_error),
            ModelRuntime::Direct(direct) => {
                direct.optimize().map(|_| ()).map_err(to_py_runtime_error)
            }
            ModelRuntime::Poisoned(state) => Err(poisoned_py_error(state)),
        }
    }
    fn update(&mut self) -> PyResult<()> {
        match &self.runtime {
            ModelRuntime::Cached(cached) => lock_bridge(cached.bridge())?
                .update()
                .map_err(to_py_runtime_error),
            ModelRuntime::Direct(direct) => direct.update().map_err(to_py_runtime_error),
            ModelRuntime::Poisoned(state) => Err(poisoned_py_error(state)),
        }
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("Model(name={})", self.name))
    }
    #[getter]
    #[pyo3(name = "ObjVal")]
    pub fn get_objval(&self) -> PyResult<Option<f64>> {
        match &self.runtime {
            ModelRuntime::Cached(cached) => lock_bridge(cached.bridge())?
                .get_objective_value()
                .map_err(to_py_runtime_error),
            ModelRuntime::Direct(direct) => {
                direct.get_objective_value().map_err(to_py_runtime_error)
            }
            ModelRuntime::Poisoned(state) => Err(poisoned_py_error(state)),
        }
    }
}

fn poisoned_py_error(state: &crate::runtime::PoisonedState) -> PyErr {
    to_py_runtime_error(MoiError::BackendState(format!(
        "model is poisoned after '{}': {}",
        state.operation, state.reason
    )))
}

fn validate_objective_coefficients(values: &[f64]) -> PyResult<()> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "obj coefficients must be finite",
        ));
    }
    if values.iter().any(|value| *value != 0.0) {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "nonzero obj coefficients are not supported; use setObjective instead",
        ));
    }
    Ok(())
}
