# moi-rs Roadmap

This document tracks design and implementation work that remains after the
initial LP/MILP correctness and backend-boundary refactor.

## Public API correctness

- Replace result getters that currently return only `Option<T>` with an API
  that distinguishes unavailable results from backend query failures.
- Replace character variable types with a solver-independent `VariableType`
  enum while preserving a stable Python protocol.
- Define stable frontend/backend index mappings before adding variable or
  constraint deletion, model rebuilds, or native reordering.
- Decide atomicity and recovery semantics for native batch APIs that can fail
  after partially changing solver state.

## Python API

- Convert unsupported Python expression operations from Rust `panic!` paths to
  `PyTypeError` or another appropriate Python exception.
- Replace poisoned-lock `unwrap()` calls at PyO3 boundaries with Python errors.
- Centralize low-level Python backend method names, argument ordering, and
  status encoding so future solver packages do not duplicate the protocol.
- Decide whether a single `Var` should be accepted directly by
  `Model.setObjective`, in addition to `LinExpr`.

## Solver backends

- Implement the initial COPT LP/MILP backend described in
  [`COPT_BACKEND_PLAN.md`](COPT_BACKEND_PLAN.md).
- Improve native error reporting with solver-provided messages while retaining
  structured solver name, operation, and error code information.
- Add conflict/IIS support as an explicit capability instead of returning a
  generic unsupported error.
- Add solver-independent tests for time/node limits with and without incumbent
  solutions.

## Modeling scope

- Design common representations and capabilities for quadratic objectives,
  quadratic constraints, conic constraints, and nonlinear expressions.
- Add MIP starts, callbacks, lazy constraints, and solution pools only after
  their solver-independent semantics are defined.
- Define model copy, clear, serialization, and read/write behavior.

## Quality and release engineering

- Add CI jobs for formatting, workspace Clippy, solver-independent tests, and
  optional native-solver integration tests.
- Test native-library discovery on Windows, Linux, and macOS.
- Document API stability and version compatibility before publishing stable
  package releases.
