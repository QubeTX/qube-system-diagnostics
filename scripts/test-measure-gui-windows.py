"""Native accounting fixtures for the GUI performance harness; no GUI is opened."""
import importlib.util
import os
from pathlib import Path
import sys
import time
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("gui_measure", Path(__file__).with_name("measure-gui-windows.py"))
measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(measure)
PYTHON = sys._base_executable  # Avoid the Windows venv launcher's extra process.


class OwnedMeasurementTests(unittest.TestCase):
    def test_worker_expectations_match_frontend_subscriptions(self):
        self.assertEqual(measure.required_topics("Overview", False), set())
        self.assertEqual(measure.required_topics("Processes", False), set())
        self.assertEqual(measure.required_topics("Overview", True), {"slow"})
        self.assertEqual(measure.required_topics("Thermals", False), {"slow", "activity", "connections", "diagnostics"})

    def test_composite_cli_requires_owned_bundle_and_prefers_root(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            app = root / "app"
            app.mkdir()
            binary = app / "sd300-gui.exe"
            with self.assertRaisesRegex(RuntimeError, "incomplete"):
                measure.composite_cli(binary)
            adjacent = app / "sd300.exe"
            adjacent.touch()
            self.assertEqual(measure.composite_cli(binary), adjacent)
            (root / "bin").mkdir()
            preferred = root / "bin" / "sd300.exe"
            preferred.touch()
            self.assertEqual(measure.composite_cli(binary), preferred)

    def test_job_contains_descendants_and_retains_exited_cpu(self):
        job = measure.Job()
        try:
            child = "import time; end=time.perf_counter()+0.15\nwhile time.perf_counter()<end: pass"
            parent = f"import subprocess,sys; subprocess.run([sys.executable,'-c',{child!r}],check=True)"
            process = measure.spawn_owned(job, [PYTHON,"-c",parent], dict(os.environ), True)
            self.assertEqual(process.wait(timeout=10), 0)
            account = job.accounting()
            self.assertEqual(account.total, 2)
            self.assertEqual(account.active, 0)
            self.assertGreater((account.user+account.kernel)/1e7, .04)
            self.assertEqual(job.pids(), [])
        finally:
            job.close()
            job.close()  # Cleanup remains safe on exceptional and normal exits.

    def test_job_cleanup_terminates_owned_children(self):
        job = measure.Job()
        processes = []
        try:
            parent = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c','import time; time.sleep(30)']); time.sleep(30)"
            process = measure.spawn_owned(job, [PYTHON,"-c",parent], dict(os.environ), True)
            deadline = time.monotonic()+5
            while job.accounting().total < 2 and time.monotonic()<deadline:
                time.sleep(.01)
            self.assertEqual(job.accounting().total, 2)
            processes = [measure.psutil.Process(pid) for pid in job.pids()]
        finally:
            job.close()
        for process in processes:
            process.wait(timeout=5)
            self.assertFalse(process.is_running())

    def test_failed_start_has_no_owned_process(self):
        job = measure.Job()
        try:
            with self.assertRaises(OSError):
                measure.spawn_owned(job, [str(Path(sys.executable).with_name("sd300-missing-fixture.exe"))], dict(os.environ), True)
            self.assertEqual(job.accounting().total, 0)
        finally:
            job.close()


if __name__ == "__main__":
    unittest.main()
