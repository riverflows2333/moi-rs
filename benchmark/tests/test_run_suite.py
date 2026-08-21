import unittest

from benchmark.run_suite import markdown_report


class MarkdownReportTests(unittest.TestCase):
    def test_report_contains_summary_and_detailed_timing_boundary(self):
        metadata = {
            "python": "test-python",
            "platform": "test-platform",
            "rustc": "test-rustc",
            "copt_version": "test-copt",
            "extension_artifacts": {},
        }
        result = {
            "status": "ok",
            "case": "1-1",
            "tool": "moirspy-early",
            "variables": 10,
            "constraints": 20,
            "nonzeros": 30,
            "timing": {"median": 1.25, "p95": 1.5, "samples": 3},
            "memory": {"peak_rss_bytes": None, "peak_delta_bytes": None},
        }

        report = markdown_report(metadata, [result])

        self.assertIn("solve is excluded", report)
        self.assertIn("## Median construction time", report)
        self.assertIn("| 1-1 | 10 | 20 | 1.250000 |", report)
        self.assertIn("## Detailed samples and memory", report)


if __name__ == "__main__":
    unittest.main()
