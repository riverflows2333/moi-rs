use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;
use moi_solver_gurobi::dynamic::*;
use moi_solver_gurobi::wrapper::*;
use std::sync::{Arc, Mutex};

#[test]
fn test_bridge_optimizer() {
    let Some((library, _)) = find_library() else {
        return;
    };
    let Ok(gurobi_api) = GurobiApi::new(library) else {
        return;
    };
    let api = Arc::new(gurobi_api);
    let Ok(env) = GurobiEnv::new(api) else {
        return;
    };
    let env = Arc::new(Mutex::new(env));
    let solver = GurobiOptimizer::new(env, None).unwrap();
    let mut bridge = BridgeOptimizer::new();
    let var_id1 = bridge
        .add_variable(Some("x"), Some('B'), None, None)
        .unwrap();
    let var_id2 = bridge
        .add_variable(Some("y"), Some('B'), None, None)
        .unwrap();
    let var_id3 = bridge
        .add_variable(Some("z"), Some('B'), None, None)
        .unwrap();
    let mut f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 2.0);
        afn.push_term(var_id3, 3.0);
        afn.simplify();
    }
    let mut s = ScalarSetType::LessThan(4.0);
    let constr_id = bridge.add_constraint(f, s, Some("c0".to_string())).unwrap();
    assert_eq!(constr_id.0, 0);
    f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 1.0);
        afn.simplify();
    }
    s = ScalarSetType::GreaterThan(1.0);
    let constr_id2 = bridge.add_constraint(f, s, Some("c1".to_string())).unwrap();
    assert_eq!(constr_id2.0, 1);
    f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 1.0);
        afn.push_term(var_id3, 2.0);
        afn.simplify();
    }
    bridge.set_objective(f, ModelSense::Maximize).unwrap();
    bridge
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .unwrap();
    bridge.attach_backend(Box::new(solver)).unwrap();
    let status = bridge.optimize().unwrap();
    assert_eq!(status, SolveStatus::Optimal);
    assert_eq!(bridge.get_var_value(var_id1).unwrap(), Some(1.0));
    assert_eq!(bridge.get_var_value(var_id2).unwrap(), Some(0.0));
    assert_eq!(bridge.get_var_value(var_id3).unwrap(), Some(1.0));
}
