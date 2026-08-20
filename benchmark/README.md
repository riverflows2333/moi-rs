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
$env:VIRTUAL_ENV = (Resolve-Path .venv).Path
maturin develop --release --manifest-path python\moirspy\Cargo.toml
maturin develop --release --manifest-path python\moirspy-copt\Cargo.toml
.\.venv\Scripts\python.exe -m benchmark.run --case input\phys\effi\1-1 --repeat 5
```

Always rebuild both extensions with `--release` before recording a baseline after
Rust changes. The suite reports package versions, extension paths, binary sizes,
and an inferred debug/release profile so stale local installations are visible.

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

For an isolated multi-case comparison with median, p95, raw samples, environment
metadata, and process peak RSS:

```powershell
.\.venv\Scripts\python.exe -m benchmark.run_suite `
    --cases 1-1 1-2 1-3 `
    --warmup 1 `
    --repeat 5 `
    --json benchmark\results\baseline.json
```

Each case/tool pair runs in a fresh child process. Solver environment creation is
outside the timed samples, while process isolation prevents a previous model's
allocator state or peak working set from contaminating the next tool. Peak RSS is
an OS process high-water mark; `Peak delta` subtracts the high-water mark already
reached immediately after builder environment preparation.

Use `1-1` while iterating. Run `1-2` through `1-5` only after correctness and
memory behavior are stable; the same parser and formulation scale without case-
specific branches.

For coarse high-level stage timings, run:

```powershell
.\.venv\Scripts\python.exe -m benchmark.profile_moirspy
```

This diagnostic separates variables, constraint families, objective construction,
and backend attachment. Its few timer calls are outside the normal benchmark path
unless profiling is requested.

To isolate `quicksum` scaling across Var, two/three-term expressions, repeated
variables, and a weighted objective:

```powershell
.\.venv\Scripts\python.exe -m benchmark.profile_expr --sizes 2400 4800 9600
```

The output includes growth ratios and an empirical complexity exponent. Once the
linear accumulator is implemented, `--check --max-exponent 1.35` can be used as a
regression gate for the weighted-expression path.
