from moirspy import MOI
from unittest import TestCase

class TestMOI(TestCase):
    def test_moi_import(self):
        continuous = MOI.CONTINUOUS
        self.assertEqual(str(continuous), "VarType.CONTINUOUS")

if __name__ == "__main__":
    import unittest

    unittest.main()
