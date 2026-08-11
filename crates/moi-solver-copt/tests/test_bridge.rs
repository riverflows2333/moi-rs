use moi_bridge::BridgeOptimizer;
use moi_core::{
    AffineTerm, ModelAttr, ModelSense, ScalarAffineFn, ScalarFunctionType, ScalarSetType, VarId,
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

fn affine(terms: &[(usize, f64)]) -> ScalarFunctionType {
    ScalarFunctionType::Affine(ScalarAffineFn {
        terms: terms
            .iter()
            .map(|(variable, coefficient)| AffineTerm {
                var: VarId(*variable),
                coeff: *coefficient,
            })
            .collect(),
        constant: 0.0,
    })
}

#[test]
fn bridge_supports_initial_sync_and_incremental_modeling() {
    let Some(optimizer) = configured_optimizer() else {
        return;
    };
    let mut bridge = BridgeOptimizer::new();
    bridge
        .add_variables(
            2,
            Some(NameType::Vector(vec!["x".into(), "y".into()])),
            Some(vec!['B', 'B']),
            Some(BoundType::Single(0.0)),
            Some(BoundType::Single(1.0)),
        )
        .unwrap();
    bridge
        .add_constraint(
            affine(&[(0, 1.0), (1, 1.0)]),
            ScalarSetType::LessThan(1.0),
            Some("capacity".into()),
        )
        .unwrap();
    bridge
        .set_objective(affine(&[(0, 1.0), (1, 2.0)]), ModelSense::Maximize)
        .unwrap();

    bridge.attach_backend(Box::new(optimizer)).unwrap();

    let z = bridge
        .add_variable(Some("z"), Some('I'), Some(0.0), Some(4.0))
        .unwrap();
    assert_eq!(z, VarId(2));
    bridge
        .add_constraint(
            affine(&[(1, 1.0), (2, 1.0)]),
            ScalarSetType::Interval(1.0, 3.0),
            Some("incremental".into()),
        )
        .unwrap();
    bridge.update().unwrap();

    assert_eq!(
        bridge.get_model_attr(ModelAttr::NumberOfVariables),
        Some(3_usize.into())
    );
    assert_eq!(
        bridge.get_model_attr(ModelAttr::NumberOfConstraints),
        Some(2_usize.into())
    );
}
