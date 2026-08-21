from __future__ import annotations

import argparse
import math
from pathlib import Path
from typing import Any

from benchmark.builders import BUILDERS
from benchmark.common import MinimalUcData, Section, StorageUnit, ThermalUnit


def verification_case() -> MinimalUcData:
    """Small feasible case that activates every constraint family."""

    periods = 4
    units = (
        ThermalUnit(1, 100.0, 20.0, 30.0, 30.0, 2, 2, 10.0, 1, 40.0, 2, ((1.0, 0.0),)),
        ThermalUnit(2, 80.0, 10.0, 25.0, 25.0, 2, 2, 8.0, 0, 0.0, 2, ((1.5, 0.0),)),
    )
    storages = (
        StorageUnit(1, 2, 2, 2, 50.0, 50.0, 100.0, 20.0, 30.0, 5.0, 1, 2.0, ((3.0, 0.0),)),
    )
    sections = (
        Section(1, (1,), 1_000.0, (0.2, -0.1), (0.2,), (0.0,) * periods),
    )
    return MinimalUcData(Path("synthetic-complete-2bin"), (50.0, 60.0, 55.0, 45.0), units, storages, sections)


def solve_objective(tool: str, model: Any) -> float:
    if tool.startswith("moirspy-") and tool != "moirspy-copt":
        model.optimize()
        value = model.ObjVal
    elif tool == "moirspy-copt":
        model.optimize()
        value = model.get_objective_value()
    else:
        model.solve()
        value = model.objval
    if value is None or not math.isfinite(value):
        raise RuntimeError(f"{tool} did not return a finite objective")
    return float(value)


def main() -> int:
    parser = argparse.ArgumentParser(description="Solve a small complete 2-bin model with every benchmark backend")
    parser.add_argument("--tools", default=",".join(BUILDERS))
    parser.add_argument("--tolerance", type=float, default=1e-7)
    args = parser.parse_args()
    tools = [name.strip() for name in args.tools.split(",") if name.strip()]
    unknown = [name for name in tools if name not in BUILDERS]
    if unknown:
        raise SystemExit(f"unknown builder(s): {', '.join(unknown)}")

    data = verification_case()
    objectives: dict[str, float] = {}
    for tool in tools:
        builder = BUILDERS[tool]
        context = builder.prepare()
        model = builder.build(data, context)
        objectives[tool] = solve_objective(tool, model)
        print(f"{tool},{objectives[tool]:.10f}")

    reference_tool = tools[0]
    reference = objectives[reference_tool]
    expected = 5_040.0
    if not math.isclose(reference, expected, rel_tol=args.tolerance, abs_tol=args.tolerance):
        raise SystemExit(f"unexpected verification objective: expected {expected}, got {reference}")
    for tool, objective in objectives.items():
        if not math.isclose(objective, reference, rel_tol=args.tolerance, abs_tol=args.tolerance):
            raise SystemExit(
                f"objective mismatch: {reference_tool}={reference}, {tool}={objective}"
            )
    print(
        f"verified {len(tools)} builders: {data.num_variables} variables, "
        f"{data.num_constraints} constraints, objective={reference:.10f}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
