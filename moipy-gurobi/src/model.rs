use std::sync::Arc;

use crate::loader::*;
use bincode::config;
use moi_bridge::BridgeOptimizer;
use moi_solver_api::optimizer::Optimizer;
use moi_solver_gurobi::*;
use pyo3::prelude::*;
use std::path::PathBuf;

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
        let optimizer = GurobiOptimizer::new(Arc::new(api), name).unwrap();
        Self { optimizer }
    }
    pub fn decode_and_update(&mut self, data: &[u8]) -> PyResult<String> {
        let config = config::standard();
        let model: BridgeOptimizer = bincode::decode_from_slice(data, config)
            .map_err(|e| {
                PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                    "Deserialization error: {}",
                    e
                ))
            })?
            .0;
        let _ = self.optimizer.update(Some(model));
        Ok("Model updated successfully".into())
    }
    pub fn optimize(&mut self) -> PyResult<String> {
        let _ = self.optimizer.optimize().map_err(|e| {
            PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(format!(
                "Optimization error: {}",
                e
            ))
        })?;
        Ok("Optimization completed".into())
    }
}

impl Model {
    //
}
