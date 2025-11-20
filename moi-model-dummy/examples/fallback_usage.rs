use moi_model_dummy::DummyModel;
use moi_core::functions::ScalarAffineFn;
use moi_core::attributes::{ModelAttribute, OptimizerAttribute, VariableAttribute, AttributeValue, Sense};
use moi_solver_api::ModelLike;

fn main() {
    let mut uf = DummyModel::default();

    // 设置模型名称与求解器属性
    uf.set_model_attr(ModelAttribute::Name, AttributeValue::String("demo_model".into())).unwrap();
    uf.set_optimizer_attr(OptimizerAttribute::Silent, AttributeValue::Bool(true)).unwrap();
    uf.set_optimizer_attr(OptimizerAttribute::SolverName, AttributeValue::String("DummySolver".into())).unwrap();

    // 添加变量
    let v = uf.add_variable();
    let w = uf.add_variable();
    // 变量数量与索引列表为派生只读属性，使用 get 读取
    if let Some(nv) = uf.get_model_attr(&ModelAttribute::NumberOfVariables) { println!("num vars: {:?}", nv.as_usize()); }
    if let Some(idx) = uf.get_model_attr(&ModelAttribute::ListOfVariableIndices) { println!("var indices: {:?}", idx); }

    // 构造并设置目标（仿射）
    let mut obj = ScalarAffineFn::default();
    obj.push_term(v, 1.0);
    obj.push_term(w, 2.0);
    uf.set_model_attr(ModelAttribute::ObjectiveFunction, AttributeValue::Affine(obj)).unwrap();
    uf.set_model_attr(ModelAttribute::ObjectiveSense, AttributeValue::Sense(Sense::Minimize)).unwrap();

    // 模拟解后填充结果属性
    uf.set_model_attr(ModelAttribute::TerminationStatus, AttributeValue::TerminationStatus("Optimal".into())).unwrap();
    uf.set_model_attr(ModelAttribute::ObjectiveValue, AttributeValue::F64(0.0)).unwrap();
    uf.set_variable_attr(v, VariableAttribute::Primal, AttributeValue::Primal(0.0)).unwrap();
    uf.set_variable_attr(w, VariableAttribute::Primal, AttributeValue::Primal(0.0)).unwrap();

    // 读取并打印部分属性
    if let Some(val) = uf.get_model_attr(&ModelAttribute::Name) { println!("Model name: {:?}", val.as_str()); }
    if let Some(val) = uf.get_optimizer_attr(&OptimizerAttribute::SolverName) { println!("Solver: {:?}", val.as_str()); }
    if let Some(val) = uf.get_model_attr(&ModelAttribute::ObjectiveSense) { println!("Sense: {:?}", val); }
    if let Some(val) = uf.get_variable_attr(v, &VariableAttribute::Primal) { println!("x primal: {:?}", val.as_f64()); }
}
