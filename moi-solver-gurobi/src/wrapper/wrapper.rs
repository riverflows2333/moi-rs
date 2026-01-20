use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use crate::wrapper::utils::*;
use moi_core::*;
use moi_solver_api::*;
use std::collections::HashMap;
use std::ffi::{CString, c_char, c_double, c_int, c_void};
use std::sync::Arc;
#[derive(Clone)]
pub struct GurobiOptimizer {
    api: Arc<GurobiApi>,
    env: *mut c_void,
    model: *mut c_void,
    needs_update: bool,
    sense: Option<i32>,
    obj: Option<ScalarFunctionType>,
    vars: Vec<VarInfo>,
    constrs: HashMap<ConstrId, ConstrInfo>, // 使用 Gurobi 行索引作为键
}

#[derive(Clone)]
struct VarInfo {
    col_index: usize, // Gurobi 内部的列索引 (0, 1, 2...)
    // 缓存上下界，避免频繁查询 C API
    lb: f64,
    ub: f64,
    vtype: char, // 'C', 'B', 'I'
    name: String,
}
#[derive(Clone)]
pub struct ConstrInfo {
    pub row_index: usize, // Gurobi 内部的行索引
    pub name: String,     // 可以在这里存约束类型，方便后续查询
    pub f: ScalarFunctionType,
    pub s: ScalarSetType,
}

impl GurobiOptimizer {
    pub fn new(api: Arc<GurobiApi>, name: Option<&str>) -> Result<Self, String> {
        let mut env: *mut c_void = std::ptr::null_mut();
        let mut model: *mut c_void = std::ptr::null_mut();
        unsafe {
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
            let cname = match name {
                Some(n) => CString::new(n).unwrap(),
                None => CString::new("model").unwrap(),
            };
            let ret = (api.GRBnewmodel)(
                env,
                &mut model as *mut *mut c_void,
                cname.as_ptr(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );
        }
        Ok(Self {
            api,
            env,
            model,
            needs_update: false,
            vars: Vec::new(),
            constrs: HashMap::new(),
            obj: None,
            sense: None,
        })
    }
    pub fn set_objective(&mut self, f: ScalarFunctionType, sense: i32) {
        self.obj = Some(f);
        self.sense = Some(sense);
        self.needs_update = true;
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
        let (varid, coeff, _) = match &self.obj {
            Some(f) => scalar_function_to_grb(f)?,
            None => (Vec::new(), Vec::new(), 0.0),
        };
        // 这里需要注意，如果变量在目标函数中没有出现，需要补0
        let mut obj = vec![0.0; numvars as usize];
        for (v, c) in varid.iter().zip(coeff.iter()) {
            obj[v.0] = *c;
        }
        unsafe {
            ret = (self.api.GRBaddvars)(
                self.model,
                numvars,
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                obj.as_ptr() as *const c_double,
                lb.as_ptr(),
                ub.as_ptr(),
                vtype.as_ptr(),
                std::ptr::null(),
            );
            if ret != 0 {
                return Err(format!("Failed to add variables: error code {}", ret));
            }
        }
        // 目标函数方向
        if let Some(s) = self.sense {
            unsafe {
                ret = (self.api.GRBsetintattr)(
                    self.model,
                    GRB_INT_ATTR_MODELSENSE.as_ptr() as *const c_char,
                    s,
                );
                if ret != 0 {
                    return Err(format!("Failed to set objective sense: error code {}", ret));
                }
            }
        }

        // 更新约束
        // TODO:采用GRBaddconstrs批量添加约束
        let (cbeg, cind, cval, sense, rhs) = build_constr_matrix(
            &self
                .constrs.clone()
                .into_iter()
                .map(|(_cid, constr)| constr.clone())
                .collect::<Vec<ConstrInfo>>(),
        )?;
        let numconstrs = self.constrs.len() as c_int;
        let numnz = cind.len() as c_int;
        unsafe {
            ret = (self.api.GRBaddconstrs)(
                self.model,
                numconstrs,
                numnz,
                cbeg.as_ptr() as *const c_int,
                cind.as_ptr() as *const c_int,
                cval.as_ptr() as *const c_double,
                sense.as_ptr() as *const c_char,
                rhs.as_ptr() as *const c_double,
                std::ptr::null(),
            );
            if ret != 0 {
                return Err(format!("Failed to add constraints: error code {}", ret));
            }
        }
        self.needs_update = false; 
        // for (_cid, constr) in &self.constrs {
        //     let (var_ids, coeffs, senses, rhss) = scalar_constraint_to_grb(constr)?;
        //     let numnz = var_ids.len() as c_int;
        //     let vind: Vec<c_int> = var_ids.iter().map(|v| v.0 as c_int).collect();
        //     unsafe {
        //         ret = (self.api.GRBaddconstr)(
        //             self.model as *mut c_void,
        //             numnz,
        //             vind.as_ptr(),
        //             coeffs.as_ptr(),
        //             senses as c_char,
        //             rhss,
        //             std::ptr::null(),
        //         );
        //         if ret != 0 {
        //             return Err(format!("Failed to add constraint: error code {}", ret));
        //         }
        //     }
        // }
        Ok(())
    }
}

impl ModelLike for GurobiOptimizer {
    fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> VarId {
        // Implementation of adding a single variable
        let var_id = self.vars.len();
        // Add variable to Gurobi model here
        self.vars.push(VarInfo {
            col_index: var_id,
            lb: lb.unwrap_or(0.0),
            ub: ub.unwrap_or(f64::INFINITY),
            vtype: vtype.unwrap_or('C'),
            name: name.unwrap_or("").to_string(),
        });
        self.needs_update = true;
        VarId(var_id)
    }
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Vec<VarId> {
        // Implementation of adding multiple variables
        let start_id = self.vars.len();
        let lb = match lb {
            Some(BoundType::Single(val)) => vec![val; n],
            Some(BoundType::Vector(vec)) => vec,
            None => vec![0.0; n],
        };
        let ub = match ub {
            Some(BoundType::Single(val)) => vec![val; n],
            Some(BoundType::Vector(vec)) => vec,
            None => vec![f64::INFINITY; n],
        };
        for i in 0..n {
            let var_id = start_id + i;
            self.vars.push(VarInfo {
                col_index: var_id,
                lb: lb[i],
                ub: ub[i],
                vtype: vtype.as_ref().map_or('C', |v| v[i]),
                name: match &name {
                    Some(NameType::Single(s)) => format!("{}_{}", s.clone(), i),
                    Some(NameType::Vector(vec)) => vec[i].clone(),
                    None => "".to_string(),
                },
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

impl Optimizer for GurobiOptimizer {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        unsafe {
            let ret = (self.api.GRBoptimize)(self.model);
            if ret != 0 {
                return Err(MoiError::Msg(format!(
                    "Failed to optimize Gurobi model: error code {}",
                    ret
                )));
            }
        }
        Ok(SolveStatus::Optimal)
    }
    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        unimplemented!()
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
        let var_id = solver.add_variable(Some("x1"), Some('C'), None, None);
        assert_eq!(var_id.0, 0);
        let var_id2 = solver.add_variable(Some("x2"), None, None, None);
        assert_eq!(var_id2.0, 1);
        solver.update().unwrap();
    }
    #[test]
    fn test_gurobi_solver_solve() {
        let gurobi_api =
            GurobiApi::new(find_library_from("/usr/local/gurobi1203".to_string()).unwrap())
                .unwrap();
        let mut solver = GurobiOptimizer::new(Arc::new(gurobi_api), None).unwrap();
        let var_id1 = solver.add_variable(Some("x"), Some('B'), None, None);
        let var_id2 = solver.add_variable(Some("y"), Some('B'), None, None);
        let var_id3 = solver.add_variable(Some("z"), Some('B'), None, None);
        let mut f = ScalarFunctionType::Affine(ScalarAffineFn::new());
        if let ScalarFunctionType::Affine(ref mut afn) = f {
            afn.push_term(var_id1, 1.0);
            afn.push_term(var_id2, 2.0);
            afn.push_term(var_id3, 3.0);
            afn.simplify();
        }
        let mut s = ScalarSetType::LessThan(4.0);
        let constr_id = solver.add_constraint(f, s);
        assert_eq!(constr_id.0, 0);
        f = ScalarFunctionType::Affine(ScalarAffineFn::new());
        if let ScalarFunctionType::Affine(ref mut afn) = f {
            afn.push_term(var_id1, 1.0);
            afn.push_term(var_id2, 1.0);
            afn.simplify();
        }
        s = ScalarSetType::GreaterThan(1.0);
        let constr_id2 = solver.add_constraint(f, s);
        assert_eq!(constr_id2.0, 1);
        f = ScalarFunctionType::Affine(ScalarAffineFn::new());
        if let ScalarFunctionType::Affine(ref mut afn) = f {
            afn.push_term(var_id1, 1.0);
            afn.push_term(var_id2, 1.0);
            afn.push_term(var_id3, 2.0);
            afn.simplify();
        }
        solver.set_objective(f, GRB_MAXIMIZE);
        solver.update().unwrap();
        let status = solver.optimize().unwrap();
        assert_eq!(status, SolveStatus::Optimal);
    }
    #[test]
    fn test_gurobi_solver_add_variables() {
        let gurobi_api =
            GurobiApi::new(find_library_from("/usr/local/gurobi1203".to_string()).unwrap())
                .unwrap();
        let mut solver = GurobiOptimizer::new(Arc::new(gurobi_api), None).unwrap();
        let var_ids = solver.add_variables(
            3,
            Some(NameType::Single("x".to_string())),
            Some(vec!['C', 'I', 'B']),
            None,
            Some(BoundType::Vector(vec![10.0, 20.0, 30.0])),
        );
        assert_eq!(var_ids.len(), 3);
        assert_eq!(var_ids[0].0, 0);
        assert_eq!(var_ids[1].0, 1);
        assert_eq!(var_ids[2].0, 2);
        solver.update().unwrap();
    }
}
