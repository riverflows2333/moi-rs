use std::env;
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

/// Resolve the COPT library from `COPT_HOME`.
pub fn find_library() -> Option<PathBuf> {
    env::var_os("COPT_HOME").and_then(find_library_from)
}

/// Resolve an exact library file or a COPT installation root.
pub fn find_library_from(path: impl AsRef<Path>) -> Option<PathBuf> {
    let path = path.as_ref();
    if path.is_file() {
        return path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(library_file_name()))
            .then(|| path.to_path_buf());
    }

    let library = if cfg!(target_os = "windows") {
        path.join("bin").join(library_file_name())
    } else {
        path.join("lib").join(library_file_name())
    };
    library.is_file().then_some(library)
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
}
