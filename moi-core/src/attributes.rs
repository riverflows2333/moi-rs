pub trait Attribute {
    type Value: Clone + 'static;
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum Sense {
    Minimize,
    Maximize,
}

pub struct ObjectiveSense;
impl Attribute for ObjectiveSense {
    type Value = Sense;
}

pub struct NumberOfVariables;
impl Attribute for NumberOfVariables {
    type Value = usize;
}

pub struct NumberOfConstraints;
impl Attribute for NumberOfConstraints {
    type Value = usize;
}


// 新的枚举分类属性体系（初始子集，后续可扩展）

/// 求解器级别可配置 / 信息属性
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum OptimizerAttribute {
    /// 求解器名称（只读）
    SolverName,
    /// 静默模式（可写）
    Silent,
}

/// 模型级别属性：构建期 + 求解后只读 + 诊断
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ModelAttribute {
    // 构建期
    ObjectiveSense,          // Sense
    ObjectiveFunction,       // ScalarFunctionType (占位: 目前可用 Affine)
    NumberOfVariables,       // usize
    NumberOfConstraints,     // usize （暂不区分 (F,S) 类型细分）
    ListOfVariableIndices,   // Vec<VarId> 或 Vec<usize>（占位）
    Name,                    // String (模型名称)
    // 求解后（只读）
    TerminationStatus,       // SolveStatus
    ResultCount,             // usize （可能用于多解场景占位）
    ObjectiveValue,          // f64 (标量目标值)
    // 诊断/易用
    SolverName,              // 冗余映射到 OptimizerAttribute::SolverName
    Silent,                  // 冗余映射到 OptimizerAttribute::Silent
}

/// 变量级别属性（按变量索引查询）；后续可加入 LowerBound/UpperBound 等
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum VariableAttribute {
    /// 变量的原始值 (primal) after solve
    Primal, // f64
}

/// 约束级别属性（后续可加入 Dual, Slack 等）；当前占位
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ConstraintAttribute {
    /// 约束的原始松弛量 (slack) after solve (占位)
    Slack, // f64
    /// 约束的对偶值 (dual) after solve (占位)
    Dual,  // f64
}



// 说明：目前仍保留旧的单 struct Attribute 定义方式以兼容现存代码。
// 新枚举体系后续可以配套一个统一的 AttributeValue 枚举或映射表来存储异构值。

/// 统一的属性值枚举，用于 UniversalFallback 中的异构存储。
#[derive(Clone, Debug)]
pub enum AttributeValue {
    Sense(Sense),
    Function,              // 占位：当前仅支持 Affine，可后续携带具体函数对象引用或 ID
    USize(usize),
    F64(f64),
    Bool(bool),
    String(String),
    VarIndices(Vec<usize>),
    TerminationStatus(String), // 简化: 使用字符串表示 SolveStatus
    // 变量/约束属性
    Primal(f64),
    Slack(f64),
    Dual(f64),
}

impl AttributeValue {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            AttributeValue::F64(v)
            | AttributeValue::Primal(v)
            | AttributeValue::Slack(v)
            | AttributeValue::Dual(v) => Some(*v),
            _ => None,
        }
    }
    pub fn as_usize(&self) -> Option<usize> {
        match self { AttributeValue::USize(v) => Some(*v), _ => None }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self { AttributeValue::String(s) | AttributeValue::TerminationStatus(s) => Some(s), _ => None }
    }
}