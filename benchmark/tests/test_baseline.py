import unittest

from benchmark.baseline import (
    assert_formulation_matches,
    assert_runtime_dimensions,
    load_baseline,
)
from benchmark.common import formulation_stats
from benchmark.verify_model import verification_case


class DirectMigrationBaselineTests(unittest.TestCase):
    def test_tracked_baseline_has_expected_schema(self):
        baseline = load_baseline()

        self.assertEqual(baseline["verification_objective"], 5_040.0)
        self.assertEqual(set(baseline["cases"]), {"1-1", "1-3", "1-5"})

    def test_common_formulation_assertion_accepts_exact_statistics(self):
        stats = formulation_stats(verification_case(), include_fingerprint=True)

        assert_formulation_matches(stats, stats.as_dict(), label="verification")

    def test_common_formulation_assertion_reports_drift(self):
        stats = formulation_stats(verification_case(), include_fingerprint=True)
        expected = stats.as_dict()
        expected["constraints"] = stats.constraints + 1

        with self.assertRaisesRegex(AssertionError, "constraints"):
            assert_formulation_matches(stats, expected, label="verification")

    def test_runtime_dimension_assertion_is_backend_independent(self):
        expected = {"variables": 3, "constraints": 2, "nonzeros": 4}

        assert_runtime_dimensions(expected, expected, label="cached")
        with self.assertRaisesRegex(AssertionError, "nonzeros"):
            assert_runtime_dimensions(
                {**expected, "nonzeros": 5}, expected, label="direct"
            )


if __name__ == "__main__":
    unittest.main()
