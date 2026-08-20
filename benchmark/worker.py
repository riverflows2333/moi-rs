from __future__ import annotations

import argparse
import gc
import json
import time

from benchmark.builders import BUILDERS
from benchmark.common import load_minimal_uc
from benchmark.metrics import TimingSummary, process_memory_bytes


RESULT_PREFIX = "MOI_BENCH_RESULT="


def main() -> int:
    parser = argparse.ArgumentParser(description="Isolated benchmark worker")
    parser.add_argument("--case", required=True)
    parser.add_argument("--tool", required=True, choices=BUILDERS)
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--repeat", type=int, default=5)
    args = parser.parse_args()
    if args.warmup < 0 or args.repeat < 1:
        raise SystemExit("--warmup must be >= 0 and --repeat must be >= 1")

    data = load_minimal_uc(args.case)
    builder = BUILDERS[args.tool]
    try:
        context = builder.prepare()
    except Exception as error:
        return emit({"status": "unavailable", "error": f"{type(error).__name__}: {error}"})

    baseline_rss, baseline_peak_rss = process_memory_bytes()
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
        return emit({"status": "failed", "error": f"{type(error).__name__}: {error}"})

    current_rss, peak_rss = process_memory_bytes()
    summary = TimingSummary.from_samples(samples)
    return emit(
        {
            "status": "ok",
            "case": data.case_dir.name,
            "tool": args.tool,
            "units": data.num_units,
            "periods": data.num_periods,
            "variables": data.num_variables,
            "constraints": data.num_constraints,
            "nonzeros": data.num_nonzeros,
            "timing": {
                "median": summary.median,
                "p95": summary.p95,
                "minimum": summary.minimum,
                "maximum": summary.maximum,
                "samples": summary.samples,
                "raw": samples,
            },
            "memory": {
                "baseline_rss_bytes": baseline_rss,
                "baseline_peak_rss_bytes": baseline_peak_rss,
                "final_rss_bytes": current_rss,
                "peak_rss_bytes": peak_rss,
                "peak_delta_bytes": (
                    max(0, peak_rss - (baseline_peak_rss or baseline_rss))
                    if peak_rss is not None
                    and (baseline_peak_rss is not None or baseline_rss is not None)
                    else None
                ),
            },
        }
    )


def emit(result: dict[str, object]) -> int:
    print(RESULT_PREFIX + json.dumps(result, ensure_ascii=False))
    return 0 if result.get("status") == "ok" else 1


if __name__ == "__main__":
    raise SystemExit(main())
