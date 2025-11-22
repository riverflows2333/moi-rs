use moi_core::attributes::{
    downcast_ref_value, into_box_value, Dual, ListOfVariableIndices, NumberOfConstraints,
    NumberOfVariables, ObjectiveFunction, Primal, Silent, Slack, SolverName,
};
use moi_core::errors::MoiError;
use moi_core::functions::ScalarAffineFn;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
use moi_solver_api::{ModelLike, Optimizer};
use std::any::Any;
use std::any::TypeId;
use std::collections::HashMap;

use moi_core::constraint::ScalarConstraint;
use moi_core::variable::Variable;
// unified: no fallback wrapper

#[derive(Default)]
pub struct DummyModel {
    // core storage
    // removed counters & generic attr box storage
    name: String, // reserved for future naming attributes
    variables: Vec<Variable>,
    constraints: Vec<ScalarConstraint>,
    var_to_name: HashMap<usize, String>,
    name_to_var: HashMap<String, VarId>,
    con_to_name: HashMap<usize, String>,
    name_to_con: HashMap<String, usize>,
    // unified attribute store (non-derived)
    attr_store: HashMap<TypeId, Box<dyn Any>>,
}

impl DummyModel {
    fn derived_get<A: moi_core::Attribute + 'static>(&self) -> Option<A::Value> {
        let tid = TypeId::of::<A>();
        if tid == TypeId::of::<NumberOfVariables>() {
            let v = self.variables.len();
            let ptr = &v as *const usize as *const A::Value;
            unsafe {
                return Some((*ptr).clone());
            }
        }
        if tid == TypeId::of::<NumberOfConstraints>() {
            let v = self.constraints.len();
            let ptr = &v as *const usize as *const A::Value;
            unsafe {
                return Some((*ptr).clone());
            }
        }
        if tid == TypeId::of::<ListOfVariableIndices>() {
            let list: Vec<usize> = (0..self.variables.len()).collect();
            let ptr = &list as *const Vec<usize> as *const A::Value;
            unsafe {
                return Some((*ptr).clone());
            }
        }
        if tid == TypeId::of::<SolverName>() {
            return self
                .attr_store
                .get(&tid)
                .and_then(downcast_ref_value::<A::Value>)
                .cloned();
        }
        if tid == TypeId::of::<Silent>() {
            return self
                .attr_store
                .get(&tid)
                .and_then(downcast_ref_value::<A::Value>)
                .cloned();
        }
        None
    }

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
    fn supports<A: moi_core::Attribute + 'static>(&self) -> bool {
        let tid = TypeId::of::<A>();
        // mark unsupported per-variable/constraint attributes (old Primal/Slack/Dual) since API lacks indexing
        if tid == TypeId::of::<Primal>()
            || tid == TypeId::of::<Slack>()
            || tid == TypeId::of::<Dual>()
        {
            return false;
        }
        true
    }
    fn get<A: moi_core::Attribute + 'static>(&self) -> Option<A::Value> {
        // derived first
        if let Some(v) = self.derived_get::<A>() {
            return Some(v);
        }
        self.attr_store
            .get(&TypeId::of::<A>())
            .and_then(downcast_ref_value::<A::Value>)
            .cloned()
    }
    fn set<A: moi_core::Attribute + 'static>(
        &mut self,
        mut value: A::Value,
    ) -> Result<(), MoiError> {
        let tid = TypeId::of::<A>();
        if !self.supports::<A>() {
            return Err(MoiError::UnsupportedAttribute);
        }
        // derived read-only guard
        if tid == TypeId::of::<NumberOfVariables>()
            || tid == TypeId::of::<NumberOfConstraints>()
            || tid == TypeId::of::<ListOfVariableIndices>()
        {
            return Err(MoiError::SetAttributeNotAllowed);
        }
        if tid == TypeId::of::<ObjectiveFunction>() {
            let f = &mut value as *mut A::Value as *mut ScalarAffineFn;
            unsafe {
                (*f).simplify();
            }
        }
        self.attr_store.insert(tid, into_box_value(value));
        Ok(())
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
        self.attr_store.clear();
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
