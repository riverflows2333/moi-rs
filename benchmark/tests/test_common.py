from pathlib import Path
import unittest

from benchmark.common import load_minimal_uc


ROOT = Path(__file__).resolve().parents[2]


class MinimalUcDataTests(unittest.TestCase):
    def test_loads_supplied_small_case(self):
        data = load_minimal_uc(ROOT / "input" / "phys" / "effi" / "1-1")

        self.assertEqual(data.num_units, 100)
        self.assertEqual(data.num_periods, 96)
        self.assertEqual(data.loads[:3], (14000.0, 14250.0, 14500.0))
        self.assertEqual(data.units[0].unit_id, 1)
        self.assertEqual(data.units[0].p_max, 542.0)
        self.assertEqual(data.num_variables, 28_800)
        self.assertEqual(data.num_constraints, 48_192)
        self.assertEqual(data.num_nonzeros, 153_200)

    def test_accepts_model_directory_directly(self):
        data = load_minimal_uc(ROOT / "input" / "phys" / "effi" / "1-1" / "model")
        self.assertEqual(data.num_units, 100)


if __name__ == "__main__":
    unittest.main()
