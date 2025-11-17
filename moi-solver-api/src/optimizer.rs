use moi_core::errors::MoiError;
use moi_core::indices::{ConstrId, VarId};
use moi_core::traits::{Function, Set};
use moi_core::attributes::Attribute;

pub trait ModelLike {
    fn add_variable(&mut self) -> VarId;
    fn add_variables(&mut self, n: usize) -> Vec<VarId>;

    fn add_constraint<F, S>(&mut self, f: F, s: S) -> ConstrId<F, S>
    where
        F: Function,
        S: Set;

    fn supports_constraint<F, S>(&self) -> bool
    where
        F: Function,
        S: Set;

    fn get_attr<A: Attribute>(&self, key: &A) -> Option<A::Value>;
    fn set_attr<A: Attribute>(&mut self, key: &A, val: A::Value) -> Result<(), MoiError>;

    fn is_empty(&self) -> bool;
    fn empty(&mut self);
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<crate::status::SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
}
