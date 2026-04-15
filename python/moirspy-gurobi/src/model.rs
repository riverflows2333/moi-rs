use crate::loader::*;
use moi_core::*;
use moi_solver_api::*;
use moi_solver_gurobi::*;
use pyo3::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

#[pyclass]
pub struct Model {
    optimizer: GurobiOptimizer,
}

#[pymethods]
impl Model {
    #[new]
    #[pyo3(signature = (name=None, dll_path=None))]
    pub fn new(name: Option<&str>, dll_path: Option<String>) -> Self {
        let loader;
        if let Some(path) = dll_path {
            loader = EnvLoader::LibPath(path);
        } else {
            println!(
                "No DLL path provided, attempting to load Gurobi library from environment variables and common locations."
            );
            loader = match load_gurobi(None) {
                Ok(l) => l,
                Err(e) => panic!("Failed to load Gurobi library: {}", e),
            };
        }
        let api =
            GurobiApi::new(PathBuf::from(loader_to_dll_path(&loader.clone()).unwrap())).unwrap();
        let api_arc = Arc::new(api);
        let env = Arc::new(GurobiEnv::new(api_arc).unwrap());
        let optimizer = GurobiOptimizer::new(env, name).unwrap();
        Self { optimizer }
    }

    #[pyo3(signature = (name=None, vtype=None, lb=None, ub=None))]
    pub fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> PyResult<usize> {
        Ok(self.optimizer.add_variable(name, vtype, lb, ub).0)
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
        let names_arg = names.map(|v| moi_solver_api::utils::NameType::Vector(v));
        let lbs_arg = lbs.map(|v| moi_solver_api::utils::BoundType::Vector(v));
        let ubs_arg = ubs.map(|v| moi_solver_api::utils::BoundType::Vector(v));
        let ids = self
            .optimizer
            .add_variables(n, names_arg, vtypes, lbs_arg, ubs_arg);
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
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    "Invalid sense character",
                ));
            }
        };
        Ok(self.optimizer.add_constraint(f, s, name).0)
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
        let mut fs = Vec::with_capacity(fs_vars.len());
        for i in 0..fs_vars.len() {
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
            .zip(rhss.into_iter())
            .map(|(sense, rhs)| match sense {
                '<' => ScalarSetType::LessThan(rhs),
                '>' => ScalarSetType::GreaterThan(rhs),
                '=' => ScalarSetType::EqualTo(rhs),
                _ => ScalarSetType::EqualTo(rhs),
            })
            .collect();
        let ids = self.optimizer.add_constraints(fs, ss, names);
        Ok(ids.iter().map(|id| id.0).collect())
    }

    pub fn set_objective(
        &mut self,
        vars: Vec<usize>,
        coeffs: Vec<f64>,
        constant: f64,
        sense: i32,
    ) -> PyResult<()> {
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
        let s = if sense == 1 {
            ModelSense::Maximize
        } else {
            ModelSense::Minimize
        };
        self.optimizer
            .set_objective(f, s)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", e)))?;
        Ok(())
    }

    pub fn update(&mut self) -> PyResult<()> {
        self.optimizer
            .update()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", e)))
    }

    pub fn optimize(&mut self) -> PyResult<u32> {
        let status = self.optimizer.optimize().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Optimization error: {:?}",
                e
            ))
        })?;
        Ok(status as u32)
    }

    pub fn get_var_value(&self, var_id: usize) -> Option<f64> {
        self.optimizer.get_var_value(VarId(var_id))
    }

    pub fn get_objective_value(&self) -> Option<f64> {
        self.optimizer.get_objective_value()
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

        let val = if let Ok(v) = value.extract::<bool>() {
            AttrValue::Bool(v) // order matters, extract bool first before int/float in some cases, although pyo3 is smart
        } else if let Ok(v) = value.extract::<i64>() {
            AttrValue::Int(v)
        } else if let Ok(v) = value.extract::<f64>() {
            AttrValue::Float(v)
        } else if let Ok(v) = value.extract::<String>() {
            AttrValue::String(v)
        } else {
            return Err(PyErr::new::<pyo3::exceptions::PyTypeError, _>(
                "Unsupported attribute value type",
            ));
        };

        self.optimizer
            .set_optimizer_attr(attr_enum, val)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!("{:?}", e)))?;
        Ok(())
    }
}
