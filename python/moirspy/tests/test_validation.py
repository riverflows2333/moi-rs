import math
import sys
import types
from unittest import TestCase

from moirspy import MOI, Model, Var, dot, quicksum


class TestPythonInputValidation(TestCase):
    def test_invalid_expression_operands_raise_python_errors(self):
        x = Var(0)
        expr = x + 1.0

        for operation in (
            lambda: x + object(),
            lambda: x * x,
            lambda: expr * x,
            lambda: expr <= object(),
        ):
            with self.assertRaises(TypeError):
                operation()

    def test_nonfinite_expression_values_are_rejected(self):
        x = Var(0)
        for value in (math.nan, math.inf, -math.inf):
            with self.assertRaises(ValueError):
                _ = x + value
            with self.assertRaises(ValueError):
                _ = x <= value
            with self.assertRaises(ValueError):
                quicksum((x, value))
            with self.assertRaises(ValueError):
                dot([value], [x])

    def test_obj_keyword_is_never_silently_ignored(self):
        model = Model("objective-validation")

        with self.assertRaisesRegex(ValueError, "setObjective"):
            model.addVar(obj=1.0)
        with self.assertRaisesRegex(ValueError, "finite"):
            model.addVar(obj=math.nan)
        with self.assertRaisesRegex(ValueError, "setObjective"):
            model.addVars(2, obj=[0.0, 1.0])

    def test_shape_lengths_names_bounds_and_indices_are_validated(self):
        model = Model("shape-validation")

        with self.assertRaises(ValueError):
            model.addVars()
        with self.assertRaises(ValueError):
            model.addVars(0)
        with self.assertRaises(OverflowError):
            model.addVars(sys.maxsize, 2)
        with self.assertRaises(ValueError):
            model.addVars(2, lb=[0.0])
        with self.assertRaises(ValueError):
            model.addVar(name="bad\0name")
        with self.assertRaises(ValueError):
            model.addVar(lb=math.nan)
        with self.assertRaises(ValueError):
            model.addVar(lb=2.0, ub=1.0)

        variables = model.addVars(2, 3)
        with self.assertRaises(IndexError):
            _ = variables[2, 0]
        with self.assertRaises(IndexError):
            _ = variables[0]
        with self.assertRaises(TypeError):
            _ = variables["0", 0]

    def test_unsolved_result_is_distinct_from_an_error(self):
        model = Model("unsolved-result")
        x = model.addVar()

        self.assertIsNone(x.X)
        self.assertIsNone(model.ObjVal)

    def test_direct_constructor_requires_a_supported_backend(self):
        with self.assertRaisesRegex(ValueError, "env requires"):
            Model("env-without-backend", env=object())
        with self.assertRaisesRegex(ValueError, "currently supported: 'copt'"):
            Model("unsupported-direct", backend="not-a-solver")

    def test_backend_getter_exceptions_are_propagated(self):
        class BrokenBackend:
            def __init__(self, *_args):
                pass

            def add_variables(self, n, *_args):
                return list(range(n))

            def set_optimizer_attr(self, *_args):
                pass

            def optimize(self):
                return 1

            def get_var_value(self, _variable):
                raise RuntimeError("injected getter failure")

            def get_objective_value(self):
                raise RuntimeError("injected objective getter failure")

        module_name = "moirspy_broken"
        sys.modules[module_name] = types.SimpleNamespace(Model=BrokenBackend)
        try:
            model = Model("getter-propagation")
            x = model.addVar()
            model.setBackend("broken")
            model.optimize()

            with self.assertRaisesRegex(RuntimeError, "get_var_value failed"):
                _ = x.X
            with self.assertRaisesRegex(RuntimeError, "get_objective_value failed"):
                _ = model.ObjVal
        finally:
            sys.modules.pop(module_name, None)

    def test_legacy_python_backend_protocol_remains_supported(self):
        class LegacyBackend:
            instance = None

            def __init__(self, *_args):
                type(self).instance = self
                self.num_variables = 0
                self.num_constraints = 0
                self.rows = None
                self.objective = None

            def add_variables(self, n, *_args):
                start = self.num_variables
                self.num_variables += n
                return list(range(start, self.num_variables))

            def add_constraints(
                self, variables, coefficients, constants, senses, rhs, names
            ):
                self.rows = (variables, coefficients, constants, senses, rhs, names)
                start = self.num_constraints
                self.num_constraints += len(variables)
                return list(range(start, self.num_constraints))

            def set_objective(self, variables, coefficients, constant, sense):
                self.objective = (variables, coefficients, constant, sense)

            def set_optimizer_attr(self, *_args):
                pass

            def update(self):
                pass

        module_name = "moirspy_legacy"
        sys.modules[module_name] = types.SimpleNamespace(Model=LegacyBackend)
        try:
            model = Model("legacy-protocol")
            x = model.addVars(2)
            model.addConstrs((x[i] >= i + 1 for i in range(2)))
            model.setObjective(x[0] + 2 * x[1], MOI.MINIMIZE)
            model.setBackend("legacy")

            backend = LegacyBackend.instance
            self.assertEqual(backend.rows[0], [[0], [1]])
            self.assertEqual(backend.rows[1], [[1.0], [1.0]])
            self.assertEqual(backend.rows[3], [">", ">"])
            self.assertEqual(backend.objective[:2], ([0, 1], [1.0, 2.0]))
        finally:
            sys.modules.pop(module_name, None)


if __name__ == "__main__":
    import unittest

    unittest.main()
