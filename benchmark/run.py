from __future__ import annotations

import argparse
import gc
import statistics
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from benchmark.builders import BUILDERS
from benchmark.common import load_minimal_uc


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Benchmark UC model construction without solving")
    parser.add_argument(
        "--case",
        type=Path,
        default=Path("input/phys/effi/1-1"),
        help="case directory (containing model/) or the model directory itself",
    )
    parser.add_argument(
        "--tools",
        default="moirspy-early,moirspy-late,moirspy-copt,coptpy",
        help=f"comma-separated builders: {','.join(BUILDERS)}",
    )
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--repeat", type=int, default=3)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.warmup < 0 or args.repeat < 1:
        raise SystemExit("--warmup must be >= 0 and --repeat must be >= 1")

    requested = [name.strip() for name in args.tools.split(",") if name.strip()]
    unknown = [name for name in requested if name not in BUILDERS]
    if unknown:
        raise SystemExit(f"unknown builder(s): {', '.join(unknown)}")

    before_parse = time.perf_counter()
    data = load_minimal_uc(args.case)
    parse_seconds = time.perf_counter() - before_parse
    print(f"case: {data.case_dir}")
    print(
        f"common input: {data.num_units} units x {data.num_periods} periods; "
        f"{data.num_variables:,} vars, {data.num_constraints:,} constrs, "
        f"{data.num_nonzeros:,} nonzeros"
    )
    print(f"untimed input parse/preparation: {parse_seconds:.6f} s")
    print("timed boundary: API model creation through native-model update; solve excluded\n")

    succeeded = 0
    for name in requested:
        builder = BUILDERS[name]
        try:
            context = builder.prepare()
        except Exception as error:
            print(f"{name:14} unavailable: {type(error).__name__}: {error}")
            continue

        samples: list[float] = []
        try:
            for iteration in range(args.warmup + args.repeat):
                gc.collect()
                start = time.perf_counter()
                model = builder.build(data, context)
                elapsed = time.perf_counter() - start
                if iteration >= args.warmup:
                    samples.append(elapsed)
                del model
        except Exception as error:
            print(f"{name:14} failed: {type(error).__name__}: {error}")
            continue

        succeeded += 1
        print(
            f"{name:14} median={statistics.median(samples):.6f} s  "
            f"min={min(samples):.6f} s  max={max(samples):.6f} s  n={len(samples)}"
        )
    return 0 if succeeded else 1


if __name__ == "__main__":
    raise SystemExit(main())
