use crate::CoptApi;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

/// Owning wrapper around a native COPT environment.
///
/// Construction and destruction will be implemented after binding generation.
pub struct CoptEnv {
    _api: Arc<CoptApi>,
    _raw: *mut c_void,
}

/// Shared environment type used by one or more optimizer instances.
pub type SharedCoptEnv = Arc<Mutex<CoptEnv>>;
