use crate::{ScalarFunctionType, functions::ScalarAffineFn};

// 属性值枚举
#[derive(Clone, Debug, PartialEq)]
pub enum AttrValue {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    ModelSense(ModelSense),
    ScalarFn(ScalarFunctionType),
    Status(SolveStatus),
    VecUsize(Vec<usize>),
    Usize(usize),
}

impl From<String> for AttrValue {
    fn from(v: String) -> Self {
        Self::String(v)
    }
}
impl From<i64> for AttrValue {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}
impl From<f64> for AttrValue {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}
impl From<bool> for AttrValue {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}
impl From<ModelSense> for AttrValue {
    fn from(v: ModelSense) -> Self {
        Self::ModelSense(v)
    }
}
impl From<ScalarFunctionType> for AttrValue {
    fn from(v: ScalarFunctionType) -> Self {
        Self::ScalarFn(v)
    }
}
impl From<Vec<usize>> for AttrValue {
    fn from(v: Vec<usize>) -> Self {
        Self::VecUsize(v)
    }
}
impl From<usize> for AttrValue {
    fn from(v: usize) -> Self {
        Self::Usize(v)
    }
}

// 求解相关枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ModelSense {
    Minimize,
    Maximize,
}

// 模型属性枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ModelAttr {
    ObjectiveSense,
    ObjectiveFunction,
    ModelName,
    NumberOfVariables,
    NumberOfConstraints,
    ListOfVariableIndices,
    TerminationStatus,
    ResultCount,
    ObjectiveValue,
}

// 优化器属性枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum OptimizerAttr {
    SolverName,
    Silent,
    TimeLimit,
}

// 变量属性枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum VariableAttr {
    VariableName,
    Primal,
}

// 约束属性枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum ConstraintAttr {
    ConstraintName,
    ConstraintPrimal,
    ConstraintDual
}

// 求解状态
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SolveStatus {
    Unknown,
    Optimal,
    Infeasible,
    Unbounded,
    Feasible,
}