use moi_core::{ScalarAffineFn, ScalarFunctionType, ScalarSetType};
use moi_model_dummy::{DummyModel, RecordedCall};
use moi_solver_api::{BoundType, ModelLike, NameType};

#[test]
fn variables_constraints_and_names_are_recorded() {
    let (mut model, handle) = DummyModel::new();
    let variables = model
        .add_variables(
            2,
            Some(NameType::Vector(vec!["x".into(), "y".into()])),
            Some(vec!['C', 'B']),
            Some(BoundType::Vector(vec![-1.0, 0.0])),
            Some(BoundType::Vector(vec![4.0, 1.0])),
        )
        .unwrap();
    assert_eq!(model.get_var_name(variables[0]).as_deref(), Some("x"));
    assert_eq!(model.get_var_by_name("y"), Some(variables[1]));

    let mut function = ScalarAffineFn::default();
    function.push_term(variables[0], 2.0);
    let constraint = model
        .add_constraint(
            ScalarFunctionType::Affine(function),
            ScalarSetType::GreaterThan(1.0),
            Some("demand".into()),
        )
        .unwrap();
    assert_eq!(constraint.0, 0);

    let state = handle.snapshot().unwrap();
    assert_eq!(state.variables.len(), 2);
    assert_eq!(state.variables[0].lb, -1.0);
    assert_eq!(state.variables[1].vtype, 'B');
    assert_eq!(state.constraints.len(), 1);
    assert_eq!(state.constraints[0].name, "demand");
    assert!(matches!(state.calls[0], RecordedCall::AddVariables { .. }));
    assert!(matches!(
        state.calls[1],
        RecordedCall::AddConstraints { .. }
    ));
}

#[test]
fn invalid_batch_does_not_partially_modify_state() {
    let (mut model, handle) = DummyModel::new();
    assert!(
        model
            .add_variables(
                2,
                Some(NameType::Vector(vec!["only-one".into()])),
                None,
                None,
                None,
            )
            .is_err()
    );
    assert!(handle.snapshot().unwrap().variables.is_empty());

    assert!(
        model
            .add_variable(Some("bad-bounds"), None, Some(2.0), Some(1.0))
            .is_err()
    );
    assert!(handle.snapshot().unwrap().variables.is_empty());
}
