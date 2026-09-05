//! Owning linear batches. Signed 32-bit indices match the native LP/MILP APIs.
use moi_core::*;

#[derive(Debug, Clone)]
pub struct LinearRows {
    pub num_cols: usize,
    pub row_offsets: Vec<i32>,
    pub col_indices: Vec<i32>,
    pub coefficients: Vec<f64>,
    pub row_lower: Vec<f64>,
    pub row_upper: Vec<f64>,
    pub names: Option<Vec<String>>,
}

pub fn linear_index(value: usize) -> Result<i32, MoiError> {
    i32::try_from(value)
        .map_err(|_| MoiError::InvalidInput("linear index exceeds i32 range".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn function() -> ScalarFunctionType {
        ScalarFunctionType::Affine(ScalarAffineFn {
            terms: vec![AffineTerm {
                var: VarId(0),
                coeff: 2.0,
            }],
            constant: 3.0,
        })
    }
    #[test]
    fn shifts_constants_and_supports_empty_rows() {
        let mut rows = LinearRows::new(1);
        rows.push(&function(), &ScalarSetType::Interval(4.0, 8.0))
            .unwrap();
        rows.push(
            &ScalarFunctionType::Affine(ScalarAffineFn::default()),
            &ScalarSetType::EqualTo(0.0),
        )
        .unwrap();
        rows.validate().unwrap();
        assert_eq!(rows.row_offsets, [0, 1, 1]);
        assert_eq!(rows.row_lower, [1.0, 0.0]);
        assert_eq!(rows.row_upper, [5.0, 0.0]);
    }
    #[test]
    fn failed_append_keeps_batch_intact() {
        let mut rows = LinearRows::new(1);
        let bad = ScalarFunctionType::Affine(ScalarAffineFn {
            terms: vec![
                AffineTerm {
                    var: VarId(0),
                    coeff: 1.0,
                },
                AffineTerm {
                    var: VarId(1),
                    coeff: 1.0,
                },
            ],
            constant: 0.0,
        });
        assert!(rows.push(&bad, &ScalarSetType::EqualTo(0.0)).is_err());
        assert_eq!(rows.nnz(), 0);
        rows.validate().unwrap();
        assert!(
            rows.push(&function(), &ScalarSetType::LessThan(f64::NAN))
                .is_err()
        );
        assert!(linear_index(i32::MAX as usize + 1).is_err());
    }
    #[test]
    fn malformed_buffers_are_rejected() {
        let mut valid = LinearRows::new(1);
        valid
            .push(&function(), &ScalarSetType::LessThan(4.0))
            .unwrap();
        for mutation in 0..7 {
            let mut rows = valid.clone();
            match mutation {
                0 => rows.row_offsets[0] = -1,
                1 => rows.row_offsets[1] = 2,
                2 => rows.col_indices[0] = -1,
                3 => rows.coefficients[0] = f64::NAN,
                4 => rows.row_upper.clear(),
                5 => rows.names = Some(vec![]),
                _ => rows.names = Some(vec!["bad\0name".into()]),
            }
            assert!(rows.validate().is_err());
        }
    }

    #[test]
    fn objective_buffers_and_shift_overflow_are_rejected() {
        let mut objective = LinearObjective {
            num_cols: 1,
            col_indices: vec![0],
            coefficients: vec![1.0],
            constant: 0.0,
        };
        objective.validate().unwrap();
        objective.coefficients[0] = f64::INFINITY;
        assert!(objective.validate().is_err());
        objective.coefficients[0] = 1.0;
        objective.col_indices[0] = 1;
        assert!(objective.validate().is_err());
        let mut rows = LinearRows::new(0);
        let f = ScalarFunctionType::Affine(ScalarAffineFn {
            terms: vec![],
            constant: -f64::MAX,
        });
        assert!(rows.push(&f, &ScalarSetType::EqualTo(f64::MAX)).is_err());
        rows.validate().unwrap();
    }
}

impl LinearRows {
    pub fn new(num_cols: usize) -> Self {
        Self {
            num_cols,
            row_offsets: vec![0],
            col_indices: Vec::new(),
            coefficients: Vec::new(),
            row_lower: Vec::new(),
            row_upper: Vec::new(),
            names: None,
        }
    }

    pub fn num_rows(&self) -> usize {
        self.row_lower.len()
    }
    pub fn nnz(&self) -> usize {
        self.coefficients.len()
    }

    /// Append a borrowed expression without cloning its terms. Failure leaves
    /// this batch unchanged; the solver is never involved in construction.
    pub fn push(
        &mut self,
        function: &ScalarFunctionType,
        set: &ScalarSetType,
    ) -> Result<(), MoiError> {
        let constant = match function {
            ScalarFunctionType::Variable(_) => 0.0,
            ScalarFunctionType::Affine(f) => f.constant,
        };
        if !constant.is_finite() {
            return Err(MoiError::InvalidInput(
                "constraint constant must be finite".into(),
            ));
        }
        let (lower, upper) = match *set {
            ScalarSetType::LessThan(v) => (f64::NEG_INFINITY, v),
            ScalarSetType::GreaterThan(v) => (v, f64::INFINITY),
            ScalarSetType::EqualTo(v) => (v, v),
            ScalarSetType::Interval(l, u) => (l, u),
        };
        let valid = match *set {
            ScalarSetType::LessThan(v)
            | ScalarSetType::GreaterThan(v)
            | ScalarSetType::EqualTo(v) => v.is_finite(),
            ScalarSetType::Interval(l, u) => l.is_finite() && u.is_finite() && l <= u,
        };
        if !valid
            || (lower.is_finite() && !(lower - constant).is_finite())
            || (upper.is_finite() && !(upper - constant).is_finite())
        {
            return Err(MoiError::InvalidInput(
                "invalid or overflowing constraint bounds".into(),
            ));
        }
        linear_index(
            self.num_rows()
                .checked_add(1)
                .ok_or_else(|| MoiError::InvalidInput("row count overflow".into()))?,
        )?;
        let start = self.nnz();
        let result = match function {
            ScalarFunctionType::Variable(v) => self.push_term(*v, 1.0),
            ScalarFunctionType::Affine(f) => f
                .terms
                .iter()
                .try_for_each(|t| self.push_term(t.var, t.coeff)),
        }
        .and_then(|()| linear_index(self.nnz()));
        let end = match result {
            Ok(end) => end,
            Err(error) => {
                self.col_indices.truncate(start);
                self.coefficients.truncate(start);
                return Err(error);
            }
        };
        self.row_offsets.push(end);
        self.row_lower.push(lower - constant);
        self.row_upper.push(upper - constant);
        if let Some(names) = &mut self.names {
            names.push(String::new());
        }
        Ok(())
    }

    fn push_term(&mut self, variable: VarId, coefficient: f64) -> Result<(), MoiError> {
        if variable.0 >= self.num_cols {
            return Err(MoiError::InvalidVariableIndex(variable.0));
        }
        if !coefficient.is_finite() {
            return Err(MoiError::InvalidInput("coefficient must be finite".into()));
        }
        self.col_indices.push(linear_index(variable.0)?);
        self.coefficients.push(coefficient);
        Ok(())
    }

    pub fn validate(&self) -> Result<(), MoiError> {
        linear_index(self.num_cols)?;
        linear_index(self.num_rows())?;
        let end = linear_index(self.nnz())?;
        if self.row_offsets.len() != self.num_rows() + 1
            || self.row_offsets.first() != Some(&0)
            || self.row_offsets.last() != Some(&end)
            || self.row_offsets.windows(2).any(|w| w[0] > w[1])
            || self.row_upper.len() != self.num_rows()
            || self.col_indices.len() != self.nnz()
        {
            return Err(MoiError::InvalidInput(
                "invalid linear row lengths or offsets".into(),
            ));
        }
        for (&col, &coefficient) in self.col_indices.iter().zip(&self.coefficients) {
            if col < 0 || col as usize >= self.num_cols || !coefficient.is_finite() {
                return Err(MoiError::InvalidInput(
                    "invalid linear column or coefficient".into(),
                ));
            }
        }
        for (&lower, &upper) in self.row_lower.iter().zip(&self.row_upper) {
            if lower.is_nan()
                || upper.is_nan()
                || lower > upper
                || lower == f64::INFINITY
                || upper == f64::NEG_INFINITY
            {
                return Err(MoiError::InvalidInput("invalid linear row bounds".into()));
            }
        }
        if let Some(names) = &self.names {
            if names.len() != self.num_rows() {
                return Err(MoiError::InvalidInput(
                    "constraint name count mismatch".into(),
                ));
            }
            if names.iter().any(|n| n.contains('\0')) {
                return Err(MoiError::InvalidName("constraint name contains NUL".into()));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LinearObjective {
    pub num_cols: usize,
    pub col_indices: Vec<i32>,
    pub coefficients: Vec<f64>,
    pub constant: f64,
}

impl LinearObjective {
    pub fn from_affine(num_cols: usize, f: &ScalarAffineFn) -> Result<Self, MoiError> {
        let mut value = Self {
            num_cols,
            col_indices: Vec::with_capacity(f.terms.len()),
            coefficients: Vec::with_capacity(f.terms.len()),
            constant: f.constant,
        };
        for t in &f.terms {
            value.col_indices.push(linear_index(t.var.0)?);
            value.coefficients.push(t.coeff);
        }
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<(), MoiError> {
        linear_index(self.num_cols)?;
        linear_index(self.col_indices.len())?;
        if !self.constant.is_finite()
            || self.col_indices.len() != self.coefficients.len()
            || self
                .col_indices
                .iter()
                .any(|&i| i < 0 || i as usize >= self.num_cols)
            || self.coefficients.iter().any(|v| !v.is_finite())
        {
            return Err(MoiError::InvalidInput("invalid linear objective".into()));
        }
        Ok(())
    }
    pub fn into_function(self) -> ScalarFunctionType {
        ScalarFunctionType::Affine(ScalarAffineFn {
            constant: self.constant,
            terms: self
                .col_indices
                .into_iter()
                .zip(self.coefficients)
                .map(|(i, coeff)| AffineTerm {
                    var: VarId(i as usize),
                    coeff,
                })
                .collect(),
        })
    }
}
