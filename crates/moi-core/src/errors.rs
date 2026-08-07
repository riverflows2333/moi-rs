use thiserror::Error;

#[derive(Error, Debug)]
pub enum MoiError {
    #[error("Unsupported constraint: function={func}, set={set}")]
    UnsupportedConstraint {
        func: &'static str,
        set: &'static str,
    },

    #[error("Add constraint not allowed")]
    AddConstraintNotAllowed,

    #[error("Unsupported attribute")]
    UnsupportedAttribute,

    #[error("Setting attribute not allowed (read-only or derived)")]
    SetAttributeNotAllowed,

    #[error("Scalar function constant not zero: {value}")]
    ScalarFunctionConstantNotZero { value: f64 },

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Invalid variable index: {0}")]
    InvalidVariableIndex(usize),

    #[error("Invalid name: {0}")]
    InvalidName(String),

    #[error("Backend state error: {0}")]
    BackendState(String),

    #[error("Backend protocol error: {0}")]
    BackendProtocol(String),

    #[error("{solver} error in {context}: code {code}")]
    NativeSolver {
        solver: &'static str,
        context: &'static str,
        code: i32,
    },

    #[error("{0}")]
    Msg(String),
}
