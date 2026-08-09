use moi_bridge::BridgeOptimizer;
use moi_core::{
    AttrValue, ModelSense, OptimizerAttr, ScalarAffineFn, ScalarFunctionType, ScalarSetType,
    SolveStatus,
};
use moi_model_dummy::{DummyModel, RecordedCall, RecordedOperation};
use moi_solver_api::{ModelLike, Optimizer};

#[test]
fn bridge_sync_and_incremental_calls_can_be_inspected_after_move() {
    let (backend, handle) = DummyModel::new();
    let mut bridge = BridgeOptimizer::new();
    let x = bridge
        .add_variable(Some("x"), Some('C'), Some(0.0), Some(10.0))
        .unwrap();
    let mut row = ScalarAffineFn::default();
    row.push_term(x, 1.0);
    bridge
        .add_constraint(
            ScalarFunctionType::Affine(row.clone()),
            ScalarSetType::GreaterThan(2.0),
            Some("minimum".into()),
        )
        .unwrap();
    bridge
        .set_objective(ScalarFunctionType::Affine(row), ModelSense::Minimize)
        .unwrap();
    bridge
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .unwrap();
    handle
        .set_solution(SolveStatus::Optimal, Some(2.0), [(x, 2.0)])
        .unwrap();

    bridge.attach_backend(Box::new(backend)).unwrap();
    let y = bridge
        .add_variable(Some("y"), Some('C'), Some(0.0), Some(5.0))
        .unwrap();
    assert_eq!(y.0, 1);
    bridge
        .set_optimizer_attr(OptimizerAttr::Raw("Threads".into()), AttrValue::Int(1))
        .unwrap();
    assert_eq!(bridge.optimize().unwrap(), SolveStatus::Optimal);
    assert_eq!(bridge.get_var_value(x), Some(2.0));
    assert_eq!(bridge.get_objective_value(), Some(2.0));

    let state = handle.snapshot().unwrap();
    assert!(matches!(state.calls[0], RecordedCall::AddVariables { .. }));
    assert!(matches!(
        state.calls[1],
        RecordedCall::AddConstraints { .. }
    ));
    assert!(matches!(state.calls[2], RecordedCall::SetObjective { .. }));
    assert!(matches!(
        state.calls[3],
        RecordedCall::SetOptimizerAttr { .. }
    ));
    assert!(matches!(state.calls[4], RecordedCall::AddVariables { .. }));
    assert!(matches!(
        state.calls[5],
        RecordedCall::SetOptimizerAttr { .. }
    ));
    assert!(matches!(state.calls[6], RecordedCall::Optimize));
}

#[test]
fn injected_backend_failure_does_not_change_bridge_state() {
    let (backend, handle) = DummyModel::new();
    let mut bridge = BridgeOptimizer::new();
    bridge.attach_backend(Box::new(backend)).unwrap();
    handle
        .fail_next(RecordedOperation::AddVariables, "injected add failure")
        .unwrap();

    let error = bridge
        .add_variable(Some("x"), None, None, None)
        .unwrap_err();
    assert!(error.to_string().contains("injected add failure"));
    assert!(bridge.vars.is_empty());
    assert!(handle.snapshot().unwrap().variables.is_empty());
}
