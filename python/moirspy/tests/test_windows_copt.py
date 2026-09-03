import os
import sys
import unittest
from pathlib import Path


COPT_HOME = Path(os.environ.get("COPT_HOME", r"D:\env\copt80"))
COPT_DLL = COPT_HOME / "bin" / "copt.dll"

if sys.platform == "win32" and COPT_DLL.exists():
    os.environ["COPT_HOME"] = str(COPT_HOME)

try:
    from moirspy import CoptEnv, CoptEnvConfig, MOI, Model
except ImportError:
    MOI = None
    Model = None
    CoptEnv = None
    CoptEnvConfig = None


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

        model.addConstr(x <= 8.0)
        self.assertIsNone(x.X)
        self.assertIsNone(model.ObjVal)

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

    def test_configured_environment_through_high_level_model(self):
        config = CoptEnvConfig(str(COPT_DLL))
        config.set("NoBanner", 1)
        env = CoptEnv(config=config)

        model = self.new_model("copt-configured-env")
        x = model.addVar(lb=4.0, ub=4.0, name="x")
        model.setObjective(1.0 * x, MOI.MINIMIZE)
        model.setBackend("copt", env=env)
        model.optimize()

        self.assertAlmostEqual(model.ObjVal, 4.0, places=7)
        self.assertAlmostEqual(x.X, 4.0, places=7)

    def test_license_directory_environment_can_be_reused(self):
        env = CoptEnv(license_dir=str(COPT_HOME))
        objectives = []
        for index, value in enumerate((2.0, 5.0)):
            model = self.new_model(f"copt-shared-env-{index}")
            x = model.addVar(lb=value, ub=value)
            model.setObjective(1.0 * x, MOI.MINIMIZE)
            model.setBackend("copt", env=env)
            model.optimize()
            objectives.append(model.ObjVal)

        self.assertEqual(objectives, [2.0, 5.0])

    def test_environment_config_rejects_embedded_nul(self):
        config = CoptEnvConfig(str(COPT_DLL))
        with self.assertRaisesRegex(ValueError, "embedded NUL") as context:
            config.set("OEM", "private-value\0suffix")
        self.assertNotIn("private-value", str(context.exception))

    def test_builtin_backend_does_not_import_python_solver_package(self):
        previous = sys.modules.get("moirspy_copt")
        sys.modules["moirspy_copt"] = None
        try:
            model = self.new_model("copt-native-no-plugin")
            x = model.addVar(lb=2.0, ub=2.0)
            model.setObjective(1.0 * x, MOI.MINIMIZE)
            model.setBackend("copt")
            model.optimize()
            self.assertAlmostEqual(model.ObjVal, 2.0, places=7)
        finally:
            if previous is None:
                sys.modules.pop("moirspy_copt", None)
            else:
                sys.modules["moirspy_copt"] = previous

    def test_legacy_solver_environment_remains_accepted(self):
        try:
            from moirspy_copt import Env as LegacyCoptEnv
        except ImportError as error:
            self.skipTest(f"optional moirspy-copt package is unavailable: {error}")

        env = LegacyCoptEnv(str(COPT_DLL))
        model = self.new_model("copt-legacy-env")
        x = model.addVar(lb=3.0, ub=3.0)
        model.setObjective(1.0 * x, MOI.MINIMIZE)
        model.setBackend("copt", env=env)
        model.optimize()

        self.assertAlmostEqual(model.ObjVal, 3.0, places=7)


if __name__ == "__main__":
    unittest.main()
