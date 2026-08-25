from __future__ import annotations

from dataclasses import dataclass
import hashlib
from pathlib import Path
import struct
from typing import Iterable, Iterator


THERMAL_BLOCKS = ("thermal_p", "thermal_x", "thermal_u", "thermal_cost_p", "thermal_cost_u")
STORAGE_BLOCKS = (
    "storage_op", "storage_ip", "storage_o", "storage_i", "storage_soc",
    "storage_cost_o", "storage_cost_i",
)
VARIABLE_BLOCKS = THERMAL_BLOCKS + STORAGE_BLOCKS
CONSTRAINT_FAMILIES = (
    "thermal_state", "thermal_output", "thermal_ramping", "thermal_cost",
    "storage_output", "storage_state", "storage_energy", "storage_duration",
    "storage_cost", "sections", "balance", "reserve",
)


@dataclass(frozen=True, slots=True)
class ThermalUnit:
    unit_id: int
    p_max: float
    p_min: float
    ramp_up: float
    ramp_down: float
    minimum_on: int
    minimum_off: int
    startup_cost: float
    initial_on: int
    initial_output: float
    initial_duration: int
    cost_lines: tuple[tuple[float, float], ...]


@dataclass(frozen=True, slots=True)
class StorageUnit:
    storage_id: int
    minimum_charge: int
    minimum_off: int
    minimum_discharge_time: int
    initial_energy: float
    final_energy: float
    maximum_energy: float
    charge_power: float
    maximum_discharge: float
    minimum_discharge_power: float
    same_bus_unit: int
    charge_cost: float
    discharge_cost_lines: tuple[tuple[float, float], ...]


@dataclass(frozen=True, slots=True)
class Section:
    section_id: int
    branches: tuple[int, ...]
    limit: float
    thermal_sensitivity: tuple[float, ...]
    storage_sensitivity: tuple[float, ...]
    load_offset: tuple[float, ...]


@dataclass(frozen=True, slots=True)
class VarRef:
    block: str
    index: int


@dataclass(frozen=True, slots=True)
class LinearRow:
    terms: tuple[tuple[VarRef, float], ...]
    sense: str
    rhs: float


@dataclass(frozen=True, slots=True)
class VariableLayout:
    block_sizes: dict[str, int]
    block_offsets: dict[str, int]
    total: int

    def global_index(self, variable: VarRef) -> int:
        return self.block_offsets[variable.block] + variable.index


@dataclass(frozen=True, slots=True)
class MinimalUcData:
    """Complete, direct 2-bin UC input shared by every benchmark builder."""

    case_dir: Path
    loads: tuple[float, ...]
    units: tuple[ThermalUnit, ...]
    storages: tuple[StorageUnit, ...]
    sections: tuple[Section, ...]
    reserve_ratio: float = 0.10

    @property
    def num_units(self) -> int:
        return len(self.units)

    @property
    def num_storages(self) -> int:
        return len(self.storages)

    @property
    def num_periods(self) -> int:
        return len(self.loads)

    @property
    def layout(self) -> VariableLayout:
        gt, et = self.num_units * self.num_periods, self.num_storages * self.num_periods
        sizes = {**{b: gt for b in THERMAL_BLOCKS}, **{b: et for b in STORAGE_BLOCKS}}
        offsets: dict[str, int] = {}
        total = 0
        for block in VARIABLE_BLOCKS:
            offsets[block], total = total, total + sizes[block]
        return VariableLayout(sizes, offsets, total)

    @property
    def num_variables(self) -> int:
        return self.layout.total

    @property
    def num_constraints(self) -> int:
        return sum(1 for _, rows in iter_constraint_groups(self) for _ in rows)

    @property
    def num_nonzeros(self) -> int:
        return sum(len(row.terms) for _, rows in iter_constraint_groups(self) for row in rows)


@dataclass(frozen=True, slots=True)
class ConstraintFamilyStats:
    rows: int
    nonzeros: int


@dataclass(frozen=True, slots=True)
class FormulationStats:
    variables: int
    constraints: int
    nonzeros: int
    objective_nonzeros: int
    families: tuple[tuple[str, ConstraintFamilyStats], ...]
    fingerprint: str | None = None

    def as_dict(self) -> dict[str, object]:
        result: dict[str, object] = {
            "variables": self.variables,
            "constraints": self.constraints,
            "nonzeros": self.nonzeros,
            "objective_nonzeros": self.objective_nonzeros,
            "families": {
                name: {"rows": stats.rows, "nonzeros": stats.nonzeros}
                for name, stats in self.families
            },
        }
        if self.fingerprint is not None:
            result["fingerprint"] = self.fingerprint
        return result


def formulation_stats(
    data: MinimalUcData, *, include_fingerprint: bool = False
) -> FormulationStats:
    """Return solver-independent statistics for Direct/Cached comparisons.

    The optional fingerprint covers the variable layout, every linear row, and
    the objective using a versioned little-endian encoding. It is deliberately
    computed outside benchmark timing regions.
    """

    layout = data.layout
    digest = hashlib.sha256() if include_fingerprint else None
    if digest is not None:
        digest.update(b"moi-rs-complete-2bin-v1\0")
        for block in VARIABLE_BLOCKS:
            encoded = block.encode("utf-8")
            digest.update(struct.pack("<Q", len(encoded)))
            digest.update(encoded)
            digest.update(struct.pack("<Q", layout.block_sizes[block]))

    families: list[tuple[str, ConstraintFamilyStats]] = []
    constraints = 0
    nonzeros = 0
    for family, rows in iter_constraint_groups(data):
        family_rows = 0
        family_nonzeros = 0
        if digest is not None:
            encoded = family.encode("utf-8")
            digest.update(struct.pack("<Q", len(encoded)))
            digest.update(encoded)
        for row in rows:
            family_rows += 1
            family_nonzeros += len(row.terms)
            if digest is not None:
                digest.update(row.sense.encode("ascii"))
                digest.update(struct.pack("<dQ", row.rhs, len(row.terms)))
                for variable, coefficient in row.terms:
                    digest.update(
                        struct.pack(
                            "<Qd", layout.global_index(variable), coefficient
                        )
                    )
        constraints += family_rows
        nonzeros += family_nonzeros
        families.append(
            (family, ConstraintFamilyStats(family_rows, family_nonzeros))
        )

    objective_nonzeros = 0
    if digest is not None:
        digest.update(b"objective\0")
    for variable, coefficient in iter_objective_terms(data):
        objective_nonzeros += 1
        if digest is not None:
            digest.update(
                struct.pack("<Qd", layout.global_index(variable), coefficient)
            )

    return FormulationStats(
        variables=layout.total,
        constraints=constraints,
        nonzeros=nonzeros,
        objective_nonzeros=objective_nonzeros,
        families=tuple(families),
        fingerprint=digest.hexdigest() if digest is not None else None,
    )


def _numeric_rows(path: Path, columns: int | None = None) -> list[list[float]]:
    if not path.is_file():
        raise FileNotFoundError(f"missing input file: {path}")
    rows: list[list[float]] = []
    with path.open("r", encoding="gb18030", errors="replace") as stream:
        next(stream, None)
        for line_number, line in enumerate(stream, 2):
            fields = line.split()
            if not fields:
                continue
            if columns is not None and len(fields) != columns:
                raise ValueError(f"{path}:{line_number}: expected {columns} columns, got {len(fields)}")
            try:
                rows.append([float(field) for field in fields])
            except ValueError as error:
                raise ValueError(f"{path}:{line_number}: non-numeric data") from error
    return rows


def _indexed(rows: list[list[float]], filename: str) -> dict[int, tuple[float, ...]]:
    result: dict[int, tuple[float, ...]] = {}
    for expected, row in enumerate(rows, 1):
        index = int(row[0])
        if index != expected or index in result:
            raise ValueError(f"{filename} ids must be unique, consecutive, and start at 1")
        result[index] = tuple(row[1:])
    return result


def _piecewise(prices: Iterable[float], widths: Iterable[float], start: float) -> tuple[tuple[float, float], ...]:
    prices, widths = tuple(prices), tuple(widths)
    if len(prices) != len(widths) + 1:
        raise ValueError("piecewise prices must contain one more point than widths")
    lines, cumulative = [], start
    for index, width in enumerate(widths):
        if width <= 0.0:
            raise ValueError("piecewise widths must be positive")
        cumulative += width
        slope = (prices[index + 1] - prices[index]) / width
        lines.append((slope, prices[index + 1] - cumulative * slope))
    return tuple(lines)


def load_minimal_uc(case_dir: str | Path) -> MinimalUcData:
    supplied = Path(case_dir).resolve()
    model_dir = supplied / "model" if (supplied / "model").is_dir() else supplied
    display_dir = supplied if model_dir != supplied else supplied.parent
    loads = _numeric_rows(model_dir / "slf.txt", 2)
    periods = [int(row[0]) for row in loads]
    if not periods or periods != list(range(1, len(periods) + 1)):
        raise ValueError("slf.txt periods must be consecutive and start at 1")

    unit_widths = _indexed(_numeric_rows(model_dir / "bidcapacity.txt", 6), "bidcapacity.txt")
    unit_prices = _indexed(_numeric_rows(model_dir / "bidprice.txt", 8), "bidprice.txt")
    units: list[ThermalUnit] = []
    for row in _numeric_rows(model_dir / "unitdata.txt", 12):
        uid, p_max, p_min, initial_on = int(row[0]), row[2], row[3], int(row[9])
        if not 0 <= p_min <= p_max or initial_on not in (0, 1):
            raise ValueError(f"invalid thermal unit {uid}")
        units.append(ThermalUnit(
            uid, p_max, p_min, row[4], row[5], int(row[6]), int(row[7]), row[8],
            initial_on, row[10], int(row[11]),
            _piecewise(unit_prices[uid][1:], unit_widths[uid], p_min),
        ))

    storage_widths = _indexed(_numeric_rows(model_dir / "stbidcapacity.txt", 7), "stbidcapacity.txt")
    storage_prices = _indexed(_numeric_rows(model_dir / "stbidprice.txt", 8), "stbidprice.txt")
    storages: list[StorageUnit] = []
    for row in _numeric_rows(model_dir / "storagebasic.txt", 13):
        sid, prices, widths = int(row[0]), storage_prices[int(row[0])], storage_widths[int(row[0])]
        if row[6] <= 0:
            raise ValueError(f"invalid maximum energy for storage unit {sid}")
        storages.append(StorageUnit(
            sid, int(row[1]), int(row[2]), int(row[3]), row[4], row[5], row[6],
            row[8], row[9], row[10], int(row[12]), abs(prices[0]),
            _piecewise(prices[1:], widths[1:], 0.0),
        ))
    sections = _load_sections(model_dir, len(units), storages, len(periods))
    return MinimalUcData(display_dir, tuple(row[1] for row in loads), tuple(units), tuple(storages), sections)


def _load_sections(model_dir: Path, num_units: int, storages: list[StorageUnit], periods: int) -> tuple[Section, ...]:
    definitions, required = [], set()
    section_counts = _indexed(_numeric_rows(model_dir / "sectionnum.txt", 2), "sectionnum.txt")
    for row in _numeric_rows(model_dir / "section.txt"):
        if len(row) < 3:
            raise ValueError("section.txt rows require an id, branches, and a limit")
        branches = tuple(int(value) for value in row[1:-1])
        section_id = int(row[0])
        if section_id not in section_counts or int(section_counts[section_id][0]) != len(branches):
            raise ValueError(f"section {section_id} branch count does not match sectionnum.txt")
        definitions.append((section_id, branches, row[-1]))
        required.update(branches)
    sensitivities: dict[int, tuple[float, ...]] = {}
    offsets = {branch: [0.0] * periods for branch in required}
    in_sens = in_data = False
    with (model_dir / "branch_1.log").open("r", encoding="gb18030", errors="replace") as stream:
        for line in stream:
            if line.startswith("<BranchUnitSensi"):
                in_sens, in_data = True, False
                continue
            if line.startswith("</BranchUnitSensi"):
                in_sens = False
                continue
            if line.startswith("<BranchData"):
                in_data = True
                continue
            if line.startswith("</BranchData"):
                in_data = False
                continue
            if not line.startswith("#"):
                continue
            fields = line.split()
            if in_sens and len(fields) >= 4:
                branch = int(fields[1])
                if branch in required:
                    values = tuple(float(value) for value in fields[4:])
                    if len(values) != num_units:
                        raise ValueError(f"branch {branch} has {len(values)} sensitivities, expected {num_units}")
                    sensitivities[branch] = values
            elif in_data and len(fields) == 7:
                period, branch = int(fields[2]), int(fields[3])
                if branch in offsets and 1 <= period <= periods:
                    offsets[branch][period - 1] = float(fields[-1])
    missing = required - sensitivities.keys()
    if missing:
        raise ValueError(f"missing branch sensitivities: {sorted(missing)}")
    result = []
    for sid, branches, limit in definitions:
        thermal = tuple(sum(sensitivities[b][g] for b in branches) for g in range(num_units))
        storage = tuple(thermal[item.same_bus_unit - 1] for item in storages)
        load_offset = tuple(sum(offsets[b][t] for b in branches) for t in range(periods))
        result.append(Section(sid, branches, limit, thermal, storage, load_offset))
    return tuple(result)


def _v(block: str, entity: int, period: int, periods: int) -> VarRef:
    return VarRef(block, entity * periods + period)


def _row(terms: Iterable[tuple[VarRef, float]], sense: str, rhs: float) -> LinearRow:
    merged: dict[VarRef, float] = {}
    for variable, coefficient in terms:
        merged[variable] = merged.get(variable, 0.0) + coefficient
    return LinearRow(tuple((var, coef) for var, coef in merged.items() if coef != 0.0), sense, float(rhs))


def iter_constraint_groups(data: MinimalUcData) -> Iterator[tuple[str, Iterator[LinearRow]]]:
    yield "thermal_state", _thermal_state(data)
    yield "thermal_output", _thermal_output(data)
    yield "thermal_ramping", _thermal_ramping(data)
    yield "thermal_cost", _thermal_cost(data)
    yield "storage_output", _storage_output(data)
    yield "storage_state", _storage_state(data)
    yield "storage_energy", _storage_energy(data)
    yield "storage_duration", _storage_duration(data)
    yield "storage_cost", _storage_cost(data)
    yield "sections", _sections(data)
    yield "balance", _balance(data)
    yield "reserve", _reserve(data)


def iter_objective_terms(data: MinimalUcData) -> Iterator[tuple[VarRef, float]]:
    periods = data.num_periods
    for g in range(data.num_units):
        for t in range(periods):
            yield _v("thermal_cost_p", g, t, periods), 1.0
            yield _v("thermal_cost_u", g, t, periods), 1.0
    for e in range(data.num_storages):
        for t in range(periods):
            yield _v("storage_cost_o", e, t, periods), 1.0
            yield _v("storage_cost_i", e, t, periods), -1.0


def _thermal_state(data: MinimalUcData) -> Iterator[LinearRow]:
    periods, delta = data.num_periods, 96.0 / data.num_periods
    for g, unit in enumerate(data.units):
        on, off = max(1, int(unit.minimum_on / delta)), max(1, int(unit.minimum_off / delta))
        remaining = (unit.minimum_on if unit.initial_on else unit.minimum_off) - unit.initial_duration
        hold = max(1, int(remaining / delta)) if remaining >= 0.5 else 0
        for t in range(min(hold, periods)):
            yield _row([(_v("thermal_x", g, t, periods), 1)], "=", unit.initial_on)
        yield _row([(_v("thermal_x", g, 0, periods), 1), (_v("thermal_u", g, 0, periods), -1)], "<", unit.initial_on)
        for t in range(1, periods):
            yield _row([(_v("thermal_x", g, t, periods), 1), (_v("thermal_x", g, t - 1, periods), -1), (_v("thermal_u", g, t, periods), -1)], "<", 0)
        for t in range(periods):
            start = max(0, t - on + 1)
            yield _row([*[( _v("thermal_u", g, k, periods), 1) for k in range(start, t + 1)], (_v("thermal_x", g, t, periods), -1)], "<", 0)
            start = max(0, t - off + 1)
            terms = [(_v("thermal_u", g, k, periods), 1) for k in range(start, t + 1)]
            if start == 0:
                yield _row(terms, "<", 1 - unit.initial_on)
            else:
                yield _row([*terms, (_v("thermal_x", g, t - off, periods), 1)], "<", 1)


def _thermal_output(data: MinimalUcData) -> Iterator[LinearRow]:
    periods = data.num_periods
    for g, unit in enumerate(data.units):
        for t in range(periods):
            p, x = _v("thermal_p", g, t, periods), _v("thermal_x", g, t, periods)
            yield _row([(p, 1), (x, -unit.p_max)], "<", 0)
            yield _row([(p, 1), (x, -unit.p_min)], ">", 0)


def _thermal_ramping(data: MinimalUcData) -> Iterator[LinearRow]:
    periods, delta = data.num_periods, 96.0 / data.num_periods
    for g, unit in enumerate(data.units):
        up, down = unit.ramp_up * delta, unit.ramp_down * delta
        if unit.p_max - unit.p_min <= min(unit.ramp_up, unit.ramp_down):
            continue
        p0, x0 = _v("thermal_p", g, 0, periods), _v("thermal_x", g, 0, periods)
        yield _row([(p0, 1)], "<", unit.initial_output + up * unit.initial_on + unit.p_max * (1 - unit.initial_on))
        yield _row([(p0, -1), (x0, unit.p_max - down)], "<", unit.p_max - unit.initial_output)
        for t in range(1, periods):
            previous, current = _v("thermal_p", g, t - 1, periods), _v("thermal_p", g, t, periods)
            yield _row([(current, 1), (previous, -1), (_v("thermal_x", g, t - 1, periods), unit.p_max - up)], "<", unit.p_max)
            yield _row([(previous, 1), (current, -1), (_v("thermal_x", g, t, periods), unit.p_max - down)], "<", unit.p_max)


def _thermal_cost(data: MinimalUcData) -> Iterator[LinearRow]:
    periods, delta = data.num_periods, 96.0 / data.num_periods
    for g, unit in enumerate(data.units):
        for t in range(periods):
            p, x, cost = _v("thermal_p", g, t, periods), _v("thermal_x", g, t, periods), _v("thermal_cost_p", g, t, periods)
            for slope, intercept in unit.cost_lines:
                yield _row([(cost, 1), (p, -delta * slope), (x, -delta * intercept)], ">", 0)
            yield _row([(_v("thermal_cost_u", g, t, periods), 1), (_v("thermal_u", g, t, periods), -unit.startup_cost)], "=", 0)


def _storage_output(data: MinimalUcData) -> Iterator[LinearRow]:
    periods = data.num_periods
    for e, storage in enumerate(data.storages):
        for t in range(periods):
            op, ip, o, i = (_v(block, e, t, periods) for block in ("storage_op", "storage_ip", "storage_o", "storage_i"))
            yield _row([(op, 1), (o, -storage.minimum_discharge_power)], ">", 0)
            yield _row([(op, 1), (o, -storage.maximum_discharge)], "<", 0)
            yield _row([(ip, 1), (i, -storage.charge_power)], "=", 0)


def _storage_state(data: MinimalUcData) -> Iterator[LinearRow]:
    periods = data.num_periods
    for e in range(data.num_storages):
        for t in range(periods):
            o, i = _v("storage_o", e, t, periods), _v("storage_i", e, t, periods)
            yield _row([(o, 1), (i, 1)], "<", 1)
            if t + 1 < periods:
                yield _row([(o, 1), (_v("storage_i", e, t + 1, periods), 1)], "<", 1)
                yield _row([(i, 1), (_v("storage_o", e, t + 1, periods), 1)], "<", 1)


def _storage_energy(data: MinimalUcData) -> Iterator[LinearRow]:
    periods, factor = data.num_periods, 0.25 * 96.0 / data.num_periods
    for e, storage in enumerate(data.storages):
        cumulative: list[tuple[VarRef, float]] = []
        for t in range(periods):
            cumulative.extend([(_v("storage_ip", e, t, periods), -factor), (_v("storage_op", e, t, periods), factor)])
            yield _row([(_v("storage_soc", e, t, periods), storage.maximum_energy), *cumulative], "=", storage.initial_energy)
        yield _row([(_v("storage_soc", e, periods - 1, periods), 1)], ">", storage.final_energy / storage.maximum_energy)


def _minimum_state(entity: int, block: str, minimum: int, periods: int) -> Iterator[LinearRow]:
    if minimum < 2:
        return
    for t in range(1, periods):
        length = min(minimum, periods - t)
        terms = [(_v(block, entity, tau, periods), 1) for tau in range(t, t + length)]
        terms.extend([(_v(block, entity, t, periods), -length), (_v(block, entity, t - 1, periods), length)])
        yield _row(terms, ">", 0)


def _storage_duration(data: MinimalUcData) -> Iterator[LinearRow]:
    periods, delta = data.num_periods, 96.0 / data.num_periods
    for e, storage in enumerate(data.storages):
        discharge = max(1, int(storage.minimum_discharge_time / delta))
        charge = max(1, int(storage.minimum_charge / delta))
        off = max(1, int(storage.minimum_off / delta))
        yield _row([*[(_v("storage_o", e, t, periods), 1) for t in range(min(discharge, periods))], (_v("storage_o", e, 0, periods), -discharge)], ">", 0)
        yield _row([*[(_v("storage_i", e, t, periods), 1) for t in range(min(charge, periods))], (_v("storage_i", e, 0, periods), -charge)], ">", 0)
        yield from _minimum_state(e, "storage_o", discharge, periods)
        yield from _minimum_state(e, "storage_i", charge, periods)
        if storage.minimum_off >= 2:
            for t in range(1, periods):
                length = min(off, periods - t)
                terms = [( _v(block, e, tau, periods), -1) for tau in range(t, t + length) for block in ("storage_o", "storage_i")]
                terms.extend([(_v("storage_o", e, t - 1, periods), -length), (_v("storage_i", e, t - 1, periods), -length), (_v("storage_o", e, t, periods), length), (_v("storage_i", e, t, periods), length)])
                yield _row(terms, ">", -length)


def _storage_cost(data: MinimalUcData) -> Iterator[LinearRow]:
    periods, delta = data.num_periods, 96.0 / data.num_periods
    for e, storage in enumerate(data.storages):
        for t in range(periods):
            op, o, cost = _v("storage_op", e, t, periods), _v("storage_o", e, t, periods), _v("storage_cost_o", e, t, periods)
            for slope, intercept in storage.discharge_cost_lines:
                yield _row([(cost, 1), (op, -delta * slope), (o, -delta * intercept)], ">", 0)
            yield _row([(_v("storage_cost_i", e, t, periods), 1), (_v("storage_i", e, t, periods), -delta * storage.charge_cost)], "=", 0)


def _sections(data: MinimalUcData) -> Iterator[LinearRow]:
    periods = data.num_periods
    for section in data.sections:
        for t in range(periods):
            terms = [(_v("thermal_p", g, t, periods), c) for g, c in enumerate(section.thermal_sensitivity)]
            for e, coefficient in enumerate(section.storage_sensitivity):
                terms.extend([(_v("storage_op", e, t, periods), coefficient), (_v("storage_ip", e, t, periods), -coefficient)])
            yield _row(terms, "<", section.limit + section.load_offset[t])
            yield _row(terms, ">", -section.limit + section.load_offset[t])


def _balance(data: MinimalUcData) -> Iterator[LinearRow]:
    periods = data.num_periods
    for t, load in enumerate(data.loads):
        terms = [(_v("thermal_p", g, t, periods), 1) for g in range(data.num_units)]
        for e in range(data.num_storages):
            terms.extend([(_v("storage_op", e, t, periods), 1), (_v("storage_ip", e, t, periods), -1)])
        yield _row(terms, "=", load)


def _reserve(data: MinimalUcData) -> Iterator[LinearRow]:
    periods = data.num_periods
    for t, load in enumerate(data.loads):
        terms = [(variable, coefficient) for g, unit in enumerate(data.units) for variable, coefficient in ((_v("thermal_x", g, t, periods), unit.p_max), (_v("thermal_p", g, t, periods), -1))]
        yield _row(terms, ">", data.reserve_ratio * load)
