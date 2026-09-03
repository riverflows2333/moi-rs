"""Build the supplied complete 2-bin UC model with the normal moirspy API.

Unlike the cross-backend benchmark builders, this example writes constraints
directly with ``addVars``, ``addConstr(s)``, and ``quicksum``.  Input parsing is
kept in ``direct_uc_data.py`` so the modeling code can be reused independently.

Build case 1-1 without solving, from the repository root:

    python -m benchmark.direct_uc_moirspy --case input/phys/effi/1-1

Add ``--solve`` for a licensed case that should also be optimized.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
from time import perf_counter
from typing import Any

from moirspy import MOI, CoptEnv, Model, quicksum

from benchmark.copt_env import create_copt_env
from benchmark.direct_uc_data import UcData, load_uc_data

ROOT = Path(__file__).resolve().parents[1]
DEFAULT_CASE = ROOT / "input" / "phys" / "effi" / "1-1"


@dataclass(slots=True)
class ThermalVariables:
    p: Any
    x: Any
    u: Any
    cost_p: Any
    cost_u: Any


@dataclass(slots=True)
class StorageVariables:
    op: Any
    ip: Any
    o: Any
    i: Any
    soc: Any
    cost_o: Any
    cost_i: Any


@dataclass(slots=True)
class UcVariables:
    thermal: ThermalVariables
    storage: StorageVariables | None


def _add_thermal_constraints(
    model: Model,
    data: UcData,
    variables: ThermalVariables,
) -> None:
    p, x, u = variables.p, variables.x, variables.u
    cost_p, cost_u = variables.cost_p, variables.cost_u
    periods = data.num_periods
    delta = 96.0 / periods

    for g, unit in enumerate(data.units):
        minimum_on = max(1, int(unit.minimum_on / delta))
        minimum_off = max(1, int(unit.minimum_off / delta))
        remaining = (
            unit.minimum_on if unit.initial_on else unit.minimum_off
        ) - unit.initial_duration
        hold = max(1, int(remaining / delta)) if remaining >= 0.5 else 0

        if hold:
            model.addConstrs(
                (x[g, t] == unit.initial_on for t in range(min(hold, periods))),
                name=f"thermal_hold_{g}",
            )

        model.addConstr(
            x[g, 0] - u[g, 0] <= unit.initial_on,
            name=f"thermal_transition_initial_{g}",
        )
        model.addConstrs(
            (
                x[g, t] - x[g, t - 1] <= u[g, t]
                for t in range(1, periods)
            ),
            name=f"thermal_transition_{g}",
        )

        model.addConstrs(
            (
                quicksum(
                    u[g, k]
                    for k in range(max(0, t - minimum_on + 1), t + 1)
                )
                <= x[g, t]
                for t in range(periods)
            ),
            name=f"thermal_minimum_on_{g}",
        )
        for t in range(periods):
            start = max(0, t - minimum_off + 1)
            starts = quicksum(u[g, k] for k in range(start, t + 1))
            rhs = (
                1 - unit.initial_on
                if start == 0
                else 1 - x[g, t - minimum_off]
            )
            model.addConstr(
                starts <= rhs,
                name=f"thermal_minimum_off_{g}_{t}",
            )

        model.addConstrs(
            (p[g, t] <= unit.p_max * x[g, t] for t in range(periods)),
            name=f"thermal_output_max_{g}",
        )
        model.addConstrs(
            (p[g, t] >= unit.p_min * x[g, t] for t in range(periods)),
            name=f"thermal_output_min_{g}",
        )

        if unit.p_max - unit.p_min > min(unit.ramp_up, unit.ramp_down):
            ramp_up = unit.ramp_up * delta
            ramp_down = unit.ramp_down * delta
            model.addConstr(
                p[g, 0]
                <= unit.initial_output
                + ramp_up * unit.initial_on
                + unit.p_max * (1 - unit.initial_on),
                name=f"thermal_ramp_up_initial_{g}",
            )
            model.addConstr(
                unit.initial_output - p[g, 0]
                <= ramp_down * x[g, 0] + unit.p_max * (1 - x[g, 0]),
                name=f"thermal_ramp_down_initial_{g}",
            )
            model.addConstrs(
                (
                    p[g, t] - p[g, t - 1]
                    <= ramp_up * x[g, t - 1]
                    + unit.p_max * (1 - x[g, t - 1])
                    for t in range(1, periods)
                ),
                name=f"thermal_ramp_up_{g}",
            )
            model.addConstrs(
                (
                    p[g, t - 1] - p[g, t]
                    <= ramp_down * x[g, t]
                    + unit.p_max * (1 - x[g, t])
                    for t in range(1, periods)
                ),
                name=f"thermal_ramp_down_{g}",
            )

        model.addConstrs(
            (
                cost_p[g, t]
                >= delta * (slope * p[g, t] + intercept * x[g, t])
                for t in range(periods)
                for slope, intercept in unit.cost_lines
            ),
            name=f"thermal_running_cost_{g}",
        )
        model.addConstrs(
            (
                cost_u[g, t] == unit.startup_cost * u[g, t]
                for t in range(periods)
            ),
            name=f"thermal_startup_cost_{g}",
        )


def _add_minimum_state_constraints(
    model: Model,
    states: Any,
    entity: int,
    minimum: int,
    periods: int,
    name: str,
) -> None:
    if minimum < 2:
        return
    model.addConstrs(
        (
            quicksum(states[entity, tau] for tau in range(t, t + length))
            >= length * (states[entity, t] - states[entity, t - 1])
            for t in range(1, periods)
            for length in (min(minimum, periods - t),)
        ),
        name=name,
    )


def _add_storage_constraints(
    model: Model,
    data: UcData,
    variables: StorageVariables,
) -> None:
    op, ip = variables.op, variables.ip
    o, i, soc = variables.o, variables.i, variables.soc
    cost_o, cost_i = variables.cost_o, variables.cost_i
    periods = data.num_periods
    delta = 96.0 / periods
    energy_factor = 0.25 * delta

    for e, storage in enumerate(data.storages):
        model.addConstrs(
            (
                op[e, t] >= storage.minimum_discharge_power * o[e, t]
                for t in range(periods)
            ),
            name=f"storage_output_min_{e}",
        )
        model.addConstrs(
            (
                op[e, t] <= storage.maximum_discharge * o[e, t]
                for t in range(periods)
            ),
            name=f"storage_output_max_{e}",
        )
        model.addConstrs(
            (ip[e, t] == storage.charge_power * i[e, t] for t in range(periods)),
            name=f"storage_charge_output_{e}",
        )

        model.addConstrs(
            (o[e, t] + i[e, t] <= 1 for t in range(periods)),
            name=f"storage_mutual_exclusion_{e}",
        )
        model.addConstrs(
            (o[e, t] + i[e, t + 1] <= 1 for t in range(periods - 1)),
            name=f"storage_no_discharge_to_charge_{e}",
        )
        model.addConstrs(
            (i[e, t] + o[e, t + 1] <= 1 for t in range(periods - 1)),
            name=f"storage_no_charge_to_discharge_{e}",
        )

        model.addConstrs(
            (
                storage.maximum_energy * soc[e, t]
                == storage.initial_energy
                + energy_factor
                * quicksum(ip[e, tau] - op[e, tau] for tau in range(t + 1))
                for t in range(periods)
            ),
            name=f"storage_energy_{e}",
        )
        model.addConstr(
            soc[e, periods - 1]
            >= storage.final_energy / storage.maximum_energy,
            name=f"storage_terminal_energy_{e}",
        )

        minimum_discharge = max(
            1, int(storage.minimum_discharge_time / delta)
        )
        minimum_charge = max(1, int(storage.minimum_charge / delta))
        minimum_off = max(1, int(storage.minimum_off / delta))
        model.addConstr(
            quicksum(
                o[e, t] for t in range(min(minimum_discharge, periods))
            )
            >= minimum_discharge * o[e, 0],
            name=f"storage_initial_discharge_{e}",
        )
        model.addConstr(
            quicksum(i[e, t] for t in range(min(minimum_charge, periods)))
            >= minimum_charge * i[e, 0],
            name=f"storage_initial_charge_{e}",
        )
        _add_minimum_state_constraints(
            model,
            o,
            e,
            minimum_discharge,
            periods,
            f"storage_minimum_discharge_{e}",
        )
        _add_minimum_state_constraints(
            model,
            i,
            e,
            minimum_charge,
            periods,
            f"storage_minimum_charge_{e}",
        )
        if storage.minimum_off >= 2:
            model.addConstrs(
                (
                    quicksum(
                        1 - o[e, tau] - i[e, tau]
                        for tau in range(t, t + length)
                    )
                    >= length
                    * (
                        o[e, t - 1]
                        + i[e, t - 1]
                        - o[e, t]
                        - i[e, t]
                    )
                    for t in range(1, periods)
                    for length in (min(minimum_off, periods - t),)
                ),
                name=f"storage_minimum_off_{e}",
            )

        model.addConstrs(
            (
                cost_o[e, t]
                >= delta * (slope * op[e, t] + intercept * o[e, t])
                for t in range(periods)
                for slope, intercept in storage.discharge_cost_lines
            ),
            name=f"storage_discharge_cost_{e}",
        )
        model.addConstrs(
            (
                cost_i[e, t] == delta * storage.charge_cost * i[e, t]
                for t in range(periods)
            ),
            name=f"storage_charge_cost_{e}",
        )


def _add_system_constraints(
    model: Model,
    data: UcData,
    thermal: ThermalVariables,
    storage: StorageVariables | None,
) -> None:
    periods = data.num_periods

    for section in data.sections:
        for t in range(periods):
            flow = quicksum(
                coefficient * thermal.p[g, t]
                for g, coefficient in enumerate(section.thermal_sensitivity)
            )
            if storage is not None:
                flow += quicksum(
                    coefficient * (storage.op[e, t] - storage.ip[e, t])
                    for e, coefficient in enumerate(section.storage_sensitivity)
                )
            model.addConstr(
                flow <= section.limit + section.load_offset[t],
                name=f"section_upper_{section.section_id}_{t}",
            )
            model.addConstr(
                flow >= -section.limit + section.load_offset[t],
                name=f"section_lower_{section.section_id}_{t}",
            )

    for t, load in enumerate(data.loads):
        total_output = quicksum(
            thermal.p[g, t] for g in range(data.num_units)
        )
        if storage is not None:
            total_output += quicksum(
                storage.op[e, t] - storage.ip[e, t]
                for e in range(data.num_storages)
            )
        model.addConstr(total_output == load, name=f"balance_{t}")

        reserve = quicksum(
            unit.p_max * thermal.x[g, t] - thermal.p[g, t]
            for g, unit in enumerate(data.units)
        )
        model.addConstr(
            reserve >= data.reserve_ratio * load,
            name=f"reserve_{t}",
        )


def build_uc_model(
    data: UcData,
    env: CoptEnv,
    *,
    attach_early: bool = True,
    logging: int = 0,
    threads: int | None = None,
) -> tuple[Model, UcVariables]:
    """Build a complete UC model directly through the public moirspy API."""
    model_name = f"direct-2bin-uc-{data.case_dir.name}"
    model = (
        Model(model_name, backend="copt", env=env)
        if attach_early
        else Model(model_name)
    )
    model.setParam("Logging", logging)
    if threads is not None:
        model.setParam("Threads", threads)

    thermal = ThermalVariables(
        p=model.addVars(data.num_units, data.num_periods, lb=0.0, name="thermal_p"),
        x=model.addVars(
            data.num_units,
            data.num_periods,
            lb=0.0,
            ub=1.0,
            vtype=MOI.BINARY,
            name="thermal_x",
        ),
        u=model.addVars(
            data.num_units,
            data.num_periods,
            lb=0.0,
            ub=1.0,
            vtype=MOI.BINARY,
            name="thermal_u",
        ),
        cost_p=model.addVars(
            data.num_units, data.num_periods, lb=0.0, name="thermal_cost_p"
        ),
        cost_u=model.addVars(
            data.num_units, data.num_periods, lb=0.0, name="thermal_cost_u"
        ),
    )

    storage: StorageVariables | None = None
    if data.num_storages:
        storage = StorageVariables(
            op=model.addVars(
                data.num_storages, data.num_periods, lb=0.0, name="storage_op"
            ),
            ip=model.addVars(
                data.num_storages, data.num_periods, lb=0.0, name="storage_ip"
            ),
            o=model.addVars(
                data.num_storages,
                data.num_periods,
                lb=0.0,
                ub=1.0,
                vtype=MOI.BINARY,
                name="storage_o",
            ),
            i=model.addVars(
                data.num_storages,
                data.num_periods,
                lb=0.0,
                ub=1.0,
                vtype=MOI.BINARY,
                name="storage_i",
            ),
            soc=model.addVars(
                data.num_storages,
                data.num_periods,
                lb=0.0,
                ub=1.0,
                name="storage_soc",
            ),
            cost_o=model.addVars(
                data.num_storages,
                data.num_periods,
                lb=0.0,
                name="storage_cost_o",
            ),
            cost_i=model.addVars(
                data.num_storages,
                data.num_periods,
                lb=0.0,
                name="storage_cost_i",
            ),
        )

    _add_thermal_constraints(model, data, thermal)
    if storage is not None:
        _add_storage_constraints(model, data, storage)
    _add_system_constraints(model, data, thermal, storage)

    objective = quicksum(
        thermal.cost_p[g, t] + thermal.cost_u[g, t]
        for g in range(data.num_units)
        for t in range(data.num_periods)
    )
    if storage is not None:
        objective += quicksum(
            storage.cost_o[e, t] - storage.cost_i[e, t]
            for e in range(data.num_storages)
            for t in range(data.num_periods)
        )
    model.setObjective(objective, MOI.MINIMIZE)

    if not attach_early:
        model.setBackend("copt", env=env)
        model.setParam("RelGap",3e-3)
    return model, UcVariables(thermal=thermal, storage=storage)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Build the complete supplied 2-bin UC model with moirspy and COPT"
    )
    parser.add_argument("--case", type=Path, default=DEFAULT_CASE)
    parser.add_argument(
        "--attach",
        choices=("early", "late"),
        default="early",
        help="attach COPT before modeling or replay the completed model afterwards",
    )
    parser.add_argument("--solve", action="store_true")
    parser.add_argument("--logging", type=int, choices=(0, 1), default=0)
    parser.add_argument("--threads", type=int)
    parser.add_argument(
        "--env-file",
        type=Path,
        help="dotenv file with COPT EnvConfig values (default: repository .env)",
    )
    args = parser.parse_args()

    started = perf_counter()
    data = load_uc_data(args.case)
    input_seconds = perf_counter() - started

    copt_env = create_copt_env(args.env_file)
    started = perf_counter()
    model, _ = build_uc_model(
        data,
        copt_env,
        attach_early=args.attach == "early",
        logging=args.logging,
        threads=args.threads,
    )
    model_seconds = perf_counter() - started

    print(
        f"case={data.case_dir.name}: {data.num_units} thermal units, "
        f"{data.num_storages} storage units, {data.num_periods} periods, "
        f"{len(data.sections)} sections"
    )
    print(f"input and preprocessing: {input_seconds:.6f} s")
    print(f"direct moirspy modeling ({args.attach} attach): {model_seconds:.6f} s")

    if args.solve:
        started = perf_counter()
        model.optimize()
        solve_seconds = perf_counter() - started
        if model.ObjVal is None:
            raise RuntimeError("COPT did not return a feasible solution")
        print(f"solve: {solve_seconds:.6f} s, objective={model.ObjVal:.10f}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
