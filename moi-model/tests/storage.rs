use moi_model::Model;
use moi_core::functions::AffineFn;
use moi_solver_api::ModelLike;

#[test]
fn variables_and_names_are_stored() {
    let mut m = Model::default();
    let v0 = m.add_variable();
    let v1 = m.add_variable();
    m.set_var_name(v0, "x").unwrap();
    m.set_var_name(v1, "y").unwrap();
    assert_eq!(m.get_var_name(v0), Some("x"));
    assert_eq!(m.get_var_by_name("y"), Some(v1));
}

#[test]
fn affine_constraints_are_stored() {
    let mut m = Model::default();
    let v = m.add_variable();
    let mut f = AffineFn::default();
    f.push_term(v, 2.0);
    let cid = m.add_affine_bound(f, moi_model::AffineSetKind::Ge(1.0));
    let c = m.get_affine_constraint(cid.raw()).unwrap();
    match c.set {
        moi_model::AffineSetKind::Ge(l) => assert_eq!(l, 1.0),
        _ => panic!("expected Ge set"),
    }
}
