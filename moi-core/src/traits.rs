pub trait Function {
    fn output_dim(&self) -> usize;
}

pub trait Set {
    fn dimension(&self) -> usize;
}
