use crate::utils::*;
use moi_core::attributes::*;
use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;

pub trait ModelLike {
    fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> Result<VarId, MoiError> {
        let mut ids = self.add_variables(
            1,
            name.map(|s| NameType::Vector(vec![s.to_string()])),
            vtype.map(|v| vec![v]),
            lb.map(|v| BoundType::Vector(vec![v])),
            ub.map(|v| BoundType::Vector(vec![v])),
        )?;
        ensure_len(ids.len(), 1, "returned variable IDs")?;
        ids.pop().ok_or_else(|| {
            MoiError::BackendProtocol("add_variables returned no variable ID".to_string())
        })
    }

    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError>;

    fn add_constraint(
        &mut self,
        f: ScalarFunctionType,
        s: ScalarSetType,
        name: Option<String>,
    ) -> Result<ConstrId, MoiError> {
        let mut ids = self.add_constraints(vec![f], vec![s], name.map(|n| vec![n]))?;
        ensure_len(ids.len(), 1, "returned constraint IDs")?;
        ids.pop().ok_or_else(|| {
            MoiError::BackendProtocol("add_constraints returned no constraint ID".to_string())
        })
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError>;

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError>;
    fn update(&mut self) -> Result<(), MoiError>;

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue>;
    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError>;

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue>;
    fn set_optimizer_attr(&mut self, attr: OptimizerAttr, value: AttrValue)
    -> Result<(), MoiError>;
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
    fn get_var_value(&self, var_id: VarId) -> Option<f64>;
    fn get_objective_value(&self) -> Option<f64>;
}
