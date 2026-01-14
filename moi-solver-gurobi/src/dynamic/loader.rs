use std::fs;
use std::path::{Path, PathBuf};

pub fn find_library() -> Option<PathBuf> {
    // 首先检查环境变量
    // if let Some(path) = check_env_var() {
    //     return Some();
    // }
    None
}
//
pub fn find_library_from(path: String) -> Option<PathBuf> {
    let lib_path = format!("{}/lib", path);
    let suffix = if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };
    // 查找路径下的libgurobi动态库文件
    let entries = fs::read_dir(&lib_path).unwrap();
    for entry in entries {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_str().unwrap();
        if file_name.contains("gurobi") && file_name.ends_with(suffix) {
            return Some(path);
        }
    }
    None
}

fn check_env_var() -> Option<PathBuf> {
    std::env::var("GUROBI_HOME").ok().map(|p| PathBuf::from(p))
}

fn get_library_name(version: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("gurobi{}.dll", version)
    } else if cfg!(target_os = "macos") {
        format!("libgurobi{}.dylib", version)
    } else {
        format!("libgurobi{}.so", version)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_find_library_from_path() {
        let lib_path = find_library_from("/usr/local/gurobi1203".to_string());
        println!("{:?}", lib_path);
    }
}
