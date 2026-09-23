import unittest
import runpy
from pathlib import Path
from types import SimpleNamespace
import psutil
from resource_metrics import sample_family, descriptor_summary, FamilyAttribution, linux_mapping_totals, remaining_family


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
    def test_completed_window_metrics_survive_a_later_shutdown_failure(self):
        record = runpy.run_path(str(Path(__file__).with_name("measure-gui-unix.py")))["record_window"]
        report = {}
        record(report, [(10, 4, 1), (12, None, 2)], 330, {(42, 100): Process()}, FamilyAttribution(42))
        self.assertEqual(report["measured_seconds"], 330)
        self.assertEqual(report["rss_mib_max"], 12)
        self.assertEqual(report["process_count_max"], 2)
        self.assertIsNone(report["fd_count_max"])
        self.assertEqual(report["rss_last_window_delta"], 2)
        self.assertNotIn("clean_shutdown", report)
        self.assertNotIn("cpu_percent_one_core", report)  # Requires final wait4 accounting.

    def test_shutdown_evidence_distinguishes_zombies_without_private_paths(self):
        child = Process()
        child.is_running = lambda: True
        child.status = lambda: psutil.STATUS_ZOMBIE
        child.ppid = lambda: 1
        attribution = FamilyAttribution(1)
        known = {}
        sample_family(child, known, attribution)
        report = remaining_family(known, attribution)
        self.assertEqual(report["count"], 1)
        self.assertEqual(report["processes"][0]["state"], "zombie")
        self.assertEqual(report["processes"][0]["role"], "collector:slow")
        self.assertNotIn("private", str(report))

    def test_collector_presence_does_not_inspect_protected_helper_executables(self):
        topics = runpy.run_path(str(Path(__file__).with_name("measure-gui-unix.py")))["topics"]
        def denied():
            raise psutil.AccessDenied(44)
        cli = Path("sd300").resolve()
        helper = SimpleNamespace(cmdline=lambda: ["ping", "example.invalid"], exe=denied)
        worker = SimpleNamespace(cmdline=lambda: [str(cli), "collect-server", "slow"], exe=lambda: str(cli))
        unreadable = SimpleNamespace(cmdline=denied)
        root = SimpleNamespace(children=lambda recursive: [helper, worker, unreadable])
        self.assertEqual(topics(root, cli), {"slow"})
        root.children = lambda recursive: [helper, unreadable]
        self.assertEqual(topics(root, cli), set())

    def test_mapping_units_and_redaction(self):
        report = linux_mapping_totals("""1000-2000 r-xp 000000 00:00 0 /private/user/libLLVM.so
Rss: 2048 kB
2000-3000 rw-p 000000 00:00 0
Rss: 1024 kB
3000-4000 r-xp 000000 00:00 0 /private/user/sd300-gui
Rss: 512 kB
""")
        self.assertEqual(report, {"graphics_libraries": 2, "anonymous": 1, "product_files": .5})
        self.assertNotIn("private", str(report))
        with self.assertRaises(ValueError):
            linux_mapping_totals("")
        with self.assertRaises(ValueError):
            linux_mapping_totals("Rss: 12 bytes")

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
