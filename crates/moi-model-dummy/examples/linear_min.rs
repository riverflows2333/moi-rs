use moi_core::{ModelSense, ScalarAffineFn, ScalarFunctionType, ScalarSetType};
use moi_model_dummy::DummyModel;
use moi_solver_api::{ModelLike, Optimizer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut model, handle) = DummyModel::new();
    let x = model.add_variable(Some("x"), None, Some(0.0), None)?;
    let y = model.add_variable(Some("y"), None, Some(0.0), None)?;

    let mut demand = ScalarAffineFn::default();
    demand.push_term(x, 1.0);
    demand.push_term(y, 1.0);
    model.add_constraint(
        ScalarFunctionType::Affine(demand),
        ScalarSetType::GreaterThan(1.0),
        Some("demand".into()),
    )?;

    let mut objective = ScalarAffineFn::default();
    objective.push_term(x, 1.0);
    objective.push_term(y, 2.0);
    model.set_objective(ScalarFunctionType::Affine(objective), ModelSense::Minimize)?;

    println!("status without a real solver: {:?}", model.optimize()?);
    println!("recorded calls: {:#?}", handle.snapshot()?.calls);
    Ok(())
}
