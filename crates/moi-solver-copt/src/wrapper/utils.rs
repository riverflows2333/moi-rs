//! COPT-specific checked conversions and sparse matrix lowering helpers.

use crate::{CoptApi, bindings};
use moi_core::MoiError;
use std::ffi::{CStr, c_char, c_int, c_void};

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
