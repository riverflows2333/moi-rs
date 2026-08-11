import os
import sys
import unittest
from pathlib import Path


COPT_HOME = Path(os.environ.get("COPT_HOME", r"D:\env\copt80"))
COPT_DLL = COPT_HOME / "bin" / "copt.dll"

if sys.platform == "win32" and COPT_DLL.exists():
    os.environ["COPT_HOME"] = str(COPT_HOME)

try:
    from moirspy import MOI, Model
    from moirspy_copt import Env as CoptEnv
except ImportError:
    MOI = None
    Model = None
    CoptEnv = None


@unittest.skipUnless(
    sys.platform == "win32"
    and COPT_DLL.exists()
    and Model is not None
    and CoptEnv is not None,
    "requires the Windows COPT 8 runtime and built Python extensions",
)
class WindowsCoptModelTests(unittest.TestCase):
    def new_model(self, name):
        model = Model(name)
        model.setParam("Logging", 0)
        return model

    def test_binary_model_and_solution_values(self):
        model = self.new_model("copt-binary")
        x = model.addVars(3, name="x", vtype=MOI.BINARY, lb=0.0, ub=1.0)
        model.addConstr(x[0] + 2 * x[1] + 3 * x[2] <= 4, name="capacity")
        model.addConstr(x[0] + x[1] >= 1, name="selection")
        model.setObjective(x[0] + x[1] + 2 * x[2] + 1.0, MOI.MAXIMIZE)

        model.setBackend("copt")
        model.optimize()

        self.assertAlmostEqual(model.ObjVal, 4.0, places=7)
        self.assertEqual([round(x[i].X) for i in range(3)], [1, 0, 1])

    def test_vector_bounds_are_forwarded(self):
        model = self.new_model("copt-vector-bounds")
        x = model.addVars(
            2,
            name="x",
            vtype=MOI.CONTINUOUS,
            lb=[2.0, 3.0],
            ub=[4.0, 8.0],
        )
        model.addConstr(x[0] + x[1] >= 7.0)
        model.setObjective(x[0] + x[1], MOI.MINIMIZE)

        model.setBackend("copt")
        model.optimize()

        values = [x[i].X for i in range(2)]
        self.assertAlmostEqual(model.ObjVal, 7.0, places=7)
        self.assertAlmostEqual(sum(values), 7.0, places=7)
        self.assertGreaterEqual(values[0], 2.0 - 1e-7)
        self.assertGreaterEqual(values[1], 3.0 - 1e-7)

    def test_incremental_modeling_after_backend_attach(self):
        model = self.new_model("copt-incremental")
        model.setBackend("copt")

        x = model.addVar(lb=0.0, ub=10.0, name="x")
        model.addConstr(x >= 6.0)
        model.setObjective(x + 1.0, MOI.MINIMIZE)
        model.optimize()

        self.assertAlmostEqual(x.X, 6.0, places=7)
        self.assertAlmostEqual(model.ObjVal, 7.0, places=7)

    def test_infeasible_and_unbounded_models_have_no_values(self):
        infeasible = self.new_model("copt-infeasible")
        x = infeasible.addVar(lb=0.0, ub=1.0, name="x")
        infeasible.addConstr(x >= 2.0)
        infeasible.setObjective(1.0 * x, MOI.MINIMIZE)
        infeasible.setBackend("copt")
        infeasible.optimize()
        self.assertIsNone(infeasible.ObjVal)
        self.assertIsNone(x.X)

        unbounded = self.new_model("copt-unbounded")
        y = unbounded.addVar(lb=0.0, name="y")
        unbounded.setObjective(1.0 * y, MOI.MAXIMIZE)
        unbounded.setBackend("copt")
        unbounded.optimize()
        self.assertIsNone(unbounded.ObjVal)
        self.assertIsNone(y.X)

    def test_explicit_environment_through_high_level_model(self):
        env = CoptEnv(str(COPT_DLL))
        model = self.new_model("copt-explicit-env")
        x = model.addVar(lb=3.0, ub=5.0, name="x")
        model.setObjective(1.0 * x, MOI.MINIMIZE)
        model.setBackend("copt", env=env)
        model.optimize()

        self.assertAlmostEqual(x.X, 3.0, places=7)


if __name__ == "__main__":
    unittest.main()
