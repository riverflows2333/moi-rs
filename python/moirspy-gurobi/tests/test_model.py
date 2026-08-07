import os
import sys
import unittest
from pathlib import Path


GUROBI_HOME = Path(os.environ.get("GUROBI_HOME", r"D:\env\gurobi1203\win64"))
GUROBI_DLL = GUROBI_HOME / "bin" / "gurobi120.dll"

try:
    from moirspy_gurobi import Model
except ImportError:
    Model = None


@unittest.skipUnless(
    sys.platform == "win32" and GUROBI_DLL.exists() and Model is not None,
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


if __name__ == "__main__":
    unittest.main()
