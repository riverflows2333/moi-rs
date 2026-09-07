# moirspy-copt

`moirspy-copt` is the optional low-level and compatibility COPT package for
[`moirspy`](https://pypi.org/project/moirspy/). It supports linear and
mixed-integer linear models through the COPT 8.0 C API. Current `moirspy`
releases already include the native high-level COPT backend, so this package is
only needed for the `moirspy_copt` low-level import path.

## Installation

```bash
python -m pip install moirspy-copt
```

This package does not bundle COPT or a license. Install COPT separately and set
`COPT_HOME` to its installation root before creating a backend:

```bash
export COPT_HOME=/opt/copt80
```

The low-level constructor also accepts an installation root or exact native
library through `dll_path`.

## Built-in high-level API

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
later changes are forwarded incrementally. This workflow is implemented inside
`moirspy` and does not import or call `moirspy_copt.Model`.

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

## Environment configuration

`EnvrConfig` mirrors COPT's client-configuration workflow. Configuration values
may be strings, Booleans, integers, or floats; they are converted to the string
values required by `COPT_SetEnvConfig` before an environment is created.

```python
from moirspy_copt import COPT, Env, EnvrConfig

config = EnvrConfig()
config.set("NoBanner", 1)
config.set(COPT.CLIENT_CAFILE, "/path/to/ca.pem")
config.set(COPT.CLIENT_CERTFILE, "/path/to/client.pem")
config.set(COPT.CLIENT_CERTKEYFILE, "/path/to/client-key.pem")

env = Env(config=config)
```

The `COPT` namespace exposes all client configuration names published in the
COPT 8.0.6 header: CA/certificate files, cluster and floating servers, password,
port, priority, wait time, and web-license settings. Arbitrary configuration
names are also forwarded unchanged, which supports vendor-provided OEM fields.
Keep OEM material outside source control, for example:

```python
import os

config = EnvrConfig()
config.set("OEM", os.environ["COPT_OEM_NAME"])
config.set("License", os.environ["COPT_OEM_LICENSE"])
config.set("Signature", os.environ["COPT_OEM_SIGNATURE"])
env = Env(config=config)
```

The repository benchmark additionally supports a local ignored `.env` file; see
the root `.env.example`. Installed library code intentionally does not load dotenv
files implicitly, so applications retain control of their configuration source.

`config` is mutually exclusive with `dll_path` and `license_dir`. If a custom
native library is needed, pass it when constructing the configuration:

```python
config = EnvrConfig(dll_path="/opt/copt80/lib/libcopt.so")
env = Env(config=config)
```

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
