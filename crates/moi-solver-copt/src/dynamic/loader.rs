use std::env;
use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

/// Return the exact native COPT C library name for the target platform.
pub const fn library_file_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "copt.dll"
    } else if cfg!(target_os = "macos") {
        "libcopt.dylib"
    } else {
        "libcopt.so"
    }
}

/// Error returned when no native COPT C library can be found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoptLibraryNotFound {
    checked_paths: Vec<PathBuf>,
}

impl CoptLibraryNotFound {
    pub fn checked_paths(&self) -> &[PathBuf] {
        &self.checked_paths
    }
}

impl fmt::Display for CoptLibraryNotFound {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "COPT native library was not found")?;
        if !self.checked_paths.is_empty() {
            write!(formatter, "; checked")?;
            for path in &self.checked_paths {
                write!(formatter, " {}", path.display())?;
            }
        }
        Ok(())
    }
}

impl Error for CoptLibraryNotFound {}

/// Resolve the COPT library using `COPT_HOME` and official installation roots.
pub fn find_library() -> Option<PathBuf> {
    resolve_library(None).ok()
}

/// Resolve an exact library file or a COPT installation root.
pub fn find_library_from(path: impl AsRef<Path>) -> Option<PathBuf> {
    let candidate = library_candidate(path.as_ref());
    is_exact_c_library(&candidate).then_some(candidate)
}

/// Resolve an explicit path, or search `COPT_HOME` followed by default roots.
///
/// An explicit path is authoritative: an invalid explicit path produces an
/// error instead of silently loading a different COPT installation.
pub fn resolve_library(explicit: Option<&Path>) -> Result<PathBuf, CoptLibraryNotFound> {
    let candidates = if let Some(path) = explicit {
        vec![library_candidate(path)]
    } else {
        search_candidates()
    };

    candidates
        .iter()
        .find(|path| is_exact_c_library(path))
        .cloned()
        .ok_or(CoptLibraryNotFound {
            checked_paths: candidates,
        })
}

fn library_candidate(path: &Path) -> PathBuf {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(library_file_name()))
    {
        path.to_path_buf()
    } else if cfg!(target_os = "windows") {
        path.join("bin").join(library_file_name())
    } else {
        path.join("lib").join(library_file_name())
    }
}

fn is_exact_c_library(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(library_file_name()))
}

fn search_candidates() -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(home) = env::var_os("COPT_HOME") {
        candidates.push(library_candidate(Path::new(&home)));
    }

    #[cfg(target_os = "windows")]
    candidates.push(library_candidate(Path::new(r"C:\Program Files\copt80")));

    #[cfg(not(target_os = "windows"))]
    candidates.push(library_candidate(Path::new("/opt/copt80")));

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_library_name_is_exact() {
        let name = library_file_name();
        assert!(matches!(name, "copt.dll" | "libcopt.so" | "libcopt.dylib"));
        assert!(!name.contains("copt_cpp"));
    }

    #[test]
    fn configured_home_contains_native_library() {
        if env::var_os("COPT_HOME").is_some() {
            assert!(find_library().is_some());
        }
    }

    #[test]
    fn missing_explicit_path_reports_checked_candidate() {
        let root = env::temp_dir().join("moi-rs-copt-path-that-does-not-exist");
        let error = resolve_library(Some(&root)).unwrap_err();
        assert_eq!(error.checked_paths(), [library_candidate(&root)]);
        assert!(error.to_string().contains(library_file_name()));
    }

    #[test]
    fn copt_cpp_is_not_accepted_as_the_c_library() {
        let path = env::temp_dir().join(if cfg!(target_os = "windows") {
            "copt_cpp.dll"
        } else if cfg!(target_os = "macos") {
            "libcopt_cpp.dylib"
        } else {
            "libcopt_cpp.so"
        });
        assert!(find_library_from(path).is_none());
    }
}
