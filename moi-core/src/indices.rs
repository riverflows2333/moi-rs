#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VarId(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConstrId(pub usize);

impl ConstrId {
    pub fn new(id: usize) -> Self { Self(id) }
    pub fn raw(&self) -> usize { self.0 }
}
