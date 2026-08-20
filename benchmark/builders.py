from __future__ import annotations

import os
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from benchmark.common import MinimalUcData


@dataclass(slots=True)
class Builder:
    name: str
    prepare: Callable[[], Any]
    build: Callable[[MinimalUcData, Any], Any]


def _copt_dll() -> str | None:
    home = os.environ.get("COPT_HOME")
    if not home:
        return None
    candidate = Path(home) / "bin" / ("copt.dll" if sys.platform == "win32" else "libcopt.so")
    return str(candidate) if candidate.is_file() else None


def prepare_moirspy() -> Any:
    from moirspy_copt import Env

    return Env(_copt_dll())


def build_moirspy(data: MinimalUcData, env: Any) -> Any:
    from moirspy import MOI, Model, quicksum

    g_count, t_count = data.num_units, data.num_periods
    model = Model("minimal-uc-moirspy")
    on = model.addVars(g_count, t_count, lb=0.0, ub=1.0, vtype=MOI.BINARY, name="on")
    output = model.addVars(g_count, t_count, lb=0.0, vtype=MOI.CONTINUOUS, name="p")
    startup = model.addVars(g_count, t_count, lb=0.0, ub=1.0, vtype=MOI.BINARY, name="start")

    model.addConstrs(
        (output[g, t] <= data.units[g].p_max * on[g, t] for g in range(g_count) for t in range(t_count)),
        name="output_ub",
    )
    model.addConstrs(
        (output[g, t] >= data.units[g].p_min * on[g, t] for g in range(g_count) for t in range(t_count)),
        name="output_lb",
    )
    model.addConstrs(
        (
            startup[g, t]
            >= on[g, t] - (data.units[g].initial_on if t == 0 else on[g, t - 1])
            for g in range(g_count)
            for t in range(t_count)
        ),
        name="startup",
    )
    model.addConstrs(
        (
            output[g, t] - (data.units[g].initial_output if t == 0 else output[g, t - 1])
            <= data.units[g].ramp_up
            + data.units[g].p_max
            * (1 - (data.units[g].initial_on if t == 0 else on[g, t - 1]))
            for g in range(g_count)
            for t in range(t_count)
        ),
        name="ramp_up",
    )
    model.addConstrs(
        (
            (data.units[g].initial_output if t == 0 else output[g, t - 1]) - output[g, t]
            <= data.units[g].ramp_down + data.units[g].p_max * (1 - on[g, t])
            for g in range(g_count)
            for t in range(t_count)
        ),
        name="ramp_down",
    )
    model.addConstrs(
        (quicksum(output[g, t] for g in range(g_count)) == data.loads[t] for t in range(t_count)),
        name="balance",
    )
    model.addConstrs(
        (
            quicksum(data.units[g].p_max * on[g, t] - output[g, t] for g in range(g_count))
            >= data.reserve_ratio * data.loads[t]
            for t in range(t_count)
        ),
        name="reserve",
    )
    model.setObjective(
        quicksum(
            data.units[g].marginal_cost * output[g, t]
            + data.units[g].no_load_cost * on[g, t]
            + data.units[g].startup_cost * startup[g, t]
            for g in range(g_count)
            for t in range(t_count)
        ),
        MOI.MINIMIZE,
    )
    model.setParam("Logging", 0)
    model.setBackend("copt", env=env)
    return model


def prepare_moirspy_copt() -> Any:
    from moirspy_copt import Env

    return Env(_copt_dll())


def build_moirspy_copt(data: MinimalUcData, env: Any) -> Any:
    """Build the same formulation through the low-level batched COPT binding."""

    from moirspy_copt import Model

    g_count, t_count = data.num_units, data.num_periods
    block = g_count * t_count
    model = Model("minimal-uc-moirspy-copt", env=env)
    model.set_optimizer_attr("Logging", 0)

    on = model.add_variables(block, vtypes=["B"] * block, lbs=[0.0] * block, ubs=[1.0] * block)
    output = model.add_variables(block, vtypes=["C"] * block, lbs=[0.0] * block)
    startup = model.add_variables(block, vtypes=["B"] * block, lbs=[0.0] * block, ubs=[1.0] * block)

    def idx(g: int, t: int) -> int:
        return g * t_count + t

    row_vars: list[list[int]] = []
    row_coeffs: list[list[float]] = []
    constants: list[float] = []
    senses: list[str] = []
    rhs: list[float] = []

    def row(vars_: list[int], coeffs: list[float], sense: str, bound: float) -> None:
        row_vars.append(vars_)
        row_coeffs.append(coeffs)
        constants.append(0.0)
        senses.append(sense)
        rhs.append(bound)

    for g, unit in enumerate(data.units):
        for t in range(t_count):
            i = idx(g, t)
            row([output[i], on[i]], [1.0, -unit.p_max], "<", 0.0)
            row([output[i], on[i]], [1.0, -unit.p_min], ">", 0.0)
            if t == 0:
                row([startup[i], on[i]], [1.0, -1.0], ">", -float(unit.initial_on))
                row(
                    [output[i]],
                    [1.0],
                    "<",
                    unit.initial_output + unit.ramp_up + unit.p_max * (1 - unit.initial_on),
                )
            else:
                previous = idx(g, t - 1)
                row([startup[i], on[i], on[previous]], [1.0, -1.0, 1.0], ">", 0.0)
                row(
                    [output[i], output[previous], on[previous]],
                    [1.0, -1.0, unit.p_max],
                    "<",
                    unit.ramp_up + unit.p_max,
                )
            row(
                [output[i], on[i]] if t == 0 else [output[idx(g, t - 1)], output[i], on[i]],
                [-1.0, unit.p_max] if t == 0 else [1.0, -1.0, unit.p_max],
                "<",
                unit.ramp_down + unit.p_max - (unit.initial_output if t == 0 else 0.0),
            )

    for t, load in enumerate(data.loads):
        row([output[idx(g, t)] for g in range(g_count)], [1.0] * g_count, "=", load)
        reserve_vars: list[int] = []
        reserve_coeffs: list[float] = []
        for g, unit in enumerate(data.units):
            reserve_vars.extend((on[idx(g, t)], output[idx(g, t)]))
            reserve_coeffs.extend((unit.p_max, -1.0))
        row(reserve_vars, reserve_coeffs, ">", data.reserve_ratio * load)

    model.add_constraints(row_vars, row_coeffs, constants, senses, rhs)
    objective_vars: list[int] = []
    objective_coeffs: list[float] = []
    for g, unit in enumerate(data.units):
        for t in range(t_count):
            i = idx(g, t)
            objective_vars.extend((output[i], on[i], startup[i]))
            objective_coeffs.extend((unit.marginal_cost, unit.no_load_cost, unit.startup_cost))
    model.set_objective(objective_vars, objective_coeffs, 0.0, -1)
    model.update()
    return model


def prepare_coptpy() -> Any:
    dll_handles: list[Any] = []
    try:
        import coptpy
    except ImportError:
        home = os.environ.get("COPT_HOME")
        if not home:
            raise
        python_dir = Path(home) / "lib" / "python"
        package_dir = python_dir / f"{sys.version_info.major}{sys.version_info.minor}"
        if not package_dir.is_dir():
            raise ImportError(f"COPT has no coptpy build for Python {sys.version_info.major}.{sys.version_info.minor}")
        sys.path.insert(0, str(package_dir))
        if sys.platform == "win32":
            # The installer normally copies copt_python.dll into site-packages.
            # Directly using the bundled module also needs its dependency folder.
            dll_handles.append(os.add_dll_directory(str(python_dir / "deps")))
            dll_handles.append(os.add_dll_directory(str(Path(home) / "bin")))
        import coptpy

    return coptpy.Envr(), dll_handles


def build_coptpy(data: MinimalUcData, env: Any) -> Any:
    import coptpy
    from coptpy import COPT

    g_count, t_count = data.num_units, data.num_periods
    copt_env, _dll_handles = env
    model = copt_env.createModel("minimal-uc-coptpy")
    model.setParam(COPT.Param.Logging, 0)
    on = model.addVars(g_count, t_count, lb=0.0, ub=1.0, vtype=COPT.BINARY, nameprefix="on")
    output = model.addVars(g_count, t_count, lb=0.0, vtype=COPT.CONTINUOUS, nameprefix="p")
    startup = model.addVars(g_count, t_count, lb=0.0, ub=1.0, vtype=COPT.BINARY, nameprefix="start")

    model.addConstrs(
        (output[g, t] <= data.units[g].p_max * on[g, t] for g in range(g_count) for t in range(t_count)),
        nameprefix="output_ub",
    )
    model.addConstrs(
        (output[g, t] >= data.units[g].p_min * on[g, t] for g in range(g_count) for t in range(t_count)),
        nameprefix="output_lb",
    )
    model.addConstrs(
        (
            startup[g, t] >= on[g, t] - (data.units[g].initial_on if t == 0 else on[g, t - 1])
            for g in range(g_count)
            for t in range(t_count)
        ),
        nameprefix="startup",
    )
    model.addConstrs(
        (
            output[g, t] - (data.units[g].initial_output if t == 0 else output[g, t - 1])
            <= data.units[g].ramp_up
            + data.units[g].p_max
            * (1 - (data.units[g].initial_on if t == 0 else on[g, t - 1]))
            for g in range(g_count)
            for t in range(t_count)
        ),
        nameprefix="ramp_up",
    )
    model.addConstrs(
        (
            (data.units[g].initial_output if t == 0 else output[g, t - 1]) - output[g, t]
            <= data.units[g].ramp_down + data.units[g].p_max * (1 - on[g, t])
            for g in range(g_count)
            for t in range(t_count)
        ),
        nameprefix="ramp_down",
    )
    model.addConstrs(
        (coptpy.quicksum(output[g, t] for g in range(g_count)) == data.loads[t] for t in range(t_count)),
        nameprefix="balance",
    )
    model.addConstrs(
        (
            coptpy.quicksum(data.units[g].p_max * on[g, t] - output[g, t] for g in range(g_count))
            >= data.reserve_ratio * data.loads[t]
            for t in range(t_count)
        ),
        nameprefix="reserve",
    )
    model.setObjective(
        coptpy.quicksum(
            data.units[g].marginal_cost * output[g, t]
            + data.units[g].no_load_cost * on[g, t]
            + data.units[g].startup_cost * startup[g, t]
            for g in range(g_count)
            for t in range(t_count)
        ),
        COPT.MINIMIZE,
    )
    model.update()
    return model


BUILDERS = {
    "moirspy": Builder("moirspy", prepare_moirspy, build_moirspy),
    "moirspy-copt": Builder("moirspy-copt", prepare_moirspy_copt, build_moirspy_copt),
    "coptpy": Builder("coptpy", prepare_coptpy, build_coptpy),
}
