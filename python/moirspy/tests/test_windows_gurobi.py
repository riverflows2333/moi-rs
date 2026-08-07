import os
import sys
import unittest
from pathlib import Path


GUROBI_HOME = Path(os.environ.get("GUROBI_HOME", r"D:\env\gurobi1203\win64"))
GUROBI_DLL = GUROBI_HOME / "bin" / "gurobi120.dll"

if sys.platform == "win32" and GUROBI_DLL.exists():
    os.environ["GUROBI_HOME"] = str(GUROBI_HOME)

try:
    from moirspy import MOI, Model
except ImportError:
    MOI = None
    Model = None


@unittest.skipUnless(
    sys.platform == "win32" and GUROBI_DLL.exists() and Model is not None,
    "requires the Windows Gurobi 12 runtime and built Python extensions",
)
class WindowsGurobiModelTests(unittest.TestCase):
    def new_model(self, name):
        model = Model(name)
        model.setParam("OutputFlag", False)
        return model

    def test_binary_model_and_solution_values(self):
        model = self.new_model("windows-binary")
        x = model.addVars(3, name="x", vtype=MOI.BINARY)
        model.addConstr(x[0] + 2 * x[1] + 3 * x[2] <= 4, name="capacity")
        model.addConstr(x[0] + x[1] >= 1, name="selection")
        model.setObjective(x[0] + x[1] + 2 * x[2], MOI.MAXIMIZE)

        model.setBackend("gurobi")
        model.optimize()

        self.assertAlmostEqual(model.ObjVal, 3.0, places=7)
        self.assertEqual([round(x[i].X) for i in range(3)], [1, 0, 1])

    def test_vector_bounds_are_forwarded_to_backend(self):
        model = self.new_model("windows-vector-bounds")
        x = model.addVars(
            2,
            name="x",
            vtype=MOI.CONTINUOUS,
            lb=[2.0, 3.0],
            ub=[4.0, 8.0],
        )
        model.addConstr(x[0] + x[1] >= 7.0, name="demand")
        model.setObjective(x[0] + x[1], MOI.MINIMIZE)

        model.setBackend("gurobi")
        model.optimize()

        values = [x[i].X for i in range(2)]
        self.assertAlmostEqual(model.ObjVal, 7.0, places=7)
        self.assertAlmostEqual(sum(values), 7.0, places=7)
        self.assertGreaterEqual(values[0], 2.0 - 1e-7)
        self.assertGreaterEqual(values[1], 3.0 - 1e-7)
        self.assertLessEqual(values[0], 4.0 + 1e-7)
        self.assertLessEqual(values[1], 8.0 + 1e-7)

    def test_single_variable_keeps_bridge_for_solution_query(self):
        model = self.new_model("windows-single-variable")
        x = model.addVar(lb=0.0, ub=10.0, name="x")
        model.addConstr(x >= 4.0, name="minimum")
        model.setObjective(2.0 * x, MOI.MINIMIZE)

        model.setBackend("gurobi")
        model.optimize()

        self.assertAlmostEqual(x.X, 4.0, places=7)
        self.assertAlmostEqual(model.ObjVal, 8.0, places=7)

    def test_incremental_modeling_after_backend_attach(self):
        model = self.new_model("windows-incremental")
        model.setBackend("gurobi")

        x = model.addVar(lb=0.0, ub=10.0, name="x")
        model.addConstr(x >= 6.0, name="minimum")
        model.setObjective(x + 1.0, MOI.MINIMIZE)
        model.optimize()

        self.assertAlmostEqual(x.X, 6.0, places=7)
        self.assertAlmostEqual(model.ObjVal, 7.0, places=7)

    def test_infeasible_model_has_no_solution_values(self):
        model = self.new_model("windows-infeasible")
        x = model.addVar(lb=0.0, ub=1.0, name="x")
        model.addConstr(x >= 2.0, name="impossible")
        model.setObjective(1.0 * x, MOI.MINIMIZE)

        model.setBackend("gurobi")
        model.optimize()

        self.assertIsNone(model.ObjVal)
        self.assertIsNone(x.X)

    def test_invalid_vector_lengths_fail_before_backend_attach(self):
        model = self.new_model("windows-invalid-input")
        with self.assertRaises(RuntimeError):
            model.addVars(2, lb=[0.0], name="x")


if __name__ == "__main__":
    unittest.main()
