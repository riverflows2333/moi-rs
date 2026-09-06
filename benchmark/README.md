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

### Local COPT environment configuration

Benchmarks load COPT environment configuration from process variables and then
from the repository-root `.env` file. Existing process variables take precedence.
To use OEM, client certificate, cluster, or Web License settings without placing
credentials in source code:

```powershell
Copy-Item .env.example .env
# Edit .env locally; never commit it.
```

The real `.env` and `.env.*` files are ignored; only `.env.example` is tracked.
OEM setup accepts either a complete `COPT_OEM_LICENSE` value using `\n` escapes,
or `COPT_OEM_VERSION`, `COPT_OEM_EXPIRY`, and `COPT_OEM_TYPE` from which the
license payload is constructed. `COPT_OEM_NAME` and `COPT_OEM_SIGNATURE` are
required whenever OEM configuration is enabled. The direct public-API example
also accepts an explicit file:

```powershell
uv run python -m benchmark.direct_uc_moirspy `
    --case input\phys\effi\1-1 `
    --env-file .env
```

Available builders are:

- `moirspy-early`: construct the benchmark's solver-independent `LinearRow`
  stream inside the timed region, convert each row with `dot`, and send each
  constraint family through `addConstrs` to a Direct COPT model;
- `moirspy-direct-api`: write the same UC formulation directly from the in-memory
  unit/storage/section fields with normal moirspy expressions and a Direct COPT
  model, without constructing the benchmark `LinearRow`/`VarRef` layer;
- `moirspy-direct-nonames`: use the same Direct API formulation while passing
  `name=None` for every variable and constraint, measuring the recommended
  production path when generated solver names are unnecessary;
- `moirspy-late`: build through the public API first, then attach COPT and replay
  the completed model;
- `moirspy-copt`: low-level batched Rust/COPT binding;
- `coptpy`: official COPT Python modeling API;
- `pyoptinterface-rows`: PyOptInterface receives the exact same `LinearRow`
  coefficient stream as the other cross-framework builders and constructs each
  affine function with `ScalarAffineFunction.add_term`;
- `pyoptinterface-direct`: the same complete 2-bin UC formulation is written
  directly with PyOptInterface variables, expressions, `quicksum`, and
  `add_linear_constraint`, without the benchmark's `VarRef`/`LinearRow` layer.

`moirspy-direct-nonames` and `pyoptinterface-direct` are the representative
names-off application-style comparison. `moirspy-direct-api` retains explicit
names to quantify their allocation cost. All three start from already parsed UC
fields and assemble expressions during the timed build. `moirspy-early` and
`pyoptinterface-rows` remain useful for coefficient-for-coefficient regression
and for measuring the cost of the shared sparse-row abstraction.

Install the optional comparison package into the benchmark environment with:

```powershell
uv pip install --python .\.venv\Scripts\python.exe pyoptinterface==0.6.1 numpy
```

Both PyOptInterface builders use its COPT backend. Their reusable `copt.Env` is
created outside the timed region, just like the other COPT builders. Variable
and constraint names are omitted because that is PyOptInterface's normal fast
API default; formulation equality is checked through dimensions, nonzeros, and
the solved verification case rather than generated names.

The rows builder intentionally does not use PyOptInterface's
`add_m_linear_constraints`: that helper accepts one common sense per matrix and
internally loops over its dense/sparse rows. The UC formulation mixes equality,
less-than, and greater-than rows, so constructing `ScalarAffineFunction`
directly from the shared `LinearRow` stream gives the closest coefficient-for-
coefficient comparison without adding a separate SciPy preparation step.

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

### Direct migration baseline

Before changing the model runtime, verify the machine-independent formulation
contract for `1-1`, `1-3`, and `1-5`:

```powershell
uv run python -m benchmark.baseline
```

The tracked `direct_migration_contract_v1.json` records variables, rows, nonzeros,
objective nonzeros, per-family row/nonzero counts, and a versioned SHA-256 over the
complete coefficient/RHS/objective stream. Timing and RSS remain machine-local in
the ignored `benchmark/results` directory. A reproducible D0 timing run is:

```powershell
uv run python -m benchmark.run_suite `
    --cases 1-1 1-3 1-5 `
    --tools moirspy-early,moirspy-late,moirspy-copt,pyoptinterface-rows,pyoptinterface-direct,coptpy `
    --warmup 1 --repeat 5 `
    --json benchmark\results\direct_migration_d0_windows.json `
    --markdown benchmark\results\direct_migration_d0_windows.md
```

The synthetic verification model fixes the expected solved objective at `5040.0`.
The Windows differential test additionally compares the same binary model across
COPT/Gurobi and early/late attachment. These assertions are intended to be shared
by the later Direct and Cached runtime paths.

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
