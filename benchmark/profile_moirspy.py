from __future__ import annotations

import argparse
import gc
import sys
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from benchmark.builders import (
    build_moirspy_early,
    build_moirspy_late,
    prepare_moirspy,
    profile_moirspy,
)
from benchmark.common import load_minimal_uc
from benchmark.metrics import TimingSummary


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
    parser.add_argument("--warmup", type=int, default=0)
    parser.add_argument("--repeat", type=int, default=1)
    args = parser.parse_args()
    if args.warmup < 0 or args.repeat < 1:
        raise SystemExit("--warmup must be >= 0 and --repeat must be >= 1")

    env = prepare_moirspy()
    mode = "early" if args.early else "late"
    warmup_builder = build_moirspy_early if args.early else build_moirspy_late
    print("case,mode,units,periods,stage,median_seconds,p95_seconds,samples")
    for case in args.cases:
        data = load_minimal_uc(case)
        for _ in range(args.warmup):
            gc.collect()
            model = warmup_builder(data, env)
            del model
        stage_samples = {stage: [] for stage in STAGES}
        for _ in range(args.repeat):
            gc.collect()
            model, timings = profile_moirspy(data, env, attach_early=args.early)
            for stage in STAGES:
                stage_samples[stage].append(timings.get(stage, 0.0))
            del model
        for stage in STAGES:
            summary = TimingSummary.from_samples(stage_samples[stage])
            print(
                f"{case.name},{mode},{data.num_units},{data.num_periods},{stage},"
                f"{summary.median:.6f},{summary.p95:.6f},{summary.samples}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
