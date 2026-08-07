use crate::ScalarFunctionType;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
// 属性值枚举
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Encode, Decode)]
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
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub enum ModelSense {
    Minimize,
    Maximize,
}

// 模型属性枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Encode, Decode)]
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
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum OptimizerAttr {
    SolverName,
    Silent,
    TimeLimit,
    Raw(String),
}

// 变量属性枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum VariableAttr {
    VariableName,
    Primal,
}

// 约束属性枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum ConstraintAttr {
    ConstraintName,
    ConstraintPrimal,
    ConstraintDual,
}

// 求解状态
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize, Encode, Decode)]
pub enum SolveStatus {
    Unknown = 0,
    Optimal = 1,
    Infeasible = 2,
    Unbounded = 3,
    Feasible = 4,
}

impl SolveStatus {
    pub const fn code(self) -> u32 {
        self as u32
    }

    pub const fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Optimal,
            2 => Self::Infeasible,
            3 => Self::Unbounded,
            4 => Self::Feasible,
            _ => Self::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SolveStatus;

    #[test]
    fn solve_status_protocol_codes_are_stable() {
        for status in [
            SolveStatus::Unknown,
            SolveStatus::Optimal,
            SolveStatus::Infeasible,
            SolveStatus::Unbounded,
            SolveStatus::Feasible,
        ] {
            assert_eq!(SolveStatus::from_code(status.code()), status);
        }
    }
}
