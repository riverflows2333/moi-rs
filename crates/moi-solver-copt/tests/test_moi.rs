use moi_core::{
    AffineTerm, AttrValue, ModelAttr, ModelSense, OptimizerAttr, ScalarAffineFn,
    ScalarFunctionType, ScalarSetType, SolveStatus, VarId,
};
use moi_solver_api::{BoundType, ModelLike, NameType, Optimizer};
use moi_solver_copt::{CoptApi, CoptEnv, CoptOptimizer, find_library};
use std::sync::{Arc, Mutex};

fn configured_optimizer() -> Option<CoptOptimizer> {
    let Some(path) = find_library() else {
        eprintln!("skipping native COPT test: COPT native library was not found");
        return None;
    };
    let api =
        Arc::new(CoptApi::new(path).expect("configured COPT library should expose the MILP API"));
    let env = Arc::new(Mutex::new(
        CoptEnv::new(api).expect("COPT environment should be created"),
    ));
    let mut optimizer = CoptOptimizer::new(env).expect("COPT problem should be created");
    optimizer
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .expect("COPT logging should be disabled");
    Some(optimizer)
}

fn affine(terms: &[(usize, f64)], constant: f64) -> ScalarFunctionType {
    ScalarFunctionType::Affine(ScalarAffineFn {
        terms: terms
            .iter()
            .map(|(variable, coefficient)| AffineTerm {
                var: VarId(*variable),
                coeff: *coefficient,
            })
            .collect(),
        constant,
    })
}

#[test]
fn constructs_lp_and_milp_data() {
    let Some(mut optimizer) = configured_optimizer() else {
        return;
    };
    let variables = optimizer
        .add_variables(
            3,
            Some(NameType::Vector(vec!["x".into(), "y".into(), "z".into()])),
            Some(vec!['C', 'B', 'I']),
            Some(BoundType::Vector(vec![f64::NEG_INFINITY, 0.0, 0.0])),
            Some(BoundType::Vector(vec![f64::INFINITY, 1.0, 10.0])),
        )
        .unwrap();
    assert_eq!(variables, vec![VarId(0), VarId(1), VarId(2)]);

    let constraints = optimizer
        .add_constraints(
            vec![
                ScalarFunctionType::Variable(VarId(0)),
                affine(&[(1, 1.0)], 1.0),
                ScalarFunctionType::Variable(VarId(2)),
                affine(&[(0, 1.0), (2, 1.0)], 0.0),
            ],
            vec![
                ScalarSetType::GreaterThan(-2.0),
                ScalarSetType::LessThan(2.0),
                ScalarSetType::EqualTo(3.0),
                ScalarSetType::Interval(-1.0, 4.0),
            ],
            Some(vec![
                "lower".into(),
                "upper".into(),
                "fixed".into(),
                "range".into(),
            ]),
        )
        .unwrap();
    assert_eq!(
        constraints.iter().map(|id| id.0).collect::<Vec<_>>(),
        vec![0, 1, 2, 3]
    );

    optimizer
        .set_objective(
            affine(&[(0, 1.0), (0, 2.0), (2, -1.0)], 4.0),
            ModelSense::Maximize,
        )
        .unwrap();
    optimizer.update().unwrap();

    assert_eq!(optimizer.num_variables(), 3);
    assert_eq!(optimizer.num_constraints(), 4);
    assert_eq!(
        optimizer.get_model_attr(ModelAttr::NumberOfVariables),
        Some(AttrValue::Usize(3))
    );
    assert_eq!(
        optimizer.get_model_attr(ModelAttr::NumberOfConstraints),
        Some(AttrValue::Usize(4))
    );
    assert_eq!(
        optimizer.get_model_attr(ModelAttr::ObjectiveSense),
        Some(AttrValue::ModelSense(ModelSense::Maximize))
    );

    optimizer
        .set_objective(ScalarFunctionType::Variable(VarId(1)), ModelSense::Minimize)
        .unwrap();
    optimizer.update().unwrap();
    assert_eq!(
        optimizer.get_model_attr(ModelAttr::ObjectiveSense),
        Some(AttrValue::ModelSense(ModelSense::Minimize))
    );
}

#[test]
fn invalid_inputs_do_not_change_rust_model_counts() {
    let Some(mut optimizer) = configured_optimizer() else {
        return;
    };
    optimizer.add_variable(Some("x"), None, None, None).unwrap();

    assert!(
        optimizer
            .add_variables(
                2,
                Some(NameType::Vector(vec!["only_one".into()])),
                None,
                None,
                None,
            )
            .is_err()
    );
    assert!(optimizer.add_variable(None, Some('S'), None, None).is_err());
    assert_eq!(optimizer.num_variables(), 1);

    assert!(
        optimizer
            .add_constraint(
                ScalarFunctionType::Variable(VarId(5)),
                ScalarSetType::LessThan(1.0),
                None,
            )
            .is_err()
    );
    assert!(
        optimizer
            .add_constraint(
                ScalarFunctionType::Variable(VarId(0)),
                ScalarSetType::Interval(2.0, 1.0),
                None,
            )
            .is_err()
    );
    assert!(
        optimizer
            .add_constraint(
                ScalarFunctionType::Variable(VarId(0)),
                ScalarSetType::LessThan(1.0),
                Some("bad\0name".into()),
            )
            .is_err()
    );
    assert_eq!(optimizer.num_constraints(), 0);
}

#[test]
fn solves_lp_and_invalidates_cached_solution_after_mutation() {
    let Some(mut optimizer) = configured_optimizer() else {
        return;
    };
    optimizer.add_variables(2, None, None, None, None).unwrap();
    optimizer
        .add_constraint(
            affine(&[(0, 1.0), (1, 2.0)], 0.0),
            ScalarSetType::GreaterThan(4.0),
            None,
        )
        .unwrap();
    optimizer
        .set_objective(affine(&[(0, 1.0), (1, 1.0)], 3.0), ModelSense::Minimize)
        .unwrap();

    assert_eq!(optimizer.optimize().unwrap(), SolveStatus::Optimal);
    assert!((optimizer.get_var_value(VarId(0)).unwrap().unwrap() - 0.0).abs() < 1e-7);
    assert!((optimizer.get_var_value(VarId(1)).unwrap().unwrap() - 2.0).abs() < 1e-7);
    assert!((optimizer.get_objective_value().unwrap().unwrap() - 5.0).abs() < 1e-7);
    assert_eq!(
        optimizer.get_model_attr(ModelAttr::ResultCount),
        Some(AttrValue::Usize(1))
    );

    optimizer
        .add_variable(Some("new"), None, None, None)
        .unwrap();
    assert_eq!(optimizer.get_var_value(VarId(0)).unwrap(), None);
    assert_eq!(optimizer.get_objective_value().unwrap(), None);
    assert_eq!(optimizer.get_model_attr(ModelAttr::TerminationStatus), None);

    assert_eq!(optimizer.optimize().unwrap(), SolveStatus::Optimal);
    optimizer
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .unwrap();
    assert_eq!(optimizer.get_var_value(VarId(0)).unwrap(), None);
    assert_eq!(optimizer.get_objective_value().unwrap(), None);
    assert_eq!(optimizer.get_model_attr(ModelAttr::TerminationStatus), None);
}

#[test]
fn solves_binary_milp_with_parameters_and_objective_constant() {
    let Some(mut optimizer) = configured_optimizer() else {
        return;
    };
    optimizer
        .set_optimizer_attr(OptimizerAttr::TimeLimit, AttrValue::Float(10.0))
        .unwrap();
    optimizer
        .set_optimizer_attr(OptimizerAttr::Raw("Threads".into()), AttrValue::Int(1))
        .unwrap();
    optimizer
        .set_optimizer_attr(OptimizerAttr::Raw("Logging".into()), AttrValue::Bool(false))
        .unwrap();
    optimizer
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .unwrap();
    optimizer
        .add_variables(
            2,
            None,
            Some(vec!['B', 'B']),
            Some(BoundType::Single(0.0)),
            Some(BoundType::Single(1.0)),
        )
        .unwrap();
    optimizer
        .add_constraint(
            affine(&[(0, 2.0), (1, 1.0)], 0.0),
            ScalarSetType::LessThan(2.0),
            None,
        )
        .unwrap();
    optimizer
        .set_objective(affine(&[(0, 2.0), (1, 1.0)], 5.0), ModelSense::Maximize)
        .unwrap();

    assert_eq!(optimizer.optimize().unwrap(), SolveStatus::Optimal);
    assert!((optimizer.get_var_value(VarId(0)).unwrap().unwrap() - 1.0).abs() < 1e-7);
    assert!((optimizer.get_var_value(VarId(1)).unwrap().unwrap() - 0.0).abs() < 1e-7);
    assert!((optimizer.get_objective_value().unwrap().unwrap() - 7.0).abs() < 1e-7);
}

#[test]
fn infeasible_and_unbounded_models_have_no_cached_result() {
    let Some(mut infeasible) = configured_optimizer() else {
        return;
    };
    infeasible
        .add_variable(Some("x"), None, Some(0.0), Some(1.0))
        .unwrap();
    infeasible
        .add_constraint(
            ScalarFunctionType::Variable(VarId(0)),
            ScalarSetType::GreaterThan(2.0),
            None,
        )
        .unwrap();
    assert_eq!(infeasible.optimize().unwrap(), SolveStatus::Infeasible);
    assert_eq!(infeasible.get_var_value(VarId(0)).unwrap(), None);
    assert_eq!(infeasible.get_objective_value().unwrap(), None);

    let Some(mut unbounded) = configured_optimizer() else {
        return;
    };
    unbounded
        .add_variable(Some("x"), None, Some(0.0), None)
        .unwrap();
    unbounded
        .set_objective(ScalarFunctionType::Variable(VarId(0)), ModelSense::Maximize)
        .unwrap();
    assert_eq!(unbounded.optimize().unwrap(), SolveStatus::Unbounded);
    assert_eq!(unbounded.get_var_value(VarId(0)).unwrap(), None);
    assert_eq!(unbounded.get_objective_value().unwrap(), None);
}

#[test]
fn invalid_parameter_values_are_rejected_before_native_calls() {
    let Some(mut optimizer) = configured_optimizer() else {
        return;
    };
    assert!(
        optimizer
            .set_optimizer_attr(OptimizerAttr::TimeLimit, AttrValue::Int(1))
            .is_err()
    );
    assert!(
        optimizer
            .set_optimizer_attr(OptimizerAttr::TimeLimit, AttrValue::Float(-1.0))
            .is_err()
    );
    assert!(
        optimizer
            .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Float(1.0))
            .is_err()
    );
    assert!(
        optimizer
            .set_optimizer_attr(
                OptimizerAttr::Raw("Threads".into()),
                AttrValue::String("one".into()),
            )
            .is_err()
    );
    assert!(
        optimizer
            .set_optimizer_attr(
                OptimizerAttr::Raw("Threads".into()),
                AttrValue::Int(i64::MAX),
            )
            .is_err()
    );
    assert!(
        optimizer
            .set_optimizer_attr(OptimizerAttr::Raw("bad\0name".into()), AttrValue::Int(1),)
            .is_err()
    );
}
