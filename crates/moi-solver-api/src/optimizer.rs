use crate::utils::*;
use moi_core::attributes::*;
use moi_core::errors::MoiError;
use moi_core::functions::ScalarFunctionType;
use moi_core::indices::{ConstrId, VarId};
use moi_core::sets::ScalarSetType;

pub trait ModelLike {
    /// Compatibility fallback; native backends override this to consume buffers.
    fn add_linear_rows(
        &mut self,
        rows: crate::LinearRows,
    ) -> Result<std::ops::Range<usize>, MoiError> {
        rows.validate()?;
        if rows.num_rows() == 0 {
            return match self.get_model_attr(ModelAttr::NumberOfConstraints) {
                Some(AttrValue::Usize(count)) => Ok(count..count),
                _ => Err(MoiError::UnsupportedAttribute),
            };
        }
        let mut fs = Vec::with_capacity(rows.num_rows());
        let mut ss = Vec::with_capacity(rows.num_rows());
        for r in 0..rows.num_rows() {
            let terms = (rows.row_offsets[r] as usize..rows.row_offsets[r + 1] as usize)
                .map(|i| moi_core::AffineTerm {
                    var: VarId(rows.col_indices[i] as usize),
                    coeff: rows.coefficients[i],
                })
                .collect();
            fs.push(ScalarFunctionType::Affine(moi_core::ScalarAffineFn {
                terms,
                constant: 0.0,
            }));
            let (l, u) = (rows.row_lower[r], rows.row_upper[r]);
            ss.push(if l == u {
                ScalarSetType::EqualTo(l)
            } else if l == f64::NEG_INFINITY {
                ScalarSetType::LessThan(u)
            } else if u == f64::INFINITY {
                ScalarSetType::GreaterThan(l)
            } else {
                ScalarSetType::Interval(l, u)
            });
        }
        let count = rows.num_rows();
        let ids = self.add_constraints(fs, ss, rows.names)?;
        let start = ids.first().map_or(0, |id| id.0);
        let end = start
            .checked_add(count)
            .ok_or_else(|| MoiError::BackendProtocol("constraint ID overflow".into()))?;
        if ids.len() != count
            || ids
                .iter()
                .zip(start..end)
                .any(|(id, expected)| id.0 != expected)
        {
            return Err(MoiError::BackendProtocol(
                "noncontiguous linear row IDs".into(),
            ));
        }
        Ok(start..end)
    }

    fn set_linear_objective(
        &mut self,
        objective: crate::LinearObjective,
        sense: ModelSense,
    ) -> Result<(), MoiError> {
        objective.validate()?;
        self.set_objective(objective.into_function(), sense)
    }

    fn add_variable(
        &mut self,
        name: Option<&str>,
        vtype: Option<char>,
        lb: Option<f64>,
        ub: Option<f64>,
    ) -> Result<VarId, MoiError> {
        let mut ids = self.add_variables(
            1,
            name.map(|s| NameType::Vector(vec![s.to_string()])),
            vtype.map(|v| vec![v]),
            lb.map(|v| BoundType::Vector(vec![v])),
            ub.map(|v| BoundType::Vector(vec![v])),
        )?;
        ensure_len(ids.len(), 1, "returned variable IDs")?;
        ids.pop().ok_or_else(|| {
            MoiError::BackendProtocol("add_variables returned no variable ID".to_string())
        })
    }

    fn add_variables(
        &mut self,
        n: usize,
        name: Option<NameType>,
        vtype: Option<Vec<char>>,
        lb: Option<BoundType>,
        ub: Option<BoundType>,
    ) -> Result<Vec<VarId>, MoiError>;

    fn add_constraint(
        &mut self,
        f: ScalarFunctionType,
        s: ScalarSetType,
        name: Option<String>,
    ) -> Result<ConstrId, MoiError> {
        let mut ids = self.add_constraints(vec![f], vec![s], name.map(|n| vec![n]))?;
        ensure_len(ids.len(), 1, "returned constraint IDs")?;
        ids.pop().ok_or_else(|| {
            MoiError::BackendProtocol("add_constraints returned no constraint ID".to_string())
        })
    }

    fn add_constraints(
        &mut self,
        fs: Vec<ScalarFunctionType>,
        ss: Vec<ScalarSetType>,
        names: Option<Vec<String>>,
    ) -> Result<Vec<ConstrId>, MoiError>;

    fn set_objective(&mut self, f: ScalarFunctionType, sense: ModelSense) -> Result<(), MoiError>;
    fn update(&mut self) -> Result<(), MoiError>;

    fn get_model_attr(&self, attr: ModelAttr) -> Option<AttrValue>;
    fn set_model_attr(&mut self, attr: ModelAttr, value: AttrValue) -> Result<(), MoiError>;

    fn get_optimizer_attr(&self, attr: OptimizerAttr) -> Option<AttrValue>;
    fn set_optimizer_attr(&mut self, attr: OptimizerAttr, value: AttrValue)
    -> Result<(), MoiError>;
}

pub trait Optimizer: ModelLike {
    fn optimize(&mut self) -> Result<SolveStatus, MoiError>;
    fn compute_conflict(&mut self) -> Result<(), MoiError>;
    fn get_var_value(&self, var_id: VarId) -> Result<Option<f64>, MoiError>;
    fn get_objective_value(&self) -> Result<Option<f64>, MoiError>;
}
