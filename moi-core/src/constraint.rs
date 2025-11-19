use crate::traits::{Function, Set};

#[derive(Debug)]
pub struct Constraint<F, S> {
    pub id: usize,
    pub func: F,
    pub set: S,
}

impl<F, S> Constraint<F, S>
where
    F: Function,
    S: Set,
{
    pub fn new(id: usize, func: F, set: S) -> Self {
        Self { id, func, set }
    }
}

pub struct ConstraintDyn {
    pub id: usize,
    pub func: Box<dyn Function>,
    pub set: Box<dyn Set>,
}

impl ConstraintDyn {
    pub fn new<F, S>(id: usize, func: F, set: S) -> Self
    where
        F: Function + 'static,
        S: Set + 'static,
    {
        Self { id, func: Box::new(func), set: Box::new(set) }
    }
}
