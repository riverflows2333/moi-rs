# moirspy

`moirspy` is a solver-independent Python optimization-modeling API implemented
in Rust. It currently supports linear and mixed-integer linear models (LP/MILP)
through variables, affine expressions, constraints, objectives, parameters, and
pluggable solver backends.

> The project is under active development and its API is not yet stable.

## Installation

Install the modeling package and the optional Gurobi backend:

```bash
python -m pip install moirspy moirspy-gurobi
python -m pip install moirspy
```

Solver backends require the corresponding native solver installation and
license. COPT discovery uses `COPT_HOME` in the same way that Gurobi discovery
uses `GUROBI_HOME`.

> **Important:** `GUROBI_HOME` must be set to the Gurobi installation directory
> before calling `setBackend("gurobi")`.

Set it in the shell:

```bash
export GUROBI_HOME=/opt/gurobi1203
```

Or set it at the beginning of the Python program, before creating the model or
attaching the backend:

```python
import os

os.environ["GUROBI_HOME"] = "/opt/gurobi1203"
```

If the environment variable cannot be set, instantiate the low-level
`moirspy_gurobi.Model` backend with the full native-library path in `dll_path`.

## Example

```python
from moirspy import MOI, Model, dot, quicksum

model = Model("binary-example")
x = model.addVars(3, name="x", vtype=MOI.BINARY)

model.addConstr(x[0] + 2 * x[1] + 3 * x[2] <= 4, name="capacity")
model.addConstr(x[0] + x[1] >= 1, name="selection")
model.setObjective(quicksum([x[0], x[1], 2 * x[2]]), MOI.MAXIMIZE)
# For large coefficient/variable vectors, dot(coefficients, variables) builds
# the same affine form directly with one normalization pass.
weighted = dot([1.0, 2.0, 3.0], [x[0], x[1], x[2]])
model.setParam("OutputFlag", 0)

model.setBackend("gurobi")
model.optimize()

print("objective:", model.ObjVal)
print("x:", [x[i].X for i in range(3)])
```

The COPT backend is built into `moirspy` and calls the native Rust optimizer
without importing `moirspy_copt`. The separate package remains available as a
low-level compatibility API. Gurobi still uses its backend package in this
release. The bridge synchronizes existing model data when a backend is attached.

## Main API

- `Model.addVar(...)` creates one variable.
- `Model.addVars(*shape, ...)` creates row-major indexed variables. Scalar
  bounds, types, and names are broadcast; lists provide per-variable values.
- `Model.addConstr(...)` and `Model.addConstrs(...)` add linear comparisons.
- `quicksum(...)` sums any iterable of variables, linear expressions, or numbers.
- `dot(coefficients, variables)` constructs a weighted affine expression directly.
- `Model.setObjective(expr, MOI.MINIMIZE | MOI.MAXIMIZE)` sets the objective.
- `Model.setParam(name, value)` stores a solver parameter.
- `Model.setBackend("gurobi", env=None)` or `Model.setBackend("copt", env=None)`
  attaches the installed backend and optionally forwards an explicit solver
  environment.
- `Model(name, backend="copt", env=None)` creates a native Direct model. Its
  variables, constraints, objective, and parameters go straight to COPT without
  retaining a replayable model cache. Use the default constructor followed by
  `setBackend` when late solver selection or replay is required.
- `CoptEnv(...)` and `CoptEnvConfig(...)` configure the built-in native COPT
  backend, including explicit library, license-directory, client, and OEM setup.
- `Model.optimize()` solves the model; use `Model.ObjVal` and `Var.X` for results.

Available variable types are `MOI.CONTINUOUS`, `MOI.BINARY`, and `MOI.INTEGER`.
Quadratic and nonlinear products are not supported yet. The `obj` argument on
variable creation is currently reserved; define the objective with
`setObjective`.

Full documentation and Rust source are available in the
[`moi-rs` repository](https://github.com/riverflows2333/moi-rs).
