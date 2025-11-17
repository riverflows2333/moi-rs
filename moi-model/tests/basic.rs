use moi_core::attributes::{ObjectiveSense, Sense};
use moi_core::functions::AffineFn;
use moi_core::sets::GreaterThan;
use moi_model::Model;
use moi_solver_api::{ModelLike, Optimizer, SolveStatus};

#[test]
fn test_supports_affine_scalar_bounds() {
    let model = Model::default();
    assert!(model.supports_constraint::<AffineFn, GreaterThan>());
}

#[test]
fn test_set_get_attribute() {
    let mut model = Model::default();
    let key = ObjectiveSense;
    model.set_attr(&key, Sense::Minimize).unwrap();
    let got = model.get_attr(&key);
    assert_eq!(got, Some(Sense::Minimize));
}

#[test]
fn test_optimize_dummy() {
    let mut model = Model::default();
    let status = model.optimize().unwrap();
    assert_eq!(status, SolveStatus::Optimal);
}
