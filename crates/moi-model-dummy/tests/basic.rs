use moi_core::{AttrValue, ModelAttr, MoiError, OptimizerAttr, SolveStatus, VarId};
use moi_model_dummy::DummyModel;
use moi_solver_api::{ModelLike, Optimizer};

#[test]
fn derived_attributes_and_default_status_are_honest() {
    let mut model = DummyModel::default();
    model
        .set_model_attr(ModelAttr::ModelName, AttrValue::String("recording".into()))
        .unwrap();

    assert_eq!(
        model.get_model_attr(ModelAttr::ModelName),
        Some(AttrValue::String("recording".into()))
    );
    assert_eq!(
        model.get_optimizer_attr(OptimizerAttr::SolverName),
        Some(AttrValue::String("DummyRecordingOptimizer".into()))
    );
    assert_eq!(model.optimize().unwrap(), SolveStatus::Unknown);
    assert!(matches!(
        model.get_var_value(VarId(0)),
        Err(MoiError::InvalidVariableIndex(0))
    ));
    assert_eq!(model.get_objective_value().unwrap(), None);
    assert_eq!(
        model.get_model_attr(ModelAttr::TerminationStatus),
        Some(AttrValue::Status(SolveStatus::Unknown))
    );
    assert_eq!(
        model.get_model_attr(ModelAttr::ResultCount),
        Some(AttrValue::Usize(0))
    );
}

#[test]
fn configured_solution_is_visible_only_after_optimize() {
    let (mut model, handle) = DummyModel::new();
    let variable = model
        .add_variable(Some("x"), Some('C'), Some(0.0), Some(10.0))
        .unwrap();
    handle
        .set_solution(SolveStatus::Optimal, Some(6.0), [(variable, 3.0)])
        .unwrap();

    assert_eq!(model.get_var_value(variable).unwrap(), None);
    assert_eq!(model.optimize().unwrap(), SolveStatus::Optimal);
    assert_eq!(model.get_var_value(variable).unwrap(), Some(3.0));
    assert_eq!(model.get_objective_value().unwrap(), Some(6.0));
    assert_eq!(
        model.get_model_attr(ModelAttr::ResultCount),
        Some(AttrValue::Usize(1))
    );
}

#[test]
fn derived_attributes_are_read_only() {
    let mut model = DummyModel::default();
    let error = model
        .set_model_attr(ModelAttr::NumberOfVariables, AttrValue::Usize(10))
        .unwrap_err();
    assert!(matches!(error, MoiError::SetAttributeNotAllowed));
}
