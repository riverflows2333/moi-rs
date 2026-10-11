import itertools
import unittest

import numpy as np

from moirspy import LinExpr, Model, quicksum


class VarsSumTests(unittest.TestCase):
    def test_explicit_ids_and_repeated_variables(self):
        # Vars can also be explicitly constructed, with unordered/repeated IDs.
        vars_type = type(Model("explicit-sum").addVars(1))
        x = vars_type([2, 2], [9, 3, 9, 5])
        self.assertEqual(str(x.sum()), str(quicksum([x[0, 0], x[0, 1], x[1, 0], x[1, 1]])))
        self.assertEqual(str(x.sum(dim=0)[0]), str(2 * x[0, 0]))
        self.assertEqual(str(x.sum("*", 1)), str(x[0, 1] + x[1, 1]))

    def test_patterns_match_explicit_sums(self):
        x = Model("sum-patterns").addVars(2, 3, 4)
        for pattern in itertools.product((0, 1, "*"), (0, 2, "*"), (0, 3, "*")):
            expected = quicksum(
                x[i, j, k]
                for i, j, k in itertools.product(range(2), range(3), range(4))
                if all(p == "*" or p == index for p, index in zip(pattern, (i, j, k)))
            )
            self.assertEqual(str(x.sum(*pattern)), str(expected))
        self.assertEqual(str(x.sum()), str(x.sum("*", "*", "*")))
        self.assertEqual(str(x.sum(99, "*", "*")), str(quicksum([])))
        self.assertEqual(str(x.sum(-1, "*", "*")), str(quicksum([])))

    def test_dimension_reduction_preserves_remaining_shape(self):
        x = Model("sum-dimensions").addVars(2, 3, 4)
        for dim in (0, 1, 2, -1, -2, -3):
            axis = dim % 3
            result = x.sum(dim=dim)
            shape = [2, 3, 4]
            shape.pop(axis)
            self.assertIsInstance(result, np.ndarray)
            self.assertEqual(result.shape, tuple(shape))
            self.assertEqual(result.dtype, np.dtype(object))
            for indices in np.ndindex(*shape):
                expected_indices = list(indices)
                expected_indices.insert(axis, "*")
                self.assertIsInstance(result[indices], LinExpr)
                self.assertEqual(str(result[indices]), str(x.sum(*expected_indices)))
        one = Model("one-dimensional-sum").addVars(3)
        self.assertIsInstance(one.sum(dim=0), LinExpr)
        self.assertEqual(str(one.sum(dim=-1)), str(one.sum("*")))
        self.assertEqual(str(one.sum(dim=None)), str(one.sum()))

    def test_invalid_patterns_and_dimensions(self):
        x = Model("invalid-sums").addVars(2, 3)
        for pattern in (("*",), ("*", "*", "*"), ("a*", 0), ("?", 0)):
            with self.assertRaises(ValueError):
                x.sum(*pattern)
        for pattern in (([0, 1], "*"), (1.5, "*"), (None, "*")):
            with self.assertRaises(TypeError):
                x.sum(*pattern)
        with self.assertRaises(TypeError):
            x.sum("*", 0, dim=0)
        for dim in (2, -3):
            with self.assertRaises(IndexError):
                x.sum(dim=dim)


if __name__ == "__main__":
    unittest.main()
