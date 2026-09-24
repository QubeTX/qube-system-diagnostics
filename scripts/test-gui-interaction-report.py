"""Privacy and identity fixtures for retained native interaction diagnostics."""
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("interaction", Path(__file__).with_name("qualify-gui-interaction.py"))
interaction = importlib.util.module_from_spec(spec)
spec.loader.exec_module(interaction)


class NumericObservationTests(unittest.TestCase):
    def test_keeps_input_and_frame_identity_without_private_widget_text(self):
        snapshot = '''ready=true runtime_uptime_ns=8000 publisher_pid=27
  view @w1/main-canvas gpu_frame=14 gpu_timestamp_ns=7900 gpu_input_timestamp_ns=7800 gpu_input_latency_ns=100
    widget @w1/main-canvas#91 role=listitem name="private-device" focused=true enabled=true
    widget @w1/main-canvas#92 role=button name="private-process" focused=false enabled=true
frame_profile input_latency_n=2 frame_work_n=4 present_n=3
'''
        self.assertEqual(interaction.numeric_observation(snapshot), {
            "runtime_uptime_ns": 8000, "gpu_frame": 14, "gpu_timestamp_ns": 7900,
            "gpu_input_timestamp_ns": 7800, "gpu_input_latency_ns": 100,
            "input_latency_n": 2, "frame_work_n": 4, "present_n": 3,
            "focus": [{"id": 91, "role": "listitem"}]})

    def test_bounds_focus_and_does_not_copy_unknown_roles(self):
        snapshot = "\n".join(f'widget @w1/main-canvas#{i} role=private_name name="secret" focused=true' for i in range(30))
        result = interaction.numeric_observation(snapshot)
        self.assertEqual(len(result["focus"]), 16)
        self.assertTrue(all(item["role"] == "other" for item in result["focus"]))
        self.assertNotIn("private", str(result))
        self.assertNotIn("secret", str(result))

    def test_missing_measurements_remain_absent(self):
        self.assertEqual(interaction.numeric_observation(""), {"focus": []})

    def test_latency_splits_are_numeric_and_do_not_replace_end_to_end_latency(self):
        result = interaction.numeric_observation(
            'gpu_input_latency_ns=650000000 input_dispatch_latest_us=3000 '
            'input_wait_latest_us=640000 automation_publish_latest_us=600000 '
            'automation_publish_total_max_us=620000 private_detail="secret"')
        self.assertEqual(result['gpu_input_latency_ns'], 650000000)
        self.assertEqual(result['input_wait_latest_us'], 640000)
        self.assertEqual(result['automation_publish_latest_us'], 600000)
        self.assertNotIn('secret', str(result))


class SnapshotReadTests(unittest.TestCase):
    valid = b"ready=true protocol=7 publisher_pid=27 runtime_uptime_ns=8000 dispatch_errors=0\n"

    def test_partial_snapshot_is_retried_within_the_existing_deadline(self):
        for data in (b"", b"ready=true", self.valid[:-1], b"ready=true\n", self.valid + b"\xc3"):
            self.assertEqual(interaction.decode_snapshot(data, 27), "")
        self.assertEqual(interaction.decode_snapshot(self.valid, 27), self.valid.decode())

    def test_complete_wrong_identity_and_dispatch_errors_still_fail(self):
        with self.assertRaisesRegex(RuntimeError, "identity changed"):
            interaction.decode_snapshot(self.valid, 99)
        with self.assertRaisesRegex(RuntimeError, "dispatch reported"):
            interaction.decode_snapshot(self.valid.replace(b"dispatch_errors=0", b"dispatch_errors=1"), 27)
        with self.assertRaisesRegex(RuntimeError, "exceeds its bound"):
            interaction.decode_snapshot(b"x" * (1024 * 1024 + 1), 27)


class TimingPolicyTests(unittest.TestCase):
    def test_duplicate_or_lost_input_remains_a_functional_failure(self):
        interaction.require_input_count({"input_latency_n": 20}, 20)
        for actual in (None, 0, 19, 21, 38):
            with self.assertRaisesRegex(RuntimeError, "lost or duplicated"):
                interaction.require_input_count({"input_latency_n": actual}, 20)

    def cohorts(self, frame=28000, input_latency=42000, refresh=21000):
        return [{"kind": "navigation", "frame_work_p95_us": frame, "input_latency_p95_us": input_latency},
                {"kind": "refresh", "frame_work_p95_us": 12000, "frame_work_total_max_us": refresh}]

    def test_operator_policy_applies_to_all_three_operating_systems(self):
        for platform in ("win32", "linux", "darwin"):
            result = interaction.timing_verdict(self.cohorts(), "4.0.0", platform)
            self.assertTrue(result["timing_passed"])
            self.assertFalse(result["frame_gate"])
            self.assertTrue(result["input_gate"])
            self.assertEqual(result["timing_limits_us"]["frame_p95"], 100000)

    def test_future_versions_and_unknown_platforms_keep_original_limits(self):
        for version, platform in (("4.0.2", "darwin"), ("4.1.0", "win32"), ("4.0.0-rc.1", "linux"), ("4.0.0", "unknown")):
            result = interaction.timing_verdict(self.cohorts(), version, platform)
            self.assertFalse(result["timing_passed"])
            self.assertEqual(result["timing_policy"], "original-targets")

    def test_authorized_updater_correction_keeps_the_same_bounded_policy(self):
        for platform in ("win32", "linux", "darwin"):
            self.assertTrue(interaction.timing_verdict(self.cohorts(), "4.0.1", platform)["timing_passed"])
            self.assertFalse(interaction.timing_verdict(self.cohorts(input_latency=100001), "4.0.1", platform)["timing_passed"])

    def test_each_release_limit_remains_enforced(self):
        for cohorts in (self.cohorts(frame=100001), self.cohorts(input_latency=100001), self.cohorts(refresh=100001)):
            self.assertFalse(interaction.timing_verdict(cohorts, "4.0.0", "darwin")["timing_passed"])
        self.assertTrue(interaction.timing_verdict(self.cohorts(frame=100000, input_latency=100000, refresh=100000), "4.0.0", "darwin")["timing_passed"])

    def test_missing_measurements_do_not_pass(self):
        for cohorts in ([], self.cohorts()[:1], self.cohorts()[1:]):
            self.assertFalse(interaction.timing_verdict(cohorts, "4.0.0", "linux")["timing_passed"])
        with self.assertRaises(KeyError):
            interaction.timing_verdict([{"kind": "navigation"}], "4.0.0", "linux")


if __name__ == "__main__":
    unittest.main()
