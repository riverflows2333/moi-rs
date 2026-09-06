use moi_core::{
    AttrValue, ConstrId, ModelSense, MoiError, OptimizerAttr, ScalarFunctionType, ScalarSetType,
    SolveStatus, VarId,
};
use moi_solver_api::{BoundType, NameType, Optimizer};
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Building,
    Attached,
    Solved,
    Dirty,
}

struct DirectState {
    optimizer: Option<Box<dyn Optimizer + Send>>,
    num_variables: usize,
    num_constraints: usize,
    lifecycle: LifecycleState,
    poisoned: Option<(String, String)>,
}

/// A native optimizer plus only the metadata needed to validate its lifecycle.
///
/// This type does not retain variable definitions, constraint functions, sets,
/// names, or an objective function that could replay the model.
#[derive(Clone)]
pub struct DirectModel {
    state: Arc<Mutex<DirectState>>,
}

impl fmt::Debug for DirectModel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state.lock() {
            Ok(state) => formatter
                .debug_struct("DirectModel")
                .field("num_variables", &state.num_variables)
                .field("num_constraints", &state.num_constraints)
                .field("lifecycle", &state.lifecycle)
                .field("poisoned", &state.poisoned.is_some())
                .finish(),
            Err(_) => formatter
                .debug_struct("DirectModel")
                .field("state", &"lock poisoned")
                .finish(),
        }
    }
}

impl DirectModel {
    pub fn add_linear_rows(&self, rows: moi_solver_api::LinearRows) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        let count = rows.num_rows();
        let start = state.num_constraints;
        let end = start
            .checked_add(count)
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        let result = Self::optimizer(&mut state)?.add_linear_rows(rows);
        let range = Self::finish(&mut state, "add linear rows", result)?;
        if range != (start..end) {
            return Self::finish(
                &mut state,
                "add linear rows",
                Err(MoiError::BackendProtocol(
                    "unexpected linear row IDs".into(),
                )),
            );
        }
        state.num_constraints = end;
        if count != 0 {
            Self::invalidate_solution(&mut state);
        }
        Ok(())
    }

    pub fn set_linear_objective(
        &self,
        objective: moi_solver_api::LinearObjective,
        sense: ModelSense,
    ) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        let result = Self::optimizer(&mut state)?.set_linear_objective(objective, sense);
        Self::finish(&mut state, "set linear objective", result)?;
        Self::invalidate_solution(&mut state);
        Ok(())
    }
    /// Metadata-only constructor retained for runtime state tests.
    pub fn new() -> Self {
        Self::from_optimizer(None)
    }

    pub fn with_optimizer(optimizer: Box<dyn Optimizer + Send>) -> Self {
        Self::from_optimizer(Some(optimizer))
    }

    fn from_optimizer(optimizer: Option<Box<dyn Optimizer + Send>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(DirectState {
                optimizer,
                num_variables: 0,
                num_constraints: 0,
                lifecycle: LifecycleState::Building,
                poisoned: None,
            })),
        }
    }

    fn lock(&self) -> Result<MutexGuard<'_, DirectState>, MoiError> {
        let state = self
            .state
            .lock()
            .map_err(|_| MoiError::BackendState("model state lock is poisoned".into()))?;
        if let Some((operation, reason)) = &state.poisoned {
            return Err(MoiError::BackendState(format!(
                "model is poisoned after '{operation}': {reason}"
            )));
        }
        Ok(state)
    }

    fn optimizer(state: &mut DirectState) -> Result<&mut (dyn Optimizer + Send + '_), MoiError> {
        match state.optimizer.as_mut() {
            Some(optimizer) => Ok(optimizer.as_mut()),
            None => Err(MoiError::BackendState(
                "direct model has no native optimizer attached".into(),
            )),
        }
    }

    fn finish<T>(
        state: &mut DirectState,
        operation: &str,
        result: Result<T, MoiError>,
    ) -> Result<T, MoiError> {
        if let Err(error) = &result
            && matches!(
                error,
                MoiError::NativeSolver { .. } | MoiError::BackendProtocol(_) | MoiError::Msg(_)
            )
        {
            state.poisoned = Some((operation.to_string(), error.to_string()));
        }
        result
    }

    pub fn num_variables(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.num_variables)
            .unwrap_or(0)
    }

    pub fn num_constraints(&self) -> usize {
        self.state
            .lock()
            .map(|state| state.num_constraints)
            .unwrap_or(0)
    }

    pub fn lifecycle(&self) -> LifecycleState {
        self.state
            .lock()
            .map(|state| state.lifecycle)
            .unwrap_or(LifecycleState::Dirty)
    }

    pub fn add_variable(
        &self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> Result<VarId, MoiError> {
        let mut state = self.lock()?;
        let end = state
            .num_variables
            .checked_add(1)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;
        let result = Self::optimizer(&mut state)?.add_variable(name, vtype, lb, ub);
        let id = Self::finish(&mut state, "add variable", result)?;
        let expected = state.num_variables;
        if id.0 != expected {
            let error = MoiError::BackendProtocol(format!(
                "backend returned variable ID {}, expected {expected}",
                id.0
            ));
            return Self::finish(&mut state, "add variable", Err(error));
        }
        state.num_variables = end;
        Self::invalidate_solution(&mut state);
        Ok(id)
    }

    pub fn add_variables(
        &self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError> {
        let mut state = self.lock()?;
        let start = state.num_variables;
        let end = start
            .checked_add(n)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;
        let result = Self::optimizer(&mut state)?.add_variables(n, name, vtype, lb, ub);
        let ids = Self::finish(&mut state, "add variables", result)?;
        if let Err(error) = Self::validate_ids(ids.iter().map(|id| id.0), start, n, "variable") {
            state.poisoned = Some(("add variables".into(), error.to_string()));
            return Err(error);
        }
        state.num_variables = end;
        if n != 0 {
            Self::invalidate_solution(&mut state);
        }
        Ok(ids)
    }

    pub fn add_constraint(
        &self,
        function: ScalarFunctionType,
        set: ScalarSetType,
        name: Option<String>,
    ) -> Result<ConstrId, MoiError> {
        let mut state = self.lock()?;
        let end = state
            .num_constraints
            .checked_add(1)
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        let result = Self::optimizer(&mut state)?.add_constraint(function, set, name);
        let id = Self::finish(&mut state, "add constraint", result)?;
        let expected = state.num_constraints;
        if id.0 != expected {
            let error = MoiError::BackendProtocol(format!(
                "backend returned constraint ID {}, expected {expected}",
                id.0
            ));
            return Self::finish(&mut state, "add constraint", Err(error));
        }
        state.num_constraints = end;
        Self::invalidate_solution(&mut state);
        Ok(id)
    }

    pub fn add_constraints(
        &self,
        functions: Vec<ScalarFunctionType>,
        sets: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError> {
        let mut state = self.lock()?;
        let start = state.num_constraints;
        let count = functions.len();
        let end = start
            .checked_add(count)
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        let result = Self::optimizer(&mut state)?.add_constraints(functions, sets, names);
        let ids = Self::finish(&mut state, "add constraints", result)?;
        if let Err(error) =
            Self::validate_ids(ids.iter().map(|id| id.0), start, count, "constraint")
        {
            state.poisoned = Some(("add constraints".into(), error.to_string()));
            return Err(error);
        }
        state.num_constraints = end;
        if count != 0 {
            Self::invalidate_solution(&mut state);
        }
        Ok(ids)
    }

    pub fn set_objective(
        &self,
        function: ScalarFunctionType,
        sense: ModelSense,
    ) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        let result = Self::optimizer(&mut state)?.set_objective(function, sense);
        Self::finish(&mut state, "set objective", result)?;
        Self::invalidate_solution(&mut state);
        Ok(())
    }

    pub fn set_optimizer_attr(
        &self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        let result = Self::optimizer(&mut state)?.set_optimizer_attr(attr, value);
        Self::finish(&mut state, "set parameter", result)?;
        Self::invalidate_solution(&mut state);
        Ok(())
    }

    pub fn update(&self) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        let result = Self::optimizer(&mut state)?.update();
        Self::finish(&mut state, "update", result)
    }

    pub fn optimize(&self) -> Result<SolveStatus, MoiError> {
        let mut state = self.lock()?;
        let result = Self::optimizer(&mut state)?.optimize();
        let status = Self::finish(&mut state, "optimize", result)?;
        state.lifecycle = LifecycleState::Solved;
        Ok(status)
    }

    pub fn get_var_value(&self, id: VarId) -> Result<Option<f64>, MoiError> {
        let mut state = self.lock()?;
        if state.lifecycle != LifecycleState::Solved {
            return Ok(None);
        }
        Self::optimizer(&mut state)?.get_var_value(id)
    }

    pub fn get_objective_value(&self) -> Result<Option<f64>, MoiError> {
        let mut state = self.lock()?;
        if state.lifecycle != LifecycleState::Solved {
            return Ok(None);
        }
        Self::optimizer(&mut state)?.get_objective_value()
    }

    pub fn record_variables(&self, count: usize) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        state.num_variables = state
            .num_variables
            .checked_add(count)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;
        Self::invalidate_solution(&mut state);
        Ok(())
    }

    pub fn record_constraints(&self, count: usize) -> Result<(), MoiError> {
        let mut state = self.lock()?;
        state.num_constraints = state
            .num_constraints
            .checked_add(count)
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        Self::invalidate_solution(&mut state);
        Ok(())
    }

    pub fn mark_attached(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.lifecycle = LifecycleState::Attached;
        }
    }

    pub fn mark_solved(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.lifecycle = LifecycleState::Solved;
        }
    }

    fn invalidate_solution(state: &mut DirectState) {
        state.lifecycle = match state.lifecycle {
            LifecycleState::Solved | LifecycleState::Dirty => LifecycleState::Dirty,
            LifecycleState::Building | LifecycleState::Attached => LifecycleState::Attached,
        };
    }

    fn validate_ids(
        ids: impl ExactSizeIterator<Item = usize>,
        start: usize,
        expected_len: usize,
        kind: &str,
    ) -> Result<(), MoiError> {
        if ids.len() != expected_len {
            return Err(MoiError::BackendProtocol(format!(
                "backend returned {} {kind} IDs, expected {expected_len}",
                ids.len()
            )));
        }
        for (offset, actual) in ids.into_iter().enumerate() {
            let expected = start + offset;
            if actual != expected {
                return Err(MoiError::BackendProtocol(format!(
                    "backend returned {kind} ID {actual}, expected {expected}"
                )));
            }
        }
        Ok(())
    }
}

impl Default for DirectModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod linear_failure_tests {
    use super::*;
    #[test]
    fn diagnostic_native_error_poison_is_shared_with_result_handles() {
        let direct = DirectModel::new();
        let handle = direct.clone();
        {
            let mut state = direct.lock().unwrap();
            // COPT returns Msg when its error API provides a diagnostic string.
            let result: Result<(), MoiError> = Err(MoiError::Msg("native failure".into()));
            assert!(DirectModel::finish(&mut state, "add linear rows", result).is_err());
        }
        assert!(
            handle
                .get_objective_value()
                .unwrap_err()
                .to_string()
                .contains("poisoned")
        );
        assert!(
            direct
                .update()
                .unwrap_err()
                .to_string()
                .contains("add linear rows")
        );
    }
}
