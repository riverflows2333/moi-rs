"""Input parsing and preprocessing for the direct UC example.

This module deliberately contains no solver or modeling-library code.  It can
be moved together with ``direct_uc_moirspy.py`` without depending on the
benchmark's generic row representation.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Iterable


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
class UcData:
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
                raise ValueError(
                    f"{path}:{line_number}: expected {columns} columns, got {len(fields)}"
                )
            try:
                rows.append([float(field) for field in fields])
            except ValueError as error:
                raise ValueError(f"{path}:{line_number}: non-numeric data") from error
    return rows


def _indexed(
    rows: list[list[float]], filename: str
) -> dict[int, tuple[float, ...]]:
    result: dict[int, tuple[float, ...]] = {}
    for expected, row in enumerate(rows, 1):
        index = int(row[0])
        if index != expected or index in result:
            raise ValueError(
                f"{filename} ids must be unique, consecutive, and start at 1"
            )
        result[index] = tuple(row[1:])
    return result


def _piecewise_lines(
    prices: Iterable[float], widths: Iterable[float], start: float
) -> tuple[tuple[float, float], ...]:
    prices = tuple(prices)
    widths = tuple(widths)
    if len(prices) != len(widths) + 1:
        raise ValueError("piecewise prices must contain one more point than widths")

    lines: list[tuple[float, float]] = []
    cumulative = start
    for index, width in enumerate(widths):
        if width <= 0.0:
            raise ValueError("piecewise widths must be positive")
        cumulative += width
        slope = (prices[index + 1] - prices[index]) / width
        lines.append((slope, prices[index + 1] - cumulative * slope))
    return tuple(lines)


def load_uc_data(case_dir: str | Path) -> UcData:
    """Read one supplied case directory and prepare coefficients for modeling."""

    supplied = Path(case_dir).resolve()
    model_dir = supplied / "model" if (supplied / "model").is_dir() else supplied
    display_dir = supplied if model_dir != supplied else supplied.parent

    load_rows = _numeric_rows(model_dir / "slf.txt", 2)
    periods = [int(row[0]) for row in load_rows]
    if not periods or periods != list(range(1, len(periods) + 1)):
        raise ValueError("slf.txt periods must be consecutive and start at 1")

    unit_widths = _indexed(
        _numeric_rows(model_dir / "bidcapacity.txt", 6), "bidcapacity.txt"
    )
    unit_prices = _indexed(
        _numeric_rows(model_dir / "bidprice.txt", 8), "bidprice.txt"
    )
    units: list[ThermalUnit] = []
    for row in _numeric_rows(model_dir / "unitdata.txt", 12):
        unit_id = int(row[0])
        p_max = row[2]
        p_min = row[3]
        initial_on = int(row[9])
        if not 0.0 <= p_min <= p_max or initial_on not in (0, 1):
            raise ValueError(f"invalid thermal unit {unit_id}")
        if unit_id not in unit_widths or unit_id not in unit_prices:
            raise ValueError(f"missing bid data for thermal unit {unit_id}")
        units.append(
            ThermalUnit(
                unit_id=unit_id,
                p_max=p_max,
                p_min=p_min,
                ramp_up=row[4],
                ramp_down=row[5],
                minimum_on=int(row[6]),
                minimum_off=int(row[7]),
                startup_cost=row[8],
                initial_on=initial_on,
                initial_output=row[10],
                initial_duration=int(row[11]),
                cost_lines=_piecewise_lines(
                    unit_prices[unit_id][1:], unit_widths[unit_id], p_min
                ),
            )
        )

    storage_widths = _indexed(
        _numeric_rows(model_dir / "stbidcapacity.txt", 7), "stbidcapacity.txt"
    )
    storage_prices = _indexed(
        _numeric_rows(model_dir / "stbidprice.txt", 8), "stbidprice.txt"
    )
    storages: list[StorageUnit] = []
    for row in _numeric_rows(model_dir / "storagebasic.txt", 13):
        storage_id = int(row[0])
        if storage_id not in storage_widths or storage_id not in storage_prices:
            raise ValueError(f"missing bid data for storage unit {storage_id}")
        if row[6] <= 0.0:
            raise ValueError(f"invalid maximum energy for storage unit {storage_id}")
        prices = storage_prices[storage_id]
        widths = storage_widths[storage_id]
        storages.append(
            StorageUnit(
                storage_id=storage_id,
                minimum_charge=int(row[1]),
                minimum_off=int(row[2]),
                minimum_discharge_time=int(row[3]),
                initial_energy=row[4],
                final_energy=row[5],
                maximum_energy=row[6],
                charge_power=row[8],
                maximum_discharge=row[9],
                minimum_discharge_power=row[10],
                same_bus_unit=int(row[12]),
                charge_cost=abs(prices[0]),
                discharge_cost_lines=_piecewise_lines(
                    prices[1:], widths[1:], 0.0
                ),
            )
        )

    sections = _load_sections(
        model_dir, len(units), storages, len(periods)
    )
    return UcData(
        case_dir=display_dir,
        loads=tuple(row[1] for row in load_rows),
        units=tuple(units),
        storages=tuple(storages),
        sections=sections,
    )


def _load_sections(
    model_dir: Path,
    num_units: int,
    storages: list[StorageUnit],
    periods: int,
) -> tuple[Section, ...]:
    definitions: list[tuple[int, tuple[int, ...], float]] = []
    required_branches: set[int] = set()
    section_counts = _indexed(
        _numeric_rows(model_dir / "sectionnum.txt", 2), "sectionnum.txt"
    )

    for row in _numeric_rows(model_dir / "section.txt"):
        if len(row) < 3:
            raise ValueError("section.txt rows require an id, branches, and a limit")
        section_id = int(row[0])
        branches = tuple(int(value) for value in row[1:-1])
        if (
            section_id not in section_counts
            or int(section_counts[section_id][0]) != len(branches)
        ):
            raise ValueError(
                f"section {section_id} branch count does not match sectionnum.txt"
            )
        definitions.append((section_id, branches, row[-1]))
        required_branches.update(branches)

    sensitivities: dict[int, tuple[float, ...]] = {}
    load_offsets = {
        branch: [0.0] * periods for branch in required_branches
    }
    in_sensitivity_data = False
    in_branch_data = False
    branch_log = model_dir / "branch_1.log"
    with branch_log.open("r", encoding="gb18030", errors="replace") as stream:
        for line in stream:
            if line.startswith("<BranchUnitSensi"):
                in_sensitivity_data, in_branch_data = True, False
                continue
            if line.startswith("</BranchUnitSensi"):
                in_sensitivity_data = False
                continue
            if line.startswith("<BranchData"):
                in_branch_data = True
                continue
            if line.startswith("</BranchData"):
                in_branch_data = False
                continue
            if not line.startswith("#"):
                continue

            fields = line.split()
            if in_sensitivity_data and len(fields) >= 4:
                branch = int(fields[1])
                if branch in required_branches:
                    values = tuple(float(value) for value in fields[4:])
                    if len(values) != num_units:
                        raise ValueError(
                            f"branch {branch} has {len(values)} sensitivities, "
                            f"expected {num_units}"
                        )
                    sensitivities[branch] = values
            elif in_branch_data and len(fields) == 7:
                period = int(fields[2])
                branch = int(fields[3])
                if branch in load_offsets and 1 <= period <= periods:
                    load_offsets[branch][period - 1] = float(fields[-1])

    missing = required_branches - sensitivities.keys()
    if missing:
        raise ValueError(f"missing branch sensitivities: {sorted(missing)}")

    result: list[Section] = []
    for section_id, branches, limit in definitions:
        thermal_sensitivity = tuple(
            sum(sensitivities[branch][unit] for branch in branches)
            for unit in range(num_units)
        )
        storage_sensitivity = tuple(
            thermal_sensitivity[storage.same_bus_unit - 1]
            for storage in storages
        )
        load_offset = tuple(
            sum(load_offsets[branch][period] for branch in branches)
            for period in range(periods)
        )
        result.append(
            Section(
                section_id=section_id,
                branches=branches,
                limit=limit,
                thermal_sensitivity=thermal_sensitivity,
                storage_sensitivity=storage_sensitivity,
                load_offset=load_offset,
            )
        )
    return tuple(result)
