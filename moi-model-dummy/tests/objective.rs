use moi_core::attributes::{ObjectiveSense, Sense};
use moi_core::functions::ScalarAffineFn;
use moi_model_dummy::Model;
use moi_solver_api::{ModelLike, Optimizer};

#[test]
fn set_and_get_objective_affine() {
    let mut m = Model::default();
    let v = m.add_variable();
    let mut obj = ScalarAffineFn::default();
    obj.push_term(v, 3.0);
    obj.push_term(v, 2.0); // will simplify to 5.0
    m.set_objective_affine(obj).unwrap();

    let got = m.get_objective_affine().unwrap();
    assert_eq!(got.terms.len(), 1);
    assert_eq!(got.terms[0].coeff, 5.0);
}

#[test]
fn set_objective_sense_via_attr() {
    let mut m = Model::default();
    m.set_attr(&ObjectiveSense, Sense::Minimize).unwrap();
    assert_eq!(m.get_attr(&ObjectiveSense), Some(Sense::Minimize));
}

#[test]
fn optimize_after_setting_objective() {
    let mut m = Model::default();
    let _v = m.add_variable();
    m.set_objective_affine(ScalarAffineFn::with_constant(1.0)).unwrap();
    let status = m.optimize().unwrap();
    assert_eq!(status, moi_solver_api::SolveStatus::Optimal);
}
