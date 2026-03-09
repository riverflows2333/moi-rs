use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GurobiVersion {
    pub major: u32,
    pub minor: u32,
    pub technical: u32, // often 0 in filename
}

impl ToString for GurobiVersion {
    fn to_string(&self) -> String {
        format!("{}{}", self.major, self.minor)
    }
}

pub fn find_library() -> Option<(PathBuf, String)> {
    // Check environment variable first
    if let Some(path_str) = env::var("GUROBI_HOME").ok() {
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
    let path_prefix = if cfg!(target_os = "windows") {
        // NOTE: Gurobi求解器在windows当中会有一个win64的子目录，这里需要GUROBI_HOME指向到这个子目录
        ""
    } else if cfg!(target_os = "macos") {
        ""
    } else {
        ""
    };
    let lib_dir = base_path.join(path_prefix).join("lib");
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
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.contains("gurobi")
                && file_name.ends_with(suffix)
                && !file_name.contains("_light")
            {
                if let Some(version) = parse_version_from_filename(file_name) {
                    return Some((path, version));
                }
            }
        }
    }
    None
}

fn parse_version_from_filename(filename: &str) -> Option<String> {
    // format like libgurobi120.so or gurobi120.dll
    // extract digits '120'
    let digits: String = filename.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    // Simplistic: Use the digit string as the version identifier (e.g. "120")
    // This matches the module name gen120
    Some(digits)
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
    }
}
