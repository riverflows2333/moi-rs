use super::affine::AffineFn;

#[derive(Clone, Debug)]
pub enum FunctionType {
    Affine(AffineFn),
}

impl FunctionType {
    pub fn output_dim(&self) -> usize {
        match self {
            FunctionType::Affine(_f) => 1,
        }
    }
}
