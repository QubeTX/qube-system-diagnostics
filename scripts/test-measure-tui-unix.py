"""Native qualification of completed descendant CPU and terminal cleanup."""
import importlib.util
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest


@unittest.skipIf(os.name == "nt", "Unix wait4 and PTY fixture")
class Accounting(unittest.TestCase):
    def test_waited_descendant_cpu_is_counted_and_terminal_restores(self):
        spec = importlib.util.spec_from_file_location("measure", Path(__file__).with_name("measure-tui-unix.py"))
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory(prefix="sd300-accounting-") as directory:
            fixture = Path(directory) / "fixture"
            fixture.write_text(f"#!{sys.executable}\n" + '''import os,subprocess,sys,tty
tty.setraw(0)
subprocess.run([sys.executable, "-c", "import time; start=time.process_time(); exec('while time.process_time()-start<.25: pass')"],check=True)
os.write(1,b"\\x1b[?1049hready")
while os.read(0,1)!=b"q": pass
os.write(1,b"\\x1b[?1049l")
''', encoding="utf-8")
            fixture.chmod(0o700)
            terminal = module.Terminal(fixture, dict(os.environ), 80, 24)
            try:
                deadline = time.monotonic() + 8
                while b"ready" not in terminal.tail and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertIn(b"ready", terminal.tail)
                usage = terminal.quit()
                self.assertGreaterEqual(usage.ru_utime + usage.ru_stime, .20)
            finally:
                terminal.close()


if __name__ == "__main__":
    unittest.main()
