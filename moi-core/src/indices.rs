use core::marker::PhantomData;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VarId(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConstrId<F, S>(pub usize, pub PhantomData<(F, S)>);

impl<F, S> ConstrId<F, S> {
    pub fn new(id: usize) -> Self {
        Self(id, PhantomData)
    }
    pub fn raw(&self) -> usize {
        self.0
    }
}
