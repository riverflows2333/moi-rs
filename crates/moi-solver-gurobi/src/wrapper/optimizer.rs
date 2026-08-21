use crate::bindings::*;
use crate::dynamic::api::GurobiApi;
use crate::wrapper::utils::*;
use moi_core::*;
use moi_solver_api::*;
use std::ffi::{CString, c_char, c_double, c_int, c_void};
use std::sync::{Arc, Mutex};

pub type SharedGurobiEnv = Arc<Mutex<GurobiEnv>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GurobiEnvState {
    Empty,
    Started,
}

#[derive(Debug)]
pub struct GurobiEnv {
    pub(crate) api: Arc<GurobiApi>,
    pub(crate) env: *mut c_void,
    state: GurobiEnvState,
}

unsafe impl Send for GurobiEnv {}

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
        Ok(Self {
            api,
            env,
            state: GurobiEnvState::Started,
        })
    }

    pub fn empty(api: Arc<GurobiApi>) -> Result<Self, String> {
        let mut env: *mut c_void = std::ptr::null_mut();
        let ret = unsafe { (api.GRBemptyenv)(&mut env as *mut *mut c_void) };
        if ret != 0 {
            if !env.is_null() {
                unsafe { (api.GRBfreeenv)(env) };
            }
            return Err(format!(
                "Failed to create empty Gurobi environment: error code {ret}"
            ));
        }
        Ok(Self {
            api,
            env,
            state: GurobiEnvState::Empty,
        })
    }

    pub fn start(&mut self) -> Result<(), MoiError> {
        if self.state == GurobiEnvState::Started {
            return Ok(());
        }
        check_native(unsafe { (self.api.GRBstartenv)(self.env) }, "GRBstartenv")?;
        self.state = GurobiEnvState::Started;
        Ok(())
    }

    pub fn is_started(&self) -> bool {
        self.state == GurobiEnvState::Started
    }

    pub fn set_param(&mut self, name: &str, value: AttrValue) -> Result<(), MoiError> {
        set_param_value(&self.api, self.env, name, value)
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
    _env: SharedGurobiEnv,
    api: Arc<GurobiApi>,
    model: *mut c_void,
    // 追踪变量和约束数量
    num_vars: usize,
    num_constrs: usize,
    solution_valid: bool,
    cached_status: Option<SolveStatus>,
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

    pub fn new(env: SharedGurobiEnv, name: Option<&str>) -> Result<Self, String> {
        let mut model: *mut c_void = std::ptr::null_mut();
        let env_guard = env
            .lock()
            .map_err(|_| "Gurobi environment lock is poisoned".to_string())?;
        if !env_guard.is_started() {
            return Err("Gurobi environment must be started before creating a model".to_string());
        }
        let api = env_guard.api.clone();
        unsafe {
            let cname = CString::new(name.unwrap_or("model"))
                .map_err(|_| "model name contains an embedded NUL byte".to_string())?;
            let ret = (api.GRBnewmodel)(
                env_guard.env,
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
                    (api.GRBfreemodel)(model);
                }
                return Err(format!("Failed to create Gurobi model: error code {}", ret));
            }
        }
        drop(env_guard);
        Ok(Self {
            _env: env.clone(),
            api,
            model,
            num_vars: 0,
            num_constrs: 0,
            solution_valid: false,
            cached_status: None,
        })
    }

    fn invalidate_solution(&mut self) {
        self.solution_valid = false;
        self.cached_status = None;
    }

    fn has_solution(&self) -> Result<bool, MoiError> {
        let mut count = 0;
        Self::check(
            unsafe {
                (self.api.GRBgetintattr)(
                    self.model,
                    GRB_INT_ATTR_SOLCOUNT.as_ptr().cast::<c_char>(),
                    &mut count,
                )
            },
            "GRBgetintattr(SolCount)",
        )?;
        Ok(count > 0)
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
        let end_idx = start_idx
            .checked_add(n)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;
        let native_n = c_int::try_from(n)
            .map_err(|_| MoiError::InvalidInput("variable count exceeds c_int range".into()))?;

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
                v.into_iter()
                    .enumerate()
                    .map(|(index, value)| {
                        if matches!(value, 'C' | 'B' | 'I') {
                            Ok(value as c_char)
                        } else {
                            Err(MoiError::InvalidInput(format!(
                                "variable type at index {index} must be 'C', 'B', or 'I', got '{value}'"
                            )))
                        }
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => vec!['C' as c_char; n],
        };
        for (index, (lower, upper)) in lbs.iter().zip(&ubs).enumerate() {
            if lower.is_nan() || upper.is_nan() {
                return Err(MoiError::InvalidInput(format!(
                    "variable bounds at index {index} cannot contain NaN"
                )));
            }
            if lower > upper {
                return Err(MoiError::InvalidInput(format!(
                    "lower bound {lower} exceeds upper bound {upper} for variable {index}"
                )));
            }
        }

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
                native_n,
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

        let vids = (start_idx..end_idx).map(VarId).collect();
        self.num_vars = end_idx;
        if n != 0 {
            self.invalidate_solution();
        }
        Ok(vids)
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError> {
        let n = fs.len();
        let native_n = c_int::try_from(n)
            .map_err(|_| MoiError::InvalidInput("constraint count exceeds c_int range".into()))?;
        let end_idx = self
            .num_constrs
            .checked_add(n)
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
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
            cbeg.push(c_int::try_from(cind.len()).map_err(|_| {
                MoiError::InvalidInput("constraint nonzero count exceeds c_int range".into())
            })?);
            cind.extend(
                vars.into_iter()
                    .map(|var| {
                        c_int::try_from(var.0).map_err(|_| {
                            MoiError::InvalidInput(
                                "constraint variable index exceeds c_int range".into(),
                            )
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            );
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
                native_n,
                c_int::try_from(cind.len()).map_err(|_| {
                    MoiError::InvalidInput("constraint nonzero count exceeds c_int range".into())
                })?,
                cbeg.as_ptr(),
                cind.as_ptr(),
                cval.as_ptr(),
                senses.as_ptr(),
                rhs_values.as_ptr(),
                name_ptrs.as_ptr(),
            )
        };
        Self::check(ret, "GRBaddconstrs")?;

        self.num_constrs = end_idx;
        if n != 0 {
            self.invalidate_solution();
        }
        Ok((start_idx..end_idx).map(ConstrId).collect())
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
        self.invalidate_solution();
        Ok(())
    }

    fn update(&mut self) -> Result<(), MoiError> {
        unsafe {
            let ret = (self.api.GRBupdatemodel)(self.model);
            if ret != 0 {
                return Err(MoiError::Msg(format!("Failed to update model: {}", ret)));
            }
        }
        self.invalidate_solution();
        Ok(())
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        match attr {
            ModelAttr::TerminationStatus => self.cached_status.map(AttrValue::Status),
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
                        if !v.is_finite() || v < 0.0 {
                            return Err(MoiError::InvalidInput(
                                "TimeLimit must be finite and nonnegative".into(),
                            ));
                        }
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
                    let AttrValue::Bool(silent) = value else {
                        return Err(MoiError::InvalidInput(
                            "Silent requires a boolean value".into(),
                        ));
                    };
                    Self::check(
                        (self.api.GRBsetintparam)(
                            mod_env,
                            GRB_INT_PAR_OUTPUTFLAG.as_ptr() as *const c_char,
                            if silent { 0 } else { 1 },
                        ),
                        "GRBsetintparam(OutputFlag)",
                    )?;
                }
                OptimizerAttr::Raw(s) => {
                    set_param_value(&self.api, mod_env, &s, value)?;
                }
                _ => return Err(MoiError::UnsupportedAttribute),
            }
        }
        self.invalidate_solution();
        Ok(())
    }
}

fn check_native(code: c_int, context: &'static str) -> Result<(), MoiError> {
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

fn set_param_value(
    api: &GurobiApi,
    env: *mut c_void,
    name: &str,
    value: AttrValue,
) -> Result<(), MoiError> {
    let name = CString::new(name).map_err(|_| {
        MoiError::InvalidName("parameter name contains an embedded NUL byte".to_string())
    })?;
    let ret = unsafe {
        match value {
            AttrValue::Float(value) => {
                if !value.is_finite() {
                    return Err(MoiError::InvalidInput(
                        "raw Gurobi floating-point parameter must be finite".into(),
                    ));
                }
                (api.GRBsetdblparam)(env, name.as_ptr(), value)
            }
            AttrValue::Int(value) => {
                let value = c_int::try_from(value).map_err(|_| {
                    MoiError::InvalidInput(
                        "raw Gurobi integer parameter exceeds c_int range".into(),
                    )
                })?;
                (api.GRBsetintparam)(env, name.as_ptr(), value)
            }
            AttrValue::Bool(value) => (api.GRBsetintparam)(env, name.as_ptr(), i32::from(value)),
            AttrValue::String(value) => {
                let value = CString::new(value).map_err(|_| {
                    MoiError::InvalidName(
                        "parameter value contains an embedded NUL byte".to_string(),
                    )
                })?;
                (api.GRBsetstrparam)(env, name.as_ptr(), value.as_ptr())
            }
            _ => {
                return Err(MoiError::InvalidInput(
                    "raw parameter requires bool, int, float, or string".to_string(),
                ));
            }
        }
    };
    check_native(ret, "set raw parameter")
}

impl Optimizer for GurobiOptimizer {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        self.invalidate_solution();
        unsafe {
            let ret = (self.api.GRBoptimize)(self.model);
            if ret != 0 {
                return Err(MoiError::Msg(format!(
                    "Gurobi optimization failed: {}",
                    ret
                )));
            }
        }
        let mut native_status: c_int = 0;
        check_native(
            unsafe {
                (self.api.GRBgetintattr)(
                    self.model,
                    GRB_INT_ATTR_STATUS.as_ptr() as *const c_char,
                    &mut native_status,
                )
            },
            "GRBgetintattr(Status)",
        )?;
        let status = match native_status as u32 {
            GRB_OPTIMAL => SolveStatus::Optimal,
            GRB_INFEASIBLE => SolveStatus::Infeasible,
            GRB_UNBOUNDED => SolveStatus::Unbounded,
            GRB_SUBOPTIMAL => SolveStatus::Feasible,
            _ => SolveStatus::Unknown,
        };
        self.solution_valid = true;
        self.cached_status = Some(status);
        Ok(status)
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        Err(MoiError::Msg(
            "Gurobi conflict computation is not implemented".to_string(),
        ))
    }

    fn get_var_value(&self, var_id: VarId) -> Result<Option<f64>, MoiError> {
        if var_id.0 >= self.num_vars {
            return Err(MoiError::InvalidVariableIndex(var_id.0));
        }
        if !self.solution_valid || !self.has_solution()? {
            return Ok(None);
        }
        let mut val: f64 = 0.0;
        unsafe {
            let ret = (self.api.GRBgetdblattrelement)(
                self.model,
                GRB_DBL_ATTR_X.as_ptr() as *const c_char,
                var_id.0 as c_int,
                &mut val as *mut c_double,
            );
            if ret != 0 {
                return Err(MoiError::NativeSolver {
                    solver: "Gurobi",
                    context: "GRBgetdblattrelement(X)",
                    code: ret,
                });
            }
        }
        Ok(Some(val))
    }

    fn get_objective_value(&self) -> Result<Option<f64>, MoiError> {
        if !self.solution_valid || !self.has_solution()? {
            return Ok(None);
        }
        let mut val: f64 = 0.0;
        unsafe {
            let ret = (self.api.GRBgetdblattr)(
                self.model,
                GRB_DBL_ATTR_OBJVAL.as_ptr() as *const c_char,
                &mut val as *mut c_double,
            );
            if ret != 0 {
                return Err(MoiError::NativeSolver {
                    solver: "Gurobi",
                    context: "GRBgetdblattr(ObjVal)",
                    code: ret,
                });
            }
        }
        Ok(Some(val))
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
