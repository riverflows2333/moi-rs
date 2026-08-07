use moi_bridge::BridgeOptimizer;
use moi_core::{
    AffineTerm, AttrValue, ModelAttr, ModelSense, ScalarAffineFn, ScalarFunctionType,
    ScalarSetType, VarId,
};
use moi_solver_api::{BoundType, ModelLike, NameType};

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
