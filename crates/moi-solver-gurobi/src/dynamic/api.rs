#![allow(non_snake_case)]

use libloading::Library;
use std::{
    ffi::{c_char, c_double, c_int, c_void},
    path::PathBuf,
};
#[derive(Debug)]
pub struct GurobiApi {
    _lib: Library,
    // version functions
    pub GRBversion: unsafe extern "C" fn(majorP: *mut c_int, minorP: *mut c_int, techP: *mut c_int),
    // environment functions
    pub GRBloadenv:
        unsafe extern "C" fn(env: *mut *mut c_void, logfilename: *const c_char) -> c_int,
    pub GRBemptyenv: unsafe extern "C" fn(env: *mut *mut c_void) -> c_int,
    pub GRBstartenv: unsafe extern "C" fn(env: *mut c_void) -> c_int,
    pub GRBfreeenv: unsafe extern "C" fn(env: *mut c_void),
    pub GRBgetenv: unsafe extern "C" fn(model: *mut c_void) -> *mut c_void,
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
    pub GRBfreemodel: unsafe extern "C" fn(model: *mut c_void) -> c_int,
    // var functions
    pub GRBaddvar: unsafe extern "C" fn(
        model: *mut c_void,
        numnz: c_int,
        vind: *const c_int,
        vval: *const c_double,
        obj: c_double,
        lb: c_double,
        ub: c_double,
        vtype: c_char,
        varname: *const c_char,
    ) -> c_int,

    pub GRBaddvars: unsafe extern "C" fn(
        model: *mut c_void,
        numvars: c_int,
        numnz: c_int,
        vbeg: *const c_int,
        vind: *const c_int,
        vval: *const c_double,
        obj: *const c_double,
        lb: *const c_double,
        ub: *const c_double,
        vtype: *const c_char,
        varnames: *const *const c_char,
    ) -> c_int,

    // constr functions
    pub GRBaddconstr: unsafe extern "C" fn(
        model: *mut c_void,
        numnz: c_int,
        cind: *const c_int,
        cval: *const c_double,
        sense: c_char,
        rhs: c_double,
        constrname: *const c_char,
    ) -> c_int,

    pub GRBaddconstrs: unsafe extern "C" fn(
        model: *mut c_void,
        numconstrs: c_int,
        numnz: c_int,
        cbeg: *const c_int,
        cind: *const c_int,
        cval: *const c_double,
        sense: *const c_char,
        rhs: *const c_double,
        constrnames: *const *const c_char,
    ) -> c_int,
    // update function
    pub GRBupdatemodel: unsafe extern "C" fn(model: *mut c_void) -> c_int,

    pub GRBgetdblattrelement: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        element: c_int,
        valueP: *mut c_double,
    ) -> c_int,

    pub GRBsetdblattrelement: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        element: c_int,
        newvalue: c_double,
    ) -> c_int,

    pub GRBsetdblattrarray: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        first: c_int,
        len: c_int,
        newvalues: *const c_double,
    ) -> c_int,

    // attr functions
    pub GRBgetintattr: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        valueP: *mut c_int,
    ) -> c_int,

    pub GRBsetintattr:
        unsafe extern "C" fn(model: *mut c_void, attrname: *const c_char, newvalue: c_int) -> c_int,

    pub GRBgetdblattr: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        valueP: *mut c_double,
    ) -> c_int,

    pub GRBsetdblattr: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        newvalue: c_double,
    ) -> c_int,

    pub GRBgetdblattrarray: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        start: c_int,
        len: c_int,
        values: *mut c_double,
    ) -> c_int,

    pub GRBgetstrattr: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        valueP: *mut *mut c_char,
    ) -> c_int,

    pub GRBsetstrattr: unsafe extern "C" fn(
        model: *mut c_void,
        attrname: *const c_char,
        newvalue: *const c_char,
    ) -> c_int,
    // parameter functions
    pub GRBgetdblparam: unsafe extern "C" fn(
        env: *mut c_void,
        paramname: *const c_char,
        valueP: *mut c_double,
    ) -> c_int,

    pub GRBgetintparam: unsafe extern "C" fn(
        env: *mut c_void,
        paramname: *const c_char,
        valueP: *mut c_int,
    ) -> c_int,

    pub GRBgetstrparam: unsafe extern "C" fn(
        env: *mut c_void,
        paramname: *const c_char,
        valueP: *mut *mut c_char,
    ) -> c_int,

    pub GRBsetdblparam: unsafe extern "C" fn(
        env: *mut c_void,
        paramname: *const c_char,
        newvalue: c_double,
    ) -> c_int,

    pub GRBsetintparam:
        unsafe extern "C" fn(env: *mut c_void, paramname: *const c_char, newvalue: c_int) -> c_int,

    pub GRBsetstrparam: unsafe extern "C" fn(
        env: *mut c_void,
        paramname: *const c_char,
        newvalue: *const c_char,
    ) -> c_int,

    pub GRBsetparam: unsafe extern "C" fn(
        env: *mut c_void,
        paramname: *const c_char,
        newvalue: *const c_char,
    ) -> c_int,

    // optimize functions
    pub GRBoptimize: unsafe extern "C" fn(model: *mut c_void) -> c_int,
}

impl GurobiApi {
    pub fn new(lib_path: PathBuf) -> Result<Self, libloading::Error> {
        unsafe {
            let lib = Library::new(lib_path)?;
            Ok(Self {
                GRBversion: *lib.get(b"GRBversion")?,
                GRBstartenv: *lib.get(b"GRBstartenv")?,
                GRBloadenv: *lib.get(b"GRBloadenv")?,
                GRBemptyenv: *lib.get(b"GRBemptyenv")?,
                GRBfreeenv: *lib.get(b"GRBfreeenv")?,
                GRBgetenv: *lib.get(b"GRBgetenv")?,
                GRBnewmodel: *lib.get(b"GRBnewmodel")?,
                GRBfreemodel: *lib.get(b"GRBfreemodel")?,
                GRBaddvar: *lib.get(b"GRBaddvar")?,
                GRBaddvars: *lib.get(b"GRBaddvars")?,
                GRBaddconstr: *lib.get(b"GRBaddconstr")?,
                GRBaddconstrs: *lib.get(b"GRBaddconstrs")?,
                GRBgetintattr: *lib.get(b"GRBgetintattr")?,
                GRBsetintattr: *lib.get(b"GRBsetintattr")?,
                GRBgetdblattr: *lib.get(b"GRBgetdblattr")?,
                GRBsetdblattr: *lib.get(b"GRBsetdblattr")?,
                GRBgetdblattrarray: *lib.get(b"GRBgetdblattrarray")?,
                GRBgetstrattr: *lib.get(b"GRBgetstrattr")?,
                GRBsetstrattr: *lib.get(b"GRBsetstrattr")?,
                GRBoptimize: *lib.get(b"GRBoptimize")?,
                GRBgetdblparam: *lib.get(b"GRBgetdblparam")?,
                GRBgetintparam: *lib.get(b"GRBgetintparam")?,
                GRBgetstrparam: *lib.get(b"GRBgetstrparam")?,
                GRBsetdblparam: *lib.get(b"GRBsetdblparam")?,
                GRBsetintparam: *lib.get(b"GRBsetintparam")?,
                GRBsetstrparam: *lib.get(b"GRBsetstrparam")?,
                GRBsetparam: *lib.get(b"GRBsetparam")?,
                GRBupdatemodel: *lib.get(b"GRBupdatemodel")?,
                GRBgetdblattrelement: *lib.get(b"GRBgetdblattrelement")?,
                GRBsetdblattrelement: *lib.get(b"GRBsetdblattrelement")?,
                GRBsetdblattrarray: *lib.get(b"GRBsetdblattrarray")?,
                _lib: lib,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_gurobi_api() {
        let Some((library, _)) = crate::dynamic::loader::find_library() else {
            return;
        };
        let Ok(_gurobi_api) = GurobiApi::new(library) else {
            return;
        };
    }
    #[test]
    fn test_version_function() {
        let Some((library, _)) = crate::dynamic::loader::find_library() else {
            return;
        };
        let Ok(gurobi_api) = GurobiApi::new(library) else {
            return;
        };
        unsafe {
            let mut major = 0;
            let mut minor = 0;
            let mut tech = 0;
            (gurobi_api.GRBversion)(&mut major, &mut minor, &mut tech);
            println!("Gurobi version: {}.{}.{}", major, minor, tech);
        }
    }
}
