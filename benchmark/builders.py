from __future__ import annotations

import os
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from benchmark.common import (
    STORAGE_BLOCKS,
    THERMAL_BLOCKS,
    MinimalUcData,
    iter_constraint_groups,
    iter_objective_terms,
)
from benchmark.copt_env import create_copt_env, create_legacy_copt_env
from benchmark.pyoptinterface_builders import (
    build_pyoptinterface_direct,
    build_pyoptinterface_rows,
    prepare_pyoptinterface,
)


@dataclass(slots=True)
class Builder:
    name: str
    prepare: Callable[[], Any]
    build: Callable[[MinimalUcData, Any], Any]


def prepare_moirspy() -> Any:
    return create_copt_env()


def _block_kind(block: str) -> tuple[str, float]:
    if block in {"thermal_x", "thermal_u", "storage_o", "storage_i"}:
        return "B", 1.0
    if block == "storage_soc":
        return "C", 1.0
    return "C", float("inf")


def _build_moirspy(
    data: MinimalUcData,
    env: Any,
    *,
    attach_early: bool,
    stage_times: dict[str, float] | None = None,
) -> Any:
    from moirspy import MOI, Model, dot

    last_mark = time.perf_counter() if stage_times is not None else 0.0

    def mark(stage: str) -> None:
        nonlocal last_mark
        if stage_times is not None:
            now = time.perf_counter()
            stage_times[stage] = stage_times.get(stage, 0.0) + now - last_mark
            last_mark = now

    model_name = f"complete-2bin-uc-moirspy-{'direct' if attach_early else 'late'}"
    model = (
        Model(model_name, backend="copt", env=env)
        if attach_early
        else Model(model_name)
    )
    mark("model")
    if attach_early:
        mark("attach")

    blocks: dict[str, Any] = {}
    for block in THERMAL_BLOCKS + STORAGE_BLOCKS:
        kind, upper = _block_kind(block)
        blocks[block] = model.addVars(
            data.layout.block_sizes[block],
            lb=0.0,
            ub=upper,
            vtype=MOI.BINARY if kind == "B" else MOI.CONTINUOUS,
            name=block,
        )
    mark("variables")

    def expression(row: Any) -> Any:
        return dot(
            (coefficient for _, coefficient in row.terms),
            (blocks[variable.block][variable.index] for variable, _ in row.terms),
        )

    def constraint(row: Any) -> Any:
        expr = expression(row)
        if row.sense == "<":
            return expr <= row.rhs
        if row.sense == ">":
            return expr >= row.rhs
        return expr == row.rhs

    for family, rows in iter_constraint_groups(data):
        model.addConstrs((constraint(row) for row in rows), name=family)
        mark(family)

    objective = tuple(iter_objective_terms(data))
    model.setObjective(
        dot(
            (coefficient for _, coefficient in objective),
            (blocks[variable.block][variable.index] for variable, _ in objective),
        ),
        MOI.MINIMIZE,
    )
    mark("objective")
    model.setParam("Logging", 0)
    mark("parameter")
    if not attach_early:
        model.setBackend("copt", env=env)
        mark("attach")
    return model


def build_moirspy_early(data: MinimalUcData, env: Any) -> Any:
    return _build_moirspy(data, env, attach_early=True)


def build_moirspy_direct_api(data: MinimalUcData, env: Any) -> Any:
    """Build directly from UC fields with the normal high-level modeling API.

    Unlike ``build_moirspy_early``, this path does not first materialize the
    benchmark's solver-independent ``LinearRow``/``VarRef`` representation.
    """

    from benchmark.direct_uc_moirspy import build_uc_model

    model, _variables = build_uc_model(data, env, attach_early=True, logging=0)
    return model


def build_moirspy_late(data: MinimalUcData, env: Any) -> Any:
    return _build_moirspy(data, env, attach_early=False)


def profile_moirspy(
    data: MinimalUcData, env: Any, *, attach_early: bool = False
) -> tuple[Any, dict[str, float]]:
    timings: dict[str, float] = {}
    model = _build_moirspy(data, env, attach_early=attach_early, stage_times=timings)
    return model, timings


def prepare_moirspy_copt() -> Any:
    return create_legacy_copt_env()


def build_moirspy_copt(data: MinimalUcData, env: Any) -> Any:
    from moirspy_copt import Model

    model = Model("complete-2bin-uc-moirspy-copt", env=env)
    model.set_optimizer_attr("Logging", 0)
    blocks: dict[str, list[int]] = {}
    for block in THERMAL_BLOCKS + STORAGE_BLOCKS:
        kind, upper = _block_kind(block)
        size = data.layout.block_sizes[block]
        blocks[block] = model.add_variables(
            size,
            vtypes=[kind] * size,
            lbs=[0.0] * size,
            ubs=[upper] * size,
        )

    row_vars: list[list[int]] = []
    row_coeffs: list[list[float]] = []
    senses: list[str] = []
    rhs: list[float] = []
    for _, rows in iter_constraint_groups(data):
        for row in rows:
            row_vars.append([blocks[var.block][var.index] for var, _ in row.terms])
            row_coeffs.append([coefficient for _, coefficient in row.terms])
            senses.append(row.sense)
            rhs.append(row.rhs)
    model.add_constraints(row_vars, row_coeffs, [0.0] * len(rhs), senses, rhs)

    objective = tuple(iter_objective_terms(data))
    model.set_objective(
        [blocks[var.block][var.index] for var, _ in objective],
        [coefficient for _, coefficient in objective],
        0.0,
        -1,
    )
    model.update()
    return model


def prepare_coptpy() -> Any:
    handles: list[Any] = []
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
            handles.append(os.add_dll_directory(str(python_dir / "deps")))
            handles.append(os.add_dll_directory(str(Path(home) / "bin")))
        import coptpy
    return coptpy.Envr(), handles


def build_coptpy(data: MinimalUcData, env: Any) -> Any:
    import coptpy
    from coptpy import COPT, ExprBuilder

    copt_env, _handles = env
    model = copt_env.createModel("complete-2bin-uc-coptpy")
    model.setParam(COPT.Param.Logging, 0)
    blocks: dict[str, Any] = {}
    for block in THERMAL_BLOCKS + STORAGE_BLOCKS:
        kind, upper = _block_kind(block)
        blocks[block] = model.addVars(
            data.layout.block_sizes[block],
            lb=0.0,
            ub=upper,
            vtype=COPT.BINARY if kind == "B" else COPT.CONTINUOUS,
            nameprefix=block,
        )

    def constraint(row: Any) -> Any:
        variables = [blocks[var.block][var.index] for var, _ in row.terms]
        coefficients = [coefficient for _, coefficient in row.terms]
        expr = ExprBuilder(variables, coefficients)
        if row.sense == "<":
            return expr <= row.rhs
        if row.sense == ">":
            return expr >= row.rhs
        return expr == row.rhs

    for family, rows in iter_constraint_groups(data):
        model.addConstrs((constraint(row) for row in rows), nameprefix=family)
    objective = tuple(iter_objective_terms(data))
    model.setObjective(
        ExprBuilder(
            [blocks[var.block][var.index] for var, _ in objective],
            [coefficient for _, coefficient in objective],
        ),
        COPT.MINIMIZE,
    )
    model.update()
    return model


BUILDERS = {
    "moirspy-early": Builder("moirspy-early", prepare_moirspy, build_moirspy_early),
    "moirspy-direct-api": Builder(
        "moirspy-direct-api", prepare_moirspy, build_moirspy_direct_api
    ),
    "moirspy-late": Builder("moirspy-late", prepare_moirspy, build_moirspy_late),
    "moirspy-copt": Builder("moirspy-copt", prepare_moirspy_copt, build_moirspy_copt),
    "coptpy": Builder("coptpy", prepare_coptpy, build_coptpy),
    "pyoptinterface-rows": Builder(
        "pyoptinterface-rows", prepare_pyoptinterface, build_pyoptinterface_rows
    ),
    "pyoptinterface-direct": Builder(
        "pyoptinterface-direct", prepare_pyoptinterface, build_pyoptinterface_direct
    ),
}
