use moi_core::attributes::{AttrValue, ConstraintAttr, ModelAttr, OptimizerAttr, VariableAttr};
use moi_core::errors::MoiError;
use moi_core::functions::{ScalarAffineFn, ScalarFunctionType};
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
use moi_solver_api::{ModelLike, Optimizer, SolveStatus};
use std::collections::HashMap;

use moi_core::constraint::ScalarConstraint;
use moi_core::variable::Variable;

#[derive(Default)]
pub struct DummyModel {
    name: String,
    variables: Vec<Variable>,
    constraints: Vec<ScalarConstraint>,
    obj: Option<ScalarAffineFn>,
    var_to_name: HashMap<usize, String>,
    name_to_var: HashMap<String, VarId>,
    con_to_name: HashMap<usize, String>,
    name_to_con: HashMap<String, usize>,

    // Separate attribute stores
    model_attrs: HashMap<ModelAttr, AttrValue>,
    optimizer_attrs: HashMap<OptimizerAttr, AttrValue>,
}

impl DummyModel {
    // variable naming helpers (restored)
    pub fn set_var_name<S: Into<String>>(&mut self, v: VarId, name: S) -> Result<(), MoiError> {
        let idx = v.0;
        if idx >= self.variables.len() {
            return Err(MoiError::Msg("invalid VarId".into()));
        }
        let name = name.into();
        self.variables[idx].name = Some(name.clone());
        self.var_to_name.insert(idx, name.clone());
        self.name_to_var.insert(name, v);
        Ok(())
    }
    pub fn get_var_name(&self, v: VarId) -> Option<&str> {
        self.variables.get(v.0).and_then(|x| x.name.as_deref())
    }
    pub fn get_var_by_name(&self, name: &str) -> Option<VarId> {
        self.name_to_var.get(name).copied()
    }
}

// 校验函数
// validation removed: handled by generic typing

// move implementations into the existing ModelLike impl above

// Implement new attribute methods from ModelLike trait
impl ModelLike for DummyModel {
    // core build operations
    fn add_variable(&mut self) -> VarId {
        let id = VarId(self.variables.len());
        self.variables.push(Variable { id, name: None });
        id
    }
    fn add_variables(&mut self, n: usize) -> Vec<VarId> {
        (0..n).map(|_| self.add_variable()).collect()
    }
    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId {
        let id = self.constraints.len();
        let c = ScalarConstraint::new(id, f, s);
        self.constraints.push(c);
        ConstrId::new(id)
    }
    fn supports_constraint(&self, f: &ScalarFunctionType, s: &ScalarSetType) -> bool {
        matches!(f, ScalarFunctionType::Affine(_))
            && matches!(
                s,
                ScalarSetType::GreaterThan(_)
                    | ScalarSetType::LessThan(_)
                    | ScalarSetType::EqualTo(_)
                    | ScalarSetType::Interval(..)
            )
    }

    // unified attribute APIs
    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue> {
        match attr {
            ModelAttr::NumberOfVariables => Some(AttrValue::Usize(self.variables.len())),
            ModelAttr::NumberOfConstraints => Some(AttrValue::Usize(self.constraints.len())),
            ModelAttr::ListOfVariableIndices => {
                let list: Vec<usize> = (0..self.variables.len()).collect();
                Some(AttrValue::VecUsize(list))
            }
            ModelAttr::ObjectiveFunction => self.obj.clone().map(AttrValue::ScalarAffineFn),
            ModelAttr::ModelName => Some(AttrValue::String(self.name.clone())),
            _ => self.model_attrs.get(&attr).cloned(),
        }
    }

    fn set_model_attr(&mut self, attr: ModelAttr, mut value: AttrValue) -> Result<(), MoiError> {
        // derived read-only check
        match attr {
            ModelAttr::NumberOfVariables
            | ModelAttr::NumberOfConstraints
            | ModelAttr::ListOfVariableIndices => {
                return Err(MoiError::SetAttributeNotAllowed);
            }
            ModelAttr::ObjectiveFunction => {
                if let AttrValue::ScalarAffineFn(ref mut f) = value {
                    f.simplify();
                    self.obj = Some(f.clone());
                }
                // We update the specific field but also allow storing it if useful,
                // though derived get logic prefers the field.
            }
            ModelAttr::ModelName => {
                if let AttrValue::String(ref name) = value {
                    self.name = name.clone();
                }
            }
            _ => {}
        }
        self.model_attrs.insert(attr, value);
        Ok(())
    }

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue> {
        self.optimizer_attrs.get(&attr).cloned()
    }

    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttr,
        value: AttrValue,
    ) -> Result<(), MoiError> {
        self.optimizer_attrs.insert(attr, value);
        Ok(())
    }

    fn get_variable_attr(&self, _attr: VariableAttr, _v: VarId) -> Option<AttrValue> {
        None
    }

    fn set_variable_attr(
        &mut self,
        _attr: VariableAttr,
        _v: VarId,
        _value: AttrValue,
    ) -> Result<(), MoiError> {
        Err(MoiError::UnsupportedAttribute)
    }

    fn get_constraint_attr(&self, _attr: ConstraintAttr, _c: ConstrId) -> Option<AttrValue> {
        None
    }

    fn set_constraint_attr(
        &mut self,
        _attr: ConstraintAttr,
        _c: ConstrId,
        _value: AttrValue,
    ) -> Result<(), MoiError> {
        Err(MoiError::UnsupportedAttribute)
    }

    // objective & housekeeping
    fn is_empty(&self) -> bool {
        self.variables.is_empty() && self.constraints.is_empty()
    }
    fn empty(&mut self) {
        self.variables.clear();
        self.var_to_name.clear();
        self.name_to_var.clear();
        self.con_to_name.clear();
        self.name_to_con.clear();
        self.constraints.clear();
        self.model_attrs.clear();
        self.optimizer_attrs.clear();
    }

    // 支持查询：统一版本已在 supports 泛型中实现
}

impl Optimizer for DummyModel {
    fn optimize(&mut self) -> Result<moi_solver_api::SolveStatus, MoiError> {
        Ok(moi_solver_api::SolveStatus::Optimal)
    }
    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        Ok(())
    }
}
