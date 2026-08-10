use std::path::PathBuf;

/// Resolve an explicit path first, otherwise defer to `COPT_HOME`.
pub fn resolve_library(dll_path: Option<String>) -> Option<PathBuf> {
    dll_path
        .and_then(moi_solver_copt::find_library_from)
        .or_else(moi_solver_copt::find_library)
}
