import os
import sys
import unittest
from pathlib import Path

import numpy as np


COPT_HOME = Path(os.environ.get("COPT_HOME", r"D:\env\copt80"))
COPT_DLL = COPT_HOME / "bin" / "copt.dll"

if sys.platform == "win32" and COPT_DLL.exists():
    os.environ["COPT_HOME"] = str(COPT_HOME)

try:
    from moirspy_copt import COPT, Env, EnvrConfig, Model
except ImportError:
    COPT = None
    Env = None
    EnvrConfig = None
    Model = None


@unittest.skipUnless(
    sys.platform == "win32"
    and COPT_DLL.exists()
    and Env is not None
    and EnvrConfig is not None
    and Model is not None,
    "requires the Windows COPT 8 runtime and built moirspy_copt extension",
)
class LowLevelWindowsCoptTests(unittest.TestCase):
    def new_model(self, name):
        model = Model(name, str(COPT_DLL))
        model.set_optimizer_attr("Logging", 0)
        return model

    def test_continuous_model_and_solution(self):
        model = self.new_model("low-level-lp")
        x = model.add_variables(
            2,
            names=["x", "y"],
            vtypes=["C", "C"],
            lbs=[0.0, 0.0],
            ubs=[10.0, 10.0],
        )
        model.add_constraint(x, [1.0, 2.0], 0.0, ">", 4.0, "demand")
        model.set_objective(x, [1.0, 1.0], 3.0, -1)

        self.assertEqual(model.optimize(), 1)
        self.assertAlmostEqual(model.get_var_value(x[0]), 0.0, places=7)
        self.assertAlmostEqual(model.get_var_value(x[1]), 2.0, places=7)
        self.assertAlmostEqual(model.get_objective_value(), 5.0, places=7)

    def test_binary_model_and_parameters(self):
        model = self.new_model("low-level-milp")
        model.set_optimizer_attr("TimeLimit", 10.0)
        model.set_optimizer_attr("Threads", 1)
        x = model.add_variables(
            2,
            names=["x", "y"],
            vtypes=["B", "B"],
            lbs=[0.0, 0.0],
            ubs=[1.0, 1.0],
        )
        model.add_constraint(x, [2.0, 1.0], 0.0, "<", 2.0)
        model.set_objective(x, [2.0, 1.0], 5.0, 1)

        self.assertEqual(model.optimize(), 1)
        self.assertEqual([round(model.get_var_value(i)) for i in x], [1, 0])
        self.assertAlmostEqual(model.get_objective_value(), 7.0, places=7)

    def test_flat_buffer_constraints_and_objective(self):
        model = self.new_model("low-level-flat-buffers")
        x = model.add_variables(2, lbs=[0.0, 0.0], ubs=[10.0, 10.0])
        returned_range = model.add_constraints_flat(
            np.asarray([0, 2, 3], dtype=np.uintp),
            np.asarray([x[0], x[1], x[0]], dtype=np.uintp),
            np.asarray([1.0, 2.0, 1.0], dtype=np.float64),
            np.asarray([0.0, 0.0], dtype=np.float64),
            np.asarray([ord(">"), ord("<")], dtype=np.uint8),
            np.asarray([4.0, 10.0], dtype=np.float64),
        )
        self.assertEqual(returned_range, (0, 2))
        model.set_objective_flat(
            np.asarray(x, dtype=np.uintp),
            np.asarray([1.0, 1.0], dtype=np.float64),
            3.0,
            -1,
        )

        self.assertEqual(model.optimize(), 1)
        self.assertAlmostEqual(model.get_objective_value(), 5.0, places=7)

        with self.assertRaises(ValueError):
            model.add_constraints_flat(
                np.asarray([1, 1], dtype=np.uintp),
                np.asarray([], dtype=np.uintp),
                np.asarray([], dtype=np.float64),
                np.asarray([0.0], dtype=np.float64),
                np.asarray([ord("=")], dtype=np.uint8),
                np.asarray([0.0], dtype=np.float64),
            )

    def test_invalid_shapes_senses_and_parameter_types(self):
        model = self.new_model("low-level-invalid-input")
        with self.assertRaises(ValueError):
            model.add_constraint([0, 1], [1.0], 0.0, "<", 1.0)
        with self.assertRaises(ValueError):
            model.add_constraint([0], [1.0], 0.0, "!", 1.0)
        with self.assertRaises(ValueError):
            model.add_variable(name="bad\0name")
        with self.assertRaises(ValueError):
            model.add_variable(vtype="Q")
        with self.assertRaises(ValueError):
            model.add_variable(lb=float("nan"))
        with self.assertRaises(ValueError):
            model.set_objective([], [], 0.0, 0)
        with self.assertRaises(ValueError):
            model.get_var_value(0)
        with self.assertRaises(TypeError):
            model.set_optimizer_attr("Threads", "one")

    def test_repeated_environment_and_problem_lifecycle(self):
        for index in range(10):
            env = Env(str(COPT_DLL))
            model = Model(f"lifecycle-{index}", env=env)
            model.set_optimizer_attr("Logging", 0)
            model.add_variable(name="x", lb=0.0, ub=1.0)

    def test_explicit_environment_can_be_reused(self):
        env = Env(str(COPT_DLL))
        first = Model("first", env=env)
        second = Model("second", env=env)
        first.set_optimizer_attr("Logging", 0)
        second.set_optimizer_attr("Logging", 0)

        x = first.add_variable(name="x", lb=1.0, ub=2.0)
        first.set_objective([x], [1.0], 0.0, -1)
        self.assertEqual(first.optimize(), 1)
        self.assertAlmostEqual(first.get_var_value(x), 1.0, places=7)

        y = second.add_variable(name="y", lb=2.0, ub=3.0)
        second.set_objective([y], [1.0], 0.0, -1)
        self.assertEqual(second.optimize(), 1)
        self.assertAlmostEqual(second.get_var_value(y), 2.0, places=7)

        with self.assertRaises(ValueError):
            Model("invalid", str(COPT_DLL), env=env)

    def test_explicit_license_directory(self):
        env = Env(str(COPT_DLL), str(COPT_HOME))
        model = Model("license-dir", env=env)
        model.set_optimizer_attr("Logging", 0)
        x = model.add_variable(name="x", lb=1.0, ub=1.0)
        model.set_objective([x], [1.0], 0.0, -1)
        self.assertEqual(model.optimize(), 1)

    def test_environment_config_and_client_constants(self):
        expected = {
            "CLIENT_CAFILE": "CaFile",
            "CLIENT_CERTFILE": "CertFile",
            "CLIENT_CERTKEYFILE": "CertKeyFile",
            "CLIENT_CLUSTER": "Cluster",
            "CLIENT_FLOATING": "Floating",
            "CLIENT_PASSWORD": "PassWord",
            "CLIENT_PORT": "Port",
            "CLIENT_PRIORITY": "Priority",
            "CLIENT_WAITTIME": "WaitTime",
            "CLIENT_WEBSERVER": "WebServer",
            "CLIENT_WEBLICENSEID": "WebLicenseId",
            "CLIENT_WEBACCESSKEY": "WebAccessKey",
            "CLIENT_WEBTOKENDURATION": "WebTokenDuration",
        }
        for name, value in expected.items():
            self.assertEqual(getattr(COPT, name), value)

        config = EnvrConfig(str(COPT_DLL))
        config.set("NoBanner", True)
        env = Env(config=config)
        model = Model("configured-env", env=env)
        model.set_optimizer_attr("Logging", 0)
        x = model.add_variable(name="x", lb=2.0, ub=2.0)
        model.set_objective([x], [1.0], 0.0, -1)

        self.assertEqual(model.optimize(), 1)
        self.assertAlmostEqual(model.get_objective_value(), 2.0, places=7)

    def test_environment_config_validation_and_direct_model(self):
        config = EnvrConfig(str(COPT_DLL))
        config.set("NoBanner", 1)
        with self.assertRaises(TypeError):
            config.set("NoBanner", object())
        with self.assertRaises(ValueError):
            config.set("License", "invalid\0value")
        with self.assertRaises(ValueError):
            Env(str(COPT_DLL), config=config)

        model = Model("direct-config", config=config)
        model.set_optimizer_attr("Logging", 0)
        x = model.add_variable(name="x", lb=3.0, ub=3.0)
        model.set_objective([x], [1.0], 0.0, -1)
        self.assertEqual(model.optimize(), 1)
        self.assertAlmostEqual(model.get_objective_value(), 3.0, places=7)


if __name__ == "__main__":
    unittest.main()
