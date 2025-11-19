use crate::indices::VarId;

#[derive(Clone, Debug)]
pub struct Variable {
    pub id: VarId,
    pub name: Option<String>,
}
