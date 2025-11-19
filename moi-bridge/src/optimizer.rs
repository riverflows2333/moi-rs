use moi_core::attributes::Attribute;
use moi_core::errors::MoiError;
use moi_core::functions::{ScalarAffineFn, ScalarFunctionType};
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;
use moi_solver_api::{ModelLike, Optimizer, SolveStatus};

/// BridgeOptimizer wraps an inner optimizer and attempts to bridge unsupported
/// constraints to supported ones.
pub struct BridgeOptimizer<O> {
    inner: O,
}

impl<O> BridgeOptimizer<O> {
    pub fn new(inner: O) -> Self {
        Self { inner }
    }
    pub fn into_inner(self) -> O {
        self.inner
    }
    pub fn inner(&self) -> &O {
        &self.inner
    }
    pub fn inner_mut(&mut self) -> &mut O {
        &mut self.inner
    }
}

/// 表达单侧或等式标量约束，用于桥接到区间形式。
#[derive(Clone, Debug)]
pub enum ScalarBoundSet {
    Ge(f64),
    Le(f64),
    Eq(f64),
}

impl ScalarBoundSet {
    fn to_interval(&self) -> (f64, f64) {
        match self {
            ScalarBoundSet::Ge(l) => (*l, f64::INFINITY),
            ScalarBoundSet::Le(u) => (f64::NEG_INFINITY, *u),
            ScalarBoundSet::Eq(v) => (*v, *v),
        }
    }
}

impl<O> BridgeOptimizer<O>
where
    O: ModelLike,
{
    /// Explicit API: add an affine scalar-bound constraint, bridging to Interval if needed.
    pub fn add_affine_bound(&mut self, f: ScalarAffineFn, b: ScalarBoundSet) -> ConstrId {
        let fty = ScalarFunctionType::Affine(f);
        let (l, u) = b.to_interval();
        // 优先尝试模型自身支持的原始形式
        match b {
            ScalarBoundSet::Ge(val) => {
                if self.inner.supports_constraint(&fty, &ScalarSetType::GreaterThan(val)) {
                    return self.inner.add_constraint(fty, ScalarSetType::GreaterThan(val));
                }
            }
            ScalarBoundSet::Le(val) => {
                if self.inner.supports_constraint(&fty, &ScalarSetType::LessThan(val)) {
                    return self.inner.add_constraint(fty, ScalarSetType::LessThan(val));
                }
            }
            ScalarBoundSet::Eq(val) => {
                if self.inner.supports_constraint(&fty, &ScalarSetType::EqualTo(val)) {
                    return self.inner.add_constraint(fty, ScalarSetType::EqualTo(val));
                }
            }
        }
        // 回退到区间形式
        self.inner.add_constraint(fty, ScalarSetType::Interval(l, u))
    }
}

impl<O> ModelLike for BridgeOptimizer<O>
where
    O: ModelLike,
{
    fn add_variable(&mut self) -> VarId {
        self.inner.add_variable()
    }
    fn add_variables(&mut self, n: usize) -> Vec<VarId> {
        self.inner.add_variables(n)
    }

    fn add_constraint(&mut self, f: ScalarFunctionType, s: ScalarSetType) -> ConstrId {
        // MVP: by default delegate. Use explicit APIs for bridging where needed.
        self.inner.add_constraint(f, s)
    }

    fn supports_constraint(&self, f: &ScalarFunctionType, s: &ScalarSetType) -> bool {
        // Delegate to inner; bridging capability is exposed via explicit APIs.
        self.inner.supports_constraint(f, s)
    }

    fn get_attr<A: Attribute>(&self, key: &A) -> Option<A::Value> {
        self.inner.get_attr(key)
    }
    fn set_attr<A: Attribute>(&mut self, key: &A, val: A::Value) -> Result<(), MoiError> {
        self.inner.set_attr(key, val)
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    fn empty(&mut self) {
        self.inner.empty()
    }

    fn set_objective_affine(&mut self, f: ScalarAffineFn) -> Result<(), MoiError> {
        self.inner.set_objective_affine(f)
    }
    fn get_objective_affine(&self) -> Option<&ScalarAffineFn> {
        self.inner.get_objective_affine()
    }
}

impl<O> Optimizer for BridgeOptimizer<O>
where
    O: Optimizer,
{
    fn optimize(&mut self) -> Result<SolveStatus, MoiError> {
        self.inner.optimize()
    }
    fn compute_conflict(&mut self) -> Result<(), MoiError> {
        self.inner.compute_conflict()
    }
}
