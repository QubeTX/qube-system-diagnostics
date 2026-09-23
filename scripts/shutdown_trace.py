"""Opt-in native crash attribution after the GUI measurement window."""
import subprocess
import os
import tempfile
import time


class ShutdownTrace:
    def __init__(self, pid):
        self.output = tempfile.TemporaryFile()
        self.process = None
        try:
            self.process = subprocess.Popen([
                "gdb", "--nx", "--batch", "--quiet", "--pid", str(pid),
                "-ex", "set pagination off", "-ex", "set confirm off",
                "-ex", "set print frame-arguments none",
                "-ex", "echo SD300_DEBUGGER_ATTACHED\\n",
                "-ex", "continue", "-ex", "thread apply all bt 20",
                "-ex", "info sharedlibrary", "-ex", "x/12i $pc", "-ex", "detach",
            ], stdin=subprocess.DEVNULL, stdout=self.output, stderr=self.output)
            deadline = time.monotonic() + 8
            while time.monotonic() < deadline:
                if b"SD300_DEBUGGER_ATTACHED" in self.read():
                    return
                if self.process.poll() is not None:
                    break
                time.sleep(.05)
            raise RuntimeError("Shutdown debugger could not attach: " + self.read().decode("utf-8", "replace"))
        except BaseException:
            self.close()
            raise

    def read(self):
        # Reading must not move the shared file offset used by GDB's writer.
        contents = os.pread(self.output.fileno(), 1024 * 1024 + 1, 0)
        if len(contents) > 1024 * 1024:
            raise RuntimeError("Shutdown backtrace exceeded its output bound")
        return contents

    def finish(self):
        self.process.wait(timeout=10)
        return {"debugger_exit": self.process.returncode,
                "backtrace": self.read().decode("utf-8", "replace"),
                "note": "Debugger attached only for shutdown; this diagnostic is not resource qualification"}

    def close(self):
        if self.process is not None and self.process.poll() is None:
            self.process.kill()
            self.process.wait(timeout=5)
        self.output.close()
