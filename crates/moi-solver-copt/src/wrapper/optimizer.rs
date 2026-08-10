use crate::SharedCoptEnv;

/// MOI optimizer backed by a native COPT problem.
///
/// The `ModelLike` and `Optimizer` implementations are added after the raw C
/// API bindings and function table are available.
pub struct CoptOptimizer {
    _env: SharedCoptEnv,
}
