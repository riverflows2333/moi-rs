use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use moi_core::*;
use moi_solver_api::{ModelLike, Optimizer};
use std::collections::HashMap;
use std::ffi::{CString, c_char, c_double, c_int, c_void};
use std::sync::Arc;
use crate::wrapper::utils::*;
pub struct GurobiOptimizer {
    api: Arc<GurobiApi>,
    env: *mut c_void,
    model: *mut c_void,
    needs_update: bool,
    obj: Option<ScalarFunctionType>,
    vars: Vec<VarInfo>,
    constrs: HashMap<ConstrId, ConstrInfo>, // 使用 Gurobi 行索引作为键
}

struct VarInfo {
    col_index: usize, // Gurobi 内部的列索引 (0, 1, 2...)
    // 缓存上下界，避免频繁查询 C API
    lb: f64,
    ub: f64,
    vtype: char, // 'C', 'B', 'I'
    name: String,
}

pub struct ConstrInfo {
    pub row_index: usize, // Gurobi 内部的行索引
    pub name: String,     // 可以在这里存约束类型，方便后续查询
    pub f: ScalarFunctionType,
    pub s: ScalarSetType,
}

impl GurobiOptimizer {
    pub fn new(api: Arc<GurobiApi>, name: Option<&str>) -> Result<Self, String> {
        unsafe {
            let mut env: *mut c_void = std::ptr::null_mut();
            let ret = (api.GRBloadenv)(&mut env as *mut *mut c_void, std::ptr::null());
            if ret != 0 {
                return Err(format!(
                    "Failed to load Gurobi environment: error code {}",
                    ret
                ));
            }
            let ret = (api.GRBstartenv)(env);
            if ret != 0 {
                return Err(format!(
                    "Failed to start Gurobi environment: error code {}",
                    ret
                ));
            }
            let mut model: *mut c_void = std::ptr::null_mut();
            let cname = match name {
                Some(n) => CString::new(n).unwrap().as_ptr(),
                None => std::ptr::null(),
            };
            let ret = (api.GRBnewmodel)(
                env,
                &mut model as *mut *mut c_void,
                cname,
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );
            Ok(Self {
                api,
                env,
                model,
                needs_update: false,
                vars: Vec::new(),
                constrs: HashMap::new(),
                obj: None,
            })
        }
    }
    pub fn update(&mut self) -> Result<(), String> {
        let mut ret = 0;
        if !self.needs_update {
            return Ok(());
        }
        // 更新变量
        let numvars = self.vars.len() as c_int;
        let lb = self.vars.iter().map(|v| v.lb).collect::<Vec<f64>>();
        let ub = self.vars.iter().map(|v| v.ub).collect::<Vec<f64>>();
        let vtype = self
            .vars
            .iter()
            .map(|v| v.vtype as c_char)
            .collect::<Vec<c_char>>();
        // TODO:add obj coefficients logic
        unsafe {
            ret = (self.api.GRBaddvars)(
                self.model as *mut c_void,
                numvars,
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                lb.as_ptr(),
                ub.as_ptr(),
                vtype.as_ptr(),
                std::ptr::null(),
            );
            if ret != 0 {
                return Err(format!("Failed to add variables: error code {}", ret));
            }
        }
        // 更新约束
        for (_cid, constr) in &self.constrs {
            let (var_ids, coeffs, senses, rhss) = scalar_constraint_to_grb(constr)?;
            let numnz = var_ids.len() as c_int;
            let vind: Vec<c_int> = var_ids.iter().map(|v| v.0 as c_int).collect();
            unsafe {
                ret = (self.api.GRBaddconstr)(
                    self.model as *mut c_void,
                    numnz,
                    vind.as_ptr(),
                    coeffs.as_ptr(),
                    senses[0] as c_char,
                    rhss[0],
                    std::ptr::null(),
                );
                if ret != 0 {
                    return Err(format!("Failed to add constraint: error code {}", ret));
                }
            }
        }
        Ok(())
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
    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
    ) -> Vec<ConstrId> {
        fs.into_iter()
            .zip(ss.into_iter())
            .map(|(f, s)| self.add_constraint(f, s))
            .collect()
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
        let solver = GurobiOptimizer::new(Arc::new(gurobi_api), None);
        assert!(solver.is_ok());
    }
    #[test]
    fn test_gurobi_solver_add_variable() {
        let gurobi_api =
            GurobiApi::new(find_library_from("/usr/local/gurobi1203".to_string()).unwrap())
                .unwrap();
        let mut solver = GurobiOptimizer::new(Arc::new(gurobi_api), None).unwrap();
        let var_id = solver.add_variable(Some("x1"));
        assert_eq!(var_id.0, 0);
        let var_id2 = solver.add_variable(Some("x2"));
        assert_eq!(var_id2.0, 1);
        solver.update().unwrap();
    }
}
