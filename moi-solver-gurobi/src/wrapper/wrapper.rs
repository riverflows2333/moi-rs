use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use moi_core::*;
use moi_solver_api::{ModelLike, Optimizer};
use std::collections::HashMap;
use std::ffi::{CString, c_double, c_int, c_void};
use std::sync::Arc;
pub struct GurobiOptimizer {
    api: Arc<GurobiApi>,
    env: *mut c_void,
    model: *mut c_void,
    needs_update: bool,
    vars: Vec<VarInfo>,
    constrs: HashMap<ConstrId, ConstrInfo>, // 使用 Gurobi 行索引作为键
}

struct VarInfo {
    col_index: usize, // Gurobi 内部的列索引 (0, 1, 2...)
    // 缓存上下界，避免频繁查询 C API
    lb: f64,
    ub: f64,
    vtype: char, // 'C', 'B', 'I'
    name: String
}

struct ConstrInfo {
    row_index: usize, // Gurobi 内部的行索引
    name: String,     // 可以在这里存约束类型，方便后续查询
    f: ScalarFunctionType,
    s: ScalarSetType,
}

impl GurobiOptimizer {
    pub fn new(api: Arc<GurobiApi>) -> Result<Self, String> {
        unsafe {
            let mut env: *mut c_void = std::ptr::null_mut();
            let ret = (api.GRBloadenv)(&mut env as *mut *mut c_void, std::ptr::null());
            if ret != 0 {
                return Err(format!(
                    "Failed to load Gurobi environment: error code {}",
                    ret
                ));
            }
            Ok(Self {
                api,
                env,
                model: std::ptr::null_mut(),
                needs_update: false,
                vars: Vec::new(),
                constrs: HashMap::new(),
            })
        }
    }
}

impl ModelLike for GurobiOptimizer {
    fn add_variable(&mut self, name: Option<&str>) -> VarId {
        // Implementation of adding a single variable
        let var_id = self.vars.len();
        // Add variable to Gurobi model here
        self.vars.push(VarInfo {
            col_index: var_id,
            lb: 0.0,
            ub: f64::INFINITY,
            vtype: 'C',
            name: name.unwrap_or("").to_string(),
        });
        self.needs_update = true;
        VarId(var_id)
    }
    fn add_variables(&mut self, n: usize, name: Option<&str>) -> Vec<VarId> {
        // Implementation of adding multiple variables
        let start_id = self.vars.len();
        for i in 0..n {
            let var_id = start_id + i;
            self.vars.push(VarInfo {
                col_index: var_id,
                lb: 0.0,
                ub: f64::INFINITY,
                vtype: 'C',
                name: format!("{}{}", name.unwrap_or(""), var_id),
            });
        }
        self.needs_update = true;
        (start_id..start_id + n).map(VarId).collect()
    }
    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId {
        // Implementation of adding a constraint
        let constr_id = self.constrs.len();
        // Add constraint to Gurobi model here
        self.constrs.insert(
            ConstrId(constr_id),
            ConstrInfo {
                row_index: constr_id,
                name: "".to_string(),
                f,
                s,
            },
        );
        self.needs_update = true;
        ConstrId(constr_id)
    }
    
}

impl Drop for GurobiOptimizer {
    fn drop(&mut self) {
        unsafe {
            if !self.model.is_null() {
                (self.api.GRBfreemodel)(self.model);
            }
            if !self.env.is_null() {
                (self.api.GRBfreeenv)(self.env);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::loader::find_library_from;
    #[test]
    fn test_gurobi_solver_creation() {
        let gurobi_api =
            GurobiApi::new(find_library_from("/usr/local/gurobi1203".to_string()).unwrap())
                .unwrap();
        let solver = GurobiOptimizer::new(Arc::new(gurobi_api));
        assert!(solver.is_ok());
    }
}
