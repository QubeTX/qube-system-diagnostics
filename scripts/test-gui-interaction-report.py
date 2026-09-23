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


if __name__ == "__main__":
    unittest.main()
