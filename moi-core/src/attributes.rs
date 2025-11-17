pub trait Attribute {
    type Value: Clone + 'static;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Sense {
    Minimize,
    Maximize,
}

pub struct ObjectiveSense;
impl Attribute for ObjectiveSense {
    type Value = Sense;
}

pub struct NumberOfVariables;
impl Attribute for NumberOfVariables {
    type Value = usize;
}

pub struct NumberOfConstraints;
impl Attribute for NumberOfConstraints {
    type Value = usize;
}
