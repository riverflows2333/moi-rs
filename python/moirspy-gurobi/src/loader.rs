use moi_solver_gurobi::{dynamic::find_library_from, *};
use pyo3::prelude::*;

// gurobi动态库文件类型，优先级从上到下
#[derive(Clone, Debug)]
pub enum EnvLoader {
    // python环境中是否安装gurobipy
    PyEnv(bool),
    // GUROBI_HOME环境变量
    EnvVar(String),
    // 系统安装Gurobi求解器路径
    LibPath(String),
    None,
}

pub fn load_gurobi(gurobi_path: Option<String>) -> Result<EnvLoader, String> {
    // 1. 尝试从python环境中加载gurobipy
    // if let Ok(_) = pyo3::Python::attach(|py| py.import("gurobipy").map(|_| ())) {
    //     return Ok(EnvLoader::PyEnv(true));
    // }
    // 2. 尝试从GUROBI_HOME环境变量加载
    if let Ok(gurobi_home) = std::env::var("GUROBI_HOME") {
        return Ok(EnvLoader::EnvVar(gurobi_home));
    }
    // 3. 从指定路径加载
    if let Some(path) = gurobi_path {
        if std::path::Path::new(&path).exists() {
            return Ok(EnvLoader::LibPath(path));
        }
    }
    // 4. 从其他路径加载
    let possible_paths = vec![
        "/usr/local/gurobi1203/lib/libgurobi1203.so",
        "/opt/gurobi1203/lib/libgurobi1203.so",
    ];
    for path in possible_paths {
        if std::path::Path::new(path).exists() {
            return Ok(EnvLoader::LibPath(path.to_string()));
        }
    }
    Err("Failed to load Gurobi library from any source".to_string())
}

// 将EnvLoader转换为DLL路径，按照系统类型确定动态库后缀
pub fn loader_to_dll_path(loader: &EnvLoader) -> Result<String, String> {
    match loader {
        // gurobi 12版本之后，gurobipy当中的动态库被拆分为几部分，难以直接调用
        EnvLoader::PyEnv(_) => Err("Cannot determine Gurobi library path from Python environment".to_string()),
        // 基于环境变量读取库文件路径
        EnvLoader::EnvVar(gurobi_home) => {
            let path = find_library_from(&gurobi_home);
            if let Some(path) = path {
                Ok(path.to_str().unwrap().to_string())
            } else {
                Err(format!(
                    "Gurobi library not found at expected path: {}",
                    gurobi_home
                ))
            }
        }
        EnvLoader::LibPath(path) => Ok(path.clone()),
        EnvLoader::None => Err("No valid Gurobi library source found".to_string()),
    }
}
