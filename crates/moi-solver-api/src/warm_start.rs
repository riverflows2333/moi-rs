use moi_core::{MoiError, VarId};
use std::collections::HashSet;

/// A sparse primal start expressed in solver column IDs.
///
/// A start may be partial: variables absent from `variable_ids` are left for
/// the native solver to complete.  The two vectors are kept parallel so the
/// representation can cross an FFI boundary without allocating a dense model
/// sized buffer.
#[derive(Clone, Debug, PartialEq)]
pub struct PrimalStart {
    pub variable_ids: Vec<VarId>,
    pub values: Vec<f64>,
}

impl PrimalStart {
    pub fn new(variable_ids: Vec<VarId>, values: Vec<f64>) -> Result<Self, MoiError> {
        let start = Self {
            variable_ids,
            values,
        };
        start.validate()?;
        Ok(start)
    }

    pub fn validate(&self) -> Result<(), MoiError> {
        if self.variable_ids.len() != self.values.len() {
            return Err(MoiError::InvalidInput(format!(
                "MIP start has {} variable IDs but {} values",
                self.variable_ids.len(),
                self.values.len()
            )));
        }
        if self.values.iter().any(|value| !value.is_finite()) {
            return Err(MoiError::InvalidInput(
                "MIP start values must be finite".into(),
            ));
        }
        let mut seen = HashSet::with_capacity(self.variable_ids.len());
        for id in &self.variable_ids {
            if !seen.insert(id.0) {
                return Err(MoiError::InvalidInput(format!(
                    "MIP start contains duplicate variable ID {}",
                    id.0
                )));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primal_start_accepts_sparse_partial_values() {
        let start = PrimalStart::new(vec![VarId(4), VarId(1)], vec![1.0, 0.0]).unwrap();
        assert_eq!(start.variable_ids, vec![VarId(4), VarId(1)]);
    }

    #[test]
    fn primal_start_rejects_malformed_values() {
        assert!(PrimalStart::new(vec![VarId(0)], vec![]).is_err());
        assert!(PrimalStart::new(vec![VarId(0)], vec![f64::NAN]).is_err());
        assert!(PrimalStart::new(vec![VarId(0), VarId(0)], vec![0.0, 1.0]).is_err());
    }
}
