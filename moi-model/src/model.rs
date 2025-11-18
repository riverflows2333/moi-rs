use moi_core::attributes::Attribute;
use moi_core::errors::MoiError;
use moi_core::indices::{ConstrId, VarId};
use moi_core::traits::{Function, Set};
use moi_solver_api::{ModelLike, Optimizer};
use std::any::Any;
use std::collections::HashMap;

use moi_core::functions::AffineFn;
use moi_core::sets::{EqualTo, GreaterThan, Interval, LessThan};

#[derive(Default)]
pub struct Model {
    num_vars: usize,
    num_constr: usize,
    attrs: HashMap<&'static str, Box<dyn Any>>,
    objective: Option<AffineFn>,
}

impl ModelLike for Model {
    fn add_variable(&mut self) -> VarId {
        let id = VarId(self.num_vars);
        self.num_vars += 1;
        id
    }

    fn add_variables(&mut self, n: usize) -> Vec<VarId> {
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            v.push(self.add_variable());
        }
        v
    }

    fn add_constraint<F, S>(&mut self, _f: F, _s: S) -> ConstrId<F, S>
    where
        F: Function,
        S: Set,
    {
        let id = self.num_constr;
        self.num_constr += 1;
        ConstrId::new(id)
    }

    fn supports_constraint<F, S>(&self) -> bool
    where
        F: Function,
        S: Set,
    {
        // MVP: 支持 AffineFn 与标量边界集合（通过类型名字符串判定，避免'static约束）
        let f_ok = core::any::type_name::<F>() == core::any::type_name::<AffineFn>();
        let s_name = core::any::type_name::<S>();
        let s_ok = s_name == core::any::type_name::<GreaterThan>()
            || s_name == core::any::type_name::<LessThan>()
            || s_name == core::any::type_name::<EqualTo>()
            || s_name == core::any::type_name::<Interval>();
        f_ok && s_ok
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
}

// move implementations into the existing ModelLike impl above
