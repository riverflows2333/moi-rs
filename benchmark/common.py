from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True, slots=True)
class ThermalUnit:
    unit_id: int
    p_max: float
    p_min: float
    ramp_up: float
    ramp_down: float
    startup_cost: float
    initial_on: int
    initial_output: float
    marginal_cost: float
    no_load_cost: float


@dataclass(frozen=True, slots=True)
class MinimalUcData:
    """Solver-independent input for the deliberately small UC formulation."""

    case_dir: Path
    loads: tuple[float, ...]
    units: tuple[ThermalUnit, ...]
    reserve_ratio: float = 0.10

    @property
    def num_units(self) -> int:
        return len(self.units)

    @property
    def num_periods(self) -> int:
        return len(self.loads)

    @property
    def num_variables(self) -> int:
        return 3 * self.num_units * self.num_periods

    @property
    def num_constraints(self) -> int:
        return 5 * self.num_units * self.num_periods + 2 * self.num_periods

    @property
    def num_nonzeros(self) -> int:
        # Output bounds + startup + ramping + balance + reserve.
        return 16 * self.num_units * self.num_periods - 4 * self.num_units


def _numeric_rows(path: Path, expected_columns: int) -> list[list[float]]:
    if not path.is_file():
        raise FileNotFoundError(f"missing input file: {path}")

    rows: list[list[float]] = []
    with path.open("r", encoding="gb18030", errors="replace") as stream:
        next(stream, None)  # The supplied files have one textual header row.
        for line_number, line in enumerate(stream, start=2):
            fields = line.split()
            if not fields:
                continue
            if len(fields) != expected_columns:
                raise ValueError(
                    f"{path}:{line_number}: expected {expected_columns} columns, "
                    f"got {len(fields)}"
                )
            try:
                rows.append([float(field) for field in fields])
            except ValueError as error:
                raise ValueError(f"{path}:{line_number}: non-numeric data") from error
    return rows


def load_minimal_uc(case_dir: str | Path) -> MinimalUcData:
    """Parse one supplied case and prepare the common, untimed benchmark input."""

    case_dir = Path(case_dir).resolve()
    model_dir = case_dir / "model" if (case_dir / "model").is_dir() else case_dir

    load_rows = _numeric_rows(model_dir / "slf.txt", 2)
    unit_rows = _numeric_rows(model_dir / "unitdata.txt", 12)
    price_rows = _numeric_rows(model_dir / "bidprice.txt", 8)

    periods = [int(row[0]) for row in load_rows]
    expected_periods = list(range(1, len(periods) + 1))
    if periods != expected_periods:
        raise ValueError("slf.txt periods must be consecutive and start at 1")
    if not load_rows:
        raise ValueError("slf.txt contains no periods")

    prices = {int(row[0]): row[1:] for row in price_rows}
    units: list[ThermalUnit] = []
    seen_ids: set[int] = set()
    for row in unit_rows:
        unit_id = int(row[0])
        if unit_id in seen_ids:
            raise ValueError(f"duplicate thermal unit id: {unit_id}")
        seen_ids.add(unit_id)
        if unit_id not in prices:
            raise ValueError(f"bidprice.txt has no row for thermal unit {unit_id}")

        p_max, p_min = row[2], row[3]
        if not 0.0 <= p_min <= p_max:
            raise ValueError(f"invalid output bounds for thermal unit {unit_id}")

        # Use the chord between the minimum-output and maximum-output bid points.
        # It produces slope*p + no_load*u without adding segment variables.
        price_at_min = prices[unit_id][1]
        price_at_max = prices[unit_id][-1]
        if p_max == p_min:
            marginal_cost = 0.0
            no_load_cost = price_at_max
        else:
            marginal_cost = (price_at_max - price_at_min) / (p_max - p_min)
            no_load_cost = price_at_min - marginal_cost * p_min

        initial_on = int(row[9])
        if initial_on not in (0, 1):
            raise ValueError(f"invalid initial status for thermal unit {unit_id}")
        units.append(
            ThermalUnit(
                unit_id=unit_id,
                p_max=p_max,
                p_min=p_min,
                ramp_up=row[4],
                ramp_down=row[5],
                startup_cost=row[8],
                initial_on=initial_on,
                initial_output=row[10],
                marginal_cost=marginal_cost,
                no_load_cost=no_load_cost,
            )
        )

    if not units:
        raise ValueError("unitdata.txt contains no thermal units")
    if set(prices) != seen_ids:
        raise ValueError("unitdata.txt and bidprice.txt unit ids do not match")

    return MinimalUcData(
        case_dir=case_dir,
        loads=tuple(row[1] for row in load_rows),
        units=tuple(units),
    )
