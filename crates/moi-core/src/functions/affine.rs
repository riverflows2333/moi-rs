use crate::MoiError;
use crate::functions::function::*;
use crate::indices::VarId;
use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct AffineTerm {
    pub var: VarId,
    pub coeff: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, Encode, Decode)]
pub struct ScalarAffineFn {
    pub terms: Vec<AffineTerm>,
    pub constant: f64,
}

impl ScalarAffineFn {
    pub fn new() -> Self {
        Self {
            terms: Vec::new(),
            constant: 0.0,
        }
    }
    pub fn with_constant(constant: f64) -> Self {
        Self {
            terms: Vec::new(),
            constant,
        }
    }
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            terms: Vec::with_capacity(capacity),
            constant: 0.0,
        }
    }
    pub fn reserve(&mut self, additional: usize) {
        self.terms.reserve(additional);
    }
    pub fn push_term(&mut self, var: VarId, coeff: f64) {
        if coeff != 0.0 {
            self.terms.push(AffineTerm { var, coeff });
        }
    }
    pub fn extend_terms(&mut self, terms: impl IntoIterator<Item = AffineTerm>) {
        self.terms
            .extend(terms.into_iter().filter(|term| term.coeff != 0.0));
    }
    pub fn add_assign(&mut self, rhs: &ScalarAffineFn) {
        self.reserve(rhs.terms.len());
        self.extend_terms(rhs.terms.iter().copied());
        self.constant += rhs.constant;
    }
    pub fn add_scaled_assign(&mut self, rhs: &ScalarAffineFn, scale: f64) {
        if scale == 0.0 {
            return;
        }
        self.reserve(rhs.terms.len());
        self.extend_terms(rhs.terms.iter().map(|term| AffineTerm {
            var: term.var,
            coeff: term.coeff * scale,
        }));
        self.constant += rhs.constant * scale;
    }
    pub fn simplify(&mut self) {
        if self.terms.len() > 1
            && !self
                .terms
                .windows(2)
                .all(|terms| terms[0].var.0 <= terms[1].var.0)
        {
            self.terms.sort_unstable_by_key(|term| term.var.0);
        }
        self.terms.dedup_by(|later, earlier| {
            if later.var == earlier.var {
                earlier.coeff += later.coeff;
                true
            } else {
                false
            }
        });
        self.terms.retain(|term| term.coeff != 0.0);
    }

    pub fn calculate(
        &self,
        rhs: &ScalarAffineFn,
        operation: OperationType,
    ) -> Result<ScalarAffineFn, MoiError> {
        match operation {
            OperationType::Add => {
                let mut result = ScalarAffineFn::with_capacity(self.terms.len() + rhs.terms.len());
                result.add_assign(self);
                result.add_assign(rhs);
                result.simplify();
                Ok(result)
            }
            OperationType::Sub => {
                let mut result = ScalarAffineFn::with_capacity(self.terms.len() + rhs.terms.len());
                result.add_assign(self);
                result.add_scaled_assign(rhs, -1.0);
                result.simplify();
                Ok(result)
            }
            OperationType::Mul => {
                // NOTE:只判断右侧或左侧为常数的情况
                if rhs.terms.is_empty() {
                    let mut result = ScalarAffineFn::with_capacity(self.terms.len());
                    result.add_scaled_assign(self, rhs.constant);
                    Ok(result)
                } else if self.terms.is_empty() {
                    let mut result = ScalarAffineFn::with_capacity(rhs.terms.len());
                    result.add_scaled_assign(rhs, self.constant);
                    Ok(result)
                } else {
                    Err(MoiError::InvalidInput(
                        "multiplication results in a non-affine function".into(),
                    ))
                }
            }
            OperationType::Div => Err(MoiError::InvalidInput(
                "division is not supported for scalar affine functions".into(),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplify_sorts_merges_and_removes_cancelled_terms() {
        let mut function = ScalarAffineFn::new();
        function.push_term(VarId(2), 3.0);
        function.push_term(VarId(1), 4.0);
        function.push_term(VarId(2), -3.0);

        function.simplify();

        assert_eq!(
            function.terms,
            vec![AffineTerm {
                var: VarId(1),
                coeff: 4.0
            }]
        );
    }

    #[test]
    fn add_assign_defers_normalization_until_requested() {
        let mut left = ScalarAffineFn::with_constant(2.0);
        left.push_term(VarId(0), 1.0);
        let mut right = ScalarAffineFn::with_constant(3.0);
        right.push_term(VarId(0), 4.0);
        right.push_term(VarId(1), 5.0);

        left.add_assign(&right);
        assert_eq!(left.terms.len(), 3);
        assert_eq!(left.constant, 5.0);

        left.simplify();
        assert_eq!(
            left.terms,
            vec![
                AffineTerm {
                    var: VarId(0),
                    coeff: 5.0
                },
                AffineTerm {
                    var: VarId(1),
                    coeff: 5.0
                }
            ]
        );
    }

    #[test]
    fn add_scaled_assign_applies_scale_to_terms_and_constant() {
        let mut result = ScalarAffineFn::new();
        let mut source = ScalarAffineFn::with_constant(2.0);
        source.push_term(VarId(3), 4.0);

        result.add_scaled_assign(&source, -0.5);

        assert_eq!(result.constant, -1.0);
        assert_eq!(
            result.terms,
            vec![AffineTerm {
                var: VarId(3),
                coeff: -2.0
            }]
        );
    }

    #[test]
    fn calculate_rejects_non_affine_operations_without_panicking() {
        let mut left = ScalarAffineFn::new();
        left.push_term(VarId(0), 1.0);
        let mut right = ScalarAffineFn::new();
        right.push_term(VarId(1), 1.0);

        assert!(left.calculate(&right, OperationType::Mul).is_err());
        assert!(left.calculate(&right, OperationType::Div).is_err());
    }
}
