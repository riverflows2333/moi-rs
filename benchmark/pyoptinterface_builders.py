"""PyOptInterface builders for the complete 2-bin UC benchmark.

``build_pyoptinterface_rows`` consumes the same ``LinearRow`` stream as the
other cross-framework builders. ``build_pyoptinterface_direct`` writes the UC
formulation directly with PyOptInterface's normal expression API and does not
construct ``VarRef`` or ``LinearRow`` objects.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from benchmark.common import (
    STORAGE_BLOCKS,
    THERMAL_BLOCKS,
    MinimalUcData,
    iter_constraint_groups,
    iter_objective_terms,
)


def prepare_pyoptinterface() -> Any:
    """Create the reusable COPT environment outside the timed region."""

    from pyoptinterface import copt

    return copt.Env()


def _new_model(env: Any, _name: str) -> tuple[Any, Any]:
    import pyoptinterface as poi
    from pyoptinterface import copt

    model = copt.Model(env)
    model.set_model_attribute(poi.ModelAttribute.Silent, True)
    return model, poi


def _variable_kind(block: str) -> tuple[bool, float]:
    if block in {"thermal_x", "thermal_u", "storage_o", "storage_i"}:
        return True, 1.0
    if block == "storage_soc":
        return False, 1.0
    return False, 1.0e30


def _add_variable_blocks(model: Any, poi: Any, data: MinimalUcData) -> dict[str, list[Any]]:
    blocks: dict[str, list[Any]] = {}
    for block in THERMAL_BLOCKS + STORAGE_BLOCKS:
        binary, upper = _variable_kind(block)
        domain = poi.VariableDomain.Binary if binary else poi.VariableDomain.Continuous
        blocks[block] = [
            model.add_variable(domain=domain, lb=0.0, ub=upper)
            for _ in range(data.layout.block_sizes[block])
        ]
    return blocks


def _sense(poi: Any, sense: str) -> Any:
    if sense == "<":
        return poi.Leq
    if sense == ">":
        return poi.Geq
    return poi.Eq


def build_pyoptinterface_rows(data: MinimalUcData, env: Any) -> Any:
    """Build from the exact solver-independent rows used by all other builders."""

    model, poi = _new_model(env, "complete-2bin-uc-pyoptinterface-rows")
    blocks = _add_variable_blocks(model, poi, data)

    for _, rows in iter_constraint_groups(data):
        for row in rows:
            expression = poi.ScalarAffineFunction()
            expression.reserve(len(row.terms))
            for variable, coefficient in row.terms:
                expression.add_term(blocks[variable.block][variable.index], coefficient)
            model.add_linear_constraint(expression, _sense(poi, row.sense), row.rhs)

    objective = tuple(iter_objective_terms(data))
    expression = poi.ScalarAffineFunction()
    expression.reserve(len(objective))
    for variable, coefficient in objective:
        expression.add_term(blocks[variable.block][variable.index], coefficient)
    model.set_objective(expression, poi.ObjectiveSense.Minimize)
    return model


@dataclass(slots=True)
class _DirectVariables:
    thermal_p: list[Any]
    thermal_x: list[Any]
    thermal_u: list[Any]
    thermal_cost_p: list[Any]
    thermal_cost_u: list[Any]
    storage_op: list[Any]
    storage_ip: list[Any]
    storage_o: list[Any]
    storage_i: list[Any]
    storage_soc: list[Any]
    storage_cost_o: list[Any]
    storage_cost_i: list[Any]


def _direct_variables(blocks: dict[str, list[Any]]) -> _DirectVariables:
    return _DirectVariables(**blocks)


def _at(values: list[Any], entity: int, period: int, periods: int) -> Any:
    return values[entity * periods + period]


def _add_thermal_constraints(
    model: Any, poi: Any, data: MinimalUcData, variables: _DirectVariables
) -> None:
    periods = data.num_periods
    delta = 96.0 / periods

    for g, unit in enumerate(data.units):
        p = lambda t: _at(variables.thermal_p, g, t, periods)
        x = lambda t: _at(variables.thermal_x, g, t, periods)
        u = lambda t: _at(variables.thermal_u, g, t, periods)
        cost_p = lambda t: _at(variables.thermal_cost_p, g, t, periods)
        cost_u = lambda t: _at(variables.thermal_cost_u, g, t, periods)

        minimum_on = max(1, int(unit.minimum_on / delta))
        minimum_off = max(1, int(unit.minimum_off / delta))
        remaining = (
            unit.minimum_on if unit.initial_on else unit.minimum_off
        ) - unit.initial_duration
        hold = max(1, int(remaining / delta)) if remaining >= 0.5 else 0
        for t in range(min(hold, periods)):
            model.add_linear_constraint(x(t), poi.Eq, unit.initial_on)

        model.add_linear_constraint(x(0) - u(0), poi.Leq, unit.initial_on)
        for t in range(1, periods):
            model.add_linear_constraint(x(t) - x(t - 1) - u(t), poi.Leq, 0.0)

        for t in range(periods):
            start = max(0, t - minimum_on + 1)
            starts = poi.quicksum(u(k) for k in range(start, t + 1))
            model.add_linear_constraint(starts - x(t), poi.Leq, 0.0)

            start = max(0, t - minimum_off + 1)
            starts = poi.quicksum(u(k) for k in range(start, t + 1))
            if start == 0:
                model.add_linear_constraint(starts, poi.Leq, 1 - unit.initial_on)
            else:
                model.add_linear_constraint(
                    starts + x(t - minimum_off), poi.Leq, 1.0
                )

        for t in range(periods):
            model.add_linear_constraint(p(t) - unit.p_max * x(t), poi.Leq, 0.0)
            model.add_linear_constraint(p(t) - unit.p_min * x(t), poi.Geq, 0.0)

        if unit.p_max - unit.p_min > min(unit.ramp_up, unit.ramp_down):
            ramp_up = unit.ramp_up * delta
            ramp_down = unit.ramp_down * delta
            model.add_linear_constraint(
                p(0),
                poi.Leq,
                unit.initial_output
                + ramp_up * unit.initial_on
                + unit.p_max * (1 - unit.initial_on),
            )
            model.add_linear_constraint(
                -p(0) + (unit.p_max - ramp_down) * x(0),
                poi.Leq,
                unit.p_max - unit.initial_output,
            )
            for t in range(1, periods):
                model.add_linear_constraint(
                    p(t)
                    - p(t - 1)
                    + (unit.p_max - ramp_up) * x(t - 1),
                    poi.Leq,
                    unit.p_max,
                )
                model.add_linear_constraint(
                    p(t - 1)
                    - p(t)
                    + (unit.p_max - ramp_down) * x(t),
                    poi.Leq,
                    unit.p_max,
                )

        for t in range(periods):
            for slope, intercept in unit.cost_lines:
                model.add_linear_constraint(
                    cost_p(t) - delta * slope * p(t) - delta * intercept * x(t),
                    poi.Geq,
                    0.0,
                )
            model.add_linear_constraint(
                cost_u(t) - unit.startup_cost * u(t), poi.Eq, 0.0
            )


def _add_minimum_state(
    model: Any,
    poi: Any,
    values: list[Any],
    entity: int,
    minimum: int,
    periods: int,
) -> None:
    if minimum < 2:
        return
    for t in range(1, periods):
        length = min(minimum, periods - t)
        expression = poi.quicksum(
            _at(values, entity, tau, periods) for tau in range(t, t + length)
        )
        expression -= length * _at(values, entity, t, periods)
        expression += length * _at(values, entity, t - 1, periods)
        model.add_linear_constraint(expression, poi.Geq, 0.0)


def _add_storage_constraints(
    model: Any, poi: Any, data: MinimalUcData, variables: _DirectVariables
) -> None:
    periods = data.num_periods
    delta = 96.0 / periods
    energy_factor = 0.25 * delta

    for e, storage in enumerate(data.storages):
        op = lambda t: _at(variables.storage_op, e, t, periods)
        ip = lambda t: _at(variables.storage_ip, e, t, periods)
        o = lambda t: _at(variables.storage_o, e, t, periods)
        i = lambda t: _at(variables.storage_i, e, t, periods)
        soc = lambda t: _at(variables.storage_soc, e, t, periods)
        cost_o = lambda t: _at(variables.storage_cost_o, e, t, periods)
        cost_i = lambda t: _at(variables.storage_cost_i, e, t, periods)

        for t in range(periods):
            model.add_linear_constraint(
                op(t) - storage.minimum_discharge_power * o(t), poi.Geq, 0.0
            )
            model.add_linear_constraint(
                op(t) - storage.maximum_discharge * o(t), poi.Leq, 0.0
            )
            model.add_linear_constraint(
                ip(t) - storage.charge_power * i(t), poi.Eq, 0.0
            )
            model.add_linear_constraint(o(t) + i(t), poi.Leq, 1.0)
            if t + 1 < periods:
                model.add_linear_constraint(o(t) + i(t + 1), poi.Leq, 1.0)
                model.add_linear_constraint(i(t) + o(t + 1), poi.Leq, 1.0)

            cumulative = poi.quicksum(
                energy_factor * (op(tau) - ip(tau)) for tau in range(t + 1)
            )
            model.add_linear_constraint(
                storage.maximum_energy * soc(t) + cumulative,
                poi.Eq,
                storage.initial_energy,
            )

        model.add_linear_constraint(
            soc(periods - 1),
            poi.Geq,
            storage.final_energy / storage.maximum_energy,
        )

        minimum_discharge = max(1, int(storage.minimum_discharge_time / delta))
        minimum_charge = max(1, int(storage.minimum_charge / delta))
        minimum_off = max(1, int(storage.minimum_off / delta))
        model.add_linear_constraint(
            poi.quicksum(o(t) for t in range(min(minimum_discharge, periods)))
            - minimum_discharge * o(0),
            poi.Geq,
            0.0,
        )
        model.add_linear_constraint(
            poi.quicksum(i(t) for t in range(min(minimum_charge, periods)))
            - minimum_charge * i(0),
            poi.Geq,
            0.0,
        )
        _add_minimum_state(
            model, poi, variables.storage_o, e, minimum_discharge, periods
        )
        _add_minimum_state(
            model, poi, variables.storage_i, e, minimum_charge, periods
        )
        if storage.minimum_off >= 2:
            for t in range(1, periods):
                length = min(minimum_off, periods - t)
                expression = poi.quicksum(
                    -o(tau) - i(tau) for tau in range(t, t + length)
                )
                expression -= length * o(t - 1)
                expression -= length * i(t - 1)
                expression += length * o(t)
                expression += length * i(t)
                model.add_linear_constraint(expression, poi.Geq, -float(length))

        for t in range(periods):
            for slope, intercept in storage.discharge_cost_lines:
                model.add_linear_constraint(
                    cost_o(t) - delta * slope * op(t) - delta * intercept * o(t),
                    poi.Geq,
                    0.0,
                )
            model.add_linear_constraint(
                cost_i(t) - delta * storage.charge_cost * i(t), poi.Eq, 0.0
            )


def _add_system_constraints(
    model: Any, poi: Any, data: MinimalUcData, variables: _DirectVariables
) -> None:
    periods = data.num_periods
    for section in data.sections:
        for t in range(periods):
            flow = poi.quicksum(
                coefficient * _at(variables.thermal_p, g, t, periods)
                for g, coefficient in enumerate(section.thermal_sensitivity)
            )
            flow += poi.quicksum(
                coefficient
                * (
                    _at(variables.storage_op, e, t, periods)
                    - _at(variables.storage_ip, e, t, periods)
                )
                for e, coefficient in enumerate(section.storage_sensitivity)
            )
            model.add_linear_constraint(
                flow, poi.Leq, section.limit + section.load_offset[t]
            )
            model.add_linear_constraint(
                flow, poi.Geq, -section.limit + section.load_offset[t]
            )

    for t, load in enumerate(data.loads):
        output = poi.quicksum(
            _at(variables.thermal_p, g, t, periods)
            for g in range(data.num_units)
        )
        output += poi.quicksum(
            _at(variables.storage_op, e, t, periods)
            - _at(variables.storage_ip, e, t, periods)
            for e in range(data.num_storages)
        )
        model.add_linear_constraint(output, poi.Eq, load)

        reserve = poi.quicksum(
            unit.p_max * _at(variables.thermal_x, g, t, periods)
            - _at(variables.thermal_p, g, t, periods)
            for g, unit in enumerate(data.units)
        )
        model.add_linear_constraint(reserve, poi.Geq, data.reserve_ratio * load)


def build_pyoptinterface_direct(data: MinimalUcData, env: Any) -> Any:
    """Build the same UC formulation directly with PyOptInterface expressions."""

    model, poi = _new_model(env, "complete-2bin-uc-pyoptinterface-direct")
    variables = _direct_variables(_add_variable_blocks(model, poi, data))
    _add_thermal_constraints(model, poi, data, variables)
    _add_storage_constraints(model, poi, data, variables)
    _add_system_constraints(model, poi, data, variables)

    objective = poi.quicksum(
        _at(variables.thermal_cost_p, g, t, data.num_periods)
        + _at(variables.thermal_cost_u, g, t, data.num_periods)
        for g in range(data.num_units)
        for t in range(data.num_periods)
    )
    objective += poi.quicksum(
        _at(variables.storage_cost_o, e, t, data.num_periods)
        - _at(variables.storage_cost_i, e, t, data.num_periods)
        for e in range(data.num_storages)
        for t in range(data.num_periods)
    )
    model.set_objective(objective, poi.ObjectiveSense.Minimize)
    return model
