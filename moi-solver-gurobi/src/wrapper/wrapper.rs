use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use moi_core::ConstrId;
use moi_solver_api::Optimizer;
use std::collections::HashMap;
use std::ffi::{CString, c_double, c_int, c_void};
use std::sync::Arc;
pub struct GurobiSolver {
    api: Arc<GurobiApi>,
    env: *mut c_void,
    model: *mut c_void,
    needs_update: bool,
    vars: Vec<VarInfo>,
    constrs: HashMap<ConstrId, ConstrInfo>, // 使用 Gurobi 行索引作为键
}

struct VarInfo {
    col_index: c_int, // Gurobi 内部的列索引 (0, 1, 2...)
    // 缓存上下界，避免频繁查询 C API
    lb: f64,
    ub: f64,
    vtype: char, // 'C', 'B', 'I'
}

struct ConstrInfo {
    row_index: c_int, // Gurobi 内部的行索引
                      // 可以在这里存约束类型，方便后续查询
}

impl GurobiSolver {
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

impl Drop for GurobiSolver {
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
        let solver = GurobiSolver::new(Arc::new(gurobi_api));
        assert!(solver.is_ok());
    }
}
