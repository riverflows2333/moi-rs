use crate::env::{Env, attr_value_from_py, load_api, to_py_runtime_error};
use moi_core::*;
use moi_solver_api::*;
use moi_solver_copt::{CoptEnv, CoptOptimizer};
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

/// Python adapter for the native COPT optimizer.
#[pyclass(unsendable)]
pub struct Model {
    optimizer: CoptOptimizer,
}

#[pymethods]
impl Model {
    #[new]
    #[pyo3(signature = (name=None, dll_path=None, env=None, license_dir=None))]
    pub fn new(
        name: Option<&str>,
        dll_path: Option<String>,
        env: Option<PyRef<'_, Env>>,
        license_dir: Option<String>,
    ) -> PyResult<Self> {
        let _ = name;
        let env = if let Some(env) = env {
            if dll_path.is_some() || license_dir.is_some() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "env cannot be provided together with dll_path or license_dir",
                ));
            }
            env.inner.clone()
        } else {
            let api = load_api(dll_path)?;
            let env = match license_dir {
                Some(path) => CoptEnv::with_license_dir(api, &path),
                None => CoptEnv::new(api),
            }
            .map_err(to_py_runtime_error)?;
            Arc::new(Mutex::new(env))
        };
        let optimizer = CoptOptimizer::new(env).map_err(to_py_runtime_error)?;
        Ok(Self { optimizer })
    }

    #[pyo3(signature = (name=None, vtype=None, lb=None, ub=None))]
    pub fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> PyResult<usize> {
        self.optimizer
            .add_variable(name, vtype, lb, ub)
            .map(|id| id.0)
            .map_err(to_py_runtime_error)
    }

    #[pyo3(signature = (n, names=None, vtypes=None, lbs=None, ubs=None))]
    pub fn add_variables(
        &mut self,
        n: usize,
        names: Option<Vec<String>>,
        vtypes: Option<Vec<char>>,
        lbs: Option<Vec<f64>>,
        ubs: Option<Vec<f64>>,
    ) -> PyResult<Vec<usize>> {
        let ids = self
            .optimizer
            .add_variables(
                n,
                names.map(NameType::Vector),
                vtypes,
                lbs.map(BoundType::Vector),
                ubs.map(BoundType::Vector),
            )
            .map_err(to_py_runtime_error)?;
        Ok(ids.into_iter().map(|id| id.0).collect())
    }

    #[pyo3(signature = (vars, coeffs, constant, sense, rhs, name=None))]
    pub fn add_constraint(
        &mut self,
        vars: Vec<usize>,
        coeffs: Vec<f64>,
        constant: f64,
        sense: char,
        rhs: f64,
        name: Option<String>,
    ) -> PyResult<usize> {
        ensure_py_len(coeffs.len(), vars.len(), "coefficients")?;
        let function = affine_function(&vars, &coeffs, constant);
        let set = scalar_set(sense, rhs)?;
        self.optimizer
            .add_constraint(function, set, name)
            .map(|id| id.0)
            .map_err(to_py_runtime_error)
    }

    #[pyo3(signature = (fs_vars, fs_coeffs, fs_consts, senses, rhss, names=None))]
    pub fn add_constraints(
        &mut self,
        fs_vars: Vec<Vec<usize>>,
        fs_coeffs: Vec<Vec<f64>>,
        fs_consts: Vec<f64>,
        senses: Vec<char>,
        rhss: Vec<f64>,
        names: Option<Vec<String>>,
    ) -> PyResult<Vec<usize>> {
        let n = fs_vars.len();
        ensure_py_len(fs_coeffs.len(), n, "constraint coefficient rows")?;
        ensure_py_len(fs_consts.len(), n, "constraint constants")?;
        ensure_py_len(senses.len(), n, "constraint senses")?;
        ensure_py_len(rhss.len(), n, "constraint right-hand sides")?;
        if let Some(ref names) = names {
            ensure_py_len(names.len(), n, "constraint names")?;
        }

        let mut functions = Vec::with_capacity(n);
        for index in 0..n {
            ensure_py_len(
                fs_coeffs[index].len(),
                fs_vars[index].len(),
                "constraint coefficients",
            )?;
            functions.push(affine_function(
                &fs_vars[index],
                &fs_coeffs[index],
                fs_consts[index],
            ));
        }
        let sets = senses
            .into_iter()
            .zip(rhss)
            .map(|(sense, rhs)| scalar_set(sense, rhs))
            .collect::<PyResult<Vec<_>>>()?;
        let ids = self
            .optimizer
            .add_constraints(functions, sets, names)
            .map_err(to_py_runtime_error)?;
        Ok(ids.into_iter().map(|id| id.0).collect())
    }

    pub fn set_objective(
        &mut self,
        vars: Vec<usize>,
        coeffs: Vec<f64>,
        constant: f64,
        sense: i32,
    ) -> PyResult<()> {
        ensure_py_len(coeffs.len(), vars.len(), "objective coefficients")?;
        let sense = if sense == 1 {
            ModelSense::Maximize
        } else {
            ModelSense::Minimize
        };
        self.optimizer
            .set_objective(affine_function(&vars, &coeffs, constant), sense)
            .map_err(to_py_runtime_error)
    }

    pub fn update(&mut self) -> PyResult<()> {
        self.optimizer.update().map_err(to_py_runtime_error)
    }

    pub fn optimize(&mut self) -> PyResult<u32> {
        self.optimizer
            .optimize()
            .map(SolveStatus::code)
            .map_err(to_py_runtime_error)
    }

    pub fn get_var_value(&self, var_id: usize) -> Option<f64> {
        self.optimizer.get_var_value(VarId(var_id))
    }

    pub fn get_objective_value(&self) -> Option<f64> {
        self.optimizer.get_objective_value()
    }

    pub fn set_optimizer_attr(&mut self, attr: String, value: &Bound<'_, PyAny>) -> PyResult<()> {
        let attr = match attr.as_str() {
            "TimeLimit" => OptimizerAttr::TimeLimit,
            "Silent" => OptimizerAttr::Silent,
            name => OptimizerAttr::Raw(name.to_owned()),
        };
        let value = attr_value_from_py(value)?;
        self.optimizer
            .set_optimizer_attr(attr, value)
            .map_err(to_py_runtime_error)
    }
}

fn affine_function(vars: &[usize], coeffs: &[f64], constant: f64) -> ScalarFunctionType {
    ScalarFunctionType::Affine(ScalarAffineFn {
        terms: vars
            .iter()
            .zip(coeffs)
            .map(|(variable, coefficient)| AffineTerm {
                var: VarId(*variable),
                coeff: *coefficient,
            })
            .collect(),
        constant,
    })
}

fn scalar_set(sense: char, rhs: f64) -> PyResult<ScalarSetType> {
    match sense {
        '<' => Ok(ScalarSetType::LessThan(rhs)),
        '>' => Ok(ScalarSetType::GreaterThan(rhs)),
        '=' => Ok(ScalarSetType::EqualTo(rhs)),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "invalid sense character: {sense}"
        ))),
    }
}

fn ensure_py_len(actual: usize, expected: usize, field: &str) -> PyResult<()> {
    if actual == expected {
        Ok(())
    } else {
        Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "{field} has length {actual}, expected {expected}"
        )))
    }
}
