//! Native COPT backend for the solver-independent MOI interface.
//!
//! The crate is intentionally split into generated bindings, dynamic loading,
//! and safe wrapper layers. The COPT installation is only needed at runtime.

pub mod bindings;
pub mod dynamic;
pub mod wrapper;

pub use dynamic::*;
pub use wrapper::*;
