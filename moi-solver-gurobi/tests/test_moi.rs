use moi_core::*;
use moi_solver_api::*;
use moi_solver_gurobi::bindings::*;
use moi_solver_gurobi::dynamic::*;
use moi_solver_gurobi::wrapper::*;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::Arc;
#[test]
fn test_gurobi_solver_solve() {
    let gurobi_api =
        GurobiApi::new(find_library_from("/usr/local/gurobi1203".to_string()).unwrap()).unwrap();
    let mut solver = GurobiOptimizer::new(Arc::new(gurobi_api), None).unwrap();
    let var_id1 = solver.add_variable(Some("x"), Some('B'), None, None);
    let var_id2 = solver.add_variable(Some("y"), Some('B'), None, None);
    let var_id3 = solver.add_variable(Some("z"), Some('B'), None, None);
    let mut f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 2.0);
        afn.push_term(var_id3, 3.0);
        afn.simplify();
    }
    let mut s = ScalarSetType::LessThan(4.0);
    let constr_id = solver.add_constraint(f, s, Some("c0".to_string()));
    assert_eq!(constr_id.0, 0);
    f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 1.0);
        afn.simplify();
    }
    s = ScalarSetType::GreaterThan(1.0);
    let constr_id2 = solver.add_constraint(f, s, Some("c1".to_string()));
    assert_eq!(constr_id2.0, 1);
    f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 1.0);
        afn.push_term(var_id3, 2.0);
        afn.simplify();
    }
    solver
        .set_model_attr(
            ModelAttr::ObjectiveSense,
            AttrValue::ModelSense(ModelSense::Maximize),
        )
        .unwrap();
    solver
        .set_model_attr(ModelAttr::ObjectiveFunction, AttrValue::ScalarFn(f))
        .unwrap();
    solver.update().unwrap();
    let status = solver.optimize().unwrap();
    assert_eq!(status, SolveStatus::Optimal);
}
