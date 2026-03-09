from moipy import MOI
from unittest import TestCase

class TestMOI(TestCase):
    def test_moi_import(self):
        continuous = MOI.CONTINUOUS
        print(continuous)

if __name__ == "__main__":
    import unittest

    unittest.main()