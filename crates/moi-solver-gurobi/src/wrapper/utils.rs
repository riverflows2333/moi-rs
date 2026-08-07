use crate::bindings::*;
use moi_core::*;
use moi_solver_api::{scalar_function_to_linear, scalar_set_to_bounds};
// 提取ScalarConstraint中VarID与系数
pub fn scalar_constraint_to_grb(
    constraint: &ConstrInfo,
) -> Result<(Vec<VarId>, Vec<f64>, u8, f64), MoiError> {
    let sense;
    let mut rhs;
    let bounds = scalar_set_to_bounds(&constraint.s);
    match (bounds.lower, bounds.upper) {
        (None, Some(b)) => {
            sense = GRB_LESS_EQUAL;
            rhs = b;
        }
        (Some(b), None) => {
            sense = GRB_GREATER_EQUAL;
            rhs = b;
        }
        (Some(lower), Some(upper)) if lower == upper => {
            sense = GRB_EQUAL;
            rhs = lower;
        }
        _ => {
            return Err(MoiError::UnsupportedConstraint {
                func: "scalar linear",
                set: "interval",
            });
        }
    }
    let linear = scalar_function_to_linear(&constraint.f)?;
    rhs -= linear.constant;
    Ok((linear.variables, linear.coefficients, sense, rhs))
}

// 将函数提取VarID与系数
pub fn scalar_function_to_grb(
    function: &ScalarFunctionType,
) -> Result<(Vec<VarId>, Vec<f64>, f64), MoiError> {
    let linear = scalar_function_to_linear(function)?;
    Ok((linear.variables, linear.coefficients, linear.constant))
}

// 通过ConstraintInfo构建Gurobi格式
pub fn build_constr_matrix(
    constraints: &Vec<ConstrInfo>,
) -> Result<(Vec<u32>, Vec<u32>, Vec<f64>, Vec<u8>, Vec<f64>, Vec<String>), MoiError> {
    let mut cbeg = Vec::new();
    let mut cind = Vec::new();
    let mut cval = Vec::new();
    let mut sense = Vec::new();
    let mut rhs = Vec::new();
    let names = constraints.iter().map(|c| c.name.clone()).collect();

    for constraint in constraints {
        let (vars, coeffs, s, r) = scalar_constraint_to_grb(constraint)?;
        cbeg.push(cind.len() as u32);
        cind.extend(vars.iter().map(|v| v.0 as u32));
        cval.extend(coeffs);
        sense.push(s);
        rhs.push(r);
    }
    Ok((cbeg, cind, cval, sense, rhs, names))
}
