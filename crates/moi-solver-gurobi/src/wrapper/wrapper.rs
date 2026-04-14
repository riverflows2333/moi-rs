use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use crate::wrapper::utils::*;
use moi_core::*;
use moi_solver_api::*;
use std::f64::INFINITY;
use std::ffi::{CString, c_char, c_double, c_int, c_void};
use std::sync::Arc;

#[derive(Debug)]
pub struct GurobiEnv {
    pub(crate) api: Arc<GurobiApi>,
    pub(crate) env: *mut c_void,
}

unsafe impl Send for GurobiEnv {}
unsafe impl Sync for GurobiEnv {}

impl GurobiEnv {
    pub fn new(api: Arc<GurobiApi>) -> Result<Self, String> {
        let mut env: *mut c_void = std::ptr::null_mut();
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
                (api.GRBfreeenv)(env);
                return Err(format!(
                    "Failed to start Gurobi environment: error code {}",
                    ret
                ));
            }
        }
        Ok(Self { api, env })
    }
}

impl Drop for GurobiEnv {
    fn drop(&mut self) {
        unsafe {
            if !self.env.is_null() {
                (self.api.GRBfreeenv)(self.env);
            }
        }
    }
}

pub struct GurobiOptimizer {
    api: Arc<GurobiApi>,
    model: *mut c_void,
    // 追踪变量和约束数量
    num_vars: usize,
    num_constrs: usize,
}

unsafe impl Send for GurobiOptimizer {}
unsafe impl Sync for GurobiOptimizer {}

impl GurobiOptimizer {
    pub fn new(env: Arc<GurobiEnv>, name: Option<&str>) -> Result<Self, String> {
        let mut model: *mut c_void = std::ptr::null_mut();
        unsafe {
            let cname = match name {
                Some(n) => CString::new(n).unwrap(),
                None => CString::new("model").unwrap(),
            };
            let ret = (env.api.GRBnewmodel)(
                env.env,
                &mut model as *mut *mut c_void,
                cname.as_ptr(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
            );
            if ret != 0 {
                return Err(format!("Failed to create Gurobi model: error code {}", ret));
            }
        }
        Ok(Self {
            api: env.api.clone(),
            model,
            num_vars: 0,
            num_constrs: 0,
        })
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
        let vars = self.add_variables(
            1,
            name.map(|s| NameType::Single(s.to_string())),
            vtype.map(|v| vec![v]),
            lb.map(|v| BoundType::Single(v)),
            ub.map(|v| BoundType::Single(v)),
        );
        vars[0]
    }

    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Vec<VarId> {
        let mut vids = Vec::with_capacity(n);
        let start_idx = self.num_vars;

        let lbs = match lb {
            Some(BoundType::Single(v)) => vec![v; n],
            Some(BoundType::Vector(v)) => v,
            None => vec![0.0; n],
        };
        let ubs = match ub {
            Some(BoundType::Single(v)) => vec![v; n],
            Some(BoundType::Vector(v)) => v,
            None => vec![INFINITY; n], // Gurobi infinity
        };
        let vtypes = match vtype {
            Some(v) => v.into_iter().map(|c| c as c_char).collect(),
            None => vec!['C' as c_char; n],
        };

        let cnames: Vec<CString> = match name {
            Some(NameType::Single(s)) => (0..n)
                .map(|i| CString::new(format!("{}_{}", s, i)).unwrap())
                .collect(),
            Some(NameType::Vector(v)) => v.into_iter().map(|s| CString::new(s).unwrap()).collect(),
            None => (0..n)
                .map(|i| CString::new(format!("x{}", start_idx + i)).unwrap())
                .collect(),
        };
        let cname_ptrs: Vec<*const c_char> = cnames.iter().map(|s| s.as_ptr()).collect();

        unsafe {
            (self.api.GRBaddvars)(
                self.model,
                n as c_int,
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(), // obj coefficients set via set_objective
                lbs.as_ptr(),
                ubs.as_ptr(),
                vtypes.as_ptr(),
                cname_ptrs.as_ptr(),
            );
        }

        for i in 0..n {
            vids.push(VarId(start_idx + i));
        }
        self.num_vars += n;
        vids
    }

    fn add_constraint(
        &mut self,
        f: ScalarFunctionType,
        s: ScalarSetType,
        name: Option<String>,
    ) -> ConstrId {
        let ids = self.add_constraints(vec![f], vec![s], name.map(|n| vec![n]));
        ids[0]
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Vec<ConstrId> {
        let n = fs.len();
        let mut ids = Vec::with_capacity(n);
        let start_idx = self.num_constrs;

        for (i, (f, s)) in fs.into_iter().zip(ss.into_iter()).enumerate() {
            let info = ConstrInfo {
                row_index: start_idx + i,
                name: names
                    .as_ref()
                    .map(|ns| ns[i].clone())
                    .unwrap_or_else(|| format!("c{}", start_idx + i)),
                f,
                s,
            };

            if let Ok((vars, coeffs, sense, rhs)) = scalar_constraint_to_grb(&info) {
                let cname = CString::new(info.name).unwrap();
                unsafe {
                    (self.api.GRBaddconstr)(
                        self.model,
                        vars.len() as c_int,
                        vars.iter()
                            .map(|v| v.0 as c_int)
                            .collect::<Vec<_>>()
                            .as_ptr(),
                        coeffs.as_ptr(),
                        sense as c_char,
                        rhs,
                        cname.as_ptr(),
                    );
                }
            }
            ids.push(ConstrId(start_idx + i));
        }

        self.num_constrs += n;
        ids
    }

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError> {
        let (vars, coeffs, constant) = scalar_function_to_grb(&f).map_err(|e| MoiError::Msg(e))?;

        unsafe {
            // 首先将所有已有变量的目标系数清零
            let zeros = vec![0.0; self.num_vars];
            (self.api.GRBsetdblattrarray)(
                self.model,
                GRB_DBL_ATTR_OBJ.as_ptr() as *const c_char,
                0,
                self.num_vars as c_int,
                zeros.as_ptr() as *mut c_double,
            );

            // 设置新的目标系数
            for (v, c) in vars.iter().zip(coeffs.iter()) {
                (self.api.GRBsetdblattrelement)(
                    self.model,
                    GRB_DBL_ATTR_OBJ.as_ptr() as *const c_char,
                    v.0 as c_int,
                    *c,
                );
            }

            // 设置常数偏置
            (self.api.GRBsetdblattr)(
                self.model,
                GRB_DBL_ATTR_OBJCON.as_ptr() as *const c_char,
                constant,
            );

            // 设置优化方向
            let grb_sense = match sense {
                ModelSense::Minimize => GRB_MINIMIZE,
                ModelSense::Maximize => GRB_MAXIMIZE,
            };
            (self.api.GRBsetintattr)(
                self.model,
                GRB_INT_ATTR_MODELSENSE.as_ptr() as *const c_char,
                grb_sense,
            );
        }
        Ok(())
    }

    fn update(&mut self) -> Result<(), MoiError> {
        unsafe {
            let ret = (self.api.GRBupdatemodel)(self.model);
            if ret != 0 {
                return Err(MoiError::Msg(format!("Failed to update model: {}", ret)));
            }
        }
        Ok(())
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
            _ => None,
        }
    }

    fn set_model_attr(&mut self, _attr: ModelAttr, _value: AttrValue) -> Result<(), MoiError> {
        Ok(())
    }

    fn get_optimizer_attr(&self, _attr: OptimizerAttr) -> Option<AttrValue> {
        None
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        unsafe {
            let mod_env = (self.api.GRBgetenv)(self.model);
            match attr {
                OptimizerAttr::TimeLimit => {
                    if let AttrValue::Float(v) = value {
                        (self.api.GRBsetdblparam)(
                            mod_env,
                            GRB_DBL_PAR_TIMELIMIT.as_ptr() as *const c_char,
                            v,
                        );
                    }
                }
                OptimizerAttr::Silent => {
                    let flag = match value {
                        AttrValue::Bool(v) => {
                            if v {
                                0
                            } else {
                                1
                            }
                        }
                        AttrValue::Int(v) => {
                            if v == 0 {
                                1
                            } else {
                                0
                            }
                        }
                        _ => 1,
                    };
                    (self.api.GRBsetintparam)(
                        mod_env,
                        GRB_INT_PAR_OUTPUTFLAG.as_ptr() as *const c_char,
                        flag,
                    );
                }
                OptimizerAttr::Raw(s) => {
                    if let AttrValue::Float(v) = value {
                        (self.api.GRBsetdblparam)(mod_env, s.as_ptr() as *const c_char, v);
                    } else if let AttrValue::Int(v) = value {
                        (self.api.GRBsetintparam)(mod_env, s.as_ptr() as *const c_char, v as c_int);
                    } else if let AttrValue::String(ref v) = value {
                        (self.api.GRBsetstrparam)(
                            mod_env,
                            s.as_ptr() as *const c_char,
                            v.as_ptr() as *const c_char,
                        );
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
}

impl Optimizer for GurobiOptimizer {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        unsafe {
            let ret = (self.api.GRBoptimize)(self.model);
            if ret != 0 {
                return Err(MoiError::Msg(format!(
                    "Gurobi optimization failed: {}",
                    ret
                )));
            }
        }
        self.get_model_attr(ModelAttr::TerminationStatus)
            .and_then(|v| {
                if let AttrValue::Status(s) = v {
                    Some(s)
                } else {
                    None
                }
            })
            .ok_or(MoiError::Msg("Failed to get status".into()))
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        unimplemented!()
    }

    fn get_var_value(&self, var_id: VarId) -> Option<f64> {
        let mut val: f64 = 0.0;
        unsafe {
            let ret = (self.api.GRBgetdblattrelement)(
                self.model,
                GRB_DBL_ATTR_X.as_ptr() as *const c_char,
                var_id.0 as c_int,
                &mut val as *mut c_double,
            );
            if ret != 0 {
                return None;
            }
        }
        Some(val)
    }

    fn get_objective_value(&self) -> Option<f64> {
        let mut val: f64 = 0.0;
        unsafe {
            let ret = (self.api.GRBgetdblattr)(
                self.model,
                GRB_DBL_ATTR_OBJVAL.as_ptr() as *const c_char,
                &mut val as *mut c_double,
            );
            if ret != 0 {
                return None;
            }
        }
        Some(val)
    }
}

impl Drop for GurobiOptimizer {
    fn drop(&mut self) {
        unsafe {
            if !self.model.is_null() {
                (self.api.GRBfreemodel)(self.model);
            }
        }
    }
}
