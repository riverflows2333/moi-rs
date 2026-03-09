use moi_solver_api::ModelLike;
use moi_core::attributes::ModelAttr;

fn main() {
    let mut model = moi_model_dummy::DummyModel::default();
    let vars = model.add_variables(2,None);

    let mut f = moi_core::functions::ScalarAffineFn::default();
    f.push_term(vars[0], 1.0);
    f.push_term(vars[1], 2.0);

    let _cid = model.add_constraint(
        moi_core::functions::ScalarFunctionType::Affine(f),
        moi_core::sets::ScalarSetType::GreaterThan(1.0),
    );

    // set objective via attribute: minimize x + 2y
    let mut obj = moi_core::functions::ScalarAffineFn::default();
    obj.push_term(vars[0], 3.0);
    obj.push_term(vars[1], 5.0);
    // model.set_model_attr(ModelAttr::ObjectiveFunction, obj.into()).unwrap();

    println!("example constructed OK");
    // let ret = model.get_model_attr(ModelAttr::ObjectiveFunction);
    // println!("Objective function: {:?}", ret);
}
