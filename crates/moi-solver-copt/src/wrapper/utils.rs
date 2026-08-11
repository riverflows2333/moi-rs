//! COPT-specific checked conversions and sparse matrix lowering helpers.

use crate::{CoptApi, bindings};
use moi_core::{MoiError, ScalarFunctionType, ScalarSetType};
use moi_solver_api::{ensure_len, scalar_function_to_linear, scalar_set_to_bounds};
use std::ffi::{CStr, CString, c_char, c_int, c_void};

pub(crate) struct CStringArray {
    _strings: Vec<CString>,
    pointers: Vec<*const c_char>,
}

impl CStringArray {
    pub(crate) fn new(values: Vec<String>, context: &str) -> Result<Self, MoiError> {
        let strings = values
            .into_iter()
            .map(|value| {
                CString::new(value).map_err(|_| {
                    MoiError::InvalidName(format!("{context} contains an embedded NUL byte"))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let pointers = strings.iter().map(|value| value.as_ptr()).collect();
        Ok(Self {
            _strings: strings,
            pointers,
        })
    }

    pub(crate) fn as_ptr(&self) -> *const *const c_char {
        ptr_or_null(&self.pointers)
    }
}

pub(crate) struct CoptRows {
    pub(crate) beg: Vec<c_int>,
    pub(crate) count: Vec<c_int>,
    pub(crate) index: Vec<c_int>,
    pub(crate) value: Vec<f64>,
    pub(crate) lower: Vec<f64>,
    pub(crate) upper: Vec<f64>,
    pub(crate) names: CStringArray,
}

pub(crate) fn checked_c_int(value: usize, field: &str) -> Result<c_int, MoiError> {
    c_int::try_from(value)
        .map_err(|_| MoiError::InvalidInput(format!("{field} exceeds COPT's c_int range")))
}

pub(crate) fn normalize_bound(value: f64) -> f64 {
    let infinity = bindings::COPT_INFINITY;
    value.clamp(-infinity, infinity)
}

pub(crate) fn map_variable_type(value: char) -> Result<c_char, MoiError> {
    match value {
        'C' => Ok(bindings::COPT_CONTINUOUS as c_char),
        'B' => Ok(bindings::COPT_BINARY as c_char),
        'I' => Ok(bindings::COPT_INTEGER as c_char),
        _ => Err(MoiError::InvalidInput(format!(
            "unsupported COPT variable type '{value}'; expected 'C', 'B', or 'I'"
        ))),
    }
}

pub(crate) fn build_copt_rows(
    functions: &[ScalarFunctionType],
    sets: &[ScalarSetType],
    names: Option<&[String]>,
    start_index: usize,
    num_vars: usize,
) -> Result<CoptRows, MoiError> {
    let n = functions.len();
    ensure_len(sets.len(), n, "constraint sets")?;
    if let Some(names) = names {
        ensure_len(names.len(), n, "constraint names")?;
    }

    let mut beg = Vec::with_capacity(n);
    let mut count = Vec::with_capacity(n);
    let mut index = Vec::new();
    let mut value = Vec::new();
    let mut lower = Vec::with_capacity(n);
    let mut upper = Vec::with_capacity(n);
    let mut row_names = Vec::with_capacity(n);

    for (offset, (function, set)) in functions.iter().zip(sets).enumerate() {
        let linear = scalar_function_to_linear(function)?;
        ensure_len(
            linear.coefficients.len(),
            linear.variables.len(),
            "constraint coefficients",
        )?;
        ensure_finite(linear.constant, "constraint constant")?;
        beg.push(checked_c_int(index.len(), "constraint matrix offset")?);
        count.push(checked_c_int(
            linear.variables.len(),
            "constraint nonzero count",
        )?);

        for (variable, coefficient) in linear.variables.iter().zip(&linear.coefficients) {
            if variable.0 >= num_vars {
                return Err(MoiError::InvalidVariableIndex(variable.0));
            }
            ensure_finite(*coefficient, "constraint coefficient")?;
            index.push(checked_c_int(variable.0, "variable index")?);
            value.push(*coefficient);
        }

        let bounds = scalar_set_to_bounds(set);
        if let (Some(lower), Some(upper)) = (bounds.lower, bounds.upper)
            && (lower.is_nan() || upper.is_nan() || lower > upper)
        {
            return Err(MoiError::InvalidInput(format!(
                "constraint {} has invalid bounds [{lower}, {upper}]",
                start_index + offset
            )));
        }
        let row_lower = bounds
            .lower
            .map(|bound| bound - linear.constant)
            .unwrap_or(f64::NEG_INFINITY);
        let row_upper = bounds
            .upper
            .map(|bound| bound - linear.constant)
            .unwrap_or(f64::INFINITY);
        if row_lower.is_nan() || row_upper.is_nan() {
            return Err(MoiError::InvalidInput(format!(
                "constraint {} produces a NaN bound",
                start_index + offset
            )));
        }
        lower.push(normalize_bound(row_lower));
        upper.push(normalize_bound(row_upper));
        row_names.push(
            names
                .map(|values| values[offset].clone())
                .unwrap_or_else(|| format!("c{}", start_index + offset)),
        );
    }

    Ok(CoptRows {
        beg,
        count,
        index,
        value,
        lower,
        upper,
        names: CStringArray::new(row_names, "constraint name")?,
    })
}

pub(crate) fn ensure_finite(value: f64, field: &str) -> Result<(), MoiError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(MoiError::InvalidInput(format!(
            "{field} must be finite, got {value}"
        )))
    }
}

pub(crate) fn ptr_or_null<T>(values: &[T]) -> *const T {
    if values.is_empty() {
        std::ptr::null()
    } else {
        values.as_ptr()
    }
}

pub(crate) fn native_error(api: &CoptApi, code: c_int, context: &'static str) -> MoiError {
    let detail = retcode_message(api, code);
    if detail.is_empty() {
        MoiError::NativeSolver {
            solver: "COPT",
            context,
            code,
        }
    } else {
        MoiError::Msg(format!("COPT error in {context}: code {code}: {detail}"))
    }
}

pub(crate) fn license_message(api: &CoptApi, env: *mut c_void) -> Option<String> {
    if env.is_null() {
        return None;
    }

    read_message(|buffer, size| unsafe { (api.COPT_GetLicenseMsg)(env, buffer, size) })
}

fn retcode_message(api: &CoptApi, code: c_int) -> String {
    read_message(|buffer, size| unsafe { (api.COPT_GetRetcodeMsg)(code, buffer, size) })
        .unwrap_or_default()
}

fn read_message(call: impl FnOnce(*mut c_char, c_int) -> c_int) -> Option<String> {
    let mut buffer = vec![0 as c_char; bindings::COPT_BUFFSIZE as usize];
    let size = c_int::try_from(buffer.len()).ok()?;
    if call(buffer.as_mut_ptr(), size) != bindings::COPT_RETCODE_OK as c_int {
        return None;
    }

    // SAFETY: successful COPT diagnostic functions write a NUL-terminated
    // string into the supplied buffer.
    let message = unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .trim()
        .to_owned();
    (!message.is_empty()).then_some(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use moi_core::{AffineTerm, ScalarAffineFn, VarId};

    #[test]
    fn variable_types_are_strict() {
        assert_eq!(map_variable_type('C').unwrap(), b'C' as c_char);
        assert_eq!(map_variable_type('B').unwrap(), b'B' as c_char);
        assert_eq!(map_variable_type('I').unwrap(), b'I' as c_char);
        assert!(map_variable_type('S').is_err());
    }

    #[test]
    fn affine_constant_shifts_both_interval_bounds() {
        let function = ScalarFunctionType::Affine(ScalarAffineFn {
            terms: vec![AffineTerm {
                var: VarId(0),
                coeff: 2.0,
            }],
            constant: 3.0,
        });
        let rows = build_copt_rows(
            &[function],
            &[ScalarSetType::Interval(4.0, 8.0)],
            None,
            0,
            1,
        )
        .unwrap();

        assert_eq!(rows.beg, vec![0]);
        assert_eq!(rows.count, vec![1]);
        assert_eq!(rows.index, vec![0]);
        assert_eq!(rows.value, vec![2.0]);
        assert_eq!(rows.lower, vec![1.0]);
        assert_eq!(rows.upper, vec![5.0]);
    }

    #[test]
    fn infinite_bounds_use_copt_sentinel() {
        assert_eq!(normalize_bound(f64::INFINITY), bindings::COPT_INFINITY);
        assert_eq!(normalize_bound(f64::NEG_INFINITY), -bindings::COPT_INFINITY);
    }
}
