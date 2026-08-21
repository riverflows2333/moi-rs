use crate::wrapper::utils::{license_message, native_error};
use crate::{CoptApi, bindings};
use moi_core::MoiError;
use std::ffi::{CString, c_int, c_void};
use std::ptr;
use std::sync::{Arc, Mutex};

/// Shared environment type used by one or more optimizer instances.
pub type SharedCoptEnv = Arc<Mutex<CoptEnv>>;

/// Owning wrapper around a native COPT client configuration.
#[derive(Debug)]
pub struct CoptEnvConfig {
    api: Arc<CoptApi>,
    raw: *mut c_void,
}

impl CoptEnvConfig {
    /// Create an empty client configuration.
    pub fn new(api: Arc<CoptApi>) -> Result<Self, MoiError> {
        let mut raw = ptr::null_mut();
        // SAFETY: `raw` points to writable storage for the native handle and
        // is owned by the resulting wrapper after a successful call.
        let code = unsafe { (api.COPT_CreateEnvConfig)(&mut raw) };
        if code != bindings::COPT_RETCODE_OK as c_int {
            delete_env_config(&api, &mut raw);
            return Err(native_error(&api, code, "COPT_CreateEnvConfig"));
        }
        if raw.is_null() {
            return Err(MoiError::BackendState(
                "COPT_CreateEnvConfig succeeded without returning a configuration".into(),
            ));
        }
        Ok(Self { api, raw })
    }

    /// Set one COPT client configuration entry.
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), MoiError> {
        let name = CString::new(name).map_err(|_| {
            MoiError::InvalidInput(
                "COPT environment configuration name contains an embedded NUL byte".into(),
            )
        })?;
        let value = CString::new(value).map_err(|_| {
            MoiError::InvalidInput(
                "COPT environment configuration value contains an embedded NUL byte".into(),
            )
        })?;
        // SAFETY: all pointers remain valid for the duration of the call and
        // `raw` is a live configuration owned by this wrapper.
        let code = unsafe { (self.api.COPT_SetEnvConfig)(self.raw, name.as_ptr(), value.as_ptr()) };
        if code == bindings::COPT_RETCODE_OK as c_int {
            Ok(())
        } else {
            Err(native_error(&self.api, code, "COPT_SetEnvConfig"))
        }
    }
}

impl Drop for CoptEnvConfig {
    fn drop(&mut self) {
        delete_env_config(&self.api, &mut self.raw);
    }
}

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

    /// Create an environment from a COPT client configuration.
    pub fn with_config(config: &CoptEnvConfig) -> Result<Self, MoiError> {
        let api = config.api.clone();
        let mut raw = ptr::null_mut();
        // SAFETY: the configuration remains alive for the duration of the
        // call, and `raw` points to writable storage for the environment.
        let code = unsafe { (api.COPT_CreateEnvWithConfig)(config.raw, &mut raw) };
        Self::finish_create(api, raw, code, "COPT_CreateEnvWithConfig")
    }

    fn create(api: Arc<CoptApi>, license_dir: Option<&CString>) -> Result<Self, MoiError> {
        let mut raw = ptr::null_mut();
        let (code, operation) = match license_dir {
            Some(path) => (
                unsafe { (api.COPT_CreateEnvWithPath)(path.as_ptr(), &mut raw) },
                "COPT_CreateEnvWithPath",
            ),
            None => (unsafe { (api.COPT_CreateEnv)(&mut raw) }, "COPT_CreateEnv"),
        };

        Self::finish_create(api, raw, code, operation)
    }

    fn finish_create(
        api: Arc<CoptApi>,
        mut raw: *mut c_void,
        code: c_int,
        operation: &'static str,
    ) -> Result<Self, MoiError> {
        if code != bindings::COPT_RETCODE_OK as c_int {
            let diagnostic = license_message(&api, raw);
            let mut error = native_error(&api, code, operation);
            if let Some(diagnostic) = diagnostic {
                error = MoiError::Msg(format!("{error}; license: {diagnostic}"));
            }
            delete_env(&api, &mut raw);
            return Err(error);
        }

        if raw.is_null() {
            return Err(MoiError::BackendState(format!(
                "{operation} succeeded without returning an environment"
            )));
        }

        Ok(Self { api, raw })
    }
}

fn delete_env_config(api: &CoptApi, raw: &mut *mut c_void) {
    if raw.is_null() {
        return;
    }
    // SAFETY: `raw` is owned by this wrapper and COPT_DeleteEnvConfig accepts
    // the address of the native handle.
    let _ = unsafe { (api.COPT_DeleteEnvConfig)(raw) };
    *raw = ptr::null_mut();
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
