# UC model-construction benchmark

This directory contains a deliberately direct benchmark built from the supplied
competition data. All builders receive the same immutable, already parsed
`MinimalUcData`; file I/O and cost preprocessing are reported separately and are
not included in the model-construction samples.

The timed region starts immediately before creating a fresh model and ends after
the formulation has been synchronized to the native solver model. It includes
variable creation, expression or sparse-row assembly, constraints, objective, and
backend synchronization. It never calls the solver.

## Initial formulation

The first version is intentionally smaller than the complete competition model:

- thermal on/off, output, and startup variables for every unit and period;
- output lower and upper bounds;
- startup transition constraints, including the supplied initial state;
- ramp-up and ramp-down constraints, relaxed across startup or shutdown;
- system load balance and 10% positive spinning reserve;
- startup cost plus a linear chord approximation of each unit's running cost.

It currently excludes minimum up/down time, storage, transmission sections, and
the exact piecewise-linear bid curve. Therefore it is a modeling-throughput
workload, not a contest-feasible replacement model. Those features can be added
as identical formulation stages once the basic benchmark is stable.

## Run

From the repository root:

```powershell
$env:COPT_HOME = 'D:\env\copt80'
.\.venv\Scripts\python.exe -m benchmark.run --case input\phys\effi\1-1 --repeat 5
```

Available builders are:

- `moirspy-early`: attach COPT before adding variables and constraints, so every
  modeling operation is forwarded incrementally;
- `moirspy-late`: build through the public API first, then attach COPT and replay
  the completed model;
- `moirspy-copt`: low-level batched Rust/COPT binding;
- `coptpy`: official COPT Python modeling API.

Choose a subset with `--tools moirspy-early,moirspy-late`. Each solver environment
is created once before timing so license and environment startup do not obscure
the modeling-layer comparison. Builders that cannot be imported or initialized
are reported as unavailable and the remaining builders still run.

Use `1-1` while iterating. Run `1-2` through `1-5` only after correctness and
memory behavior are stable; the same parser and formulation scale without case-
specific branches.
