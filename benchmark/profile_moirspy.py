from __future__ import annotations

import argparse
import gc
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from benchmark.builders import prepare_moirspy, profile_moirspy
from benchmark.common import load_minimal_uc


STAGES = (
    "model",
    "variables",
    "output_bounds",
    "startup",
    "ramping",
    "balance",
    "reserve",
    "objective",
    "parameter",
    "attach",
)


def main() -> int:
    parser = argparse.ArgumentParser(description="Profile coarse moirspy construction stages")
    parser.add_argument(
        "cases",
        nargs="*",
        type=Path,
        default=[Path(f"input/phys/effi/1-{index}") for index in range(1, 4)],
    )
    parser.add_argument("--early", action="store_true", help="attach COPT before modeling")
    args = parser.parse_args()

    env = prepare_moirspy()
    print("case,units,periods," + ",".join(STAGES))
    for case in args.cases:
        data = load_minimal_uc(case)
        gc.collect()
        model, timings = profile_moirspy(data, env, attach_early=args.early)
        values = ",".join(f"{timings.get(stage, 0.0):.6f}" for stage in STAGES)
        print(f"{case.name},{data.num_units},{data.num_periods},{values}")
        del model
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
