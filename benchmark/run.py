from __future__ import annotations

import argparse
import gc
import sys
import time
from pathlib import Path

if __package__ in (None, ""):
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from benchmark.builders import BUILDERS
from benchmark.common import load_minimal_uc
from benchmark.metrics import TimingSummary, environment_metadata


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Benchmark complete direct 2-bin UC construction without solving")
    parser.add_argument(
        "--case",
        type=Path,
        default=Path("input/phys/effi/1-1"),
        help="case directory (containing model/) or the model directory itself",
    )
    parser.add_argument(
        "--tools",
        default=",".join(BUILDERS),
        help=f"comma-separated builders: {','.join(BUILDERS)}",
    )
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--metadata", action="store_true", help="print environment metadata")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.warmup < 0 or args.repeat < 1:
        raise SystemExit("--warmup must be >= 0 and --repeat must be >= 1")

    if args.metadata:
        for key, value in environment_metadata().items():
            print(f"{key}: {value}")
        print()

    requested = [name.strip() for name in args.tools.split(",") if name.strip()]
    unknown = [name for name in requested if name not in BUILDERS]
    if unknown:
        raise SystemExit(f"unknown builder(s): {', '.join(unknown)}")

    before_parse = time.perf_counter()
    data = load_minimal_uc(args.case)
    parse_seconds = time.perf_counter() - before_parse
    print(f"case: {data.case_dir}")
    print(
        f"common input: {data.num_units} thermal + {data.num_storages} storage "
        f"units x {data.num_periods} periods, {len(data.sections)} sections; "
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
        summary = TimingSummary.from_samples(samples)
        print(
            f"{name:14} median={summary.median:.6f} s  p95={summary.p95:.6f} s  "
            f"min={summary.minimum:.6f} s  max={summary.maximum:.6f} s  n={summary.samples}"
        )
    return 0 if succeeded else 1


if __name__ == "__main__":
    raise SystemExit(main())
