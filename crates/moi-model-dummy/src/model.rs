use moi_core::{
    AttrValue, ConstrId, ConstrInfo, ModelAttr, ModelSense, MoiError, OptimizerAttr,
    ScalarFunctionType, ScalarSetType, SolveStatus, VarId, VarInfo,
};
use moi_solver_api::{
    BoundType, ModelLike, NameType, Optimizer, ensure_len, scalar_function_to_linear,
};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecordedOperation {
    AddVariables,
    AddConstraints,
    SetObjective,
    SetModelAttr,
    SetOptimizerAttr,
    Update,
    Optimize,
}

#[derive(Clone, Debug)]
pub enum RecordedCall {
    AddVariables {
        variables: Vec<VarInfo>,
    },
    AddConstraints {
        constraints: Vec<ConstrInfo>,
    },
    SetObjective {
        function: ScalarFunctionType,
        sense: ModelSense,
    },
    SetModelAttr {
        attr: ModelAttr,
        value: AttrValue,
    },
    SetOptimizerAttr {
        attr: OptimizerAttr,
        value: AttrValue,
    },
    Update,
    Optimize,
}

#[derive(Clone, Debug)]
struct InjectedFailure {
    operation: RecordedOperation,
    message: String,
}

#[derive(Clone, Debug)]
pub struct RecordingState {
    pub name: String,
    pub variables: Vec<VarInfo>,
    pub constraints: Vec<ConstrInfo>,
    pub objective: Option<ScalarFunctionType>,
    pub sense: Option<ModelSense>,
    pub optimizer_attrs: HashMap<OptimizerAttr, AttrValue>,
    pub calls: Vec<RecordedCall>,
    pub solve_status: SolveStatus,
    pub objective_value: Option<f64>,
    pub variable_values: HashMap<VarId, f64>,
    pub optimized: bool,
    failure: Option<InjectedFailure>,
}

impl Default for RecordingState {
    fn default() -> Self {
        Self {
            name: String::new(),
            variables: Vec::new(),
            constraints: Vec::new(),
            objective: None,
            sense: None,
            optimizer_attrs: HashMap::new(),
            calls: Vec::new(),
            solve_status: SolveStatus::Unknown,
            objective_value: None,
            variable_values: HashMap::new(),
            optimized: false,
            failure: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RecordingHandle {
    state: Arc<Mutex<RecordingState>>,
}

impl RecordingHandle {
    pub fn snapshot(&self) -> Result<RecordingState, MoiError> {
        Ok(self.lock()?.clone())
    }

    pub fn set_solution(
        &self,
        status: SolveStatus,
        objective_value: Option<f64>,
        variable_values: impl IntoIterator<Item = (VarId, f64)>,
    ) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        state.solve_status = status;
        state.objective_value = objective_value;
        state.variable_values = variable_values.into_iter().collect();
        state.optimized = false;
        Ok(())
    }

    pub fn fail_next(
        &self,
        operation: RecordedOperation,
        message: impl Into<String>,
    ) -> Result<(), MoiError> {
        self.lock()?.failure = Some(InjectedFailure {
            operation,
            message: message.into(),
        });
        Ok(())
    }

    fn lock(&self) -> Result<MutexGuard<'_, RecordingState>, MoiError> {
        self.state
            .lock()
            .map_err(|_| MoiError::BackendState("dummy recording state lock is poisoned".into()))
    }
}

#[derive(Clone, Debug)]
pub struct DummyModel {
    handle: RecordingHandle,
}

impl DummyModel {
    pub fn new() -> (Self, RecordingHandle) {
        let handle = RecordingHandle {
            state: Arc::new(Mutex::new(RecordingState::default())),
        };
        (
            Self {
                handle: handle.clone(),
            },
            handle,
        )
    }

    pub fn handle(&self) -> RecordingHandle {
        self.handle.clone()
    }

    pub fn set_var_name(
        &mut self,
        variable: VarId,
        name: impl Into<String>,
    ) -> Result<(), MoiError> {
        let mut state = self.handle.lock()?;
        let info = state
            .variables
            .get_mut(variable.0)
            .ok_or(MoiError::InvalidVariableIndex(variable.0))?;
        info.name = name.into();
        state.optimized = false;
        Ok(())
    }

    pub fn get_var_name(&self, variable: VarId) -> Option<String> {
        self.handle
            .state
            .lock()
            .ok()?
            .variables
            .get(variable.0)
            .map(|info| info.name.clone())
    }

    pub fn get_var_by_name(&self, name: &str) -> Option<VarId> {
        self.handle
            .state
            .lock()
            .ok()?
            .variables
            .iter()
            .position(|info| info.name == name)
            .map(VarId)
    }

    fn maybe_fail(
        state: &mut RecordingState,
        operation: RecordedOperation,
    ) -> Result<(), MoiError> {
        if state
            .failure
            .as_ref()
            .is_some_and(|failure| failure.operation == operation)
            && let Some(failure) = state.failure.take()
        {
            return Err(MoiError::Msg(failure.message));
        }
        Ok(())
    }

    fn validate_function(
        state: &RecordingState,
        function: &ScalarFunctionType,
    ) -> Result<(), MoiError> {
        let linear = scalar_function_to_linear(function)?;
        if !linear.constant.is_finite() {
            return Err(MoiError::InvalidInput(
                "affine function constant must be finite".into(),
            ));
        }
        if let Some((index, _)) = linear
            .coefficients
            .iter()
            .enumerate()
            .find(|(_, coefficient)| !coefficient.is_finite())
        {
            return Err(MoiError::InvalidInput(format!(
                "affine coefficient at index {index} must be finite"
            )));
        }
        if let Some(variable) = linear
            .variables
            .into_iter()
            .find(|variable| variable.0 >= state.variables.len())
        {
            return Err(MoiError::InvalidVariableIndex(variable.0));
        }
        Ok(())
    }

    fn validate_set(set: &ScalarSetType) -> Result<(), MoiError> {
        let bounds = moi_solver_api::scalar_set_to_bounds(set);
        let lower = bounds.lower;
        let upper = bounds.upper;
        if lower.is_some_and(|value| !value.is_finite())
            || upper.is_some_and(|value| !value.is_finite())
        {
            return Err(MoiError::InvalidInput(
                "constraint bounds must be finite".into(),
            ));
        }
        if let (Some(lower), Some(upper)) = (lower, upper)
            && lower > upper
        {
            return Err(MoiError::InvalidInput(format!(
                "constraint lower bound {lower} exceeds upper bound {upper}"
            )));
        }
        Ok(())
    }

    fn validate_name(name: &str, field: &str) -> Result<(), MoiError> {
        if name.contains('\0') {
            Err(MoiError::InvalidName(format!(
                "{field} contains an embedded NUL byte"
            )))
        } else {
            Ok(())
        }
    }

    fn invalidate_solution(state: &mut RecordingState) {
        state.optimized = false;
    }
}

impl Default for DummyModel {
    fn default() -> Self {
        Self::new().0
    }
}

impl ModelLike for DummyModel {
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError> {
        let names = match name {
            Some(NameType::Single(base)) => (0..n).map(|i| format!("{base}_{i}")).collect(),
            Some(NameType::Vector(values)) => {
                ensure_len(values.len(), n, "variable names")?;
                values
            }
            None => vec![String::new(); n],
        };
        let vtypes = match vtype {
            Some(values) => {
                ensure_len(values.len(), n, "variable types")?;
                values
            }
            None => vec!['C'; n],
        };
        let lbs = match lb {
            Some(BoundType::Single(value)) => vec![value; n],
            Some(BoundType::Vector(values)) => {
                ensure_len(values.len(), n, "lower bounds")?;
                values
            }
            None => vec![0.0; n],
        };
        let ubs = match ub {
            Some(BoundType::Single(value)) => vec![value; n],
            Some(BoundType::Vector(values)) => {
                ensure_len(values.len(), n, "upper bounds")?;
                values
            }
            None => vec![f64::INFINITY; n],
        };
        for (index, name) in names.iter().enumerate() {
            Self::validate_name(name, &format!("variable name at index {index}"))?;
        }
        for (index, variable_type) in vtypes.iter().enumerate() {
            if !matches!(variable_type, 'C' | 'B' | 'I') {
                return Err(MoiError::InvalidInput(format!(
                    "variable type at index {index} must be 'C', 'B', or 'I', got '{variable_type}'"
                )));
            }
        }
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

        let mut state = self.handle.lock()?;
        Self::maybe_fail(&mut state, RecordedOperation::AddVariables)?;
        let start = state.variables.len();
        let end = start
            .checked_add(n)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;
        let variables = (0..n)
            .map(|offset| VarInfo {
                col_index: start + offset,
                lb: lbs[offset],
                ub: ubs[offset],
                vtype: vtypes[offset],
                name: names[offset].clone(),
                value: None,
            })
            .collect::<Vec<_>>();
        let ids = (start..end).map(VarId).collect::<Vec<_>>();
        state.variables.extend(variables.clone());
        state.calls.push(RecordedCall::AddVariables { variables });
        Self::invalidate_solution(&mut state);
        Ok(ids)
    }

    fn add_constraints(
        &mut self,
        functions: Vec<ScalarFunctionType>,
        sets: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError> {
        ensure_len(sets.len(), functions.len(), "constraint sets")?;
        let names = match names {
            Some(values) => {
                ensure_len(values.len(), functions.len(), "constraint names")?;
                values
            }
            None => vec![String::new(); functions.len()],
        };

        let mut state = self.handle.lock()?;
        for (index, name) in names.iter().enumerate() {
            Self::validate_name(name, &format!("constraint name at index {index}"))?;
        }
        for (function, set) in functions.iter().zip(&sets) {
            Self::validate_function(&state, function)?;
            Self::validate_set(set)?;
        }
        Self::maybe_fail(&mut state, RecordedOperation::AddConstraints)?;
        let start = state.constraints.len();
        let end = start
            .checked_add(functions.len())
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        let constraints = functions
            .into_iter()
            .zip(sets)
            .zip(names)
            .enumerate()
            .map(|(offset, ((f, s), name))| ConstrInfo {
                row_index: start + offset,
                name,
                f,
                s,
            })
            .collect::<Vec<_>>();
        let ids = (start..end).map(ConstrId).collect::<Vec<_>>();
        state.constraints.extend(constraints.clone());
        state
            .calls
            .push(RecordedCall::AddConstraints { constraints });
        Self::invalidate_solution(&mut state);
        Ok(ids)
    }

    fn set_objective(
        &mut self,
        function: ScalarFunctionType,
        sense: ModelSense,
    ) -> Result<(), MoiError> {
        let mut state = self.handle.lock()?;
        Self::validate_function(&state, &function)?;
        Self::maybe_fail(&mut state, RecordedOperation::SetObjective)?;
        state.objective = Some(function.clone());
        state.sense = Some(sense);
        state
            .calls
            .push(RecordedCall::SetObjective { function, sense });
        Self::invalidate_solution(&mut state);
        Ok(())
    }

    fn update(&mut self) -> Result<(), MoiError> {
        let mut state = self.handle.lock()?;
        Self::maybe_fail(&mut state, RecordedOperation::Update)?;
        state.calls.push(RecordedCall::Update);
        Ok(())
    }

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        let state = self.handle.state.lock().ok()?;
        match attr {
            ModelAttr::ObjectiveSense => state.sense.map(AttrValue::ModelSense),
            ModelAttr::ObjectiveFunction => state.objective.clone().map(AttrValue::ScalarFn),
            ModelAttr::ModelName => Some(AttrValue::String(state.name.clone())),
            ModelAttr::NumberOfVariables => Some(AttrValue::Usize(state.variables.len())),
            ModelAttr::NumberOfConstraints => Some(AttrValue::Usize(state.constraints.len())),
            ModelAttr::ListOfVariableIndices => {
                Some(AttrValue::VecUsize((0..state.variables.len()).collect()))
            }
            ModelAttr::TerminationStatus if state.optimized => {
                Some(AttrValue::Status(state.solve_status))
            }
            ModelAttr::ResultCount => Some(AttrValue::Usize(usize::from(
                state.optimized && state.solve_status != SolveStatus::Unknown,
            ))),
            ModelAttr::ObjectiveValue if state.optimized => {
                state.objective_value.map(AttrValue::Float)
            }
            ModelAttr::TerminationStatus | ModelAttr::ObjectiveValue => None,
        }
    }

    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError> {
        match (attr, value) {
            (ModelAttr::ObjectiveSense, AttrValue::ModelSense(sense)) => {
                let objective = self.handle.lock()?.objective.clone().unwrap_or_else(|| {
                    ScalarFunctionType::Affine(moi_core::ScalarAffineFn::default())
                });
                self.set_objective(objective, sense)
            }
            (ModelAttr::ObjectiveFunction, AttrValue::ScalarFn(function)) => {
                let sense = self.handle.lock()?.sense.unwrap_or(ModelSense::Minimize);
                self.set_objective(function, sense)
            }
            (ModelAttr::ModelName, AttrValue::String(name)) => {
                Self::validate_name(&name, "model name")?;
                let mut state = self.handle.lock()?;
                Self::maybe_fail(&mut state, RecordedOperation::SetModelAttr)?;
                state.name = name.clone();
                state.calls.push(RecordedCall::SetModelAttr {
                    attr,
                    value: AttrValue::String(name),
                });
                Ok(())
            }
            (
                ModelAttr::NumberOfVariables
                | ModelAttr::NumberOfConstraints
                | ModelAttr::ListOfVariableIndices
                | ModelAttr::TerminationStatus
                | ModelAttr::ResultCount
                | ModelAttr::ObjectiveValue,
                _,
            ) => Err(MoiError::SetAttributeNotAllowed),
            _ => Err(MoiError::InvalidInput(format!(
                "invalid value type for model attribute {attr:?}"
            ))),
        }
    }

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue> {
        let state = self.handle.state.lock().ok()?;
        if attr == OptimizerAttr::SolverName {
            Some(AttrValue::String("DummyRecordingOptimizer".to_string()))
        } else {
            state.optimizer_attrs.get(&attr).cloned()
        }
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        if attr == OptimizerAttr::SolverName {
            return Err(MoiError::SetAttributeNotAllowed);
        }
        if let OptimizerAttr::Raw(name) = &attr {
            Self::validate_name(name, "optimizer parameter name")?;
        }
        if let AttrValue::Float(value) = &value
            && !value.is_finite()
        {
            return Err(MoiError::InvalidInput(
                "optimizer parameter value must be finite".into(),
            ));
        }
        if let AttrValue::String(value) = &value {
            Self::validate_name(value, "optimizer parameter string value")?;
        }
        let mut state = self.handle.lock()?;
        Self::maybe_fail(&mut state, RecordedOperation::SetOptimizerAttr)?;
        state.optimizer_attrs.insert(attr.clone(), value.clone());
        state
            .calls
            .push(RecordedCall::SetOptimizerAttr { attr, value });
        Self::invalidate_solution(&mut state);
        Ok(())
    }
}

impl Optimizer for DummyModel {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        let mut state = self.handle.lock()?;
        Self::maybe_fail(&mut state, RecordedOperation::Optimize)?;
        state.calls.push(RecordedCall::Optimize);
        state.optimized = true;
        Ok(state.solve_status)
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        Err(MoiError::UnsupportedAttribute)
    }

    fn get_var_value(&self, variable: VarId) -> Result<Option<f64>, MoiError> {
        let state = self.handle.lock()?;
        if variable.0 >= state.variables.len() {
            return Err(MoiError::InvalidVariableIndex(variable.0));
        }
        Ok(state
            .optimized
            .then(|| state.variable_values.get(&variable).copied())
            .flatten())
    }

    fn get_objective_value(&self) -> Result<Option<f64>, MoiError> {
        let state = self.handle.lock()?;
        Ok(state.optimized.then_some(state.objective_value).flatten())
    }
}
