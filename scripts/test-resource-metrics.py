import unittest
from types import SimpleNamespace
import psutil
from resource_metrics import sample_family, descriptor_summary, FamilyAttribution


class Process:
    pid = 42
    def __init__(self, denied=False):
        self.denied = denied
    def create_time(self):
        return 100
    def children(self, recursive):
        return []
    def memory_info(self):
        return SimpleNamespace(rss=1024 * 1024)
    def num_fds(self):
        if self.denied:
            raise psutil.AccessDenied(self.pid)
        return 0
    def cpu_times(self):
        return SimpleNamespace(user=.2, system=.1)
    def cmdline(self):
        return ["/private/path/sd300", "collect-server", "slow"]


class Metrics(unittest.TestCase):
    def test_descriptor_denial_preserves_memory_and_process_identity(self):
        known = {}
        sample = sample_family(Process(denied=True), known)
        self.assertEqual(sample, (1, None, 1))
        self.assertIn((42, 100), known)
        summary = descriptor_summary([sample, (1, 9, 1)])
        self.assertIsNone(summary["fd_count_max"])
        self.assertEqual(summary["observed_fd_count_max"], 9)
        self.assertEqual(summary["fd_unavailable_samples"], 1)

    def test_measured_zero_descriptors_remains_zero(self):
        summary = descriptor_summary([sample_family(Process(), {})])
        self.assertEqual(summary["fd_count_max"], 0)
        self.assertEqual(summary["fd_unavailable_samples"], 0)

    def test_attribution_counts_each_identity_once_and_reports_only_fixed_roles(self):
        attribution = FamilyAttribution(1)
        sample_family(Process(), {}, attribution)
        sample_family(Process(), {}, attribution)
        report = attribution.report()
        role = report["diagnostic_live_role_cost"]["collector:slow"]
        self.assertAlmostEqual(role["observed_cpu_seconds"], .3)
        self.assertEqual(report["rss_peak_roles"]["collector:slow"]["rss_mib"], 1)
        self.assertNotIn("private", str(report))
        reused = Process()
        reused.create_time = lambda: 101
        sample_family(reused, {}, attribution)
        self.assertAlmostEqual(attribution.report()["diagnostic_live_role_cost"]["collector:slow"]["observed_cpu_seconds"], .6)


if __name__ == "__main__":
    unittest.main()
