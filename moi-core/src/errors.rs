use thiserror::Error;

#[derive(Error, Debug)]
pub enum MoiError {
    #[error("Unsupported constraint: function={func}, set={set}")]
    UnsupportedConstraint { func: &'static str, set: &'static str },

    #[error("Add constraint not allowed")]
    AddConstraintNotAllowed,

    #[error("Unsupported attribute")]
    UnsupportedAttribute,

    #[error("Scalar function constant not zero: {value}")]
    ScalarFunctionConstantNotZero { value: f64 },

    #[error("{0}")]
    Msg(String),
}
