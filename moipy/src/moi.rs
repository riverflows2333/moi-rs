// 建模相关常量存储

use pyo3::prelude::*;

#[pyclass(eq,eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VarType {
    CONTINUOUS = 0,
    BINARY = 1,
    INTEGER = 2,
}

#[pyclass(eq,eq_int)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Sense {
    MINIMIZE = 0,
    MAXIMIZE = 1,
}

pub fn register_moi( m: &Bound<'_,PyModule>) -> PyResult<()> {
    m.add("CONTINUOUS", VarType::CONTINUOUS)?;
    m.add("BINARY", VarType::BINARY)?;
    m.add("INTEGER", VarType::INTEGER)?;
    m.add("MINIMIZE", Sense::MINIMIZE)?;
    m.add("MAXIMIZE", Sense::MAXIMIZE)?;
    Ok(())
}

#[pymodule(submodule,name = "MOI")]
pub mod pymoi {
    use crate::moi::register_moi;
    use pyo3::prelude::*;
    #[pymodule_init]
    pub fn register(m: &Bound<'_, PyModule>) -> PyResult<()> {
        register_moi(m)?;
        Ok(())
    }
}