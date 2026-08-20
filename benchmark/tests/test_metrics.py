import unittest

from benchmark.metrics import TimingSummary, percentile, process_memory_bytes


class MetricsTests(unittest.TestCase):
    def test_percentile_interpolates_and_handles_single_sample(self):
        self.assertEqual(percentile([2.0], 0.95), 2.0)
        self.assertAlmostEqual(percentile([1.0, 2.0, 3.0], 0.95), 2.9)

    def test_summary_preserves_bounds_and_sample_count(self):
        summary = TimingSummary.from_samples([3.0, 1.0, 2.0])
        self.assertEqual(summary.median, 2.0)
        self.assertEqual(summary.minimum, 1.0)
        self.assertEqual(summary.maximum, 3.0)
        self.assertEqual(summary.samples, 3)

    def test_process_memory_is_positive_when_supported(self):
        current, peak = process_memory_bytes()
        if current is not None:
            self.assertGreater(current, 0)
        if peak is not None:
            self.assertGreaterEqual(peak, current or 0)


if __name__ == "__main__":
    unittest.main()
