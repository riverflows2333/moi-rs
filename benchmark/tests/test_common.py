from pathlib import Path
import unittest

from benchmark.common import iter_constraint_groups, load_minimal_uc
from benchmark.verify_model import verification_case


ROOT = Path(__file__).resolve().parents[2]


class CompleteUcDataTests(unittest.TestCase):
    def test_loads_supplied_small_case(self):
        data = load_minimal_uc(ROOT / "input" / "phys" / "effi" / "1-1")

        self.assertEqual(data.num_units, 100)
        self.assertEqual(data.num_storages, 15)
        self.assertEqual(len(data.sections), 7)
        self.assertEqual(data.num_periods, 96)
        self.assertEqual(data.loads[:3], (14000.0, 14250.0, 14500.0))
        self.assertEqual(data.units[0].unit_id, 1)
        self.assertEqual(data.units[0].p_max, 542.0)
        self.assertEqual(data.units[0].minimum_on, 16)
        self.assertEqual(len(data.units[0].cost_lines), 5)
        self.assertEqual(data.num_variables, 58_080)
        self.assertEqual(data.num_constraints, 147_899)
        self.assertEqual(data.num_nonzeros, 883_717)

    def test_accepts_model_directory_directly(self):
        data = load_minimal_uc(ROOT / "input" / "phys" / "effi" / "1-1" / "model")
        self.assertEqual(data.num_units, 100)

    def test_verification_case_activates_every_constraint_family(self):
        data = verification_case()
        family_counts = {
            family: sum(1 for _ in rows)
            for family, rows in iter_constraint_groups(data)
        }

        self.assertEqual(data.num_variables, 68)
        self.assertEqual(data.num_constraints, 128)
        self.assertEqual(data.num_nonzeros, 313)
        self.assertTrue(all(count > 0 for count in family_counts.values()))


if __name__ == "__main__":
    unittest.main()
