use crate::bindings::*;
use crate::wrapper::wrapper::ConstrInfo;
use moi_core::*;
// 提取ScalarConstraint中VarID与系数
pub fn scalar_constraint_to_grb(
    constraint: &ConstrInfo,
) -> Result<(Vec<VarId>, Vec<f64>, Vec<u8>, Vec<f64>), String> {
    let mut var = Vec::new();
    let mut coeff = Vec::new();
    let mut sense = Vec::new();
    let mut rhs = Vec::new();
    match &constraint.s {
        ScalarSetType::LessThan(b) => {
            sense.push(GRB_LESS_EQUAL);
            rhs.push(*b);
        }
        ScalarSetType::GreaterThan(b) => {
            sense.push(GRB_GREATER_EQUAL);
            rhs.push(*b);
        }
        ScalarSetType::EqualTo(b) => {
            sense.push(GRB_EQUAL);
            rhs.push(*b);
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
                for r in &mut rhs {
                    *r -= afn.constant;
                }
            }
        }
        _ => {
            return Err("Unsupported function type in constraint".to_string());
        }
    }
    Ok((var, coeff, sense, rhs))
}
