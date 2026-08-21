"""Solve a small assignment MILP with moirspy and the COPT backend.

The example is self-contained: its workers, jobs, and cost matrix are declared
below.  COPT itself and a valid license must be installed separately.  Point
``COPT_HOME`` at the COPT installation before running the script when COPT
cannot be discovered automatically.

Run from the repository root with:

    python examples/moirspy_copt_assignment.py
"""

from __future__ import annotations

from math import isclose

from moirspy import MOI, Model, quicksum
from moirspy_copt import Env, EnvrConfig


WORKERS = ("Alice", "Bob", "Carol", "Dave")
JOBS = ("clean", "cook", "shop", "wash")

# COSTS[worker][job]
COSTS = (
    (9.0, 2.0, 7.0, 8.0),
    (6.0, 4.0, 3.0, 7.0),
    (5.0, 8.0, 1.0, 8.0),
    (7.0, 6.0, 9.0, 4.0),
)

EXPECTED_OBJECTIVE = 13.0


def main() -> None:
    """Build, solve, print, and validate the assignment model."""
    num_workers = len(WORKERS)
    num_jobs = len(JOBS)

    # EnvrConfig mirrors COPT's client configuration API. It can also hold
    # cluster, certificate, web-license, or externally supplied OEM settings.
    config = EnvrConfig()
    config.set("NoBanner", 1)
    copt_env = Env(config=config)
    model = Model("assignment-demo")

    # Attaching COPT before modeling forwards subsequent changes incrementally
    # to the native backend and avoids replaying the completed model later.
    model.setBackend("copt", env=copt_env)
    model.setParam("Logging", 0)
    model.setParam("Threads", 1)

    # x[i, j] is 1 exactly when worker i is assigned to job j.
    x = model.addVars(
        num_workers,
        num_jobs,
        lb=0.0,
        ub=1.0,
        vtype=MOI.BINARY,
        name="x",
    )

    # Every worker receives exactly one job.
    model.addConstrs(
        (
            quicksum(x[i, j] for j in range(num_jobs)) == 1.0
            for i in range(num_workers)
        ),
        name="one_job_per_worker",
    )

    # Every job is assigned to exactly one worker.
    model.addConstrs(
        (
            quicksum(x[i, j] for i in range(num_workers)) == 1.0
            for j in range(num_jobs)
        ),
        name="one_worker_per_job",
    )

    total_cost = quicksum(
        COSTS[i][j] * x[i, j]
        for i in range(num_workers)
        for j in range(num_jobs)
    )
    model.setObjective(total_cost, MOI.MINIMIZE)

    model.optimize()

    if model.ObjVal is None:
        raise RuntimeError("COPT did not return a feasible solution")

    assignments: list[tuple[str, str, float]] = []
    for i, worker in enumerate(WORKERS):
        for j, job in enumerate(JOBS):
            value = x[i, j].X
            if value is not None and value > 0.5:
                assignments.append((worker, job, COSTS[i][j]))

    print(f"Optimal total cost: {model.ObjVal:.1f}")
    for worker, job, cost in assignments:
        print(f"  {worker:5s} -> {job:5s}  cost={cost:.1f}")

    if len(assignments) != num_workers:
        raise AssertionError(f"expected {num_workers} assignments, got {assignments}")
    if not isclose(model.ObjVal, EXPECTED_OBJECTIVE, abs_tol=1e-7):
        raise AssertionError(
            f"expected objective {EXPECTED_OBJECTIVE}, got {model.ObjVal}"
        )


if __name__ == "__main__":
    main()
