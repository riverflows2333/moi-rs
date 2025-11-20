use moi_solver_api::ModelLike;
use moi_core::attributes::{ModelAttribute, AttributeValue};

fn main() {
    let mut model = moi_model_dummy::DummyModel::default();
    let vars = model.add_variables(2);

    let mut f = moi_core::functions::ScalarAffineFn::default();
    f.push_term(vars[0], 1.0);
    f.push_term(vars[1], 2.0);

    let _cid = model.add_constraint(
        moi_core::functions::ScalarFunctionType::Affine(f),
        moi_core::sets::ScalarSetType::GreaterThan(1.0),
    );

    // set objective via attribute: minimize x + 2y
    let mut obj = moi_core::functions::ScalarAffineFn::default();
    obj.push_term(vars[0], 1.0);
    obj.push_term(vars[1], 2.0);
    model.set_model_attr(ModelAttribute::ObjectiveFunction, AttributeValue::Affine(obj)).unwrap();

    println!("example constructed OK");
}
