use moi_model_dummy::DummyModel;
use moi_core::functions::ScalarAffineFn;
use moi_core::attributes::{ModelName, SolverName, Silent, ObjectiveFunction, ObjectiveSense, ModelSense, TerminationStatus, ObjectiveValue};
use moi_solver_api::ModelLike;

fn main() {
    let mut uf = DummyModel::default();

    // 设置模型名称与求解器属性
    uf.set::<ModelName>("demo_model".into()).unwrap();
    uf.set::<Silent>(true).unwrap();
    uf.set::<SolverName>("DummySolver".into()).unwrap();

    // 添加变量
    let v = uf.add_variable();
    let w = uf.add_variable();
    // 变量数量与索引列表为派生只读属性，使用 get 读取
    if let Some(nv) = uf.get::<moi_core::attributes::NumberOfVariables>() { println!("num vars: {}", nv); }
    if let Some(idx) = uf.get::<moi_core::attributes::ListOfVariableIndices>() { println!("var indices: {:?}", idx); }

    // 构造并设置目标（仿射）
    let mut obj = ScalarAffineFn::default();
    obj.push_term(v, 1.0);
    obj.push_term(w, 2.0);
    uf.set::<ObjectiveFunction>(obj).unwrap();
    uf.set::<ObjectiveSense>(ModelSense::Minimize).unwrap();

    // 模拟解后填充结果属性
    uf.set::<TerminationStatus>("Optimal".into()).unwrap();
    uf.set::<ObjectiveValue>(0.0).unwrap();

    // 读取并打印部分属性
    if let Some(name) = uf.get::<ModelName>() { println!("Model name: {}", name); }
    if let Some(solver) = uf.get::<SolverName>() { println!("Solver: {}", solver); }
    if let Some(sense) = uf.get::<ObjectiveSense>() { println!("Sense: {:?}", sense); }
}
