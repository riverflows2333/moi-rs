use moi_bridge::BridgeOptimizer;
use moi_core::*;
use moi_solver_api::*;

#[test]
fn generic_linear_rows_fallback_preserves_constants_and_indices() {
    let mut model = BridgeOptimizer::new();
    model.add_variable(None, None, None, None).unwrap();
    let mut rows = LinearRows::new(1);
    rows.push(
        &ScalarFunctionType::Variable(VarId(0)),
        &ScalarSetType::LessThan(2.0),
    )
    .unwrap();
    assert_eq!(model.add_linear_rows(rows.clone()).unwrap(), 0..1);
    assert_eq!(model.add_linear_rows(rows).unwrap(), 1..2);
    assert_eq!(model.add_linear_rows(LinearRows::new(1)).unwrap(), 2..2);
    let objective = LinearObjective {
        num_cols: 1,
        col_indices: vec![0],
        coefficients: vec![2.0],
        constant: 3.0,
    };
    model
        .set_linear_objective(objective, ModelSense::Minimize)
        .unwrap();
    assert_eq!(model.constrs.len(), 2);
    match model.obj.unwrap() {
        ScalarFunctionType::Affine(f) => {
            assert_eq!(f.constant, 3.0);
            assert_eq!(f.terms[0].coeff, 2.0);
        }
        _ => panic!("expected affine"),
    }
}
