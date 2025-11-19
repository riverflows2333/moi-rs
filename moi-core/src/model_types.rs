use crate::functions::AffineFn;
use crate::indices::VarId;

#[derive(Clone, Debug)]
pub struct Variable {
    pub id: VarId,
    pub name: Option<String>,
}

#[derive(Clone, Debug)]
pub enum AffineSetKind {
    Ge(f64),
    Le(f64),
    Eq(f64),
    Interval { lower: f64, upper: f64 },
}

#[derive(Clone, Debug)]
pub struct AffineConstraint {
    pub id: usize,
    pub func: AffineFn,
    pub set: AffineSetKind,
}
