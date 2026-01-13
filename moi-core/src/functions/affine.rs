use crate::indices::VarId;

#[derive(Clone, Debug,PartialEq)]
pub struct AffineTerm {
    pub var: VarId,
    pub coeff: f64,
}

#[derive(Clone, Debug, Default,PartialEq)]
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
}
