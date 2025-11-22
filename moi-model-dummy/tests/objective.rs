use moi_core::attributes::{ModelSense, ObjectiveSense, ObjectiveFunction};
use moi_core::functions::ScalarAffineFn;
use moi_model_dummy::DummyModel;
use moi_solver_api::{ModelLike, Optimizer};

#[test]
fn set_and_get_objective_via_attr() {
    let mut m = DummyModel::default();
    let v = m.add_variable();
    let mut obj = ScalarAffineFn::default();
    obj.push_term(v, 3.0);
    obj.push_term(v, 2.0); // will simplify to 5.0
    m.set::<ObjectiveFunction>(obj).unwrap();
    if let Some(got) = m.get::<ObjectiveFunction>() {
        assert_eq!(got.terms.len(), 1);
        assert_eq!(got.terms[0].coeff, 5.0);
    } else { panic!("objective attr missing or wrong type"); }
}

#[test]
fn set_objective_sense_via_attr() {
    let mut m = DummyModel::default();
    m.set::<ObjectiveSense>(ModelSense::Minimize).unwrap();
    let got = m.get::<ObjectiveSense>();
    assert!(matches!(got, Some(ModelSense::Minimize)));
}

#[test]
fn optimize_after_setting_objective() {
    let mut m = DummyModel::default();
    let _v = m.add_variable();
    m.set::<ObjectiveFunction>(ScalarAffineFn::with_constant(1.0)).unwrap();
    let status = m.optimize().unwrap();
    assert_eq!(status, moi_solver_api::SolveStatus::Optimal);
}
