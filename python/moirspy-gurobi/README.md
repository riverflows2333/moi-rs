# moirspy-gurobi

`moirspy-gurobi` is the native Gurobi backend for the
[`moirspy`](https://pypi.org/project/moirspy/) optimization-modeling package.
The extension is implemented in Rust and loads the Gurobi native library at
runtime.

> The project is under active development and its API is not yet stable.

## Installation

```bash
python -m pip install moirspy moirspy-gurobi
```

This package does not bundle Gurobi or a license. Install Gurobi separately and
make its native library and license available.

> **Important:** `GUROBI_HOME` must be set to the Gurobi installation directory
> before creating the backend.

Set it in the shell:

```bash
export GUROBI_HOME=/opt/gurobi1203
```

Or set it at the beginning of the Python program, before importing or creating
the backend:

```python
import os

os.environ["GUROBI_HOME"] = "/opt/gurobi1203"
```

If the environment variable cannot be set, pass the full native-library path to
the low-level constructor explicitly:

```python
from moirspy_gurobi import Model

backend = Model(name="direct", dll_path="/opt/gurobi1203/lib/libgurobi120.so")
```

## Use with moirspy

```python
from moirspy import MOI, Model

model = Model("example")
x = model.addVars(2, lb=0, name="x")
model.addConstr(x[0] + x[1] >= 1)
model.setObjective(x[0] + x[1], MOI.MINIMIZE)
model.setParam("OutputFlag", 0)
model.setBackend("gurobi")
model.optimize()

print(model.ObjVal)
```

Existing model data is synchronized when `setBackend("gurobi")` is called;
later changes are forwarded incrementally.

## Low-level API

Most users should use `moirspy.Model`. A low-level interface is also available:

```python
from moirspy_gurobi import Model

backend = Model(name="direct", dll_path=None)
x = backend.add_variable(name="x", vtype="C", lb=0.0, ub=float("inf"))
backend.set_objective([x], [1.0], 0.0, 0)  # 1=maximize; otherwise minimize
backend.update()
status = backend.optimize()
print(status, backend.get_var_value(x))
```

The low-level API represents variables and constraints with integer IDs. Linear
functions use parallel variable/coefficient arrays, and constraint senses are
`"<"`, `">"`, or `"="`.

### Explicit environments

`Model` creates a regular environment automatically when `env` is omitted. An
explicit environment can be configured once and reused by multiple models:

```python
from moirspy_gurobi import Env, Model

env = Env(dll_path=None)
env.setParam("OutputFlag", 0)
env.setParam("Threads", 4)

model_a = Model("a", env=env)
model_b = Model("b", env=env)
```

The same environment can be passed through the high-level modeling API:

```python
from moirspy import Model
from moirspy_gurobi import Env

env = Env()
model = Model("high-level")
model.setBackend("gurobi", env=env)
```

Connection and licensing parameters that must be configured before startup use
an empty environment:

```python
env = Env(empty=True)
env.setParam("TokenServer", "server.example.com")
env.start()
model = Model("remote", env=env)
```

Environment parameters are copied when a model is created. Later changes to
the original `Env` affect future models only; use `Model.set_optimizer_attr`
to change a parameter on an existing low-level model.

Full documentation and Rust source are available in the
[`moi-rs` repository](https://github.com/riverflows2333/moi-rs).
