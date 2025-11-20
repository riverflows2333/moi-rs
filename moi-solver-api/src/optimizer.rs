use moi_core::attributes::{AttributeValue, ModelAttribute, OptimizerAttribute, VariableAttribute, ConstraintAttribute};
use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;

pub trait ModelLike {
    fn add_variable(&mut self) -> VarId;
    fn add_variables(&mut self, n: usize) -> Vec<VarId>;

    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId;

    fn supports_constraint(&self, f: &ScalarFunctionType, s: &ScalarSetType) -> bool;

    // Attribute support queries
    fn supports_optimizer_attr(&self, attr: &OptimizerAttribute) -> bool;
    fn supports_model_attr(&self, attr: &ModelAttribute) -> bool;
    fn supports_variable_attr(&self, attr: &VariableAttribute) -> bool;
    fn supports_constraint_attr(&self, attr: &ConstraintAttribute) -> bool;

    // Unified attribute APIs
    fn set_optimizer_attr(&mut self, attr: OptimizerAttribute, val: AttributeValue) -> Result<(), MoiError>;
    fn get_optimizer_attr(&self, attr: &OptimizerAttribute) -> Option<&AttributeValue>;
    fn set_model_attr(&mut self, attr: ModelAttribute, val: AttributeValue) -> Result<(), MoiError>;
    fn get_model_attr(&self, attr: &ModelAttribute) -> Option<AttributeValue>;
    fn set_variable_attr(&mut self, var: VarId, attr: VariableAttribute, val: AttributeValue) -> Result<(), MoiError>;
    fn get_variable_attr(&self, var: VarId, attr: &VariableAttribute) -> Option<&AttributeValue>;
    fn set_constraint_attr(&mut self, cid: ConstrId, attr: ConstraintAttribute, val: AttributeValue) -> Result<(), MoiError>;
    fn get_constraint_attr(&self, cid: ConstrId, attr: &ConstraintAttribute) -> Option<&AttributeValue>;

    fn is_empty(&self) -> bool;
    fn empty(&mut self);

    // Objective 现在通过 ModelAttribute::ObjectiveFunction + AttributeValue::Affine 处理
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<crate::status::SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
}
