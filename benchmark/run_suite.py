from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

from benchmark.builders import BUILDERS
from benchmark.metrics import environment_metadata, json_dumps
from benchmark.worker import RESULT_PREFIX


def main() -> int:
    parser = argparse.ArgumentParser(description="Run isolated UC benchmark cases")
    parser.add_argument("--root", type=Path, default=Path("input/phys/effi"))
    parser.add_argument("--cases", nargs="+", default=["1-1", "1-2", "1-3"])
    parser.add_argument("--tools", default=",".join(BUILDERS))
    parser.add_argument("--warmup", type=int, default=1)
    parser.add_argument("--repeat", type=int, default=5)
    parser.add_argument("--json", type=Path, help="write complete metadata and raw samples")
    args = parser.parse_args()

    tools = [name.strip() for name in args.tools.split(",") if name.strip()]
    unknown = [name for name in tools if name not in BUILDERS]
    if unknown:
        raise SystemExit(f"unknown builder(s): {', '.join(unknown)}")
    if args.warmup < 0 or args.repeat < 1:
        raise SystemExit("--warmup must be >= 0 and --repeat must be >= 1")

    results: list[dict[str, object]] = []
    failures = 0
    for case_name in args.cases:
        case = args.root / case_name
        for tool in tools:
            print(f"running {case_name} / {tool} ...", file=sys.stderr, flush=True)
            result = run_worker(case, tool, args.warmup, args.repeat)
            results.append(result)
            if result.get("status") != "ok":
                failures += 1

    metadata = environment_metadata()
    print_markdown(metadata, results)
    if args.json:
        args.json.parent.mkdir(parents=True, exist_ok=True)
        args.json.write_text(
            json_dumps({"metadata": metadata, "results": results}) + "\n",
            encoding="utf-8",
        )
    return 1 if failures == len(results) else 0


def run_worker(case: Path, tool: str, warmup: int, repeat: int) -> dict[str, object]:
    command = [
        sys.executable,
        "-m",
        "benchmark.worker",
        "--case",
        str(case),
        "--tool",
        tool,
        "--warmup",
        str(warmup),
        "--repeat",
        str(repeat),
    ]
    completed = subprocess.run(command, check=False, capture_output=True, text=True)
    for line in reversed(completed.stdout.splitlines()):
        if line.startswith(RESULT_PREFIX):
            return json.loads(line[len(RESULT_PREFIX) :])
    return {
        "status": "failed",
        "case": case.name,
        "tool": tool,
        "error": completed.stderr.strip() or completed.stdout.strip() or "worker produced no result",
    }


def print_markdown(metadata: dict[str, object], results: list[dict[str, object]]) -> None:
    print(f"Python {metadata['python']} | {metadata['platform']} | rustc: {metadata['rustc']}")
    artifacts = metadata["extension_artifacts"]
    profiles = ", ".join(
        f"{name}={details['inferred_profile']}"
        for name, details in artifacts.items()
        if details is not None
    )
    print(f"{metadata['copt_version']} | extension profiles: {profiles or 'unknown'}")
    print()
    print("| Case | Tool | Vars | Constrs | Median (s) | p95 (s) | Peak RSS (MiB) | Peak delta (MiB) | n |")
    print("|---|---|---:|---:|---:|---:|---:|---:|---:|")
    for result in results:
        if result.get("status") != "ok":
            print(
                f"| {result.get('case', '?')} | {result.get('tool', '?')} | - | - | "
                f"{result.get('status')} | - | - | - | - |"
            )
            continue
        timing = result["timing"]
        memory = result["memory"]
        print(
            f"| {result['case']} | {result['tool']} | {result['variables']:,} | "
            f"{result['constraints']:,} | {timing['median']:.6f} | {timing['p95']:.6f} | "
            f"{mib(memory['peak_rss_bytes'])} | {mib(memory['peak_delta_bytes'])} | "
            f"{timing['samples']} |"
        )


def mib(value: int | None) -> str:
    return "-" if value is None else f"{value / (1024 * 1024):.1f}"


if __name__ == "__main__":
    raise SystemExit(main())
