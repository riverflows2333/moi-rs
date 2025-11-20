use moi_core::functions::{ScalarAffineFn, ScalarFunctionType};
use moi_core::sets::ScalarSetType;
use moi_model_dummy::Model;
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
fn add_constraint_returns_increasing_ids() {
    let mut m = Model::default();
    let v = m.add_variable();
    let mut f = ScalarAffineFn::default();
    f.push_term(v, 2.0);
    let c1 = m.add_constraint(
        ScalarFunctionType::Affine(f.clone()),
        ScalarSetType::GreaterThan(1.0),
    );
    let c2 = m.add_constraint(
        ScalarFunctionType::Affine(f),
        ScalarSetType::GreaterThan(2.0),
    );
    assert_eq!(c1.raw(), 0);
    assert_eq!(c2.raw(), 1);
}
