use moi_solver_copt::CoptLibraryNotFound;
use std::path::{Path, PathBuf};

/// Resolve an explicit COPT library/root, otherwise use normal discovery.
pub fn resolve_library(dll_path: Option<String>) -> Result<PathBuf, CoptLibraryNotFound> {
    moi_solver_copt::resolve_library(dll_path.as_deref().map(Path::new))
}
