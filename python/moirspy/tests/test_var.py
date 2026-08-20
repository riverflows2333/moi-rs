from unittest import TestCase
from moirspy import LinExpr, Var, dot, quicksum


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

    def test_quicksum_merges_duplicate_terms_and_constants(self):
        variable = Var(1)
        expr = quicksum((2 * variable, 3 * variable, 5.0))

        self.assertIn("coeff: 5.0", str(expr))
        self.assertIn("constant: 5.0", str(expr))

    def test_dot_constructs_expression_and_validates_lengths(self):
        variables = [Var(1), Var(2)]
        expr = dot([2.0, 3.0], variables)

        self.assertIsInstance(expr, LinExpr)
        self.assertIn("coeff: 2.0", str(expr))
        self.assertIn("coeff: 3.0", str(expr))
        with self.assertRaises(ValueError):
            dot([1.0], variables)
        with self.assertRaises(TypeError):
            dot([1.0], [object()])


if __name__ == "__main__":
    import unittest

    unittest.main()
