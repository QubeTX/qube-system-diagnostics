"""The approved exception changes resource ceilings, never missing/failing evidence."""
import copy
import unittest
from resource_policy import resource_verdict


class ResourcePolicyTests(unittest.TestCase):
    def report(self):
        return dict(build="release", measured_seconds=900, samples=3000,
                    cpu_percent_one_core=4, rss_mib_max=200, private_mib_max=300,
                    cpu_gate=False, foreground_cpu_gate=False, rss_gate=False,
                    private_gate=True, clean_shutdown=True, terminal_restored=True)

    def assess(self, report, **kwargs):
        return resource_verdict(report, "4.0.0", frontend="gui", **kwargs)

    def test_boundaries_and_original_verdicts_are_preserved(self):
        report = self.report()
        before = copy.deepcopy(report)
        result = self.assess(report, windows=True)
        self.assertTrue(result["passed"])
        self.assertFalse(result["original_gates"]["cpu_gate"])
        self.assertEqual(report, before)
        for key in ("cpu_percent_one_core", "rss_mib_max", "private_mib_max"):
            changed = {**report, key: report[key] + .001}
            self.assertFalse(self.assess(changed, windows=True)["passed"])

    def test_hidden_limit_is_separate(self):
        report = {**self.report(), "cpu_percent_one_core": 3}
        self.assertTrue(self.assess(report, hidden=True)["passed"])
        report["cpu_percent_one_core"] = 3.001
        self.assertFalse(self.assess(report, hidden=True)["passed"])

    def test_authorized_updater_correction_keeps_the_v4_bar(self):
        for version in ("4.0.0", "4.0.1"):
            self.assertTrue(resource_verdict(self.report(), version, frontend="gui")["passed"])
            self.assertFalse(resource_verdict({**self.report(), "clean_shutdown": False}, version, frontend="gui")["passed"])

    def test_exception_expires_for_every_other_version(self):
        for version in ("3.1.3", "4.0.2", "4.1.0", "4.0.0-rc.1", ""):
            self.assertFalse(resource_verdict(self.report(), version, frontend="gui")["passed"])
        report = {**self.report(), "cpu_percent_one_core": 1, "rss_mib_max": 150}
        self.assertTrue(resource_verdict(report, "4.0.2", frontend="gui", hidden=True)["passed"])

    def test_invalid_or_missing_numbers_never_pass(self):
        for key in ("cpu_percent_one_core", "rss_mib_max", "private_mib_max", "measured_seconds"):
            for value in (None, float("nan"), float("inf"), -1, True, "0"):
                self.assertFalse(self.assess({**self.report(), key: value}, windows=True)["passed"])

    def test_functional_failures_and_partial_measurements_still_fail(self):
        for key, value in (("clean_shutdown", False), ("passed", False),
                           ("failure", "cancelled"), ("diagnostic_only", True),
                           ("build", "debug"), ("cpu_gate", None),
                           ("measured_seconds", 899), ("samples", 0)):
            self.assertFalse(self.assess({**self.report(), key: value})["passed"])

    def test_tui_requires_restored_terminal(self):
        for windows in (True, False):
            report = self.report()
            self.assertTrue(resource_verdict(report, "4.0.0", frontend="tui", windows=windows)["passed"])
            report["terminal_restored"] = False
            self.assertFalse(resource_verdict(report, "4.0.0", frontend="tui", windows=windows)["passed"])

    def test_invalid_modes_fail(self):
        with self.assertRaises(ValueError):
            resource_verdict(self.report(), "4.0.0", frontend="tui", hidden=True)
        with self.assertRaises(ValueError):
            resource_verdict(self.report(), "4.0.0", frontend="unknown")


if __name__ == "__main__":
    unittest.main()
