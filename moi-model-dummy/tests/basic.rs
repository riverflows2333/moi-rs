use moi_core::attributes::{ModelAttr, ModelSense};
use moi_core::functions::{ScalarAffineFn, ScalarFunctionType};
use moi_core::sets::ScalarSetType;
use moi_model_dummy::DummyModel;
use moi_solver_api::{ModelLike, Optimizer, SolveStatus};

#[test]
fn test_supports_affine_scalar_bounds() {
    let model = DummyModel::default();
    let f = ScalarFunctionType::Affine(ScalarAffineFn::default());
    let s = ScalarSetType::GreaterThan(0.0);
    assert!(model.supports_constraint(&f, &s));
}

#[test]
fn test_set_get_attribute() {
    let mut model = DummyModel::default();
    model
        .set_model_attr(ModelAttr::ObjectiveSense, ModelSense::Minimize.into())
        .unwrap();
    let got = model.get_model_attr(ModelAttr::ObjectiveSense).unwrap();
    assert!(matches!(
        got,
        moi_core::attributes::AttrValue::ModelSense(ModelSense::Minimize)
    ));
}

#[test]
fn test_optimize_dummy() {
    let mut model = DummyModel::default();
    let status = model.optimize().unwrap();
    assert_eq!(status, SolveStatus::Optimal);
}
