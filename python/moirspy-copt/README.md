# moirspy-copt

`moirspy-copt` is the native COPT backend for the
[`moirspy`](https://pypi.org/project/moirspy/) optimization-modeling package.
It supports linear and mixed-integer linear models through the COPT 8.0 C API.

## Installation

```bash
python -m pip install moirspy moirspy-copt
```

This package does not bundle COPT or a license. Install COPT separately and set
`COPT_HOME` to its installation root before creating a backend:

```bash
export COPT_HOME=/opt/copt80
```

The low-level constructor also accepts an installation root or exact native
library through `dll_path`.

## Use with moirspy

```python
from moirspy import MOI, Model

model = Model("example")
x = model.addVars(2, lb=0, name="x")
model.addConstr(x[0] + x[1] >= 1)
model.setObjective(x[0] + x[1], MOI.MINIMIZE)
model.setParam("Logging", 0)
model.setBackend("copt")
model.optimize()

print(model.ObjVal)
print([x[i].X for i in range(2)])
```

Existing model data and parameters are synchronized at `setBackend("copt")`;
later changes are forwarded incrementally.

## Explicit environments

An environment can use default license discovery or an explicit license
directory, and can be reused by multiple models:

```python
from moirspy_copt import Env, Model

env = Env(dll_path=None, license_dir=None)
first = Model("first", env=env)
second = Model("second", env=env)
```

COPT solve parameters belong to a problem, so configure them on each model with
`set_optimizer_attr`, or through high-level `Model.setParam`. They are not set
on `Env`.

## Low-level API

```python
from moirspy_copt import Model

backend = Model(name="direct", dll_path=None, license_dir=None)
x = backend.add_variable(name="x", vtype="C", lb=0.0, ub=10.0)
backend.add_constraint([x], [1.0], 0.0, ">", 4.0)
backend.set_objective([x], [2.0], 1.0, -1)  # 1=maximize; otherwise minimize
backend.set_optimizer_attr("Logging", 0)
status = backend.optimize()

print(status, backend.get_objective_value(), backend.get_var_value(x))
```

Variable and constraint IDs are integers. Linear functions use parallel
variable/coefficient arrays, and supported constraint senses are `"<"`, `">"`,
and `"="`. Parameter values may be Boolean, integer, or floating point.
