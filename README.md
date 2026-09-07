# moi-rs

`moi-rs` is a Rust optimization-modeling workspace inspired by
[MathOptInterface](https://jump.dev/MathOptInterface.jl/stable/). It provides a
solver-independent modeling layer, a bridge optimizer, and Python bindings with
Gurobi and COPT backends.

The Python interface follows familiar mathematical-programming APIs: create
variables, compose linear expressions with Python operators, add constraints,
select a backend, and optimize.

> The project is under active development. The current Python API supports
> linear programs and mixed-integer linear programs (LP/MILP). API stability is
> not yet guaranteed.

## Python packages

| Package | Import name | Purpose |
| --- | --- | --- |
| [`moirspy`](https://pypi.org/project/moirspy/) | `moirspy` | Modeling API with built-in native COPT backend |
| [`moirspy-gurobi`](https://pypi.org/project/moirspy-gurobi/) | `moirspy_gurobi` | Gurobi backend loaded through its native library |
| `moirspy-copt` | `moirspy_copt` | Optional low-level/compatibility COPT API |

Both packages require Python 3.8 or newer. To model and solve with Gurobi,
install both packages:

```bash
python -m pip install moirspy moirspy-gurobi
```

`moirspy-gurobi` does not bundle Gurobi or a license. Install Gurobi separately
and make sure its native library and license are available.

The high-level COPT backend is installed with `python -m pip install moirspy`
and discovers COPT through `COPT_HOME`. `moirspy-copt` remains optional for its
low-level API. Neither backend bundles its solver or license.

> **Important:** `GUROBI_HOME` must be set to the Gurobi installation directory
> before creating the Gurobi backend. Without it, `setBackend("gurobi")` may be
> unable to locate the native Gurobi library.

Set it in the shell before starting Python:

```bash
export GUROBI_HOME=/opt/gurobi1203
```

Alternatively, set it at the very beginning of the Python program, before
creating the model or attaching the backend:

```python
import os

os.environ["GUROBI_HOME"] = "/opt/gurobi1203"
```

If `GUROBI_HOME` cannot be set, use the low-level `moirspy_gurobi.Model`
constructor and pass the full native-library path through `dll_path`.

## Quick start

The following binary program maximizes `x[0] + x[1] + 2 x[2]`:

```python
from moirspy import MOI, Model

model = Model("binary-example")
x = model.addVars(3, name="x", vtype=MOI.BINARY)

model.addConstr(x[0] + 2 * x[1] + 3 * x[2] <= 4, name="capacity")
model.addConstr(x[0] + x[1] >= 1, name="selection")
model.setObjective(x[0] + x[1] + 2 * x[2], MOI.MAXIMIZE)

# Parameters are retained and forwarded when the backend is attached.
model.setParam("OutputFlag", 0)
model.setBackend("gurobi")
model.optimize()

print("objective:", model.ObjVal)
print("x:", [x[i].X for i in range(3)])
```

`setBackend("gurobi")` dynamically imports `moirspy_gurobi`. Existing model
data is synchronized when the backend is attached, and subsequent variables,
constraints, objectives, and parameters are forwarded incrementally. A successful
attach releases replay-only data by default; use
`setBackend("gurobi", keep_cache=True)` only when the model must later be replayed
into a different backend.

For COPT models whose solver is known up front, construct a native Direct model:

```python
from moirspy import CoptEnv, Model

env = CoptEnv()
model = Model("direct-copt", backend="copt", env=env)
```

This path sends model operations directly to the Rust COPT optimizer and does
not retain the replayable variable/constraint cache. The default
`Model(name)` plus `setBackend(...)` workflow remains available when the model
must be built before choosing a solver.

## Modeling API

### Variables

Create one variable with `addVar`, or an indexed collection with `addVars`:

```python
y = model.addVar(lb=0, ub=10, vtype=MOI.CONTINUOUS, name="y")

x = model.addVars(
    2,
    3,
    lb=0,
    ub=[10, 10, 10, 20, 20, 20],
    vtype=MOI.INTEGER,
    name="x",
)

first = x[0, 0]
last = x[1, 2]
```

Scalar values passed to `addVars` are broadcast over the requested shape. List
values are consumed in row-major order and should contain one item per variable.
A scalar base name such as `x` produces names such as `x[0,0]` and `x[1,2]`.

Available variable types are `MOI.CONTINUOUS`, `MOI.BINARY`, and `MOI.INTEGER`.
The `obj` arguments on `addVar` and `addVars` are currently reserved; use
`setObjective` to define the objective.

### Linear expressions and constraints

`Var` and `LinExpr` support addition, subtraction, scalar multiplication, and
comparisons with numbers, variables, and other linear expressions:

```python
expr = 2 * x[0, 0] - x[1, 2] + 4

model.addConstr(expr <= 10, name="limit")
model.addConstr(x[0, 0] == x[1, 2], name="balance")
model.addConstrs((x[i, j] >= 0 for i in range(2) for j in range(3)), name="lb")
```

Use `quicksum` for any iterable of variables, linear expressions, and numbers:

```python
from moirspy import quicksum

total = quicksum(x[i, j] for i in range(2) for j in range(3))
model.setObjective(total, MOI.MINIMIZE)
```

Quadratic and nonlinear products such as `x[0, 0] * x[1, 2]` are not supported
yet.

### Parameters and solution values

`setParam(name, value)` accepts a Boolean, integer, float, or string. With the
Gurobi backend, raw names such as `OutputFlag` and `MIPGap` are forwarded to
Gurobi. With COPT, use names such as `Logging`, `Threads`, and `TimeLimit` with
Boolean, integer, or floating-point values.

An explicit Gurobi environment can be imported from the solver package and
passed through the generic backend boundary:

```python
from moirspy_gurobi import Env

env = Env()
env.setParam("OutputFlag", 0)
model.setBackend("gurobi", env=env)
```

After `optimize()`, `model.ObjVal` is the objective value and `variable.X` is the
variable value. Either property returns `None` if its result is unavailable.

## Low-level Gurobi API

Most users should work through `moirspy.Model`. The backend package also exposes
a low-level API for integrations and backend development:

```python
from moirspy_gurobi import Model

backend = Model(name="direct", dll_path=None)
x = backend.add_variable(name="x", vtype="C", lb=0.0, ub=float("inf"))
backend.set_objective([x], [1.0], 0.0, 0)  # 1=maximize; other values=minimize
backend.update()
status = backend.optimize()
print(status, backend.get_var_value(x))
```

At this level, variables and constraints are represented by integer IDs and
linear functions by parallel variable/coefficient arrays. Constraint senses are
`"<"`, `">"`, or `"="`.

## Rust workspace

The main crates are organized by layer:

- `moi-core`: indices, sets, functions, constraints, variables, and attributes
- `moi-solver-api`: model and optimizer traits, statuses, and shared utilities
- `moi-model-dummy`: in-memory model used for tests and examples
- `moi-bridge`: solver-independent model storage and backend synchronization
- `moi-solver-gurobi`: dynamic Gurobi loading and native solver wrapper
- `moi-solver-copt`: dynamic COPT loading and native solver wrapper
- `moirspy` / `moirspy-gurobi` / `moirspy-copt`: PyO3 extension modules

Build and test the Rust workspace:

```bash
cargo build --workspace
cargo test --workspace
```

Build either Python extension from its package directory with
[`maturin`](https://www.maturin.rs/):

```bash
python -m pip install maturin

cd python/moirspy
maturin develop

cd ../moirspy-gurobi
maturin develop

cd ../moirspy-copt
maturin develop
```

Native backend tests require the corresponding solver installation and license;
COPT can also start in its documented size-limited mode when no license is
available.

## License

MIT
