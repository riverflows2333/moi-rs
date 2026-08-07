import os
import sys
import unittest
from pathlib import Path


GUROBI_HOME = Path(os.environ.get("GUROBI_HOME", r"D:\env\gurobi1203\win64"))
GUROBI_DLL = GUROBI_HOME / "bin" / "gurobi120.dll"

try:
    from moirspy_gurobi import Env, Model
except ImportError:
    Env = None
    Model = None


@unittest.skipUnless(
    sys.platform == "win32"
    and GUROBI_DLL.exists()
    and Env is not None
    and Model is not None,
    "requires the Windows Gurobi 12 runtime and built moirspy_gurobi extension",
)
class LowLevelWindowsGurobiTests(unittest.TestCase):
    def test_basic_model_operations(self):
        model = Model("low-level-windows", str(GUROBI_DLL))
        x0 = model.add_variable(name="x0", vtype="C", lb=0.0, ub=10.0)
        x1 = model.add_variable(name="x1", vtype="C", lb=0.0, ub=10.0)

        self.assertEqual((x0, x1), (0, 1))
        model.add_constraint(
            vars=[x0, x1],
            coeffs=[1.0, 1.0],
            constant=0.0,
            sense=">",
            rhs=1.0,
            name="demand",
        )
        model.set_objective(
            vars=[x0, x1],
            coeffs=[1.0, 1.0],
            constant=0.0,
            sense=0,
        )
        model.set_optimizer_attr("OutputFlag", 0)
        model.update()

        status = model.optimize()

        self.assertEqual(status, 1)
        self.assertAlmostEqual(model.get_objective_value(), 1.0, places=7)
        self.assertAlmostEqual(
            model.get_var_value(x0) + model.get_var_value(x1), 1.0, places=7
        )

    def test_invalid_parallel_array_lengths_are_rejected(self):
        model = Model("low-level-invalid-input", str(GUROBI_DLL))
        with self.assertRaises(ValueError):
            model.add_constraint(
                vars=[0, 1],
                coeffs=[1.0],
                constant=0.0,
                sense="<",
                rhs=1.0,
            )

    def test_explicit_environment_can_be_reused(self):
        env = Env(str(GUROBI_DLL))
        env.setParam("OutputFlag", 0)
        env.setParam("Threads", 1)

        first = Model("explicit-env-first", env=env)
        second = Model("explicit-env-second", env=env)

        x = first.add_variable(name="x", lb=1.0, ub=2.0)
        first.set_objective([x], [1.0], 0.0, 0)
        first.optimize()
        self.assertAlmostEqual(first.get_var_value(x), 1.0, places=7)

        y = second.add_variable(name="y", lb=2.0, ub=3.0)
        second.set_objective([y], [1.0], 0.0, 0)
        second.optimize()
        self.assertAlmostEqual(second.get_var_value(y), 2.0, places=7)

    def test_empty_environment_requires_start(self):
        env = Env(str(GUROBI_DLL), empty=True)
        self.assertFalse(env.started)
        env.setParam("OutputFlag", 0)

        with self.assertRaisesRegex(RuntimeError, "must be started"):
            Model("not-started", env=env)

        env.start()
        self.assertTrue(env.started)
        model = Model("started", env=env)
        x = model.add_variable(name="x", lb=1.0, ub=1.0)
        model.set_objective([x], [1.0], 0.0, 0)
        self.assertEqual(model.optimize(), 1)


if __name__ == "__main__":
    unittest.main()
