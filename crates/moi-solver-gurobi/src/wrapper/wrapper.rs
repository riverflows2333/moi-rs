use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use crate::wrapper::utils::*;
use moi_core::*;
use moi_solver_api::*;
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
                if !env.is_null() {
                    (api.GRBfreeenv)(env);
                }
                return Err(format!(
                    "Failed to load Gurobi environment: error code {}",
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
    _env: Arc<GurobiEnv>,
    api: Arc<GurobiApi>,
    model: *mut c_void,
    // 追踪变量和约束数量
    num_vars: usize,
    num_constrs: usize,
}

unsafe impl Send for GurobiOptimizer {}

impl GurobiOptimizer {
    fn check(code: c_int, context: &'static str) -> Result<(), MoiError> {
        if code == 0 {
            Ok(())
        } else {
            Err(MoiError::NativeSolver {
                solver: "Gurobi",
                context,
                code,
            })
        }
    }

    fn cstring(value: &str, context: &str) -> Result<CString, MoiError> {
        CString::new(value)
            .map_err(|_| MoiError::InvalidName(format!("{context} contains an embedded NUL byte")))
    }

    pub fn new(env: Arc<GurobiEnv>, name: Option<&str>) -> Result<Self, String> {
        let mut model: *mut c_void = std::ptr::null_mut();
        unsafe {
            let cname = CString::new(name.unwrap_or("model"))
                .map_err(|_| "model name contains an embedded NUL byte".to_string())?;
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
                if !model.is_null() {
                    (env.api.GRBfreemodel)(model);
                }
                return Err(format!("Failed to create Gurobi model: error code {}", ret));
            }
        }
        Ok(Self {
            _env: env.clone(),
            api: env.api.clone(),
            model,
            num_vars: 0,
            num_constrs: 0,
        })
    }
}

impl ModelLike for GurobiOptimizer {
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError> {
        let start_idx = self.num_vars;

        let lbs = match lb {
            Some(BoundType::Single(v)) => vec![v; n],
            Some(BoundType::Vector(v)) => {
                ensure_len(v.len(), n, "lower bounds")?;
                v
            }
            None => vec![0.0; n],
        };
        let ubs = match ub {
            Some(BoundType::Single(v)) => vec![v; n],
            Some(BoundType::Vector(v)) => {
                ensure_len(v.len(), n, "upper bounds")?;
                v
            }
            None => vec![f64::INFINITY; n], // Gurobi infinity
        };
        let vtypes = match vtype {
            Some(v) => {
                ensure_len(v.len(), n, "variable types")?;
                v.into_iter().map(|c| c as c_char).collect()
            }
            None => vec!['C' as c_char; n],
        };

        let names: Vec<String> = match name {
            Some(NameType::Single(s)) => (0..n).map(|i| format!("{s}_{i}")).collect(),
            Some(NameType::Vector(v)) => {
                ensure_len(v.len(), n, "variable names")?;
                v
            }
            None => (0..n).map(|i| format!("x{}", start_idx + i)).collect(),
        };
        let cnames = names
            .iter()
            .map(|name| Self::cstring(name, "variable name"))
            .collect::<Result<Vec<_>, _>>()?;
        let cname_ptrs: Vec<*const c_char> = cnames.iter().map(|s| s.as_ptr()).collect();

        let ret = unsafe {
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
            )
        };
        Self::check(ret, "GRBaddvars")?;

        let vids = (start_idx..start_idx + n).map(VarId).collect();
        self.num_vars += n;
        Ok(vids)
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError> {
        let n = fs.len();
        ensure_len(ss.len(), n, "constraint sets")?;
        if let Some(ref names) = names {
            ensure_len(names.len(), n, "constraint names")?;
        }
        let start_idx = self.num_constrs;
        let mut cbeg = Vec::with_capacity(n);
        let mut cind = Vec::new();
        let mut cval = Vec::new();
        let mut senses = Vec::with_capacity(n);
        let mut rhs_values = Vec::with_capacity(n);
        let mut constraint_names = Vec::with_capacity(n);

        for (i, (f, s)) in fs.into_iter().zip(ss).enumerate() {
            let info = ConstrInfo {
                row_index: start_idx + i,
                name: names
                    .as_ref()
                    .map(|ns| ns[i].clone())
                    .unwrap_or_else(|| format!("c{}", start_idx + i)),
                f,
                s,
            };

            let (vars, coeffs, sense, rhs) = scalar_constraint_to_grb(&info)?;
            if let Some(var) = vars.iter().find(|var| var.0 >= self.num_vars) {
                return Err(MoiError::InvalidVariableIndex(var.0));
            }
            cbeg.push(cind.len() as c_int);
            cind.extend(vars.into_iter().map(|var| var.0 as c_int));
            cval.extend(coeffs);
            senses.push(sense as c_char);
            rhs_values.push(rhs);
            constraint_names.push(Self::cstring(&info.name, "constraint name")?);
        }
        let name_ptrs = constraint_names
            .iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();
        let ret = unsafe {
            (self.api.GRBaddconstrs)(
                self.model,
                n as c_int,
                cind.len() as c_int,
                cbeg.as_ptr(),
                cind.as_ptr(),
                cval.as_ptr(),
                senses.as_ptr(),
                rhs_values.as_ptr(),
                name_ptrs.as_ptr(),
            )
        };
        Self::check(ret, "GRBaddconstrs")?;

        self.num_constrs += n;
        Ok((start_idx..start_idx + n).map(ConstrId).collect())
    }

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError> {
        let (vars, coeffs, constant) = scalar_function_to_grb(&f)?;
        if let Some(var) = vars.iter().find(|var| var.0 >= self.num_vars) {
            return Err(MoiError::InvalidVariableIndex(var.0));
        }

        unsafe {
            // 首先将所有已有变量的目标系数清零
            let zeros = vec![0.0; self.num_vars];
            Self::check(
                (self.api.GRBsetdblattrarray)(
                    self.model,
                    GRB_DBL_ATTR_OBJ.as_ptr() as *const c_char,
                    0,
                    self.num_vars as c_int,
                    zeros.as_ptr() as *mut c_double,
                ),
                "GRBsetdblattrarray(Obj)",
            )?;

            // 设置新的目标系数
            for (v, c) in vars.iter().zip(coeffs.iter()) {
                Self::check(
                    (self.api.GRBsetdblattrelement)(
                        self.model,
                        GRB_DBL_ATTR_OBJ.as_ptr() as *const c_char,
                        v.0 as c_int,
                        *c,
                    ),
                    "GRBsetdblattrelement(Obj)",
                )?;
            }

            // 设置常数偏置
            Self::check(
                (self.api.GRBsetdblattr)(
                    self.model,
                    GRB_DBL_ATTR_OBJCON.as_ptr() as *const c_char,
                    constant,
                ),
                "GRBsetdblattr(ObjCon)",
            )?;

            // 设置优化方向
            let grb_sense = match sense {
                ModelSense::Minimize => GRB_MINIMIZE,
                ModelSense::Maximize => GRB_MAXIMIZE,
            };
            Self::check(
                (self.api.GRBsetintattr)(
                    self.model,
                    GRB_INT_ATTR_MODELSENSE.as_ptr() as *const c_char,
                    grb_sense,
                ),
                "GRBsetintattr(ModelSense)",
            )?;
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
        Err(MoiError::UnsupportedAttribute)
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
                        Self::check(
                            (self.api.GRBsetdblparam)(
                                mod_env,
                                GRB_DBL_PAR_TIMELIMIT.as_ptr() as *const c_char,
                                v,
                            ),
                            "GRBsetdblparam(TimeLimit)",
                        )?;
                    } else {
                        return Err(MoiError::InvalidInput(
                            "TimeLimit requires a floating-point value".to_string(),
                        ));
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
                    Self::check(
                        (self.api.GRBsetintparam)(
                            mod_env,
                            GRB_INT_PAR_OUTPUTFLAG.as_ptr() as *const c_char,
                            flag,
                        ),
                        "GRBsetintparam(OutputFlag)",
                    )?;
                }
                OptimizerAttr::Raw(s) => {
                    let name = Self::cstring(&s, "parameter name")?;
                    let ret = match value {
                        AttrValue::Float(v) => (self.api.GRBsetdblparam)(mod_env, name.as_ptr(), v),
                        AttrValue::Int(v) => {
                            (self.api.GRBsetintparam)(mod_env, name.as_ptr(), v as c_int)
                        }
                        AttrValue::Bool(v) => {
                            (self.api.GRBsetintparam)(mod_env, name.as_ptr(), i32::from(v))
                        }
                        AttrValue::String(v) => {
                            let value = Self::cstring(&v, "parameter value")?;
                            (self.api.GRBsetstrparam)(mod_env, name.as_ptr(), value.as_ptr())
                        }
                        _ => {
                            return Err(MoiError::InvalidInput(
                                "raw parameter requires bool, int, float, or string".to_string(),
                            ));
                        }
                    };
                    Self::check(ret, "set raw parameter")?;
                }
                _ => return Err(MoiError::UnsupportedAttribute),
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
        Err(MoiError::Msg(
            "Gurobi conflict computation is not implemented".to_string(),
        ))
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
