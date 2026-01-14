use crate::dynamic::api::GurobiApi;
use moi_solver_api::Optimizer;
use std::ffi::c_void;
use std::sync::Arc;
use crate::bindings::*;
pub struct GurobiSolver {
    api: Arc<GurobiApi>,
    env: *mut c_void,
    model: *mut c_void,
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
            })
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
