"""Measure release TUI resources on native Linux/macOS without a screen observer.

CPU uses wait4's completed process-family accounting, including waited children.
Startup and orderly shutdown CPU are included conservatively; RSS/fds are sums
of live descendants every 250 ms, not the largest-child ru_maxrss statistic.
Requires psutil==7.2.2. Run after builds and interaction observers have finished.
"""
import argparse
import errno
import fcntl
import hashlib
import json
import os
from pathlib import Path
import pty
import re
import signal
import struct
import sys
import tempfile
import termios
import threading
import time

import psutil


class Terminal:
    def __init__(self, binary, env, columns, rows):
        self.pid, self.fd = pty.fork()
        if self.pid == 0:
            try:
                fcntl.ioctl(1, termios.TIOCSWINSZ, struct.pack("HHHH", rows, columns, 0, 0))
                os.execve(binary, [str(binary), "--user"], env)
            except BaseException:
                os._exit(127)
        self.result = None
        self.tail = b""
        self.reader_error = None
        self.reader = threading.Thread(target=self.drain, daemon=True)
        self.reader.start()

    def drain(self):
        pending = b""
        try:
            while chunk := os.read(self.fd, 65536):
                self.tail = (self.tail + chunk)[-16384:]
                pending += chunk
                # Answer terminal setup queries without decoding or inspecting
                # the screen. The retained tail is never exported.
                for match in re.finditer(rb"\x1b\[(?:6n|5n|c|0c)", pending):
                    query = match.group()
                    os.write(self.fd, b"\x1b[1;1R" if query.endswith(b"6n") else b"\x1b[0n" if query.endswith(b"5n") else b"\x1b[?1;2c")
                pending = pending[pending.rfind(b"\x1b"):] if b"\x1b" in pending else b""
                if pending not in (b"\x1b", b"\x1b[", b"\x1b[6", b"\x1b[5", b"\x1b[0"):
                    pending = b""
        except OSError as error:
            if error.errno not in (errno.EIO, errno.EBADF):
                self.reader_error = str(error)

    def poll(self):
        if self.result is None:
            pid, status, usage = os.wait4(self.pid, os.WNOHANG)
            if pid:
                self.result = (os.waitstatus_to_exitcode(status), usage)
        return self.result

    def quit(self):
        os.write(self.fd, b"q")
        deadline = time.monotonic() + 15
        while self.poll() is None and time.monotonic() < deadline:
            time.sleep(.05)
        if self.result is None:
            raise RuntimeError("TUI did not shut down within 15 seconds")
        self.reader.join(timeout=2)
        if self.reader.is_alive() or self.reader_error:
            raise RuntimeError("Terminal pipe did not drain cleanly")
        if self.result[0] != 0 or b"\x1b[?1049l" not in self.tail:
            raise RuntimeError(f"TUI exit/restoration failed: code {self.result[0]}")
        return self.result[1]

    def close(self):
        if self.poll() is None:
            try:
                root = psutil.Process(self.pid)
                children = root.children(recursive=True)
            except psutil.NoSuchProcess:
                children = []
            # Native helpers may own their own process group. Keep their
            # psutil creation-time identity before terminating the ancestor.
            for child in reversed(children):
                try:
                    child.kill()
                except psutil.NoSuchProcess:
                    pass
            try:
                os.killpg(self.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            _, status, usage = os.wait4(self.pid, 0)
            self.result = (os.waitstatus_to_exitcode(status), usage)
        os.close(self.fd)
        self.reader.join(timeout=2)


def main():
    def interrupted(_signal, _frame):
        raise KeyboardInterrupt("Qualification interrupted")
    signal.signal(signal.SIGTERM, interrupted)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--seconds", type=int, default=900)
    parser.add_argument("--columns", type=int, default=80)
    parser.add_argument("--rows", type=int, default=24)
    parser.add_argument("--enforce-gates", action="store_true")
    args = parser.parse_args()
    if not 5 <= args.seconds <= 7200:
        parser.error("Duration must be 5..7200 seconds")
    binary = args.binary.resolve(strict=True)
    report = {"schema": 2, "binary": str(binary), "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
              "revision": args.revision, "platform": sys.platform, "build": "release", "dimensions": [args.columns, args.rows],
              "cpu_method": "wait4 user+system including waited descendants; startup and shutdown included; 100% = one logical core",
              "memory_method": "250 ms summed live descendant RSS, conservatively counts shared mappings per process"}
    terminal = None
    known = {}
    try:
        with tempfile.TemporaryDirectory(prefix="sd300-resource-") as settings:
            env = dict(os.environ, HOME=settings, XDG_CONFIG_HOME=settings, TERM="xterm-256color")
            begin = time.monotonic()
            terminal = Terminal(binary, env, args.columns, args.rows)
            root = psutil.Process(terminal.pid)
            samples = []
            while time.monotonic() - begin < args.seconds:
                if terminal.poll() is not None:
                    raise RuntimeError(f"TUI exited during measurement: {terminal.result[0]}")
                rss = fds = count = 0
                for child in [root, *root.children(recursive=True)]:
                    try:
                        identity = (child.pid, child.create_time())
                        known[identity] = child
                        rss += child.memory_info().rss
                        fds += child.num_fds()
                        count += 1
                    except psutil.NoSuchProcess:
                        pass
                if len(known) > 4096:
                    raise RuntimeError("Observed process identities exceed the bounded qualification inventory")
                samples.append((rss / 2**20, fds, count))
                time.sleep(.25)
            elapsed = time.monotonic() - begin
            usage = terminal.quit()
            remaining = [p for p in known.values() if p.is_running()]
            if remaining:
                raise RuntimeError("Owned process remained after normal shutdown")
            cpu = (usage.ru_utime + usage.ru_stime) / elapsed * 100
            peak = max(row[0] for row in samples)
            window = max(1, len(samples)//10)
            report.update(measured_seconds=elapsed, samples=len(samples), cpu_percent_one_core=cpu,
                          cpu_gate=cpu <= 2, rss_mib_max=peak, rss_gate=peak <= 150,
                          fd_count_max=max(row[1] for row in samples), process_count_max=max(row[2] for row in samples),
                          observed_process_identities=len(known), terminal_restored=True, clean_shutdown=True,
                          rss_last_window_delta=sum(r[0] for r in samples[-window:])/window-sum(r[0] for r in samples[:window])/window)
    except BaseException as error:
        report.update(passed=False, failure_type=type(error).__name__, failure=str(error))
        raise
    finally:
        if terminal:
            terminal.close()
        for child in known.values():
            try:
                if child.is_running():
                    child.kill()
            except psutil.NoSuchProcess:
                pass
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(report, indent=2))
    if args.enforce_gates and not (report["cpu_gate"] and report["rss_gate"]):
        raise SystemExit("Measured TUI resource gates did not pass")


if __name__ == "__main__":
    main()
