use moi_core::attributes::Attribute;
use moi_core::errors::MoiError;
use moi_core::functions::AffineFn;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::{EqualTo, GreaterThan, Interval, LessThan};
use moi_core::traits::{Function, Set};
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

/// A small enum to express scalar bound sets for explicit bridging API.
#[derive(Clone)]
pub enum ScalarBoundSet {
    Ge(GreaterThan),
    Le(LessThan),
    Eq(EqualTo),
}

impl ScalarBoundSet {
    fn to_interval(&self) -> Interval {
        match self {
            ScalarBoundSet::Ge(ge) => Interval {
                lower: ge.lower,
                upper: f64::INFINITY,
            },
            ScalarBoundSet::Le(le) => Interval {
                lower: f64::NEG_INFINITY,
                upper: le.upper,
            },
            ScalarBoundSet::Eq(eq) => Interval {
                lower: eq.value,
                upper: eq.value,
            },
        }
    }
}

impl<O> BridgeOptimizer<O>
where
    O: ModelLike,
{
    /// Explicit API: add an affine scalar-bound constraint, bridging to Interval if needed.
    pub fn add_affine_bound(
        &mut self,
        f: AffineFn,
        b: ScalarBoundSet,
    ) -> ConstrId<AffineFn, Interval> {
        // If inner supports Interval, convert directly
        if self.inner.supports_constraint::<AffineFn, Interval>() {
            let set = b.to_interval();
            return self.inner.add_constraint::<AffineFn, Interval>(f, set);
        }
        // Fall back to native types if supported
        match &b {
            ScalarBoundSet::Ge(ge) => {
                if self.inner.supports_constraint::<AffineFn, GreaterThan>() {
                    let id = self
                        .inner
                        .add_constraint::<AffineFn, GreaterThan>(f, ge.clone())
                        .raw();
                    return ConstrId::new(id);
                }
            }
            ScalarBoundSet::Le(le) => {
                if self.inner.supports_constraint::<AffineFn, LessThan>() {
                    let id = self
                        .inner
                        .add_constraint::<AffineFn, LessThan>(f, le.clone())
                        .raw();
                    return ConstrId::new(id);
                }
            }
            ScalarBoundSet::Eq(eq) => {
                if self.inner.supports_constraint::<AffineFn, EqualTo>() {
                    let id = self
                        .inner
                        .add_constraint::<AffineFn, EqualTo>(f, eq.clone())
                        .raw();
                    return ConstrId::new(id);
                }
            }
        }
        // Last resort: map to Interval regardless (inner may still accept)
        let set = b.to_interval();
        self.inner.add_constraint::<AffineFn, Interval>(f, set)
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

    fn add_constraint<F, S>(&mut self, f: F, s: S) -> ConstrId<F, S>
    where
        F: Function,
        S: Set,
    {
        // MVP: by default delegate. Use explicit APIs for bridging where needed.
        self.inner.add_constraint(f, s)
    }

    fn supports_constraint<F, S>(&self) -> bool
    where
        F: Function,
        S: Set,
    {
        // If inner supports directly, true.
        if self.inner.supports_constraint::<F, S>() {
            return true;
        }
        // Otherwise check if a specialized bridge exists and its target is supported.
        // We conservatively report false; callers can attempt add and let bridge handle.
        false
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

    fn set_objective_affine(&mut self, f: AffineFn) -> Result<(), MoiError> {
        self.inner.set_objective_affine(f)
    }
    fn get_objective_affine(&self) -> Option<&AffineFn> {
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
