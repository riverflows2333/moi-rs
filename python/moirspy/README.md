# moirspy

`moirspy` is a solver-independent Python optimization-modeling API implemented
in Rust. It currently supports linear and mixed-integer linear models (LP/MILP)
through variables, affine expressions, constraints, objectives, parameters, and
built-in native COPT and Gurobi backends.

> The project is under active development and its API is not yet stable.

## Installation

Install the modeling package; it contains both native high-level backends:

```bash
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

If the environment variable cannot be set, instantiate `moirspy.GurobiEnv` with
the full native-library path in `dll_path`.

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

The COPT and Gurobi backends are built into `moirspy` and call their native Rust
optimizers without importing solver packages. Import models and explicit solver
environments directly from `moirspy`. The bridge synchronizes existing model
data when a backend is attached.

## Backend execution modes

| Mode | Construction | Model storage | Python backend transport |
| --- | --- | --- | --- |
| COPT Direct | `Model(name, backend="copt")` | Minimal counts and native state | None |
| COPT Cached attach | `Model(name)` then `setBackend("copt")` | Released after attach unless `keep_cache=True` | None |
| Gurobi Direct/Cached | `backend="gurobi"` or `setBackend("gurobi")` | Same Direct/Cached policy | None |

## Main API

- `Model.addVar(...)` creates one variable.
- `Var.lb` and `Var.ub` read or change one variable's bounds. COPT/Gurobi
  Direct models use native getters/setters; Cached models also support edits
  before backend attach and replay them. Bounds cannot contain NaN or cross
  each other. A successful edit invalidates previous solution values, while
  reading bounds preserves them.
- `Model.addVars(*shape, ...)` creates row-major indexed variables. Scalar
  bounds, types, and names are broadcast; lists provide per-variable values.
- `Model.addConstr(...)` and `Model.addConstrs(...)` add linear comparisons.
- `quicksum(...)` sums any iterable of variables, linear expressions, or numbers.
- `Vars.sum()` sums every variable; `Vars.sum(i, "*")` and
  `Vars.sum("*", j)` sum matching integer index slices. `"*"` matches any
  index in one dimension; it is not a regular expression. No match yields a
  zero `LinExpr`. `Vars.sum(dim=0)` reduces that dimension and returns a NumPy
  object array of `LinExpr` with the remaining shape (a single `LinExpr` for
  one-dimensional Vars). The default `dim=None` sums all variables; patterns
  cannot be combined with `dim`.
- `dot(coefficients, variables)` constructs a weighted affine expression directly.
- `Var / number` and `LinExpr / number` return linear expressions with divided
  coefficients and constants. The denominator must be finite and nonzero;
  division by a variable or expression is unsupported.
- `Model.setObjective(expr, MOI.MINIMIZE | MOI.MAXIMIZE)` sets the objective.
- `Model.setParam(name, value)` stores a solver parameter.
- `Model.setBackend("gurobi", env=None, keep_cache=False)` or the COPT equivalent
  attaches the installed backend and optionally forwards an explicit solver
  environment. By default, replay-only model data is released after a successful
  attach. Pass `keep_cache=True` when the model must later be copied to another
  backend.
- `Model(name, backend="copt" | "gurobi", env=None)` creates a native Direct
  model. Its variables, constraints, objective, and parameters go straight to
  the selected solver without retaining a replayable model cache. Use the default constructor followed by
  `setBackend` when late solver selection is required; opt into `keep_cache=True`
  when repeated backend switching is required.
- `CoptEnv(...)` and `CoptEnvConfig(...)` configure the built-in native COPT
  backend, including explicit library, license-directory, client, and OEM setup.
- `Model.optimize()` solves the model; use `Model.ObjVal` and `Var.X` for results.
- `Model.setMIPStart(variables, values)` submits one COPT MIP start. `variables`
  may be a `Vars` collection or an iterable of its `Var` members; `values` may
  be a NumPy array or iterable and is flattened in the same order. Partial
  starts are supported. Repeated calls append native COPT starts. COPT 8.0.6's
  C API exposes no matching remove operation, so `clearMIPStarts()` is not
  exposed yet; rebuild the model when its submitted starts must be discarded.

Available variable types are `MOI.CONTINUOUS`, `MOI.BINARY`, and `MOI.INTEGER`.
In COPT Direct mode, `addConstrs` collects a validated sparse row batch and
submits it once after the generator completes. A generator or input-validation
error submits no rows. `name=None` on `addVar`, `addVars`, `addConstr`, or
`addConstrs` omits native names and their per-item string allocations; explicit
names still work. Direct variable collections retain contiguous IDs as a compact
range, and objective expressions are submitted without retaining a replay copy.
Native failures poison a Direct model, which must then be rebuilt.

Quadratic and nonlinear products are not supported yet. The `obj` argument on
variable creation is currently reserved; define the objective with
`setObjective`.

Full documentation and Rust source are available in the
[`moi-rs` repository](https://github.com/riverflows2333/moi-rs).
