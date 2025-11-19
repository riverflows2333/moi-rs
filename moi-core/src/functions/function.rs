use super::affine::ScalarAffineFn;

#[derive(Clone, Debug)]
pub enum ScalarFunctionType {
    Affine(ScalarAffineFn),
}

impl ScalarFunctionType {
    pub fn output_dim(&self) -> usize {
        match self {
            ScalarFunctionType::Affine(_f) => 1,
        }
    }
}
