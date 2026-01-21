use crate::status::*;
use crate::utils::*;
use moi_core::attributes::*;
use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
// generic attribute trait usage; TypeId utilized in concrete model implementations

pub trait ModelLike {
    fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> VarId;
    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Vec<VarId>;
    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId;
    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
    ) -> Vec<ConstrId>;

    // fn supports_constraint(&self, f: &ScalarFunctionType, s: &ScalarSetType) -> bool;

    // // Attribute support queries (generic)
    // // Model attributes
    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue>;
    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError>;

    // // Optimizer attributes
    // fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue>;
    // fn set_optimizer_attr(&mut self, attr: OptimizerAttr, value: AttrValue)
    //     -> Result<(), MoiError>;

    // // Variable attributes (optional ID associated)
    // fn get_variable_attr(&self, attr: VariableAttr, v: VarId) -> Option<AttrValue>;
    // fn set_variable_attr(
    //     &mut self,
    //     attr: VariableAttr,
    //     v: VarId,
    //     value: AttrValue,
    // ) -> Result<(), MoiError>;

    // // Constraint attributes
    // fn get_constraint_attr(&self, attr: ConstraintAttr, c: ConstrId) -> Option<AttrValue>;
    // fn set_constraint_attr(
    //     &mut self,
    //     attr: ConstraintAttr,
    //     c: ConstrId,
    //     value: AttrValue,
    // ) -> Result<(), MoiError>;

    // fn is_empty(&self) -> bool;
    // fn empty(&mut self);
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
}
