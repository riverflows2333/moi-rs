use crate::cached::CachedModel;
use crate::direct::DirectModel;
use crate::utils::SharedBridge;
use moi_core::MoiError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeKind {
    Cached,
    Direct,
    Poisoned,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoisonedState {
    pub operation: String,
    pub reason: String,
}

pub enum ModelRuntime {
    Cached(CachedModel),
    Direct(DirectModel),
    Poisoned(PoisonedState),
}

impl ModelRuntime {
    pub fn new_cached() -> Self {
        Self::Cached(CachedModel::new())
    }

    pub fn new_direct() -> Self {
        Self::Direct(DirectModel::new())
    }

    pub fn kind(&self) -> RuntimeKind {
        match self {
            Self::Cached(_) => RuntimeKind::Cached,
            Self::Direct(_) => RuntimeKind::Direct,
            Self::Poisoned(_) => RuntimeKind::Poisoned,
        }
    }

    pub fn cached(&self, operation: &str) -> Result<&CachedModel, MoiError> {
        match self {
            Self::Cached(model) => Ok(model),
            Self::Direct(_) => Err(MoiError::BackendState(format!(
                "{operation} is unavailable while the model runtime is direct"
            ))),
            Self::Poisoned(state) => Err(poisoned_error(state)),
        }
    }

    pub fn cached_mut(&mut self, operation: &str) -> Result<&mut CachedModel, MoiError> {
        match self {
            Self::Cached(model) => Ok(model),
            Self::Direct(_) => Err(MoiError::BackendState(format!(
                "{operation} is unavailable while the model runtime is direct"
            ))),
            Self::Poisoned(state) => Err(poisoned_error(state)),
        }
    }

    pub fn cached_bridge(&self, operation: &str) -> Result<&SharedBridge, MoiError> {
        Ok(self.cached(operation)?.bridge())
    }

    pub fn direct(&self, operation: &str) -> Result<&DirectModel, MoiError> {
        match self {
            Self::Direct(model) => Ok(model),
            Self::Cached(_) => Err(MoiError::BackendState(format!(
                "{operation} is unavailable while the model runtime is cached"
            ))),
            Self::Poisoned(state) => Err(poisoned_error(state)),
        }
    }

    pub fn direct_mut(&mut self, operation: &str) -> Result<&mut DirectModel, MoiError> {
        match self {
            Self::Direct(model) => Ok(model),
            Self::Cached(_) => Err(MoiError::BackendState(format!(
                "{operation} is unavailable while the model runtime is cached"
            ))),
            Self::Poisoned(state) => Err(poisoned_error(state)),
        }
    }

    pub fn poison(&mut self, operation: impl Into<String>, reason: impl Into<String>) {
        *self = Self::Poisoned(PoisonedState {
            operation: operation.into(),
            reason: reason.into(),
        });
    }
}

impl Default for ModelRuntime {
    fn default() -> Self {
        Self::new_cached()
    }
}

fn poisoned_error(state: &PoisonedState) -> MoiError {
    MoiError::BackendState(format!(
        "model is poisoned after '{}': {}",
        state.operation, state.reason
    ))
}

#[cfg(test)]
mod tests {
    use super::{ModelRuntime, RuntimeKind};
    use crate::direct::LifecycleState;

    #[test]
    fn runtime_defaults_to_the_existing_cached_path() {
        let runtime = ModelRuntime::default();

        assert_eq!(runtime.kind(), RuntimeKind::Cached);
        assert!(runtime.cached_bridge("test").is_ok());
        assert!(runtime.direct("test").is_err());
    }

    #[test]
    fn direct_metadata_tracks_counts_and_lifecycle_without_a_cache() {
        let mut runtime = ModelRuntime::new_direct();
        let direct = runtime.direct_mut("test").unwrap();

        direct.mark_attached();
        direct.record_variables(3).unwrap();
        direct.record_constraints(2).unwrap();
        direct.mark_solved();
        assert_eq!(direct.num_variables(), 3);
        assert_eq!(direct.num_constraints(), 2);
        assert_eq!(direct.lifecycle(), LifecycleState::Solved);

        direct.record_constraints(1).unwrap();
        assert_eq!(direct.lifecycle(), LifecycleState::Dirty);
    }

    #[test]
    fn poisoned_runtime_rejects_cached_and_direct_access_consistently() {
        let mut runtime = ModelRuntime::default();
        runtime.poison("add constraints", "injected backend failure");

        assert_eq!(runtime.kind(), RuntimeKind::Poisoned);
        let cached_error = runtime.cached("optimize").err().unwrap().to_string();
        let direct_error = runtime.direct("optimize").err().unwrap().to_string();
        assert_eq!(cached_error, direct_error);
        assert!(cached_error.contains("add constraints"));
        assert!(cached_error.contains("injected backend failure"));
    }
}
