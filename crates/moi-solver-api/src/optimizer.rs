use crate::status::*;
use crate::utils::*;
use moi_core::attributes::*;
use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;

pub trait ModelLike {
    fn add_variable(&mut self, name: Option<&str>, vtype: Option<char>, lb: Option<f64>, ub: Option<f64>) -> VarId;
    fn add_variables(&mut self, n: usize, name: Option<NameType>, vtype: Option<Vec<char>>, lb: Option<BoundType>, ub: Option<BoundType>) -> Vec<VarId>;
    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType, name: Option<String>) -> ConstrId;
    fn add_constraints(&mut self, fs: Vec<ScalarFunctionType>, ss: Vec<ScalarSetType>, names: Option<Vec<String>>) -> Vec<ConstrId>;

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError>;
    fn update(&mut self) -> Result<(), MoiError>;

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue>;
    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError>;

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue>;
    fn set_optimizer_attr(&mut self, attr: OptimizerAttr, value: AttrValue) -> Result<(), MoiError>;
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
    fn get_var_value(&self, var_id: VarId) -> Option<f64>;
    fn get_objective_value(&self) -> Option<f64>;
}
