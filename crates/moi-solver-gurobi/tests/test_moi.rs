use moi_core::*;
use moi_solver_api::*;
use moi_solver_gurobi::dynamic::*;
use moi_solver_gurobi::wrapper::*;
use std::sync::{Arc, Mutex};
#[test]
fn test_gurobi_solver_solve() {
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
    let mut solver = GurobiOptimizer::new(env, None).unwrap();
    let var_id1 = solver
        .add_variable(Some("x"), Some('B'), None, None)
        .unwrap();
    let var_id2 = solver
        .add_variable(Some("y"), Some('B'), None, None)
        .unwrap();
    let var_id3 = solver
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
    let constr_id = solver.add_constraint(f, s, Some("c0".to_string())).unwrap();
    assert_eq!(constr_id.0, 0);
    f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 1.0);
        afn.simplify();
    }
    s = ScalarSetType::GreaterThan(1.0);
    let constr_id2 = solver.add_constraint(f, s, Some("c1".to_string())).unwrap();
    assert_eq!(constr_id2.0, 1);
    f = ScalarFunctionType::Affine(ScalarAffineFn::new());
    if let ScalarFunctionType::Affine(ref mut afn) = f {
        afn.push_term(var_id1, 1.0);
        afn.push_term(var_id2, 1.0);
        afn.push_term(var_id3, 2.0);
        afn.simplify();
    }
    solver.set_objective(f, ModelSense::Maximize).unwrap();
    // solver.set_optimizer_attr(OptimizerAttr::TimeLimit, AttrValue::Float(100.0)).unwrap();
    // solver.set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(false)).unwrap();
    solver
        .set_optimizer_attr(
            OptimizerAttr::Raw("OutputFlag".to_string()),
            AttrValue::Int(0),
        )
        .unwrap();
    solver.update().unwrap();
    let status = solver.optimize().unwrap();
    assert_eq!(status, SolveStatus::Optimal);
    assert_eq!(solver.get_var_value(var_id1).unwrap(), Some(1.0));
    assert_eq!(solver.get_var_value(var_id2).unwrap(), Some(0.0));
    assert_eq!(solver.get_var_value(var_id3).unwrap(), Some(1.0));
    assert_eq!(
        solver.get_model_attr(ModelAttr::TerminationStatus),
        Some(AttrValue::Status(SolveStatus::Optimal))
    );

    solver
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .unwrap();
    assert_eq!(solver.get_var_value(var_id1).unwrap(), None);
    assert_eq!(solver.get_objective_value().unwrap(), None);
    assert_eq!(solver.get_model_attr(ModelAttr::TerminationStatus), None);
}

#[test]
fn linear_rows_and_objective_use_native_batch_path() {
    let Some((library, _)) = find_library() else {
        return;
    };
    let Ok(api) = GurobiApi::new(library) else {
        return;
    };
    let Ok(env) = GurobiEnv::new(Arc::new(api)) else {
        return;
    };
    let mut solver = GurobiOptimizer::new(Arc::new(Mutex::new(env)), Some("linear-batch"))
        .expect("model creation should succeed after environment startup");
    let variables = solver
        .add_variables(
            2,
            None,
            Some(vec!['C', 'C']),
            Some(BoundType::Single(0.0)),
            Some(BoundType::Single(f64::INFINITY)),
        )
        .unwrap();

    let mut rows = LinearRows::new(2);
    let mut row = ScalarAffineFn::default();
    row.push_term(variables[0], 1.0);
    row.push_term(variables[1], 1.0);
    rows.push(
        &ScalarFunctionType::Affine(row),
        &ScalarSetType::GreaterThan(3.0),
    )
    .unwrap();
    assert_eq!(solver.add_linear_rows(rows).unwrap(), 0..1);

    let objective = LinearObjective {
        num_cols: 2,
        col_indices: vec![0, 1],
        coefficients: vec![1.0, 2.0],
        constant: 4.0,
    };
    solver
        .set_linear_objective(objective, ModelSense::Minimize)
        .unwrap();
    solver
        .set_linear_objective(
            LinearObjective {
                num_cols: 2,
                col_indices: vec![0],
                coefficients: vec![1.0],
                constant: 1.0,
            },
            ModelSense::Minimize,
        )
        .unwrap();
    solver
        .set_optimizer_attr(OptimizerAttr::Silent, AttrValue::Bool(true))
        .unwrap();
    assert_eq!(solver.optimize().unwrap(), SolveStatus::Optimal);
    assert!((solver.get_objective_value().unwrap().unwrap() - 1.0).abs() < 1e-8);
    assert_eq!(solver.num_variables(), 2);
    assert_eq!(solver.num_constraints(), 1);

    let mut interval = LinearRows::new(2);
    interval
        .push(
            &ScalarFunctionType::Variable(variables[0]),
            &ScalarSetType::Interval(0.0, 1.0),
        )
        .unwrap();
    assert!(matches!(
        solver.add_linear_rows(interval),
        Err(MoiError::UnsupportedConstraint { .. })
    ));
    assert_eq!(solver.num_constraints(), 1);
}
