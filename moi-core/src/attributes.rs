use crate::functions::ScalarAffineFn;
use std::any::Any;

// 核心属性 Trait：所有属性类型都是零尺寸标记类型（ZST），通过其关联类型 Value 指定值类型。
pub trait Attribute {
    type Value: Clone + 'static;
}

// 标记 Trait：不同层级
pub trait ModelAttribute: Attribute {}
pub trait OptimizerAttribute: Attribute {}
pub trait VariableAttribute: Attribute {}
pub trait ConstraintAttribute: Attribute {}

// 通用枚举替换为具体结构体属性定义 —— 更灵活可扩展

// 求解相关枚举
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ModelSense {
    Minimize,
    Maximize,
}

// 模型级别属性
pub struct ObjectiveSense;
impl Attribute for ObjectiveSense {
    type Value = ModelSense;
}
impl ModelAttribute for ObjectiveSense {}
pub struct ObjectiveFunction;
impl Attribute for ObjectiveFunction {
    type Value = ScalarAffineFn;
}
impl ModelAttribute for ObjectiveFunction {}
pub struct ModelName;
impl Attribute for ModelName {
    type Value = String;
}
impl ModelAttribute for ModelName {}
pub struct NumberOfVariables;
impl Attribute for NumberOfVariables {
    type Value = usize;
}
impl ModelAttribute for NumberOfVariables {}
pub struct NumberOfConstraints;
impl Attribute for NumberOfConstraints {
    type Value = usize;
}
impl ModelAttribute for NumberOfConstraints {}
pub struct ListOfVariableIndices;
impl Attribute for ListOfVariableIndices {
    type Value = Vec<usize>;
}
impl ModelAttribute for ListOfVariableIndices {}
pub struct TerminationStatus;
impl Attribute for TerminationStatus {
    type Value = String;
}
impl ModelAttribute for TerminationStatus {}
pub struct ResultCount;
impl Attribute for ResultCount {
    type Value = usize;
}
impl ModelAttribute for ResultCount {}
pub struct ObjectiveValue;
impl Attribute for ObjectiveValue {
    type Value = f64;
}
impl ModelAttribute for ObjectiveValue {}

// Optimizer 属性（可同时作为模型属性冗余映射）
pub struct SolverName;
impl Attribute for SolverName {
    type Value = String;
}
impl OptimizerAttribute for SolverName {}
impl ModelAttribute for SolverName {}
pub struct Silent;
impl Attribute for Silent {
    type Value = bool;
}
impl OptimizerAttribute for Silent {}
impl ModelAttribute for Silent {}
pub struct TimeLimit;
impl Attribute for TimeLimit {
    type Value = f64;
}
impl OptimizerAttribute for TimeLimit {}
impl ModelAttribute for TimeLimit {}

// 变量级别属性
pub struct Primal;
impl Attribute for Primal {
    type Value = f64;
}
impl VariableAttribute for Primal {}

// 约束级别属性
pub struct Slack;
impl Attribute for Slack {
    type Value = f64;
}
impl ConstraintAttribute for Slack {}
pub struct Dual;
impl Attribute for Dual {
    type Value = f64;
}
impl ConstraintAttribute for Dual {}

// 工具函数：从 Any 中按类型安全提取引用（内部使用）
pub fn downcast_ref_value<V: 'static>(b: &Box<dyn Any>) -> Option<&V> {
    b.downcast_ref::<V>()
}
pub fn into_box_value<V: 'static>(v: V) -> Box<dyn Any> {
    Box::new(v)
}
