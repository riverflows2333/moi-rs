use moi_solver_copt::{CoptApi, bindings, find_library};
use std::ffi::{CStr, c_char};

#[test]
fn configured_library_loads_minimum_api_and_returns_banner() {
    let Some(path) = find_library() else {
        return;
    };
    let api = CoptApi::new(path).expect("configured COPT library should expose the MILP API");
    let mut buffer = vec![0 as c_char; bindings::COPT_BUFFSIZE as usize];

    // SAFETY: `buffer` is writable for the declared length and the loaded
    // function signature comes from COPT 8.0.6 `copt.h`.
    let retcode = unsafe {
        (api.COPT_GetBanner)(
            buffer.as_mut_ptr(),
            i32::try_from(buffer.len()).expect("COPT banner buffer fits in c_int"),
        )
    };
    assert_eq!(retcode, bindings::COPT_RETCODE_OK as i32);

    // SAFETY: a successful COPT_GetBanner call writes a NUL-terminated string.
    let banner = unsafe { CStr::from_ptr(buffer.as_ptr()) }
        .to_string_lossy()
        .into_owned();
    let expected_version = format!(
        "v{}.{}.{}",
        bindings::COPT_VERSION_MAJOR,
        bindings::COPT_VERSION_MINOR,
        bindings::COPT_VERSION_TECHNICAL
    );
    assert!(
        banner.contains("Cardinal Optimizer") && banner.contains(&expected_version),
        "unexpected COPT banner: {banner}"
    );
}
