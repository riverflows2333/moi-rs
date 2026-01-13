use moi_core::attributes::{ConstraintAttr, ModelAttr, OptimizerAttr, VariableAttr,AttrValue,ModelSense};
use moi_core::functions::ScalarAffineFn;
use moi_model_dummy::DummyModel;
use moi_solver_api::ModelLike;

fn main() {
    let mut uf = DummyModel::default();

    // 设置模型名称与求解器属性
    uf.set_model_attr(ModelAttr::ModelName, "demo_model".to_string().into())
        .unwrap();
    uf.set_optimizer_attr(OptimizerAttr::Silent, true.into())
        .unwrap();

    // 添加变量
    let v = uf.add_variable();
    let w = uf.add_variable();
    // 变量数量与索引列表为派生只读属性，使用 get 读取
    if let Some(AttrValue::Int(nv)) = uf.get_model_attr(ModelAttr::NumberOfVariables) {
        println!("num vars: {:?}", nv);
    }
    if let Some(idx) = uf.get_model_attr(ModelAttr::ListOfVariableIndices) {
        println!("var indices: {:?}", idx);
    }

    // 构造并设置目标（仿射）
    let mut obj = ScalarAffineFn::default();
    obj.push_term(v, 1.0);
    obj.push_term(w, 2.0);
    uf.set_model_attr(ModelAttr::ObjectiveFunction, obj.into())
        .unwrap();
    uf.set_model_attr(ModelAttr::ObjectiveSense, ModelSense::Minimize.into())
        .unwrap();

    // 模拟解后填充结果属性
    uf.set_model_attr(ModelAttr::TerminationStatus, "Optimal".to_string().into())
        .unwrap();
    uf.set_model_attr(ModelAttr::ObjectiveValue, 0.0.into())
        .unwrap();

    // 读取并打印部分属性
    if let Some(AttrValue::String(name)) = uf.get_model_attr(ModelAttr::ModelName) {
        println!("Model name: {}", name);
    }
    if let Some(AttrValue::String(solver)) = uf.get_optimizer_attr(OptimizerAttr::SolverName) {
        println!("Solver: {}", solver);
    }
    if let Some(sense) = uf.get_model_attr(ModelAttr::ObjectiveSense) {
        println!("Sense: {:?}", sense);
    }
}
