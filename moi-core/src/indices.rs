use crate::functions::*;
use crate::sets::*;
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct VarId(pub usize);

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct ConstrId(pub usize);

impl ConstrId {
    pub fn new(id: usize) -> Self { Self(id) }
    pub fn raw(&self) -> usize { self.0 }
}

#[derive(Clone, Debug)]
pub struct VarInfo {
    pub col_index: usize, // Gurobi 内部的列索引 (0, 1, 2...)
    // 缓存上下界，避免频繁查询 C API
    pub lb: f64,
    pub ub: f64,
    pub vtype: char, // 'C', 'B', 'I'
    pub name: String,
    pub value: Option<f64>
}
#[derive(Clone, Debug)]
pub struct ConstrInfo {
    pub row_index: usize, // Gurobi 内部的行索引
    pub name: String,     // 可以在这里存约束类型，方便后续查询
    pub f: ScalarFunctionType,
    pub s: ScalarSetType,
}