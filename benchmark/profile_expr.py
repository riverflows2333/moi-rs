from __future__ import annotations

import argparse
import gc
import math
import time
from collections.abc import Callable, Iterable
from typing import Any

from benchmark.metrics import TimingSummary


def main() -> int:
    parser = argparse.ArgumentParser(description="Measure quicksum scaling by item type")
    parser.add_argument(
        "--sizes",
        type=int,
        nargs="+",
        default=[2_400, 4_800, 9_600],
        help="numbers of items to accumulate",
    )
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail when weighted-expression scaling exceeds --max-exponent",
    )
    parser.add_argument("--max-exponent", type=float, default=1.35)
    args = parser.parse_args()
    if args.repeat < 1 or any(size < 1 for size in args.sizes):
        raise SystemExit("--repeat and all --sizes values must be positive")
    sizes = sorted(set(args.sizes))

    from moirspy import Model, quicksum

    kinds: dict[str, Callable[[Any, int], Iterable[Any]]] = {
        "var": lambda x, n: (x[0, i] for i in range(n)),
        "two_term": lambda x, n: (x[0, i] + x[1, i] for i in range(n)),
        "three_term": lambda x, n: (x[0, i] + x[1, i] + x[2, i] for i in range(n)),
        "duplicate_var": lambda x, n: (x[0, i] + x[0, i] - x[0, i] for i in range(n)),
        "weighted": lambda x, n: (
            1.1 * x[0, i] + 2.2 * x[1, i] + 3.3 * x[2, i] for i in range(n)
        ),
    }

    print("items,kind,median_seconds,p95_seconds,growth,exponent,samples")
    previous: dict[str, tuple[int, float]] = {}
    failed = False
    for size in sizes:
        model = Model(f"quicksum-{size}")
        variables = model.addVars(3, size, name="x")
        for kind, make_items in kinds.items():
            samples: list[float] = []
            for _ in range(args.repeat):
                gc.collect()
                start = time.perf_counter()
                expression = quicksum(make_items(variables, size))
                samples.append(time.perf_counter() - start)
                del expression
            summary = TimingSummary.from_samples(samples)
            growth = exponent = None
            if kind in previous:
                previous_size, previous_time = previous[kind]
                growth = summary.median / previous_time
                exponent = math.log(growth) / math.log(size / previous_size)
            previous[kind] = (size, summary.median)
            print(
                f"{size},{kind},{summary.median:.6f},{summary.p95:.6f},"
                f"{format_optional(growth)},{format_optional(exponent)},{summary.samples}"
            )
            if args.check and kind == "weighted" and exponent is not None:
                failed |= exponent > args.max_exponent
        del variables, model
    return 1 if failed else 0


def format_optional(value: float | None) -> str:
    return "" if value is None else f"{value:.3f}"


if __name__ == "__main__":
    raise SystemExit(main())
