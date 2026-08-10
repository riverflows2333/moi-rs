use crate::wrapper::utils::{license_message, native_error};
use crate::{CoptApi, bindings};
use moi_core::MoiError;
use std::ffi::{CString, c_int, c_void};
use std::ptr;
use std::sync::{Arc, Mutex};

/// Shared environment type used by one or more optimizer instances.
pub type SharedCoptEnv = Arc<Mutex<CoptEnv>>;

/// Owning wrapper around a native COPT environment.
#[derive(Debug)]
pub struct CoptEnv {
    pub(crate) api: Arc<CoptApi>,
    pub(crate) raw: *mut c_void,
}

// COPT environments are protected by `SharedCoptEnv` before being shared.
// No concurrent access to the raw handle is performed by this wrapper.
unsafe impl Send for CoptEnv {}

impl CoptEnv {
    /// Create an environment using COPT's default license discovery.
    pub fn new(api: Arc<CoptApi>) -> Result<Self, MoiError> {
        Self::create(api, None)
    }

    /// Create an environment using a specific COPT license directory.
    pub fn with_license_dir(api: Arc<CoptApi>, license_dir: &str) -> Result<Self, MoiError> {
        let license_dir = CString::new(license_dir).map_err(|_| {
            MoiError::InvalidInput("COPT license directory contains an embedded NUL byte".into())
        })?;
        Self::create(api, Some(&license_dir))
    }

    fn create(api: Arc<CoptApi>, license_dir: Option<&CString>) -> Result<Self, MoiError> {
        let mut raw = ptr::null_mut();
        let code = match license_dir {
            Some(path) => unsafe { (api.COPT_CreateEnvWithPath)(path.as_ptr(), &mut raw) },
            None => unsafe { (api.COPT_CreateEnv)(&mut raw) },
        };

        if code != bindings::COPT_RETCODE_OK as c_int {
            let diagnostic = license_message(&api, raw);
            let mut error = native_error(&api, code, "COPT_CreateEnv");
            if let Some(diagnostic) = diagnostic {
                error = MoiError::Msg(format!("{error}; license: {diagnostic}"));
            }
            delete_env(&api, &mut raw);
            return Err(error);
        }

        if raw.is_null() {
            return Err(MoiError::BackendState(
                "COPT_CreateEnv succeeded without returning an environment".into(),
            ));
        }

        Ok(Self { api, raw })
    }
}

impl Drop for CoptEnv {
    fn drop(&mut self) {
        delete_env(&self.api, &mut self.raw);
    }
}

fn delete_env(api: &CoptApi, raw: &mut *mut c_void) {
    if raw.is_null() {
        return;
    }
    // SAFETY: `raw` is owned by this wrapper and COPT_DeleteEnv accepts the
    // address of the native handle. There is no recovery action available in
    // Drop if native cleanup reports an error.
    let _ = unsafe { (api.COPT_DeleteEnv)(raw) };
    *raw = ptr::null_mut();
}
