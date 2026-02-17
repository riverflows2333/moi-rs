use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use crate::wrapper::utils::*;
use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;
use std::ffi::{CString, c_char, c_double, c_int, c_void};
use std::ops::Index;
use std::sync::Arc;

#[derive(Clone)]
pub struct GurobiOptimizer {
    api: Arc<GurobiApi>,
    env: *mut c_void,
    model: *mut c_void,
    base: BridgeOptimizer,
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
            base: BridgeOptimizer::new(),
        })
    }

    pub fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) {
        self.base.obj = Some(f);
        self.base.sense = Some(sense);
        self.base.needs_update = true;
    }

    pub fn update(&mut self, model: Option<BridgeOptimizer>) -> Result<(), String> {
        let mut ret;
        if !self.base.needs_update {
            return Ok(());
        }
        if let Some(m) = model {
            self.base = m;
        }
        // 更新变量
        let numvars = self.base.vars.len() as c_int;
        let lb = self.base.vars.iter().map(|v| v.lb).collect::<Vec<f64>>();
        let ub = self.base.vars.iter().map(|v| v.ub).collect::<Vec<f64>>();
        let vtype = self
            .base
            .vars
            .iter()
            .map(|v| v.vtype as c_char)
            .collect::<Vec<c_char>>();
        let (varid, coeff, _) = match &self.base.obj {
            Some(f) => scalar_function_to_grb(f)?,
            None => (Vec::new(), Vec::new(), 0.0),
        };
        // 这里需要注意，如果变量在目标函数中没有出现，需要补0
        let mut obj = vec![0.0; numvars as usize];
        for (v, c) in varid.iter().zip(coeff.iter()) {
            obj[v.0] = *c;
        }
        let varnames = self
            .base
            .vars
            .iter()
            .map(|v| CString::new(v.name.clone()).unwrap())
            .collect::<Vec<CString>>();
        let varnames_ptrs = varnames
            .iter()
            .map(|s| s.as_ptr() as *const c_char)
            .collect::<Vec<*const c_char>>();
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
                varnames_ptrs.as_ptr(),
            );
            if ret != 0 {
                return Err(format!("Failed to add variables: error code {}", ret));
            }
        }
        // 目标函数方向
        if let Some(s) = self.base.sense {
            let s = match s {
                ModelSense::Minimize => GRB_MINIMIZE,
                ModelSense::Maximize => GRB_MAXIMIZE,
            };
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
        let (cbeg, cind, cval, sense, rhs, names) = build_constr_matrix(
            &self
                .base
                .constrs
                .values()
                .cloned()
                .collect::<Vec<ConstrInfo>>(),
        )?;
        let numconstrs = self.base.constrs.len() as c_int;
        let numnz = cind.len() as c_int;
        let cnames = names
            .iter()
            .map(|n| CString::new(n.clone()).unwrap())
            .collect::<Vec<CString>>();
        let cnames_ptrs = cnames
            .iter()
            .map(|s| s.as_ptr() as *const c_char)
            .collect::<Vec<*const c_char>>();
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
                cnames_ptrs.as_ptr() as *const *const c_char,
            );
            if ret != 0 {
                return Err(format!("Failed to add constraints: error code {}", ret));
            }
        }
        self.base.needs_update = false;
        Ok(())
    }
}

impl Index<VarId> for GurobiOptimizer {
    type Output = VarInfo;
    fn index(&self, index: VarId) -> &Self::Output {
        &self.base.vars[index.0]
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
        self.base.add_variable(name, vtype, lb, ub)
    }
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Vec<VarId> {
        self.base.add_variables(n, name, vtype, lb, ub)
    }
    fn add_constraint(
        &mut self,
        f: ScalarFunctionType,
        s: ScalarSetType,
        name: Option<String>,
    ) -> ConstrId {
        self.base.add_constraint(f, s, name)
    }
    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Vec<ConstrId> {
        self.base.add_constraints(fs, ss, names)
    }
    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        match attr {
            ModelAttr::TerminationStatus => {
                let mut status: i32 = 0;
                unsafe {
                    let ret = (self.api.GRBgetintattr)(
                        self.model,
                        GRB_INT_ATTR_STATUS.as_ptr() as *const c_char,
                        &mut status as *mut c_int,
                    );
                    if ret != 0 {
                        return None;
                    }
                    Some(AttrValue::Status(match status as u32 {
                        GRB_OPTIMAL => SolveStatus::Optimal,
                        GRB_INFEASIBLE => SolveStatus::Infeasible,
                        GRB_UNBOUNDED => SolveStatus::Unbounded,
                        GRB_SUBOPTIMAL => SolveStatus::Feasible,
                        _ => SolveStatus::Unknown,
                    }))
                }
            }
            _ => self.base.get_model_attr(attr),
        }
    }
    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError> {
        self.base.set_model_attr(attr, value)
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
        if let Some(AttrValue::Status(status)) = self.get_model_attr(ModelAttr::TerminationStatus) {
            let mut x = vec![0.0; self.base.vars.len()];
            unsafe {
                let ret = (self.api.GRBgetdblattrarray)(
                    self.model,
                    GRB_DBL_ATTR_X.as_ptr() as *const c_char,
                    0,
                    self.base.vars.len() as c_int,
                    x.as_mut_ptr(),
                );
                if ret != 0 {
                    return Err(MoiError::Msg(format!(
                        "Failed to get variable values: error code {}",
                        ret
                    )));
                }
            }
            // 更新变量值
            for (i, val) in x.iter().enumerate() {
                self.base.vars[i].value = Some(*val);
            }
            Ok(status)
        } else {
            Err(MoiError::Msg(
                "Failed to retrieve termination status".to_string(),
            ))
        }
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
