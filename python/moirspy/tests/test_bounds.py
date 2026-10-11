"""Bound edits must reach the solver and invalidate only modified results."""
import math
import os
from pathlib import Path
import unittest

from moirspy import Model, MOI, Var


class CachedBoundsTests(unittest.TestCase):
    def test_unattached_bounds_and_shared_handles(self):
        model = Model("cached-bounds")
        x = model.addVars(2, lb=-1, ub=10)
        first, alias = x[0], x[0]
        self.assertEqual((first.lb, first.ub), (-1, 10))
        first.lb = 3
        alias.ub = 7
        self.assertEqual((alias.lb, first.ub), (3, 7))
        self.assertEqual((x[1].lb, x[1].ub), (-1, 10))
        with self.assertRaises(ValueError):
            first.lb = 8
        with self.assertRaises(ValueError):
            first.ub = math.nan
        self.assertEqual((first.lb, first.ub), (3, 7))
        first.lb = -math.inf
        first.ub = math.inf
        self.assertEqual((alias.lb, alias.ub), (-math.inf, math.inf))

    def test_unassociated_variable_has_no_bounds(self):
        with self.assertRaisesRegex(RuntimeError, "not associated"):
            _ = Var(0).lb


class NativeBoundsTests(unittest.TestCase):
    def test_bounds_edit_and_resolve(self):
        for backend, env_name, default_home, library in (
            ("copt", "COPT_HOME", r"D:\env\copt80", "bin/copt.dll"),
            ("gurobi", "GUROBI_HOME", r"D:\env\gurobi1203\win64", "bin/gurobi120.dll"),
        ):
            home = Path(os.environ.get(env_name, default_home))
            if not (home / library).exists():
                continue
            os.environ[env_name] = str(home)
            for direct, keep_cache in ((True, False), (False, False), (False, True)):
                with self.subTest(backend=backend, direct=direct, keep_cache=keep_cache):
                    model = Model("bounds", backend=backend) if direct else Model("bounds")
                    x = model.addVars(2, lb=0, ub=10)
                    var, alias = x[0], x[0]
                    # Before native attach, edits must be replayed.
                    var.lb = 2
                    alias.ub = 8
                    model.setObjective(1.0 * var, MOI.MINIMIZE)
                    if not direct:
                        model.setBackend(backend, keep_cache=keep_cache)
                    model.setParam("Logging" if backend == "copt" else "OutputFlag", 0)
                    self.assertEqual((var.lb, var.ub), (2, 8))
                    model.optimize()
                    self.assertAlmostEqual(var.X, 2)
                    self.assertEqual(var.lb, 2)
                    self.assertAlmostEqual(model.ObjVal, 2)
                    with self.assertRaises(ValueError):
                        var.lb = 9
                    with self.assertRaises(ValueError):
                        var.ub = math.nan
                    self.assertAlmostEqual(var.X, 2)
                    var.lb = 5
                    self.assertEqual(alias.lb, 5)
                    self.assertIsNone(var.X)
                    self.assertIsNone(model.ObjVal)
                    model.optimize()
                    self.assertAlmostEqual(var.X, 5)
                    var.ub = 6
                    model.setObjective(1.0 * var, MOI.MAXIMIZE)
                    model.optimize()
                    self.assertAlmostEqual(var.X, 6)
                    if keep_cache:
                        model.setBackend(backend, keep_cache=True)
                        self.assertEqual((var.lb, var.ub), (5, 6))
                        model.optimize()
                        self.assertAlmostEqual(var.X, 6)
                    var.lb = -math.inf
                    var.ub = math.inf
                    self.assertEqual((var.lb, var.ub), (-math.inf, math.inf))


if __name__ == "__main__":
    unittest.main()
