use crate::env::{Env, attr_value_from_py, load_api, to_py_error};
use moi_core::*;
use moi_solver_api::*;
use moi_solver_gurobi::*;
use numpy::PyReadonlyArray1;
use pyo3::prelude::*;
use std::sync::{Arc, Mutex};

#[pyclass(unsendable)]
pub struct Model {
    optimizer: GurobiOptimizer,
}

#[pymethods]
impl Model {
    #[new]
    #[pyo3(signature = (name=None, dll_path=None, env=None))]
    pub fn new(
        name: Option<&str>,
        dll_path: Option<String>,
        env: Option<PyRef<'_, Env>>,
    ) -> PyResult<Self> {
        let env = if let Some(env) = env {
            if dll_path.is_some() {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "dll_path and env cannot be provided together",
                ));
            }
            env.inner.clone()
        } else {
            let api = load_api(dll_path)?;
            let env =
                GurobiEnv::new(api).map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;
            Arc::new(Mutex::new(env))
        };
        let optimizer = GurobiOptimizer::new(env, name)
            .map_err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>)?;
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
            .map_err(to_py_error)
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
        let names_arg = names.map(NameType::Vector);
        let lbs_arg = lbs.map(BoundType::Vector);
        let ubs_arg = ubs.map(BoundType::Vector);
        let ids = self
            .optimizer
            .add_variables(n, names_arg, vtypes, lbs_arg, ubs_arg)
            .map_err(to_py_error)?;
        Ok(ids.iter().map(|id| id.0).collect())
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
        let f = ScalarFunctionType::Affine(ScalarAffineFn {
            terms: vars
                .iter()
                .zip(coeffs.iter())
                .map(|(v, c)| AffineTerm {
                    var: VarId(*v),
                    coeff: *c,
                })
                .collect(),
            constant,
        });
        let s = match sense {
            '<' => ScalarSetType::LessThan(rhs),
            '>' => ScalarSetType::GreaterThan(rhs),
            '=' => ScalarSetType::EqualTo(rhs),
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "invalid sense character: {sense}"
                )));
            }
        };
        self.optimizer
            .add_constraint(f, s, name)
            .map(|id| id.0)
            .map_err(to_py_error)
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
        ensure_py_len(
            fs_coeffs.len(),
            fs_vars.len(),
            "constraint coefficient rows",
        )?;
        ensure_py_len(fs_consts.len(), fs_vars.len(), "constraint constants")?;
        ensure_py_len(senses.len(), fs_vars.len(), "constraint senses")?;
        ensure_py_len(rhss.len(), fs_vars.len(), "constraint right-hand sides")?;
        if let Some(ref values) = names {
            ensure_py_len(values.len(), fs_vars.len(), "constraint names")?;
        }
        let mut fs = Vec::with_capacity(fs_vars.len());
        for i in 0..fs_vars.len() {
            ensure_py_len(
                fs_coeffs[i].len(),
                fs_vars[i].len(),
                "constraint coefficients",
            )?;
            fs.push(ScalarFunctionType::Affine(ScalarAffineFn {
                terms: fs_vars[i]
                    .iter()
                    .zip(fs_coeffs[i].iter())
                    .map(|(v, c)| AffineTerm {
                        var: VarId(*v),
                        coeff: *c,
                    })
                    .collect(),
                constant: fs_consts[i],
            }));
        }
        let ss = senses
            .into_iter()
            .zip(rhss)
            .map(|(sense, rhs)| match sense {
                '<' => Ok(ScalarSetType::LessThan(rhs)),
                '>' => Ok(ScalarSetType::GreaterThan(rhs)),
                '=' => Ok(ScalarSetType::EqualTo(rhs)),
                _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "invalid sense character: {sense}"
                ))),
            })
            .collect::<PyResult<Vec<_>>>()?;
        let ids = self
            .optimizer
            .add_constraints(fs, ss, names)
            .map_err(to_py_error)?;
        Ok(ids.iter().map(|id| id.0).collect())
    }

    #[allow(clippy::too_many_arguments)]
    #[pyo3(signature = (row_offsets, columns, values, constants, senses, rhss, names=None))]
    pub fn add_constraints_flat(
        &mut self,
        row_offsets: PyReadonlyArray1<'_, usize>,
        columns: PyReadonlyArray1<'_, usize>,
        values: PyReadonlyArray1<'_, f64>,
        constants: PyReadonlyArray1<'_, f64>,
        senses: PyReadonlyArray1<'_, u8>,
        rhss: PyReadonlyArray1<'_, f64>,
        names: Option<Vec<String>>,
    ) -> PyResult<(usize, usize)> {
        let row_offsets = row_offsets.as_slice()?;
        let columns = columns.as_slice()?;
        let values = values.as_slice()?;
        let constants = constants.as_slice()?;
        let senses = senses.as_slice()?;
        let rhss = rhss.as_slice()?;
        let n = constants.len();
        validate_flat_rows(
            row_offsets,
            columns.len(),
            values.len(),
            senses.len(),
            rhss.len(),
            names.as_ref().map(Vec::len),
            n,
        )?;

        let mut functions = Vec::with_capacity(n);
        let mut sets = Vec::with_capacity(n);
        for row in 0..n {
            let start = row_offsets[row];
            let end = row_offsets[row + 1];
            functions.push(affine_function(
                &columns[start..end],
                &values[start..end],
                constants[row],
            ));
            sets.push(scalar_set(senses[row] as char, rhss[row])?);
        }
        let ids = self
            .optimizer
            .add_constraints(functions, sets, names)
            .map_err(to_py_error)?;
        let start = ids.first().map_or(0, |id| id.0);
        Ok((start, ids.len()))
    }

    pub fn set_objective(
        &mut self,
        vars: Vec<usize>,
        coeffs: Vec<f64>,
        constant: f64,
        sense: i32,
    ) -> PyResult<()> {
        ensure_py_len(coeffs.len(), vars.len(), "objective coefficients")?;
        let f = ScalarFunctionType::Affine(ScalarAffineFn {
            terms: vars
                .iter()
                .zip(coeffs.iter())
                .map(|(v, c)| AffineTerm {
                    var: VarId(*v),
                    coeff: *c,
                })
                .collect(),
            constant,
        });
        let s = match sense {
            1 => ModelSense::Maximize,
            -1 => ModelSense::Minimize,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "objective sense must be 1 (maximize) or -1 (minimize), got {sense}"
                )));
            }
        };
        self.optimizer.set_objective(f, s).map_err(to_py_error)?;
        Ok(())
    }

    pub fn set_objective_flat(
        &mut self,
        vars: PyReadonlyArray1<'_, usize>,
        coeffs: PyReadonlyArray1<'_, f64>,
        constant: f64,
        sense: i32,
    ) -> PyResult<()> {
        let vars = vars.as_slice()?;
        let coeffs = coeffs.as_slice()?;
        ensure_py_len(coeffs.len(), vars.len(), "objective coefficients")?;
        let sense = objective_sense(sense)?;
        self.optimizer
            .set_objective(affine_function(vars, coeffs, constant), sense)
            .map_err(to_py_error)
    }

    pub fn update(&mut self) -> PyResult<()> {
        self.optimizer.update().map_err(to_py_error)
    }

    pub fn optimize(&mut self) -> PyResult<u32> {
        let status = self.optimizer.optimize().map_err(to_py_error)?;
        Ok(status.code())
    }

    pub fn get_var_value(&self, var_id: usize) -> PyResult<Option<f64>> {
        self.optimizer
            .get_var_value(VarId(var_id))
            .map_err(to_py_error)
    }

    pub fn get_objective_value(&self) -> PyResult<Option<f64>> {
        self.optimizer.get_objective_value().map_err(to_py_error)
    }

    pub fn set_optimizer_attr(
        &mut self,
        attr: String,
        value: &pyo3::Bound<'_, pyo3::PyAny>,
    ) -> PyResult<()> {
        let attr_enum = match attr.as_str() {
            "TimeLimit" => OptimizerAttr::TimeLimit,
            "Silent" => OptimizerAttr::Silent,
            s => OptimizerAttr::Raw(s.to_string()),
        };

        let val = attr_value_from_py(value)?;

        self.optimizer
            .set_optimizer_attr(attr_enum, val)
            .map_err(to_py_error)?;
        Ok(())
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

fn objective_sense(sense: i32) -> PyResult<ModelSense> {
    match sense {
        1 => Ok(ModelSense::Maximize),
        -1 => Ok(ModelSense::Minimize),
        _ => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "objective sense must be 1 (maximize) or -1 (minimize), got {sense}"
        ))),
    }
}

fn validate_flat_rows(
    row_offsets: &[usize],
    column_count: usize,
    value_count: usize,
    sense_count: usize,
    rhs_count: usize,
    name_count: Option<usize>,
    row_count: usize,
) -> PyResult<()> {
    ensure_py_len(row_offsets.len(), row_count + 1, "row offsets")?;
    ensure_py_len(value_count, column_count, "flat constraint coefficients")?;
    ensure_py_len(sense_count, row_count, "constraint senses")?;
    ensure_py_len(rhs_count, row_count, "constraint right-hand sides")?;
    if let Some(name_count) = name_count {
        ensure_py_len(name_count, row_count, "constraint names")?;
    }
    if row_offsets.first().copied() != Some(0) {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "row offsets must start at zero",
        ));
    }
    if row_offsets.last().copied() != Some(column_count) {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
            "last row offset must equal nonzero count {column_count}"
        )));
    }
    if row_offsets
        .windows(2)
        .any(|offsets| offsets[0] > offsets[1])
    {
        return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
            "row offsets must be nondecreasing",
        ));
    }
    Ok(())
}
