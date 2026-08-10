#![allow(non_snake_case)]

use libloading::Library;
use std::error::Error;
use std::ffi::{c_char, c_int, c_void};
use std::fmt;
use std::path::{Path, PathBuf};

pub type CoptGetBanner = unsafe extern "C" fn(*mut c_char, c_int) -> c_int;
pub type CoptGetRetcodeMsg = unsafe extern "C" fn(c_int, *mut c_char, c_int) -> c_int;
pub type CoptCreateEnv = unsafe extern "C" fn(*mut *mut c_void) -> c_int;
pub type CoptCreateEnvWithPath = unsafe extern "C" fn(*const c_char, *mut *mut c_void) -> c_int;
pub type CoptDeleteEnv = unsafe extern "C" fn(*mut *mut c_void) -> c_int;
pub type CoptGetLicenseMsg = unsafe extern "C" fn(*mut c_void, *mut c_char, c_int) -> c_int;
pub type CoptCreateProb = unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> c_int;
pub type CoptDeleteProb = unsafe extern "C" fn(*mut *mut c_void) -> c_int;
pub type CoptUpdate = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type CoptAddCols = unsafe extern "C" fn(
    *mut c_void,
    c_int,
    *const f64,
    *const c_int,
    *const c_int,
    *const c_int,
    *const f64,
    *const c_char,
    *const f64,
    *const f64,
    *const *const c_char,
) -> c_int;
pub type CoptAddRows = unsafe extern "C" fn(
    *mut c_void,
    c_int,
    *const c_int,
    *const c_int,
    *const c_int,
    *const f64,
    *const c_char,
    *const f64,
    *const f64,
    *const *const c_char,
) -> c_int;
pub type CoptSetColObj =
    unsafe extern "C" fn(*mut c_void, c_int, *const c_int, *const f64) -> c_int;
pub type CoptSetObjConst = unsafe extern "C" fn(*mut c_void, f64) -> c_int;
pub type CoptSetObjSense = unsafe extern "C" fn(*mut c_void, c_int) -> c_int;
pub type CoptSetIntParam = unsafe extern "C" fn(*mut c_void, *const c_char, c_int) -> c_int;
pub type CoptSetDblParam = unsafe extern "C" fn(*mut c_void, *const c_char, f64) -> c_int;
pub type CoptGetIntAttr = unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_int) -> c_int;
pub type CoptGetDblAttr = unsafe extern "C" fn(*mut c_void, *const c_char, *mut f64) -> c_int;
pub type CoptSolve = unsafe extern "C" fn(*mut c_void) -> c_int;
pub type CoptGetSolution = unsafe extern "C" fn(*mut c_void, *mut f64) -> c_int;
pub type CoptGetLpSolution =
    unsafe extern "C" fn(*mut c_void, *mut f64, *mut f64, *mut f64, *mut f64) -> c_int;

/// Error returned while opening the native library or resolving a C API symbol.
#[derive(Debug)]
pub struct CoptApiLoadError {
    path: PathBuf,
    symbol: Option<&'static str>,
    source: libloading::Error,
}

impl CoptApiLoadError {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn symbol(&self) -> Option<&'static str> {
        self.symbol
    }
}

impl fmt::Display for CoptApiLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(symbol) = self.symbol {
            write!(
                formatter,
                "failed to load COPT symbol {symbol} from {}: {}",
                self.path.display(),
                self.source
            )
        } else {
            write!(
                formatter,
                "failed to open COPT library {}: {}",
                self.path.display(),
                self.source
            )
        }
    }
}

impl Error for CoptApiLoadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

/// Loaded function table for the COPT 8.0 C API used by the MILP backend.
///
/// Keeping the `Library` alive in this structure guarantees that every copied
/// function pointer remains valid for the lifetime of the table.
#[derive(Debug)]
pub struct CoptApi {
    _library: Library,

    pub COPT_GetBanner: CoptGetBanner,
    pub COPT_GetRetcodeMsg: CoptGetRetcodeMsg,
    pub COPT_CreateEnv: CoptCreateEnv,
    pub COPT_CreateEnvWithPath: CoptCreateEnvWithPath,
    pub COPT_DeleteEnv: CoptDeleteEnv,
    pub COPT_GetLicenseMsg: CoptGetLicenseMsg,
    pub COPT_CreateProb: CoptCreateProb,
    pub COPT_DeleteProb: CoptDeleteProb,
    pub COPT_Update: CoptUpdate,
    pub COPT_AddCols: CoptAddCols,
    pub COPT_AddRows: CoptAddRows,
    pub COPT_SetColObj: CoptSetColObj,
    pub COPT_SetObjConst: CoptSetObjConst,
    pub COPT_SetObjSense: CoptSetObjSense,
    pub COPT_SetIntParam: CoptSetIntParam,
    pub COPT_SetDblParam: CoptSetDblParam,
    pub COPT_GetIntAttr: CoptGetIntAttr,
    pub COPT_GetDblAttr: CoptGetDblAttr,
    pub COPT_Solve: CoptSolve,
    pub COPT_GetSolution: CoptGetSolution,
    pub COPT_GetLpSolution: CoptGetLpSolution,
}

impl CoptApi {
    /// Open a COPT native library and resolve the minimum MILP function table.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, CoptApiLoadError> {
        let path = path.into();
        // SAFETY: opening a user-selected native library is inherently unsafe.
        // The exact COPT filename is validated by the loader and every symbol is
        // resolved with its signature from COPT 8.0.6 `copt.h`.
        let library = unsafe { Library::new(&path) }.map_err(|source| CoptApiLoadError {
            path: path.clone(),
            symbol: None,
            source,
        })?;

        macro_rules! load {
            ($name:literal, $kind:ty) => {{
                // SAFETY: the requested symbol name and type were copied from
                // COPT 8.0.6 `copt.h`. The function pointer is `Copy`, while
                // `library` is retained by the resulting `CoptApi`.
                unsafe { library.get::<$kind>(concat!($name, "\0").as_bytes()) }
                    .map(|symbol| *symbol)
                    .map_err(|source| CoptApiLoadError {
                        path: path.clone(),
                        symbol: Some($name),
                        source,
                    })?
            }};
        }

        Ok(Self {
            COPT_GetBanner: load!("COPT_GetBanner", CoptGetBanner),
            COPT_GetRetcodeMsg: load!("COPT_GetRetcodeMsg", CoptGetRetcodeMsg),
            COPT_CreateEnv: load!("COPT_CreateEnv", CoptCreateEnv),
            COPT_CreateEnvWithPath: load!("COPT_CreateEnvWithPath", CoptCreateEnvWithPath),
            COPT_DeleteEnv: load!("COPT_DeleteEnv", CoptDeleteEnv),
            COPT_GetLicenseMsg: load!("COPT_GetLicenseMsg", CoptGetLicenseMsg),
            COPT_CreateProb: load!("COPT_CreateProb", CoptCreateProb),
            COPT_DeleteProb: load!("COPT_DeleteProb", CoptDeleteProb),
            COPT_Update: load!("COPT_Update", CoptUpdate),
            COPT_AddCols: load!("COPT_AddCols", CoptAddCols),
            COPT_AddRows: load!("COPT_AddRows", CoptAddRows),
            COPT_SetColObj: load!("COPT_SetColObj", CoptSetColObj),
            COPT_SetObjConst: load!("COPT_SetObjConst", CoptSetObjConst),
            COPT_SetObjSense: load!("COPT_SetObjSense", CoptSetObjSense),
            COPT_SetIntParam: load!("COPT_SetIntParam", CoptSetIntParam),
            COPT_SetDblParam: load!("COPT_SetDblParam", CoptSetDblParam),
            COPT_GetIntAttr: load!("COPT_GetIntAttr", CoptGetIntAttr),
            COPT_GetDblAttr: load!("COPT_GetDblAttr", CoptGetDblAttr),
            COPT_Solve: load!("COPT_Solve", CoptSolve),
            COPT_GetSolution: load!("COPT_GetSolution", CoptGetSolution),
            COPT_GetLpSolution: load!("COPT_GetLpSolution", CoptGetLpSolution),
            _library: library,
        })
    }
}
