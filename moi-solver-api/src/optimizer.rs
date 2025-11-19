use moi_core::attributes::Attribute;
use moi_core::errors::MoiError;
use moi_core::functions::{ScalarAffineFn, ScalarFunctionType};
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;

pub trait ModelLike {
    fn add_variable(&mut self) -> VarId;
    fn add_variables(&mut self, n: usize) -> Vec<VarId>;

    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId;

    fn supports_constraint(&self, f: &ScalarFunctionType, s: &ScalarSetType) -> bool;

    fn get_attr<A: Attribute>(&self, key: &A) -> Option<A::Value>;
    fn set_attr<A: Attribute>(&mut self, key: &A, val: A::Value) -> Result<(), MoiError>;

    fn is_empty(&self) -> bool;
    fn empty(&mut self);

    // Objective (MVP: Affine only)
    fn set_objective_affine(&mut self, f: ScalarAffineFn) -> Result<(), MoiError>;
    fn get_objective_affine(&self) -> Option<&ScalarAffineFn>;
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<crate::status::SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
}
