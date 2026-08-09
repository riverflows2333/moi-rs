use moi_core::{AttrValue, ModelAttr, ModelSense, ScalarAffineFn, ScalarFunctionType};
use moi_model_dummy::{DummyModel, RecordedCall};
use moi_solver_api::ModelLike;

#[test]
fn objective_is_available_through_attributes_and_recording() {
    let (mut model, handle) = DummyModel::new();
    let variable = model.add_variable(Some("x"), None, None, None).unwrap();
    let mut objective = ScalarAffineFn::default();
    objective.push_term(variable, 5.0);

    model
        .set_model_attr(
            ModelAttr::ObjectiveFunction,
            AttrValue::ScalarFn(ScalarFunctionType::Affine(objective.clone())),
        )
        .unwrap();
    model
        .set_model_attr(
            ModelAttr::ObjectiveSense,
            AttrValue::ModelSense(ModelSense::Maximize),
        )
        .unwrap();

    assert_eq!(
        model.get_model_attr(ModelAttr::ObjectiveFunction),
        Some(AttrValue::ScalarFn(ScalarFunctionType::Affine(objective)))
    );
    assert_eq!(
        model.get_model_attr(ModelAttr::ObjectiveSense),
        Some(AttrValue::ModelSense(ModelSense::Maximize))
    );
    assert!(matches!(
        handle.snapshot().unwrap().calls.last(),
        Some(RecordedCall::SetObjective {
            sense: ModelSense::Maximize,
            ..
        })
    ));
}
