#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SolveStatus {
    Unknown,
    Optimal,
    Infeasible,
    Unbounded,
    Error,
}
