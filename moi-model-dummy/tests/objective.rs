use moi_core::attributes::{AttrValue, ModelAttr};
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
    m.set_model_attr(ModelAttr::ObjectiveFunction, obj.into())
        .unwrap();
    if let Some(got) = m.get_model_attr(ModelAttr::ObjectiveFunction) {
        let got = match got {
            AttrValue::ScalarAffineFn(f) => f,
            _ => panic!("wrong attribute type"),
        };
        assert_eq!(got.terms.len(), 1);
        assert_eq!(got.terms[0].coeff, 5.0);
    } else {
        panic!("objective attr missing or wrong type");
    }
}

