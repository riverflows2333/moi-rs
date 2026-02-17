use crate::bindings::*;
use moi_core::*;
// 提取ScalarConstraint中VarID与系数
pub fn scalar_constraint_to_grb(
    constraint: &ConstrInfo,
) -> Result<(Vec<VarId>, Vec<f64>, u8, f64), String> {
    let mut var = Vec::new();
    let mut coeff = Vec::new();
    let sense;
    let mut rhs;
    match &constraint.s {
        ScalarSetType::LessThan(b) => {
            sense = GRB_LESS_EQUAL;
            rhs = *b;
        }
        ScalarSetType::GreaterThan(b) => {
            sense = GRB_GREATER_EQUAL;
            rhs = *b;
        }
        ScalarSetType::EqualTo(b) => {
            sense = GRB_EQUAL;
            rhs = *b;
        }
        _ => {
            return Err("Unsupported constraint set type".to_string());
        }
    }
    match &constraint.f {
        ScalarFunctionType::Affine(afn) => {
            for term in &afn.terms {
                var.push(term.var);
                coeff.push(term.coeff);
            }
            if afn.constant != 0.0 {
                // 处理常数项
                // 这里假设常数项移动到约束右侧
                rhs -= afn.constant;
            }
        }
        _ => {
            return Err("Unsupported function type in constraint".to_string());
        }
    }
    Ok((var, coeff, sense, rhs))
}

// 将函数提取VarID与系数
pub fn scalar_function_to_grb(
    function: &ScalarFunctionType,
) -> Result<(Vec<VarId>, Vec<f64>, f64), String> {
    let mut var = Vec::new();
    let mut coeff = Vec::new();
    let constant;
    match function {
        ScalarFunctionType::Affine(afn) => {
            for term in &afn.terms {
                var.push(term.var);
                coeff.push(term.coeff);
            }
            constant = afn.constant;
        }
        _ => {
            return Err("Unsupported function type".to_string());
        }
    }
    Ok((var, coeff, constant))
}

// 通过ConstraintInfo构建Gurobi格式
pub fn build_constr_matrix(
    constraints: &Vec<ConstrInfo>,
) -> Result<(Vec<u32>, Vec<u32>, Vec<f64>, Vec<u8>, Vec<f64>, Vec<String>), String> {
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
