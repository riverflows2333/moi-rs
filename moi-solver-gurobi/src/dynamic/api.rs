use libloading::{Library, Symbol};
use std::{
    ffi::{c_char, c_double, c_int, c_void},
    path::PathBuf,
};

pub struct GurobiApi {
    _lib: Library,
    // environment functions
    pub GRBloadenv:
        unsafe extern "C" fn(env: *mut *mut c_void, logfilename: *const c_char) -> c_int,
    pub GRBfreeenv: unsafe extern "C" fn(env: *mut *mut c_void) -> c_int,
    // model functions
    pub GRBnewmodel: unsafe extern "C" fn(
        env: *mut c_void,
        model: *mut *mut c_void,
        name: *const c_char,
        numvars: c_int,
        obj: *const c_double,
        lb: *const c_double,
        ub: *const c_double,
        vtype: *const c_char,
        varnames: *const *const c_char,
    ) -> c_int,
    pub GRBfreemodel: unsafe extern "C" fn(model: *mut *mut c_void) -> c_int,
    pub GRBoptimize: unsafe extern "C" fn(model: *mut c_void) -> c_int,
}

impl GurobiApi {
    pub fn new(lib_path: PathBuf) -> Result<Self, libloading::Error> {
        unsafe {
            let lib = Library::new(lib_path)?;
            Ok(Self {
                GRBloadenv: *lib.get(b"GRBloadenv")?,
                GRBfreeenv: *lib.get(b"GRBfreeenv")?,
                GRBnewmodel: *lib.get(b"GRBnewmodel")?,
                GRBfreemodel: *lib.get(b"GRBfreemodel")?,
                GRBoptimize: *lib.get(b"GRBoptimize")?,
                _lib: lib,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamic::loader::find_library_from;
    #[test]
    fn test_load_gurobi_api() {
        let gurobi_api =
            GurobiApi::new(find_library_from("/usr/local/gurobi1203".to_string()).unwrap());
        assert!(gurobi_api.is_ok());
    }
}
