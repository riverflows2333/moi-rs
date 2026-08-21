use moi_bridge::BridgeOptimizer;
use moi_core::{
    AffineTerm, AttrValue, ModelAttr, ModelSense, MoiError, ScalarAffineFn, ScalarFunctionType,
    ScalarSetType, VarId,
};
use moi_solver_api::{BoundType, ModelLike, NameType, Optimizer};

#[test]
fn invalid_batch_lengths_do_not_modify_the_model() {
    let mut bridge = BridgeOptimizer::new();
    let result = bridge.add_variables(
        2,
        Some(NameType::Vector(vec!["x".to_string()])),
        None,
        Some(BoundType::Vector(vec![0.0, 0.0])),
        None,
    );

    assert!(result.is_err());
    assert!(bridge.vars.is_empty());
}

#[test]
fn invalid_variable_reference_is_rejected() {
    let mut bridge = BridgeOptimizer::new();
    let function = ScalarFunctionType::Affine(ScalarAffineFn {
        terms: vec![AffineTerm {
            var: VarId(10),
            coeff: 1.0,
        }],
        constant: 0.0,
    });

    let result = bridge.add_constraint(function, ScalarSetType::LessThan(1.0), None);

    assert!(result.is_err());
    assert!(bridge.constrs.is_empty());
}

#[test]
fn setting_sense_before_objective_creates_a_zero_objective() {
    let mut bridge = BridgeOptimizer::new();

    bridge
        .set_model_attr(
            ModelAttr::ObjectiveSense,
            AttrValue::ModelSense(ModelSense::Maximize),
        )
        .unwrap();

    assert_eq!(
        bridge.get_model_attr(ModelAttr::ObjectiveSense),
        Some(AttrValue::ModelSense(ModelSense::Maximize))
    );
    assert!(matches!(
        bridge.get_model_attr(ModelAttr::ObjectiveFunction),
        Some(AttrValue::ScalarFn(ScalarFunctionType::Affine(_)))
    ));
}

#[test]
fn single_addition_uses_exact_name() {
    let mut bridge = BridgeOptimizer::new();

    let id = bridge
        .add_variable(Some("x"), Some('C'), Some(0.0), Some(1.0))
        .unwrap();

    assert_eq!(bridge.get_var_name_by_id(id).as_deref(), Some("x"));
}

#[test]
fn invalid_variable_metadata_and_nonfinite_functions_are_rejected() {
    let mut bridge = BridgeOptimizer::new();

    assert!(
        bridge
            .add_variable(Some("bad\0name"), None, None, None)
            .is_err()
    );
    assert!(
        bridge
            .add_variable(Some("x"), Some('Q'), None, None)
            .is_err()
    );
    assert!(
        bridge
            .add_variable(Some("x"), None, Some(f64::NAN), None)
            .is_err()
    );
    assert!(bridge.vars.is_empty());

    let variable = bridge
        .add_variable(Some("x"), Some('C'), Some(0.0), Some(1.0))
        .unwrap();
    let invalid_function = ScalarFunctionType::Affine(ScalarAffineFn {
        terms: vec![AffineTerm {
            var: variable,
            coeff: f64::INFINITY,
        }],
        constant: 0.0,
    });
    assert!(
        bridge
            .add_constraint(invalid_function, ScalarSetType::LessThan(1.0), None)
            .is_err()
    );
    assert!(
        bridge
            .add_constraint(
                ScalarFunctionType::Variable(variable),
                ScalarSetType::LessThan(f64::NAN),
                None,
            )
            .is_err()
    );
    assert!(bridge.constrs.is_empty());
}

#[test]
fn result_getters_distinguish_invalid_ids_from_missing_results() {
    let mut bridge = BridgeOptimizer::new();
    let variable = bridge.add_variable(None, None, None, None).unwrap();

    assert_eq!(bridge.get_var_value(variable).unwrap(), None);
    assert!(matches!(
        bridge.get_var_value(VarId(99)),
        Err(MoiError::InvalidVariableIndex(99))
    ));
    assert_eq!(bridge.get_objective_value().unwrap(), None);
}
