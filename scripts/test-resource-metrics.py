import unittest
from types import SimpleNamespace
import psutil
from resource_metrics import sample_family, descriptor_summary


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


if __name__ == "__main__":
    unittest.main()
