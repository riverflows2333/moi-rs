use moi_core::{
    AffineTerm, AttrValue, ModelAttr, ModelSense, ScalarAffineFn, ScalarFunctionType,
    ScalarSetType, VarId,
};
use moi_solver_api::{BoundType, ModelLike, NameType};
use moi_solver_copt::{CoptApi, CoptEnv, CoptOptimizer, find_library};
use std::sync::{Arc, Mutex};

fn configured_optimizer() -> Option<CoptOptimizer> {
    let path = find_library()?;
    let api =
        Arc::new(CoptApi::new(path).expect("configured COPT library should expose the MILP API"));
    let env = Arc::new(Mutex::new(
        CoptEnv::new(api).expect("COPT environment should be created"),
    ));
    Some(CoptOptimizer::new(env).expect("COPT problem should be created"))
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
