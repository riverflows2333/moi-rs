use moi_core::MoiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleState {
    Building,
    Attached,
    Solved,
    Dirty,
}

/// Native model metadata skeleton. D1 does not route public operations here.
#[derive(Debug)]
pub struct DirectModel {
    num_variables: usize,
    num_constraints: usize,
    lifecycle: LifecycleState,
}

impl DirectModel {
    pub fn new() -> Self {
        Self {
            num_variables: 0,
            num_constraints: 0,
            lifecycle: LifecycleState::Building,
        }
    }

    pub fn num_variables(&self) -> usize {
        self.num_variables
    }

    pub fn num_constraints(&self) -> usize {
        self.num_constraints
    }

    pub fn lifecycle(&self) -> LifecycleState {
        self.lifecycle
    }

    pub fn record_variables(&mut self, count: usize) -> Result<(), MoiError> {
        self.num_variables = self
            .num_variables
            .checked_add(count)
            .ok_or_else(|| MoiError::InvalidInput("variable count overflow".into()))?;
        self.invalidate_solution();
        Ok(())
    }

    pub fn record_constraints(&mut self, count: usize) -> Result<(), MoiError> {
        self.num_constraints = self
            .num_constraints
            .checked_add(count)
            .ok_or_else(|| MoiError::InvalidInput("constraint count overflow".into()))?;
        self.invalidate_solution();
        Ok(())
    }

    pub fn mark_attached(&mut self) {
        self.lifecycle = LifecycleState::Attached;
    }

    pub fn mark_solved(&mut self) {
        self.lifecycle = LifecycleState::Solved;
    }

    pub fn invalidate_solution(&mut self) {
        if self.lifecycle == LifecycleState::Solved {
            self.lifecycle = LifecycleState::Dirty;
        }
    }
}

impl Default for DirectModel {
    fn default() -> Self {
        Self::new()
    }
}
