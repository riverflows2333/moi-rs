use moi_solver_copt::{CoptApi, CoptEnv, CoptEnvConfig, CoptOptimizer, bindings, find_library};
use std::ffi::{CStr, c_char};
use std::sync::{Arc, Mutex};

fn configured_api() -> Option<Arc<CoptApi>> {
    let Some(path) = find_library() else {
        eprintln!("skipping native COPT test: COPT native library was not found");
        return None;
    };
    Some(Arc::new(CoptApi::new(path).expect(
        "configured COPT library should expose the MILP API",
    )))
}

#[test]
fn configured_library_loads_minimum_api_and_returns_banner() {
    let Some(api) = configured_api() else {
        return;
    };
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

#[test]
fn default_environment_and_problem_lifecycle() {
    let Some(api) = configured_api() else {
        return;
    };
    let env = Arc::new(Mutex::new(
        CoptEnv::new(api).expect("default COPT environment should be created"),
    ));
    let optimizer = CoptOptimizer::new(env.clone()).expect("COPT problem should be created");

    assert_eq!(optimizer.num_variables(), 0);
    assert_eq!(optimizer.num_constraints(), 0);
    assert_eq!(Arc::strong_count(&env), 2);

    drop(optimizer);
    assert_eq!(Arc::strong_count(&env), 1);
}

#[test]
fn explicit_license_directory_creates_environment() {
    let Some(api) = configured_api() else {
        return;
    };
    let Some(license_dir) = std::env::var_os("COPT_LICENSE_DIR") else {
        return;
    };
    let license_dir = license_dir
        .to_str()
        .expect("COPT_LICENSE_DIR should be valid UTF-8");

    let env = CoptEnv::with_license_dir(api, license_dir)
        .expect("explicit COPT license directory should create an environment");
    drop(env);
}

#[test]
fn configured_environment_and_problem_lifecycle() {
    let Some(api) = configured_api() else {
        return;
    };
    let mut config = CoptEnvConfig::new(api).expect("COPT client configuration should be created");
    config
        .set("NoBanner", "1")
        .expect("NoBanner should be accepted by COPT");

    let env = Arc::new(Mutex::new(
        CoptEnv::with_config(&config).expect("configured COPT environment should be created"),
    ));
    let optimizer = CoptOptimizer::new(env).expect("configured problem should be created");
    assert_eq!(optimizer.num_variables(), 0);
    assert_eq!(optimizer.num_constraints(), 0);
}

#[test]
fn environment_config_rejects_embedded_nul_without_exposing_values() {
    let Some(api) = configured_api() else {
        return;
    };
    let mut config = CoptEnvConfig::new(api).expect("COPT client configuration should be created");

    let name_error = config
        .set("bad\0name", "value")
        .expect_err("embedded NUL in a name should be rejected");
    assert!(name_error.to_string().contains("name"));

    let value_error = config
        .set("License", "sensitive\0value")
        .expect_err("embedded NUL in a value should be rejected");
    let message = value_error.to_string();
    assert!(message.contains("value"));
    assert!(!message.contains("sensitive"));
}

#[test]
fn shared_environment_supports_multiple_problems() {
    let Some(api) = configured_api() else {
        return;
    };
    let env = Arc::new(Mutex::new(
        CoptEnv::new(api).expect("default COPT environment should be created"),
    ));

    let first = CoptOptimizer::new(env.clone()).expect("first COPT problem should be created");
    let second = CoptOptimizer::new(env.clone()).expect("second COPT problem should be created");
    assert_eq!(Arc::strong_count(&env), 3);

    drop(first);
    drop(second);
    assert_eq!(Arc::strong_count(&env), 1);
}
