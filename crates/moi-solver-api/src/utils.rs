use moi_core::{MoiError, ScalarFunctionType, ScalarSetType, VarId};

#[derive(Clone, Debug)]
pub enum BoundType {
    Single(f64),
    Vector(Vec<f64>),
}

#[derive(Clone, Debug)]
pub enum NameType {
    Single(String),
    Vector(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinearScalarFunction {
    pub variables: Vec<VarId>,
    pub coefficients: Vec<f64>,
    pub constant: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScalarBounds {
    pub lower: Option<f64>,
    pub upper: Option<f64>,
}

pub fn scalar_set_to_bounds(set: &ScalarSetType) -> ScalarBounds {
    match set {
        ScalarSetType::GreaterThan(value) => ScalarBounds {
            lower: Some(*value),
            upper: None,
        },
        ScalarSetType::LessThan(value) => ScalarBounds {
            lower: None,
            upper: Some(*value),
        },
        ScalarSetType::EqualTo(value) => ScalarBounds {
            lower: Some(*value),
            upper: Some(*value),
        },
        ScalarSetType::Interval(lower, upper) => ScalarBounds {
            lower: Some(*lower),
            upper: Some(*upper),
        },
    }
}

pub fn scalar_function_to_linear(
    function: &ScalarFunctionType,
) -> Result<LinearScalarFunction, MoiError> {
    match function {
        ScalarFunctionType::Affine(affine) => Ok(LinearScalarFunction {
            variables: affine.terms.iter().map(|term| term.var).collect(),
            coefficients: affine.terms.iter().map(|term| term.coeff).collect(),
            constant: affine.constant,
        }),
        ScalarFunctionType::Variable(variable) => Ok(LinearScalarFunction {
            variables: vec![*variable],
            coefficients: vec![1.0],
            constant: 0.0,
        }),
    }
}

pub fn ensure_len(actual: usize, expected: usize, field: &str) -> Result<(), MoiError> {
    if actual == expected {
        Ok(())
    } else {
        Err(MoiError::InvalidInput(format!(
            "{field} has length {actual}, expected {expected}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moi_core::{AffineTerm, ScalarAffineFn};

    #[test]
    fn variable_function_is_lowered_to_linear_data() {
        let linear = scalar_function_to_linear(&ScalarFunctionType::Variable(VarId(3))).unwrap();
        assert_eq!(linear.variables, vec![VarId(3)]);
        assert_eq!(linear.coefficients, vec![1.0]);
        assert_eq!(linear.constant, 0.0);
    }

    #[test]
    fn affine_function_is_lowered_without_losing_constant() {
        let function = ScalarFunctionType::Affine(ScalarAffineFn {
            terms: vec![AffineTerm {
                var: VarId(2),
                coeff: 4.0,
            }],
            constant: 5.0,
        });
        let linear = scalar_function_to_linear(&function).unwrap();
        assert_eq!(linear.variables, vec![VarId(2)]);
        assert_eq!(linear.coefficients, vec![4.0]);
        assert_eq!(linear.constant, 5.0);
    }

    #[test]
    fn scalar_sets_are_lowered_to_bounds() {
        assert_eq!(
            scalar_set_to_bounds(&ScalarSetType::Interval(-1.0, 2.0)),
            ScalarBounds {
                lower: Some(-1.0),
                upper: Some(2.0),
            }
        );
    }
}
