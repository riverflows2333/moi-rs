use crate::traits::Set;

#[derive(Clone, Debug)]
pub struct GreaterThan {
    pub lower: f64,
}
impl GreaterThan {
    pub fn new(lower: f64) -> Self {
        Self { lower }
    }
}
impl Set for GreaterThan {
    fn dimension(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug)]
pub struct LessThan {
    pub upper: f64,
}
impl LessThan {
    pub fn new(upper: f64) -> Self {
        Self { upper }
    }
}
impl Set for LessThan {
    fn dimension(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug)]
pub struct EqualTo {
    pub value: f64,
}
impl EqualTo {
    pub fn new(value: f64) -> Self {
        Self { value }
    }
}
impl Set for EqualTo {
    fn dimension(&self) -> usize {
        1
    }
}

#[derive(Clone, Debug)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}
impl Interval {
    pub fn new(lower: f64, upper: f64) -> Self {
        Self { lower, upper }
    }
}
impl Set for Interval {
    fn dimension(&self) -> usize {
        1
    }
}
