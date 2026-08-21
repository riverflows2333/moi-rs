# UC model-construction benchmark

This directory contains a complete direct 2-bin UC benchmark built from the
supplied competition data and the formulation in `input/AggModel`. All builders
receive the same immutable, already parsed `MinimalUcData`; file I/O, branch
sensitivity parsing, and piecewise-cost preprocessing are reported separately
and are not included in the model-construction samples.

The timed region starts immediately before creating a fresh model and ends after
the formulation has been synchronized to the native solver model. It includes
variable creation, expression or sparse-row assembly, constraints, objective, and
backend synchronization. It never calls the solver.

## Formulation

Thermal units use the original two-binary-variable formulation: commitment `x`
and startup `u`, plus output and epigraph cost variables. The model includes:

- initial-state hold, startup transition, minimum-up, and minimum-down logic;
- thermal output bounds and startup/shutdown-relaxed ramping;
- all five supplied thermal piecewise-linear running-cost cuts and startup cost;
- storage charge/discharge/output/SOC variables and operating bounds;
- storage mutual-exclusion, no-direct-switch, minimum charge/discharge/off time,
  cumulative energy, terminal energy, and all supplied cost cuts;
- system balance, 10% positive spinning reserve, and every supplied section's
  positive and negative flow limit using `branch_1.log` sensitivities.

`benchmark.common.iter_constraint_groups` is the single formulation source used
by moirspy early/late, the low-level Rust/COPT binding, and coptpy. Sparse rows are
generated inside each timed build rather than precomputed during input parsing.

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
    --json benchmark\results\baseline.json `
    --markdown benchmark\results\baseline.md
```

Each case/tool pair runs in a fresh child process. Solver environment creation is
outside the timed samples, while process isolation prevents a previous model's
allocator state or peak working set from contaminating the next tool. Peak RSS is
an OS process high-water mark; `Peak delta` subtracts the high-water mark already
reached immediately after builder environment preparation.

Use `1-1` while iterating. Run `1-2` through `1-5` only after correctness and
memory behavior are stable; the same parser and formulation scale without case-
specific branches.

When `--cases` is omitted, the suite runs all five efficiency cases (`1-1`
through `1-5`). Large cases are construction-only and are never solved. The
Markdown report records this timing boundary explicitly and is suitable for
checking into a benchmark-results branch or attaching to a performance review.

For coarse high-level stage timings, run:

```powershell
.\.venv\Scripts\python.exe -m benchmark.profile_moirspy
```

This diagnostic separates variables, constraint families, objective construction,
and backend attachment. Its few timer calls are outside the normal benchmark path
unless profiling is requested.

To solve a small model that activates every constraint family and verify that all
four builders return the same objective:

```powershell
.\.venv\Scripts\python.exe -m benchmark.verify_model
```

To isolate `quicksum` scaling across Var, two/three-term expressions, repeated
variables, and a weighted objective, and compare the direct `dot` constructor:

```powershell
.\.venv\Scripts\python.exe -m benchmark.profile_expr --sizes 2400 4800 9600
```

The output includes growth ratios and an empirical complexity exponent. Once the
linear accumulator is implemented, `--check --max-exponent 1.35` can be used as a
regression gate for the weighted-expression path.
