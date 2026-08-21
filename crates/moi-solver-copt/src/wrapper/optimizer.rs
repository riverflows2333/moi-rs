use crate::wrapper::utils::{
    CStringArray, build_copt_rows, checked_c_int, ensure_finite, map_copt_status,
    map_variable_type, native_error, normalize_bound, ptr_or_null,
};
use crate::{CoptApi, SharedCoptEnv, bindings};
use moi_core::*;
use moi_solver_api::*;
use std::ffi::{CString, c_char, c_int, c_void};
use std::ptr;
use std::sync::Arc;

/// MOI optimizer backed by an owning native COPT problem handle.
pub struct CoptOptimizer {
    _env: SharedCoptEnv,
    pub(crate) api: Arc<CoptApi>,
    pub(crate) prob: *mut c_void,
    num_vars: usize,
    num_constrs: usize,
    objective: Option<ScalarFunctionType>,
    sense: Option<ModelSense>,
    cached_status: Option<SolveStatus>,
    cached_solution: Option<Vec<f64>>,
    cached_objective: Option<f64>,
}

// A COPT problem is only accessed through `&mut self` by the optimizer API.
// The environment itself remains protected by its mutex.
unsafe impl Send for CoptOptimizer {}

impl CoptOptimizer {
    /// Create an empty native COPT problem owned by this optimizer.
    pub fn new(env: SharedCoptEnv) -> Result<Self, MoiError> {
        let guard = env
            .lock()
            .map_err(|_| MoiError::BackendState("COPT environment lock is poisoned".into()))?;
        let api = guard.api.clone();
        let mut prob = ptr::null_mut();
        let code = unsafe { (api.COPT_CreateProb)(guard.raw, &mut prob) };

        if code != bindings::COPT_RETCODE_OK as c_int {
            delete_prob(&api, &mut prob);
            return Err(native_error(&api, code, "COPT_CreateProb"));
        }
        if prob.is_null() {
            return Err(MoiError::BackendState(
                "COPT_CreateProb succeeded without returning a problem".into(),
            ));
        }

        drop(guard);
        Ok(Self {
            _env: env,
            api,
            prob,
            num_vars: 0,
            num_constrs: 0,
            objective: None,
            sense: None,
            cached_status: None,
            cached_solution: None,
            cached_objective: None,
        })
    }

    pub fn num_variables(&self) -> usize {
        self.num_vars
    }

    pub fn num_constraints(&self) -> usize {
        self.num_constrs
    }

    fn check(&self, code: c_int, context: &'static str) -> Result<(), MoiError> {
        if code == bindings::COPT_RETCODE_OK as c_int {
            Ok(())
        } else {
            Err(native_error(&self.api, code, context))
        }
    }

    fn invalidate_solution(&mut self) {
        self.cached_status = None;
        self.cached_solution = None;
        self.cached_objective = None;
    }

    fn get_int_attr(&self, name: &[u8], context: &'static str) -> Result<c_int, MoiError> {
        let mut value = 0;
        self.check(
            unsafe {
                (self.api.COPT_GetIntAttr)(self.prob, name.as_ptr().cast::<c_char>(), &mut value)
            },
            context,
        )?;
        Ok(value)
    }

    fn get_dbl_attr(&self, name: &[u8], context: &'static str) -> Result<f64, MoiError> {
        let mut value = 0.0;
        self.check(
            unsafe {
                (self.api.COPT_GetDblAttr)(self.prob, name.as_ptr().cast::<c_char>(), &mut value)
            },
            context,
        )?;
        Ok(value)
    }

    fn set_int_param(
        &self,
        name: *const c_char,
        value: c_int,
        context: &'static str,
    ) -> Result<(), MoiError> {
        self.check(
            unsafe { (self.api.COPT_SetIntParam)(self.prob, name, value) },
            context,
        )
    }

    fn set_dbl_param(
        &self,
        name: *const c_char,
        value: f64,
        context: &'static str,
    ) -> Result<(), MoiError> {
        self.check(
            unsafe { (self.api.COPT_SetDblParam)(self.prob, name, value) },
            context,
        )
    }
}

impl ModelLike for CoptOptimizer {
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError> {
        let native_n = checked_c_int(n, "variable count")?;
        let end = self
            .num_vars
            .checked_add(n)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;

        let lower = expand_bounds(lb, n, 0.0, "lower bounds")?;
        let upper = expand_bounds(ub, n, f64::INFINITY, "upper bounds")?;
        for (offset, (lower, upper)) in lower.iter().zip(&upper).enumerate() {
            if lower > upper {
                return Err(MoiError::InvalidInput(format!(
                    "variable {} has lower bound {lower} greater than upper bound {upper}",
                    self.num_vars + offset
                )));
            }
        }
        let lower = lower.into_iter().map(normalize_bound).collect::<Vec<_>>();
        let upper = upper.into_iter().map(normalize_bound).collect::<Vec<_>>();

        let types = match vtype {
            Some(values) => {
                ensure_len(values.len(), n, "variable types")?;
                values
                    .into_iter()
                    .map(map_variable_type)
                    .collect::<Result<Vec<_>, _>>()?
            }
            None => vec![bindings::COPT_CONTINUOUS as c_char; n],
        };
        let names = match name {
            Some(NameType::Single(base)) => (0..n).map(|i| format!("{base}_{i}")).collect(),
            Some(NameType::Vector(values)) => {
                ensure_len(values.len(), n, "variable names")?;
                values
            }
            None => (self.num_vars..end).map(|i| format!("x{i}")).collect(),
        };
        let names = CStringArray::new(names, "variable name")?;

        if n != 0 {
            let code = unsafe {
                (self.api.COPT_AddCols)(
                    self.prob,
                    native_n,
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    ptr::null(),
                    ptr_or_null(&types),
                    ptr_or_null(&lower),
                    ptr_or_null(&upper),
                    names.as_ptr(),
                )
            };
            self.check(code, "COPT_AddCols")?;
        }

        let ids = (self.num_vars..end).map(VarId).collect();
        self.num_vars = end;
        if n != 0 {
            self.invalidate_solution();
        }
        Ok(ids)
    }

    fn add_constraints(
        &mut self,
        functions: Vec<ScalarFunctionType>,
        sets: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError> {
        let n = functions.len();
        let native_n = checked_c_int(n, "constraint count")?;
        let end = self
            .num_constrs
            .checked_add(n)
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        let rows = build_copt_rows(
            &functions,
            &sets,
            names.as_deref(),
            self.num_constrs,
            self.num_vars,
        )?;

        if n != 0 {
            let code = unsafe {
                (self.api.COPT_AddRows)(
                    self.prob,
                    native_n,
                    ptr_or_null(&rows.beg),
                    ptr_or_null(&rows.count),
                    ptr_or_null(&rows.index),
                    ptr_or_null(&rows.value),
                    ptr::null(),
                    ptr_or_null(&rows.lower),
                    ptr_or_null(&rows.upper),
                    rows.names.as_ptr(),
                )
            };
            self.check(code, "COPT_AddRows")?;
        }

        let ids = (self.num_constrs..end).map(ConstrId).collect();
        self.num_constrs = end;
        if n != 0 {
            self.invalidate_solution();
        }
        Ok(ids)
    }

    fn set_objective(
        &mut self,
        function: ScalarFunctionType,
        sense: ModelSense,
    ) -> Result<(), MoiError> {
        let linear = scalar_function_to_linear(&function)?;
        ensure_len(
            linear.coefficients.len(),
            linear.variables.len(),
            "objective coefficients",
        )?;
        ensure_finite(linear.constant, "objective constant")?;

        let mut coefficients = vec![0.0; self.num_vars];
        for (variable, coefficient) in linear.variables.iter().zip(&linear.coefficients) {
            if variable.0 >= self.num_vars {
                return Err(MoiError::InvalidVariableIndex(variable.0));
            }
            ensure_finite(*coefficient, "objective coefficient")?;
            coefficients[variable.0] += coefficient;
            ensure_finite(coefficients[variable.0], "summed objective coefficient")?;
        }

        if self.num_vars != 0 {
            let indices = (0..self.num_vars)
                .map(|index| checked_c_int(index, "objective variable index"))
                .collect::<Result<Vec<_>, _>>()?;
            let count = checked_c_int(self.num_vars, "objective coefficient count")?;
            let code = unsafe {
                (self.api.COPT_SetColObj)(
                    self.prob,
                    count,
                    ptr_or_null(&indices),
                    ptr_or_null(&coefficients),
                )
            };
            self.check(code, "COPT_SetColObj")?;
        }
        self.check(
            unsafe { (self.api.COPT_SetObjConst)(self.prob, linear.constant) },
            "COPT_SetObjConst",
        )?;
        let native_sense = match sense {
            ModelSense::Minimize => bindings::COPT_MINIMIZE as c_int,
            ModelSense::Maximize => bindings::COPT_MAXIMIZE as c_int,
        };
        self.check(
            unsafe { (self.api.COPT_SetObjSense)(self.prob, native_sense) },
            "COPT_SetObjSense",
        )?;

        self.objective = Some(function);
        self.sense = Some(sense);
        self.invalidate_solution();
        Ok(())
    }

    fn update(&mut self) -> Result<(), MoiError> {
        self.check(unsafe { (self.api.COPT_Update)(self.prob) }, "COPT_Update")
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        match attr {
            ModelAttr::ObjectiveSense => self.sense.map(AttrValue::ModelSense),
            ModelAttr::ObjectiveFunction => self.objective.clone().map(AttrValue::ScalarFn),
            ModelAttr::NumberOfVariables => Some(AttrValue::Usize(self.num_vars)),
            ModelAttr::NumberOfConstraints => Some(AttrValue::Usize(self.num_constrs)),
            ModelAttr::ListOfVariableIndices => {
                Some(AttrValue::VecUsize((0..self.num_vars).collect::<Vec<_>>()))
            }
            ModelAttr::TerminationStatus => self.cached_status.map(AttrValue::Status),
            ModelAttr::ResultCount => Some(AttrValue::Usize(usize::from(
                self.cached_solution.is_some(),
            ))),
            ModelAttr::ObjectiveValue => self.cached_objective.map(AttrValue::Float),
            ModelAttr::ModelName => None,
        }
    }

    fn set_model_attr(&mut self, _attr: ModelAttr, _value: AttrValue) -> Result<(), MoiError> {
        Err(MoiError::UnsupportedAttribute)
    }

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue> {
        match attr {
            OptimizerAttr::SolverName => Some(AttrValue::String("COPT".into())),
            _ => None,
        }
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        let result = match attr {
            OptimizerAttr::SolverName => Err(MoiError::SetAttributeNotAllowed),
            OptimizerAttr::TimeLimit => {
                let AttrValue::Float(value) = value else {
                    return Err(MoiError::InvalidInput(
                        "TimeLimit requires a floating-point value".into(),
                    ));
                };
                if !value.is_finite() || value < 0.0 {
                    return Err(MoiError::InvalidInput(
                        "TimeLimit must be finite and nonnegative".into(),
                    ));
                }
                self.set_dbl_param(
                    bindings::COPT_DBLPARAM_TIMELIMIT.as_ptr().cast::<c_char>(),
                    value,
                    "COPT_SetDblParam(TimeLimit)",
                )
            }
            OptimizerAttr::Silent => {
                let AttrValue::Bool(silent) = value else {
                    return Err(MoiError::InvalidInput(
                        "Silent requires a boolean value".into(),
                    ));
                };
                self.set_int_param(
                    bindings::COPT_INTPARAM_LOGGING.as_ptr().cast::<c_char>(),
                    if silent { 0 } else { 1 },
                    "COPT_SetIntParam(Logging)",
                )
            }
            OptimizerAttr::Raw(name) => {
                let name = CString::new(name).map_err(|_| {
                    MoiError::InvalidInput(
                        "raw COPT parameter name contains an embedded NUL byte".into(),
                    )
                })?;
                match value {
                    AttrValue::Bool(value) => self.set_int_param(
                        name.as_ptr(),
                        c_int::from(value),
                        "COPT_SetIntParam(Raw)",
                    ),
                    AttrValue::Int(value) => {
                        let value = c_int::try_from(value).map_err(|_| {
                            MoiError::InvalidInput(
                                "raw COPT integer parameter exceeds c_int range".into(),
                            )
                        })?;
                        self.set_int_param(name.as_ptr(), value, "COPT_SetIntParam(Raw)")
                    }
                    AttrValue::Float(value) => {
                        ensure_finite(value, "raw COPT floating-point parameter")?;
                        self.set_dbl_param(name.as_ptr(), value, "COPT_SetDblParam(Raw)")
                    }
                    _ => Err(MoiError::InvalidInput(
                        "raw COPT parameters accept only bool, int, or float values".into(),
                    )),
                }
            }
        };
        result?;
        self.invalidate_solution();
        Ok(())
    }
}

impl Optimizer for CoptOptimizer {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        self.invalidate_solution();
        self.check(unsafe { (self.api.COPT_Solve)(self.prob) }, "COPT_Solve")?;

        let native_status =
            self.get_int_attr(bindings::COPT_INTATTR_STATUS, "COPT_GetIntAttr(Status)")?;
        let has_solution =
            self.get_int_attr(bindings::COPT_INTATTR_HASSOL, "COPT_GetIntAttr(HasSol)")? != 0;
        let status = map_copt_status(native_status, has_solution);

        let (solution, objective) = if has_solution {
            let is_mip =
                self.get_int_attr(bindings::COPT_INTATTR_ISMIP, "COPT_GetIntAttr(IsMIP)")? != 0;
            let mut solution = vec![0.0; self.num_vars];
            if self.num_vars != 0 {
                let code = if is_mip {
                    unsafe { (self.api.COPT_GetSolution)(self.prob, solution.as_mut_ptr()) }
                } else {
                    unsafe {
                        (self.api.COPT_GetLpSolution)(
                            self.prob,
                            solution.as_mut_ptr(),
                            ptr::null_mut(),
                            ptr::null_mut(),
                            ptr::null_mut(),
                        )
                    }
                };
                self.check(
                    code,
                    if is_mip {
                        "COPT_GetSolution"
                    } else {
                        "COPT_GetLpSolution"
                    },
                )?;
            }
            let objective =
                self.get_dbl_attr(bindings::COPT_DBLATTR_OBJVAL, "COPT_GetDblAttr(ObjVal)")?;
            (Some(solution), Some(objective))
        } else {
            (None, None)
        };

        self.cached_status = Some(status);
        self.cached_solution = solution;
        self.cached_objective = objective;
        Ok(status)
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        Err(MoiError::Msg(
            "COPT conflict computation is not implemented".into(),
        ))
    }

    fn get_var_value(&self, var_id: VarId) -> Result<Option<f64>, MoiError> {
        if var_id.0 >= self.num_vars {
            return Err(MoiError::InvalidVariableIndex(var_id.0));
        }
        Ok(self
            .cached_solution
            .as_ref()
            .and_then(|solution| solution.get(var_id.0))
            .copied())
    }

    fn get_objective_value(&self) -> Result<Option<f64>, MoiError> {
        Ok(self.cached_objective)
    }
}

impl Drop for CoptOptimizer {
    fn drop(&mut self) {
        delete_prob(&self.api, &mut self.prob);
    }
}

fn expand_bounds(
    bounds: Option<BoundType>,
    n: usize,
    default: f64,
    field: &str,
) -> Result<Vec<f64>, MoiError> {
    let values = match bounds {
        Some(BoundType::Single(value)) => vec![value; n],
        Some(BoundType::Vector(values)) => {
            ensure_len(values.len(), n, field)?;
            values
        }
        None => vec![default; n],
    };
    if let Some(value) = values.iter().find(|value| value.is_nan()) {
        return Err(MoiError::InvalidInput(format!(
            "{field} cannot contain NaN, got {value}"
        )));
    }
    Ok(values)
}

fn delete_prob(api: &CoptApi, prob: &mut *mut c_void) {
    if prob.is_null() {
        return;
    }
    // SAFETY: `prob` is exclusively owned by this optimizer and is deleted
    // before its shared environment can be released.
    let _ = unsafe { (api.COPT_DeleteProb)(prob) };
    *prob = ptr::null_mut();
}
