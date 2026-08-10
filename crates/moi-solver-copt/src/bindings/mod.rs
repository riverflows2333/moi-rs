//! Raw COPT C API bindings.
//!
//! Keep generated declarations in the versioned module so later COPT releases
//! can coexist behind Cargo features.

#[cfg(feature = "copt80")]
mod gen80;

#[cfg(feature = "copt80")]
pub use gen80::*;
