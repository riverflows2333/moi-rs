use crate::functions::function::*;
use crate::indices::VarId;
#[derive(Clone, Debug, PartialEq)]
pub struct AffineTerm {
    pub var: VarId,
    pub coeff: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
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
    pub fn push_term(&mut self, var: VarId, coeff: f64) {
        if coeff != 0.0 {
            self.terms.push(AffineTerm { var, coeff });
        }
    }
    pub fn simplify(&mut self) {
        self.terms.sort_by(|a, b| a.var.0.cmp(&b.var.0));
        let mut merged: Vec<AffineTerm> = Vec::with_capacity(self.terms.len());
        for t in &self.terms {
            if let Some(last) = merged.last_mut() {
                if last.var == t.var {
                    last.coeff += t.coeff;
                    continue;
                }
            }
            merged.push(t.clone());
        }
        merged.retain(|t| t.coeff != 0.0);
        self.terms = merged;
    }

    pub fn calculate(&self, rhs: ScalarAffineFn, operation: OperationType) -> ScalarAffineFn {
        let mut result = ScalarAffineFn::new();
        match operation {
            OperationType::Add => {
                for term in &self.terms {
                    result.push_term(term.var, term.coeff);
                }
                for term in &rhs.terms {
                    result.push_term(term.var, term.coeff);
                }
                result.constant = self.constant + rhs.constant;
            }
            OperationType::Sub => {
                for term in &self.terms {
                    result.push_term(term.var, term.coeff);
                }
                for term in &rhs.terms {
                    result.push_term(term.var, -term.coeff);
                }
                result.constant = self.constant - rhs.constant;
            }
            OperationType::Mul => {
                // NOTE:只判断右侧或左侧为常数的情况
                if rhs.terms.is_empty() {
                    let scalar = rhs.constant;
                    for term in &self.terms {
                        result.push_term(term.var, term.coeff * scalar);
                    }
                    result.constant = self.constant * scalar;
                } else if self.terms.is_empty() {
                    let scalar = self.constant;
                    for term in &rhs.terms {
                        result.push_term(term.var, term.coeff * scalar);
                    }
                    result.constant = rhs.constant * scalar;
                } else {
                    panic!("Multiplication results in a non-affine function");
                }
            }
            _ => {
                panic!("Unsupported operation for ScalarAffineFn");
            }
        }
        result.simplify();
        result
    }
}
