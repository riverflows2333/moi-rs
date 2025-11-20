use moi_core::attributes::Attribute;
use moi_core::attributes::{AttributeValue, ModelAttribute, OptimizerAttribute, VariableAttribute, ConstraintAttribute};
use moi_core::errors::MoiError;
use moi_core::functions::{ScalarAffineFn, ScalarFunctionType};
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
use moi_solver_api::{ModelLike, Optimizer};
use std::any::Any;
use std::collections::HashMap;

use moi_core::constraint::ScalarConstraint;
use moi_core::variable::Variable;
// unified: no fallback wrapper

#[derive(Default)]
pub struct DummyModel {
    // core storage
    num_vars: usize,
    num_constr: usize,
    attrs: HashMap<&'static str, Box<dyn Any>>,
    objective: Option<ScalarAffineFn>,
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

impl ModelLike for DummyModel {
    fn add_variable(&mut self) -> VarId {
        let id = VarId(self.num_vars);
        self.num_vars += 1;
        self.variables.push(Variable { id, name: None });
        id
    }

    fn add_variables(&mut self, n: usize) -> Vec<VarId> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.add_variable());
        }
        v
    }

    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId {
        let id = self.num_constr;
        self.num_constr += 1;
        let c = ScalarConstraint::new(id, f, s);
        self.constraints.push(c);
        ConstrId::new(id)
    }

    fn supports_constraint(&self, f: &ScalarFunctionType, s: &ScalarSetType) -> bool {
        // MVP: 支持 AffineFn 与标量边界集合
        matches!(f, ScalarFunctionType::Affine(_))
            && matches!(
                s,
                ScalarSetType::GreaterThan(_)
                    | ScalarSetType::LessThan(_)
                    | ScalarSetType::EqualTo(_)
                    | ScalarSetType::Interval(..)
            )
    }

    fn get_attr<A: Attribute>(&self, _key: &A) -> Option<A::Value> {
        let key = core::any::type_name::<A>();
        self.attrs
            .get(key)
            .and_then(|b| b.downcast_ref::<A::Value>().cloned())
    }

    fn set_attr<A: Attribute>(&mut self, _key: &A, val: A::Value) -> Result<(), MoiError> {
        let key = core::any::type_name::<A>();
        self.attrs.insert(key, Box::new(val));
        Ok(())
    }

    fn is_empty(&self) -> bool {
        self.num_vars == 0 && self.num_constr == 0
    }

    fn empty(&mut self) {
        self.num_vars = 0;
        self.num_constr = 0;
        self.objective = None;
        self.variables.clear();
        self.var_to_name.clear();
        self.name_to_var.clear();
        self.con_to_name.clear();
        self.name_to_con.clear();
        self.constraints.clear();
    }

    fn set_objective_affine(&mut self, f: ScalarAffineFn) -> Result<(), MoiError> {
        self.set_objective_internal(f);
        Ok(())
    }

    fn get_objective_affine(&self) -> Option<&ScalarAffineFn> {
        self.objective.as_ref()
    }
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
    fn set_objective_internal(&mut self, mut f: ScalarAffineFn) {
        f.simplify();
        self.objective = Some(f);
    }

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
    pub fn set_optimizer_attr(&mut self, k: OptimizerAttribute, v: AttributeValue) -> Result<(), MoiError> {
        if !validate_optimizer_attr(&k, &v) { return Err(MoiError::Msg("attribute value type mismatch".into())); }
        self.optimizer_attr.insert(k, v); Ok(())
    }
    pub fn get_optimizer_attr(&self, k: &OptimizerAttribute) -> Option<&AttributeValue> {
        self.optimizer_attr.get(k)
    }
    pub fn set_model_attr(&mut self, k: ModelAttribute, v: AttributeValue) -> Result<(), MoiError> {
        if !validate_model_attr(&k, &v) { return Err(MoiError::Msg("attribute value type mismatch".into())); }
        self.model_attr.insert(k, v); Ok(())
    }
    pub fn get_model_attr(&self, k: &ModelAttribute) -> Option<AttributeValue> {
        self.model_attr.get(k).cloned()
    }
    pub fn set_variable_attr(&mut self, var: VarId, k: VariableAttribute, v: AttributeValue) -> Result<(), MoiError> {
        if !validate_variable_attr(&k, &v) { return Err(MoiError::Msg("attribute value type mismatch".into())); }
        self.variable_attr.entry(k.clone()).or_default().insert(var, v); Ok(())
    }
    pub fn get_variable_attr(&self, var: VarId, k: &VariableAttribute) -> Option<&AttributeValue> {
        self.variable_attr.get(k).and_then(|m| m.get(&var))
    }
    pub fn set_constraint_attr(&mut self, cid: ConstrId, k: ConstraintAttribute, v: AttributeValue) -> Result<(), MoiError> {
        if !validate_constraint_attr(&k, &v) { return Err(MoiError::Msg("attribute value type mismatch".into())); }
        self.constraint_attr.entry(k.clone()).or_default().insert(cid, v); Ok(())
    }
    pub fn get_constraint_attr(&self, cid: ConstrId, k: &ConstraintAttribute) -> Option<&AttributeValue> {
        self.constraint_attr.get(k).and_then(|m| m.get(&cid))
    }
}

// 校验函数
fn validate_optimizer_attr(k: &OptimizerAttribute, v: &AttributeValue) -> bool {
    match k { OptimizerAttribute::SolverName => matches!(v, AttributeValue::String(_)), OptimizerAttribute::Silent => matches!(v, AttributeValue::Bool(_)), }
}
fn validate_model_attr(k: &ModelAttribute, v: &AttributeValue) -> bool {
    match k {
        ModelAttribute::ObjectiveSense => matches!(v, AttributeValue::Sense(_)),
        ModelAttribute::ObjectiveFunction => matches!(v, AttributeValue::Function),
        ModelAttribute::NumberOfVariables => matches!(v, AttributeValue::USize(_)),
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
    match k { VariableAttribute::Primal => matches!(v, AttributeValue::Primal(_)) }
}
fn validate_constraint_attr(k: &ConstraintAttribute, v: &AttributeValue) -> bool {
    match k { ConstraintAttribute::Slack => matches!(v, AttributeValue::Slack(_)), ConstraintAttribute::Dual => matches!(v, AttributeValue::Dual(_)), }
}

// move implementations into the existing ModelLike impl above
