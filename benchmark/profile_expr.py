from __future__ import annotations

import argparse
import gc
import time


def main() -> int:
    parser = argparse.ArgumentParser(description="Compare Var and LinExpr quicksum scaling")
    parser.add_argument(
        "--sizes",
        type=int,
        nargs="+",
        default=[9_600, 19_392, 48_576],
        help="numbers of items to accumulate",
    )
    args = parser.parse_args()

    from moirspy import Model, quicksum

    print("items,var_seconds,three_term_expr_seconds")
    for size in args.sizes:
        if size < 1:
            raise SystemExit("all --sizes values must be positive")
        model = Model(f"quicksum-{size}")
        variables = model.addVars(3, size, name="x")

        gc.collect()
        start = time.perf_counter()
        var_sum = quicksum(variables[0, i] for i in range(size))
        var_seconds = time.perf_counter() - start

        gc.collect()
        start = time.perf_counter()
        expr_sum = quicksum(
            variables[0, i] + variables[1, i] + variables[2, i]
            for i in range(size)
        )
        expr_seconds = time.perf_counter() - start
        print(f"{size},{var_seconds:.6f},{expr_seconds:.6f}")

        # Keep both results live through the measurements.
        del var_sum, expr_sum, variables, model
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
