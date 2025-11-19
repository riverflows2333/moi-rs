use moi_core::attributes::Attribute;
use moi_core::errors::MoiError;
use moi_core::functions::{AffineFn, FunctionType};
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::{ScalarSetType};
use moi_solver_api::{ModelLike, Optimizer};
use std::any::Any;
use std::collections::HashMap;

use moi_core::variable::Variable;

#[derive(Default)]
pub struct Model {
    num_vars: usize,
    num_constr: usize,
    attrs: HashMap<&'static str, Box<dyn Any>>,
    objective: Option<AffineFn>,
    // storage
    name: String,
    variables: Vec<Variable>,
    var_to_name: HashMap<usize, String>,
    name_to_var: HashMap<String, VarId>,
    con_to_name: HashMap<usize, String>,
    name_to_con: HashMap<String, usize>,
    constraints: Vec<moi_core::constraint::ConstraintType>,
}

impl ModelLike for Model {
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

    fn add_constraint(&mut self, f: FunctionType, s: ScalarSetType) -> ConstrId {
        let id = self.num_constr;
        self.num_constr += 1;
        let c = moi_core::constraint::ConstraintType::new(id, f, s);
        self.constraints.push(c);
        ConstrId::new(id)
    }

    fn supports_constraint(&self, f: &FunctionType, s: &ScalarSetType) -> bool {
        // MVP: 支持 AffineFn 与标量边界集合
        matches!(f, FunctionType::Affine(_))
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

    fn set_objective_affine(&mut self, f: AffineFn) -> Result<(), MoiError> {
        self.set_objective_internal(f);
        Ok(())
    }

    fn get_objective_affine(&self) -> Option<&AffineFn> {
        self.objective.as_ref()
    }
}

impl Optimizer for Model {
    fn optimize(&mut self) -> Result<moi_solver_api::SolveStatus, MoiError> {
        // MVP: 伪求解器，直接返回Optimal
        Ok(moi_solver_api::SolveStatus::Optimal)
    }

    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        // 未实现，MVP返回Ok
        Ok(())
    }
}

impl Model {
    fn set_objective_internal(&mut self, mut f: AffineFn) {
        f.simplify();
        self.objective = Some(f);
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

// move implementations into the existing ModelLike impl above
