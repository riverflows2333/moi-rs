use crate::wrapper::utils::native_error;
use crate::{CoptApi, SharedCoptEnv, bindings};
use moi_core::MoiError;
use std::ffi::{c_int, c_void};
use std::ptr;
use std::sync::Arc;

/// MOI optimizer backed by an owning native COPT problem handle.
pub struct CoptOptimizer {
    _env: SharedCoptEnv,
    pub(crate) api: Arc<CoptApi>,
    pub(crate) prob: *mut c_void,
    num_vars: usize,
    num_constrs: usize,
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
        })
    }

    pub fn num_variables(&self) -> usize {
        self.num_vars
    }

    pub fn num_constraints(&self) -> usize {
        self.num_constrs
    }
}

impl Drop for CoptOptimizer {
    fn drop(&mut self) {
        delete_prob(&self.api, &mut self.prob);
    }
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
