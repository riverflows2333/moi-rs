use moi_solver_api::ModelLike;

fn main() {
    let mut model = moi_model::Model::default();
    let vars = model.add_variables(2);

    let mut f = moi_core::functions::AffineFn::default();
    f.push_term(vars[0], 1.0);
    f.push_term(vars[1], 2.0);

    let set = moi_core::sets::GreaterThan::new(1.0);
    let _cid = model.add_constraint(f, set);

    println!("example constructed OK");
}
