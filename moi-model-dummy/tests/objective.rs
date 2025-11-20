use moi_core::attributes::{Sense, ModelAttribute, AttributeValue};
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
    m.set_model_attr(ModelAttribute::ObjectiveFunction, AttributeValue::Affine(obj)).unwrap();
    if let Some(AttributeValue::Affine(got)) = m.get_model_attr(&ModelAttribute::ObjectiveFunction) {
        assert_eq!(got.terms.len(), 1);
        assert_eq!(got.terms[0].coeff, 5.0);
    } else { panic!("objective attr missing or wrong type"); }
}

#[test]
fn set_objective_sense_via_attr() {
    let mut m = DummyModel::default();
    m.set_model_attr(ModelAttribute::ObjectiveSense, AttributeValue::Sense(Sense::Minimize)).unwrap();
    let got = m.get_model_attr(&ModelAttribute::ObjectiveSense);
    assert!(matches!(got, Some(AttributeValue::Sense(Sense::Minimize))));
}

#[test]
fn optimize_after_setting_objective() {
    let mut m = DummyModel::default();
    let _v = m.add_variable();
    m.set_model_attr(ModelAttribute::ObjectiveFunction, AttributeValue::Affine(ScalarAffineFn::with_constant(1.0))).unwrap();
    let status = m.optimize().unwrap();
    assert_eq!(status, moi_solver_api::SolveStatus::Optimal);
}
