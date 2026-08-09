use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GurobiVersion {
    pub major: u32,
    pub minor: u32,
    pub technical: u32, // often 0 in filename
}

impl fmt::Display for GurobiVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}{}", self.major, self.minor)
    }
}

pub fn find_library() -> Option<(PathBuf, String)> {
    // Check environment variable first
    if let Ok(path_str) = env::var("GUROBI_HOME") {
        let path = PathBuf::from(path_str);
        if let Some((lib_path, version)) = find_library_in_path(&path) {
            return Some((lib_path, version));
        }
    }

    // Try standard system paths if not found in env
    #[cfg(target_os = "linux")]
    {
        for path in &["/opt", "/usr/local", "/usr/local/gurobi1203"] {
            if let Some((lib_path, version)) = find_library_in_path(&PathBuf::from(path)) {
                return Some((lib_path, version));
            }
        }
    }

    // Additional heuristics can be added here
    None
}

/// Helper to ensure running against correct binding version.
/// This doesn't change imports, but can be used for runtime validation.
/// Expected format: "120"
pub fn vers_match(detected: &str, expected: &str) -> bool {
    detected == expected
}

pub fn find_library_from(path: &String) -> Option<PathBuf> {
    find_library_in_path(&PathBuf::from(path)).map(|(p, _)| p)
}

fn find_library_in_path(base_path: &Path) -> Option<(PathBuf, String)> {
    let lib_dir = if cfg!(target_os = "windows") {
        // GUROBI_HOME should point to the win64 directory on Windows.
        base_path.join("bin")
    } else {
        base_path.join("lib")
    };
    if !lib_dir.exists() {
        return None;
    }

    let suffix = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    let entries = fs::read_dir(&lib_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str())
            && file_name.to_ascii_lowercase().ends_with(suffix)
            && let Some(version) = parse_version_from_filename(file_name)
        {
            return Some((path, version));
        }
    }
    None
}

fn parse_version_from_filename(filename: &str) -> Option<String> {
    let filename = filename.to_ascii_lowercase();
    let stem = filename
        .strip_suffix(".dll")
        .or_else(|| filename.strip_suffix(".so"))
        .or_else(|| filename.strip_suffix(".dylib"))?;
    let version = stem
        .strip_prefix("libgurobi")
        .or_else(|| stem.strip_prefix("gurobi"))?;
    (!version.is_empty() && version.chars().all(|c| c.is_ascii_digit()))
        .then(|| version.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_find_library_with_version() {
        if let Some((path, ver)) = find_library() {
            println!("Found Gurobi: {:?} version {}", path, ver);
        } else {
            println!("Gurobi not found in env");
        }
    }

    #[test]
    fn test_parse_version() {
        assert_eq!(
            parse_version_from_filename("libgurobi120.so"),
            Some("120".to_string())
        );
        assert_eq!(
            parse_version_from_filename("gurobi120.dll"),
            Some("120".to_string())
        );
        assert_eq!(
            parse_version_from_filename("libgurobi90.dylib"),
            Some("90".to_string())
        );
        assert_eq!(parse_version_from_filename("Gurobi120.NET.dll"), None);
        assert_eq!(parse_version_from_filename("gurobi120_light.dll"), None);
    }
}
