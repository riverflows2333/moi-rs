use moi_core::attributes::{
    AttributeValue, ConstraintAttribute, ModelAttribute, OptimizerAttribute, VariableAttribute,
};
use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
use moi_solver_api::{ModelLike, Optimizer};
use std::collections::HashMap;

use moi_core::constraint::ScalarConstraint;
use moi_core::variable::Variable;
// unified: no fallback wrapper

#[derive(Default)]
pub struct DummyModel {
    // core storage
    // removed counters & generic attr box storage
    name: String,
    variables: Vec<Variable>,
    constraints: Vec<ScalarConstraint>,
    var_to_name: HashMap<usize, String>,
    name_to_var: HashMap<String, VarId>,
    con_to_name: HashMap<usize, String>,
    name_to_con: HashMap<String, usize>,
    // attribute maps
    optimizer_attr: HashMap<OptimizerAttribute, AttributeValue>,
    model_attr: HashMap<ModelAttribute, AttributeValue>,
    variable_attr: HashMap<VariableAttribute, HashMap<VarId, AttributeValue>>,
    constraint_attr: HashMap<ConstraintAttribute, HashMap<ConstrId, AttributeValue>>,
}

impl Optimizer for DummyModel {
    fn optimize(&mut self) -> Result<moi_solver_api::SolveStatus, MoiError> {
        // MVP: 伪求解器，直接返回Optimal
        Ok(moi_solver_api::SolveStatus::Optimal)
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        // 未实现，MVP返回Ok
        Ok(())
    }
}

impl DummyModel {
    pub fn set_model_name<S: Into<String>>(&mut self, name: S) {
        self.name = name.into();
    }

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

    // constraint naming APIs could be added later using con_to_name/name_to_con
}

// 属性 set/get API（直接在 DummyModel 上）
impl DummyModel {
    pub fn set_optimizer_attr(
        &mut self,
        k: OptimizerAttribute,
        v: AttributeValue,
    ) -> Result<(), MoiError> {
        if !validate_optimizer_attr(&k, &v) {
            return Err(MoiError::Msg("attribute value type mismatch".into()));
        }
        self.optimizer_attr.insert(k, v);
        Ok(())
    }
    pub fn get_optimizer_attr(&self, k: &OptimizerAttribute) -> Option<&AttributeValue> {
        self.optimizer_attr.get(k)
    }
    pub fn set_model_attr(
        &mut self,
        k: ModelAttribute,
        mut v: AttributeValue,
    ) -> Result<(), MoiError> {
        if !self.supports_model_attr(&k) {
            return Err(MoiError::UnsupportedAttribute);
        }
        // 派生只读属性禁止写：变量/约束计数、变量索引列表
        match k {
            ModelAttribute::NumberOfVariables
            | ModelAttribute::NumberOfConstraints
            | ModelAttribute::ListOfVariableIndices => {
                return Err(MoiError::SetAttributeNotAllowed);
            }
            ModelAttribute::ObjectiveFunction => {
                // 需要进行仿射函数简化
                if let AttributeValue::Affine(ref mut aff) = v {
                    aff.simplify();
                }
            }
            ModelAttribute::SolverName => { /* 冗余映射到 optimizer 属性 */ }
            ModelAttribute::Silent => { /* 同上 */ }
            _ => {}
        }
        if !validate_model_attr(&k, &v) {
            return Err(MoiError::Msg("attribute value type mismatch".into()));
        }
        // 冗余同步：SolverName / Silent -> optimizer_attr
        match (&k, &v) {
            (ModelAttribute::SolverName, AttributeValue::String(s)) => {
                self.optimizer_attr.insert(
                    OptimizerAttribute::SolverName,
                    AttributeValue::String(s.clone()),
                );
            }
            (ModelAttribute::Silent, AttributeValue::Bool(b)) => {
                self.optimizer_attr
                    .insert(OptimizerAttribute::Silent, AttributeValue::Bool(*b));
            }
            _ => {}
        }
        self.model_attr.insert(k, v);
        Ok(())
    }
    pub fn get_model_attr(&self, k: &ModelAttribute) -> Option<AttributeValue> {
        if !self.supports_model_attr(k) {
            return None;
        }
        match k {
            ModelAttribute::NumberOfVariables => Some(AttributeValue::USize(self.variables.len())),
            ModelAttribute::NumberOfConstraints => {
                Some(AttributeValue::USize(self.constraints.len()))
            }
            ModelAttribute::ListOfVariableIndices => Some(AttributeValue::VarIndices(
                (0..self.variables.len()).collect(),
            )),
            ModelAttribute::SolverName => self
                .optimizer_attr
                .get(&OptimizerAttribute::SolverName)
                .cloned(),
            ModelAttribute::Silent => self
                .optimizer_attr
                .get(&OptimizerAttribute::Silent)
                .cloned(),
            _ => self.model_attr.get(k).cloned(),
        }
    }
    pub fn set_variable_attr(
        &mut self,
        var: VarId,
        k: VariableAttribute,
        v: AttributeValue,
    ) -> Result<(), MoiError> {
        if !validate_variable_attr(&k, &v) {
            return Err(MoiError::Msg("attribute value type mismatch".into()));
        }
        self.variable_attr
            .entry(k.clone())
            .or_default()
            .insert(var, v);
        Ok(())
    }
    pub fn get_variable_attr(&self, var: VarId, k: &VariableAttribute) -> Option<&AttributeValue> {
        self.variable_attr.get(k).and_then(|m| m.get(&var))
    }
    pub fn set_constraint_attr(
        &mut self,
        cid: ConstrId,
        k: ConstraintAttribute,
        v: AttributeValue,
    ) -> Result<(), MoiError> {
        if !validate_constraint_attr(&k, &v) {
            return Err(MoiError::Msg("attribute value type mismatch".into()));
        }
        self.constraint_attr
            .entry(k.clone())
            .or_default()
            .insert(cid, v);
        Ok(())
    }
    pub fn get_constraint_attr(
        &self,
        cid: ConstrId,
        k: &ConstraintAttribute,
    ) -> Option<&AttributeValue> {
        self.constraint_attr.get(k).and_then(|m| m.get(&cid))
    }
}

// 校验函数
fn validate_optimizer_attr(k: &OptimizerAttribute, v: &AttributeValue) -> bool {
    match k {
        OptimizerAttribute::SolverName => matches!(v, AttributeValue::String(_)),
        OptimizerAttribute::Silent => matches!(v, AttributeValue::Bool(_)),
    }
}
fn validate_model_attr(k: &ModelAttribute, v: &AttributeValue) -> bool {
    match k {
        ModelAttribute::ObjectiveSense => matches!(v, AttributeValue::Sense(_)),
        ModelAttribute::ObjectiveFunction => matches!(v, AttributeValue::Affine(_)),
        ModelAttribute::NumberOfVariables => matches!(v, AttributeValue::USize(_)), // set 时已阻止
        ModelAttribute::NumberOfConstraints => matches!(v, AttributeValue::USize(_)),
        ModelAttribute::ListOfVariableIndices => matches!(v, AttributeValue::VarIndices(_)),
        ModelAttribute::Name => matches!(v, AttributeValue::String(_)),
        ModelAttribute::TerminationStatus => matches!(v, AttributeValue::TerminationStatus(_)),
        ModelAttribute::ResultCount => matches!(v, AttributeValue::USize(_)),
        ModelAttribute::ObjectiveValue => matches!(v, AttributeValue::F64(_)),
        ModelAttribute::SolverName => matches!(v, AttributeValue::String(_)),
        ModelAttribute::Silent => matches!(v, AttributeValue::Bool(_)),
    }
}
fn validate_variable_attr(k: &VariableAttribute, v: &AttributeValue) -> bool {
    match k {
        VariableAttribute::Primal => matches!(v, AttributeValue::Primal(_)),
    }
}
fn validate_constraint_attr(k: &ConstraintAttribute, v: &AttributeValue) -> bool {
    match k {
        ConstraintAttribute::Slack => matches!(v, AttributeValue::Slack(_)),
        ConstraintAttribute::Dual => matches!(v, AttributeValue::Dual(_)),
    }
}

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
    // attribute APIs
    fn set_optimizer_attr(
        &mut self,
        attr: OptimizerAttribute,
        val: AttributeValue,
    ) -> Result<(), MoiError> {
        self.set_optimizer_attr(attr, val)
    }
    fn get_optimizer_attr(&self, attr: &OptimizerAttribute) -> Option<&AttributeValue> {
        self.get_optimizer_attr(attr)
    }
    fn set_model_attr(
        &mut self,
        attr: ModelAttribute,
        val: AttributeValue,
    ) -> Result<(), MoiError> {
        self.set_model_attr(attr, val)
    }
    fn get_model_attr(&self, attr: &ModelAttribute) -> Option<AttributeValue> {
        self.get_model_attr(attr)
    }
    fn set_variable_attr(
        &mut self,
        var: VarId,
        attr: VariableAttribute,
        val: AttributeValue,
    ) -> Result<(), MoiError> {
        self.set_variable_attr(var, attr, val)
    }
    fn get_variable_attr(&self, var: VarId, attr: &VariableAttribute) -> Option<&AttributeValue> {
        self.get_variable_attr(var, attr)
    }
    fn set_constraint_attr(
        &mut self,
        cid: ConstrId,
        attr: ConstraintAttribute,
        val: AttributeValue,
    ) -> Result<(), MoiError> {
        self.set_constraint_attr(cid, attr, val)
    }
    fn get_constraint_attr(
        &self,
        cid: ConstrId,
        attr: &ConstraintAttribute,
    ) -> Option<&AttributeValue> {
        self.get_constraint_attr(cid, attr)
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
        self.optimizer_attr.clear();
        self.model_attr.clear();
        self.variable_attr.clear();
        self.constraint_attr.clear();
    }

    // 支持查询：若返回 true 则 get/set 语义受保证
    fn supports_optimizer_attr(&self, _attr: &OptimizerAttribute) -> bool {
        true
    }
    fn supports_model_attr(&self, attr: &ModelAttribute) -> bool {
        match attr {
            ModelAttribute::ObjectiveSense => true,
            ModelAttribute::ObjectiveFunction => true,
            ModelAttribute::NumberOfVariables => true,
            ModelAttribute::NumberOfConstraints => true,
            ModelAttribute::ListOfVariableIndices => true,
            ModelAttribute::Name => true,
            ModelAttribute::TerminationStatus => true,
            ModelAttribute::ResultCount => true,
            ModelAttribute::ObjectiveValue => true,
            ModelAttribute::SolverName => true,
            ModelAttribute::Silent => true,
        }
    }
    fn supports_variable_attr(&self, attr: &VariableAttribute) -> bool {
        matches!(attr, VariableAttribute::Primal)
    }
    fn supports_constraint_attr(&self, attr: &ConstraintAttribute) -> bool {
        matches!(attr, ConstraintAttribute::Slack | ConstraintAttribute::Dual)
    }
}
