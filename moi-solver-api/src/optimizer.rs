use moi_core::attributes::{AttrValue, ConstraintAttr, ModelAttr, OptimizerAttr, VariableAttr};

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

    // Attribute support queries (generic)
    // Model attributes
    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue>;
    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError>;

    // Optimizer attributes
    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue>;
    fn set_optimizer_attr(&mut self, attr: OptimizerAttr, value: AttrValue)
        -> Result<(), MoiError>;

    // Variable attributes (optional ID associated)
    fn get_variable_attr(&self, attr: VariableAttr, v: VarId) -> Option<AttrValue>;
    fn set_variable_attr(
        &mut self,
        attr: VariableAttr,
        v: VarId,
        value: AttrValue,
    ) -> Result<(), MoiError>;

    // Constraint attributes
    fn get_constraint_attr(&self, attr: ConstraintAttr, c: ConstrId) -> Option<AttrValue>;
    fn set_constraint_attr(
        &mut self,
        attr: ConstraintAttr,
        c: ConstrId,
        value: AttrValue,
    ) -> Result<(), MoiError>;

    fn is_empty(&self) -> bool;
    fn empty(&mut self);
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<crate::status::SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
}
