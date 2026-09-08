import os
import sys
import unittest
from pathlib import Path


COPT_HOME = Path(os.environ.get("COPT_HOME", r"D:\env\copt80"))
GUROBI_HOME = Path(os.environ.get("GUROBI_HOME", r"D:\env\gurobi1203\win64"))
COPT_DLL = COPT_HOME / "bin" / "copt.dll"
GUROBI_DLL = GUROBI_HOME / "bin" / "gurobi120.dll"

if sys.platform == "win32":
    if COPT_DLL.exists():
        os.environ["COPT_HOME"] = str(COPT_HOME)
    if GUROBI_DLL.exists():
        os.environ["GUROBI_HOME"] = str(GUROBI_HOME)

try:
    from benchmark.copt_env import create_copt_env
    from moirspy import MOI, GurobiEnv, Model
except ImportError:
    create_copt_env = None
    MOI = None
    Model = None
    GurobiEnv = None


@unittest.skipUnless(
    sys.platform == "win32"
    and COPT_DLL.exists()
    and GUROBI_DLL.exists()
    and Model is not None
    and create_copt_env is not None
    and GurobiEnv is not None,
    "requires both Windows solver runtimes and built Python extensions",
)
class WindowsBackendDifferentialTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.copt_env = create_copt_env()
        try:
            cls.gurobi_env = GurobiEnv(str(GUROBI_DLL))
        except RuntimeError as error:
            if "10009" in str(error):
                raise unittest.SkipTest(
                    "Gurobi runtime is available but no license is active"
                ) from error
            raise
        cls.gurobi_env.setParam("OutputFlag", 0)

    def build_and_solve(self, backend, *, attach_early):
        model = Model(f"d0-{backend}-{'early' if attach_early else 'late'}")
        env = self.copt_env if backend == "copt" else self.gurobi_env
        if attach_early:
            model.setBackend(backend, env=env)

        x = model.addVars(3, lb=0.0, ub=1.0, vtype=MOI.BINARY, name="x")
        model.addConstr(x[0] + 2.0 * x[1] + 3.0 * x[2] <= 4.0)
        model.addConstr(x[0] + x[1] >= 1.0)
        model.addConstr(x[1] + x[2] == 1.0)
        model.setObjective(x[0] + x[1] + 2.0 * x[2] + 1.0, MOI.MAXIMIZE)

        if not attach_early:
            model.setBackend(backend, env=env)
        if backend == "copt":
            model.setParam("Logging", 0)
        else:
            model.setParam("OutputFlag", 0)
        model.optimize()
        return float(model.ObjVal), tuple(round(x[index].X) for index in range(3))

    def test_copt_gurobi_early_late_models_have_identical_solution(self):
        solutions = {
            (backend, attach): self.build_and_solve(
                backend, attach_early=attach == "early"
            )
            for backend in ("copt", "gurobi")
            for attach in ("early", "late")
        }

        self.assertEqual(set(solutions.values()), {(4.0, (1, 0, 1))})


if __name__ == "__main__":
    unittest.main()
