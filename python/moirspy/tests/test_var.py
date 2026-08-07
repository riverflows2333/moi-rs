from unittest import TestCase
from moirspy import LinExpr, Var


class TestVar(TestCase):
    def test_var_add(self):
        v1 = Var(1)
        v2 = Var(2)
        expr = v1 + v2 + 3
        self.assertIsInstance(expr, LinExpr)

    def test_var_expr_add(self):
        v1 = Var(1)
        v2 = Var(2)
        expr1 = v1 + v2
        v3 = Var(3)
        expr2 = expr1 + v3
        self.assertIsInstance(expr2, LinExpr)

    def test_multi_var_expr(self):
        v1 = Var(1)
        v2 = Var(2)
        v3 = Var(3)
        expr = 6 + v1 + 2 * v2 - (3 * v3 - 5) * 3
        self.assertIsInstance(expr, LinExpr)


if __name__ == "__main__":
    import unittest

    unittest.main()
