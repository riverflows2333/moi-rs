from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any, Mapping

from benchmark.common import FormulationStats, formulation_stats, load_minimal_uc


DEFAULT_BASELINE = (
    Path(__file__).resolve().parent / "baselines" / "direct_migration_v1.json"
)


def load_baseline(path: str | Path = DEFAULT_BASELINE) -> dict[str, Any]:
    with Path(path).open("r", encoding="utf-8") as stream:
        baseline = json.load(stream)
    if baseline.get("schema_version") != 1 or not isinstance(
        baseline.get("cases"), dict
    ):
        raise ValueError(f"unsupported formulation baseline: {path}")
    return baseline


def assert_formulation_matches(
    stats: FormulationStats,
    expected: Mapping[str, object],
    *,
    label: str,
) -> None:
    actual = stats.as_dict()
    differences: list[str] = []
    for field in (
        "variables",
        "constraints",
        "nonzeros",
        "objective_nonzeros",
        "fingerprint",
    ):
        if actual.get(field) != expected.get(field):
            differences.append(
                f"{field}: expected {expected.get(field)!r}, got {actual.get(field)!r}"
            )

    actual_families = actual["families"]
    expected_families = expected.get("families")
    if actual_families != expected_families:
        differences.append(
            f"families: expected {expected_families!r}, got {actual_families!r}"
        )
    if differences:
        raise AssertionError(f"{label} formulation changed:\n" + "\n".join(differences))


def assert_runtime_dimensions(
    result: Mapping[str, object],
    expected: Mapping[str, object],
    *,
    label: str,
) -> None:
    differences = []
    for field in ("variables", "constraints", "nonzeros"):
        if result.get(field) != expected.get(field):
            differences.append(
                f"{field}: expected {expected.get(field)!r}, got {result.get(field)!r}"
            )
    if differences:
        raise AssertionError(f"{label} model dimensions changed:\n" + "\n".join(differences))


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check complete 2-bin UC inputs against the Direct migration baseline"
    )
    parser.add_argument("--root", type=Path, default=Path("input/phys/effi"))
    parser.add_argument("--cases", nargs="+", default=["1-1", "1-3", "1-5"])
    parser.add_argument("--baseline", type=Path, default=DEFAULT_BASELINE)
    args = parser.parse_args()

    baseline = load_baseline(args.baseline)
    expected_cases = baseline["cases"]
    for case_name in args.cases:
        if case_name not in expected_cases:
            raise SystemExit(f"case {case_name!r} is not present in {args.baseline}")
        stats = formulation_stats(
            load_minimal_uc(args.root / case_name), include_fingerprint=True
        )
        assert_formulation_matches(stats, expected_cases[case_name], label=case_name)
        print(
            f"{case_name}: {stats.variables:,} variables, "
            f"{stats.constraints:,} constraints, {stats.nonzeros:,} nonzeros, ok"
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
