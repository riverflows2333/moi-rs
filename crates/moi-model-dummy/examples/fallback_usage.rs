use moi_core::{ModelSense, ScalarAffineFn, ScalarFunctionType, SolveStatus};
use moi_model_dummy::DummyModel;
use moi_solver_api::{ModelLike, Optimizer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut model, handle) = DummyModel::new();
    let x = model.add_variable(Some("x"), None, None, None)?;
    let mut objective = ScalarAffineFn::default();
    objective.push_term(x, 2.0);
    model.set_objective(ScalarFunctionType::Affine(objective), ModelSense::Minimize)?;

    handle.set_solution(SolveStatus::Optimal, Some(6.0), [(x, 3.0)])?;
    println!("configured status: {:?}", model.optimize()?);
    println!("x = {:?}", model.get_var_value(x)?);
    println!("objective = {:?}", model.get_objective_value()?);
    Ok(())
}
