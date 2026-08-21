use crate::constr::Constr;
use crate::expr::LinExpr;
use crate::moi::*;
use crate::utils::*;
use crate::var::*;
use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyTuple};
use std::sync::{Arc, Mutex};
#[pyclass]
pub struct Model {
    name: String,
    model: SharedBridge,
    backend: Option<Py<PyAny>>,
}

#[pymethods]
impl Model {
    #[new]
    fn new(name: String) -> Self {
        Model {
            name,
            model: Arc::new(Mutex::new(BridgeOptimizer::new())),
            backend: None,
        }
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
        let mut model = lock_bridge(&self.model)?;
        let var_id = model
            .add_variable(
                Some(name),
                vtype.map(|t| match t {
                    VarType::CONTINUOUS => 'C',
                    VarType::BINARY => 'B',
                    VarType::INTEGER => 'I',
                }),
                Some(lb),
                Some(ub),
            )
            .map_err(to_py_runtime_error)?;
        let mut var = Var::new(var_id.0);
        var.set_bridge(&self.model);
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
        let mut model = lock_bridge(&self.model)?;
        let varids = model
            .add_variables(
                num_vars,
                Some(NameType::Vector(name_param.to_vec(Some(num_vars)))),
                Some(
                    vtype_param
                        .to_vec(Some(num_vars))
                        .iter()
                        .map(|t| match t {
                            VarType::CONTINUOUS => 'C',
                            VarType::BINARY => 'B',
                            VarType::INTEGER => 'I',
                        })
                        .collect(),
                ),
                Some(BoundType::Vector(lb_param.to_vec(Some(num_vars)))),
                Some(BoundType::Vector(ub_param.to_vec(Some(num_vars)))),
            )
            .map_err(to_py_runtime_error)?;
        let var_ids = varids;
        let mut vars = Vars::new(shape_vec.clone(), var_ids);
        vars.set_bridge(&self.model);
        Ok(vars)
    }

    #[pyo3(signature = (constr, name=None),name="addConstr")]
    fn add_constr(&mut self, constr: &Bound<'_, Constr>, name: Option<&str>) -> PyResult<()> {
        let constr: Constr = constr.extract()?;

        let mut model = lock_bridge(&self.model)?;
        model
            .add_constraint(constr.get_f(), constr.get_s(), name.map(str::to_string))
            .map_err(to_py_runtime_error)?;
        Ok(())
    }
    #[pyo3(signature = (generator, name=None),name="addConstrs")]
    fn add_constrs(
        &mut self,
        generator: &Bound<'_, PyAny>,
        name: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<()> {
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

        let mut model = lock_bridge(&self.model)?;
        model
            .add_constraints(fs, ss, Some(name_param.to_vec(Some(count))))
            .map_err(to_py_runtime_error)?;
        Ok(())
    }
    #[pyo3(signature = (expr, sense),name="setObjective")]
    fn set_objective(&mut self, expr: &Bound<'_, PyAny>, sense: Sense) -> PyResult<()> {
        let obj_expr = expr.extract::<LinExpr>()?;
        let mut model = lock_bridge(&self.model)?;
        model
            .set_objective(
                ScalarFunctionType::Affine(obj_expr.get_fn()),
                match sense {
                    Sense::MINIMIZE => ModelSense::Minimize,
                    Sense::MAXIMIZE => ModelSense::Maximize,
                },
            )
            .map_err(to_py_runtime_error)?;
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

        let mut model = lock_bridge(&self.model)?;
        model
            .set_optimizer_attr(attr, val)
            .map_err(to_py_runtime_error)?;
        Ok(())
    }

    // 选择求解器后端
    #[pyo3(name = "setBackend")]
    #[pyo3(signature = (backend, env=None))]
    fn set_backend(&mut self, py: Python, backend: &str, env: Option<Py<PyAny>>) -> PyResult<()> {
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

        use crate::py_backend::PyBackend;
        let py_backend = Box::new(PyBackend::new(model_instance.clone().into()));

        let mut bridge = lock_bridge(&self.model)?;
        bridge
            .attach_backend(py_backend)
            .map_err(to_py_runtime_error)?;

        self.backend = Some(model_instance.into());
        Ok(())
    }
    // 调用底层求解器进行优化
    fn optimize(&mut self, _py: Python) -> PyResult<()> {
        let mut bridge = lock_bridge(&self.model)?;
        bridge.optimize().map(|_| ()).map_err(to_py_runtime_error)
    }
    fn __str__(&self) -> PyResult<String> {
        Ok(format!("Model(name={})", self.name))
    }
    #[getter]
    #[pyo3(name = "ObjVal")]
    pub fn get_objval(&self) -> PyResult<Option<f64>> {
        let bridge = lock_bridge(&self.model)?;
        bridge.get_objective_value().map_err(to_py_runtime_error)
    }
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
