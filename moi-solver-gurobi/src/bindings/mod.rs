// This module exposes the generated bindings.
// You might want to use features to conditionally compile valid bindings 
// if multiple are present in the future.
mod gen120;

#[cfg(feature = "gurobi120")]
use gen120 as sys;

#[cfg(feature = "gurobi130")]
use gen130 as sys;