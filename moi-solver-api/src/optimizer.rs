use moi_core::Attribute;

use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
// generic attribute trait usage; TypeId utilized in concrete model implementations

pub trait ModelLike {
    fn add_variable(&mut self) -> VarId;
    fn add_variables(&mut self, n: usize) -> Vec<VarId>;
    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId;

    fn supports_constraint(&self, f: &ScalarFunctionType, s: &ScalarSetType) -> bool;
    fn supports<A: Attribute + 'static>(&self) -> bool;
    // Attribute support queries (generic)

    // Generic attribute APIs
    fn get<A: Attribute + 'static>(&self) -> Option<A::Value>;
    fn set<A: Attribute + 'static>(&mut self, value: A::Value) -> Result<(), MoiError>;

    fn is_empty(&self) -> bool;
    fn empty(&mut self);

    // Objective 现在通过 ModelAttribute::ObjectiveFunction + AttributeValue::Affine 处理
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<crate::status::SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
}
