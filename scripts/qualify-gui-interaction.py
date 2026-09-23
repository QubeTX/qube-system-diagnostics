"""Drive a release-shaped automation build through real retained-widget input.

This is interaction qualification, never a CPU/RSS benchmark. Only aggregate
timings and authored control names leave the isolated session; snapshots may
contain private process/device names and are not exported.
"""
import argparse
from contextlib import ExitStack
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import signal
import shutil
import socket
import subprocess
import sys
import tempfile
import time

import psutil


def windows_helper():
    spec = importlib.util.spec_from_file_location("gui_measure", Path(__file__).with_name("measure-gui-windows.py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class Session:
    def __init__(self, binary, home):
        self.binary, self.home = binary, home
        self.ipc = binary.parent / ".zig-cache/native-sdk-automation"
        if self.ipc.exists():
            raise RuntimeError("Use a fresh qualification build directory; existing automation state is not owned")
        self.process = self.job = None
        self.closed = False
        self.sequence = 0
        self.win = windows_helper() if sys.platform == "win32" else None

    def start(self):
        if self.win:
            self.win.ensure_no_existing_gui()
            config = self.home / "SD-300"
            env = dict(os.environ, APPDATA=str(self.home), LOCALAPPDATA=str(self.home))
        else:
            config = self.home / ("Library/Application Support/SD-300" if sys.platform == "darwin" else "config/sd300")
            env = dict(os.environ, HOME=str(self.home), XDG_CONFIG_HOME=str(self.home / "config"), XDG_RUNTIME_DIR=str(self.home / "runtime"))
            (self.home / "runtime").mkdir(mode=0o700)
            self.endpoint = Path(f"/tmp/sd300-{os.geteuid()}/gui.sock") if sys.platform == "darwin" else self.home / "runtime/sd300/gui.sock"
            if self.endpoint.exists():
                raise RuntimeError("Existing GUI session must remain untouched")
        config.mkdir(parents=True)
        (config / "settings.json").write_text(json.dumps({"schema_version": 1, "gui": {
            "audience_mode": "user", "last_section": 0, "tray_enabled": False,
            "close_to_tray": False, "launch_at_login": False, "reduced_motion": True}}), encoding="utf-8")
        if self.win:
            self.job = self.win.Job()
            self.process = self.win.spawn_owned(self.job, [self.binary], env, False)
        else:
            self.process = subprocess.Popen([str(self.binary)], cwd=self.binary.parent, env=env,
                start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        self.wait(lambda s: "ready=true protocol=7 " in s and f"publisher_pid={self.process.pid} " in s, 30)
        # Snapshots are published on automation work, not on every live tick.
        deadline = time.monotonic() + 30
        while time.monotonic() < deadline:
            state = self.command("profile off")
            if 'name="Overview"' in state:
                return state
            time.sleep(.1)
        raise RuntimeError("GUI did not publish its navigation controls")

    def snapshot(self):
        path = self.ipc / "snapshot.txt"
        try:
            with path.open("rb") as stream:
                data = stream.read(1024 * 1024 + 1)
        except FileNotFoundError:
            return ""
        if len(data) > 1024 * 1024:
            raise RuntimeError("Automation snapshot exceeds its bound")
        value = data.decode("utf-8")
        if "ready=true" in value:
            if not re.search(rf"\bpublisher_pid={self.process.pid}\b", value):
                raise RuntimeError("Automation publisher identity changed")
            if not re.search(r"\bdispatch_errors=0\b", value) or "error event=" in value:
                # Do not copy arbitrary potentially identifying trace strings.
                raise RuntimeError("Native input dispatch reported an error")
        return value

    def wait(self, predicate, seconds=10):
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            if not psutil.pid_exists(self.process.pid):
                raise RuntimeError("GUI exited during interaction qualification")
            value = self.snapshot()
            if predicate(value):
                return value
            time.sleep(.025)
        raise RuntimeError("Timed out waiting for owned GUI state")

    def command(self, command):
        if "\n" in command or len(command) > 1024:
            raise ValueError("Invalid qualification command")
        prior = self.snapshot()
        count = int(re.search(r"\bruntime_uptime_ns=(\d+)", prior)[1])
        self.sequence += 1
        target = self.ipc / f"command-{self.sequence}.txt"
        temporary = self.ipc / f"qualification-{self.sequence}.tmp"
        with temporary.open("x", encoding="utf-8", newline="\n") as stream:
            stream.write(command + "\n")
        if target.exists():
            raise RuntimeError("Refusing to replace an existing queued command")
        temporary.rename(target)
        return self.wait(lambda s: not target.exists() and (m := re.search(r"\bruntime_uptime_ns=(\d+)", s)) and int(m[1]) > count)

    def control(self, name):
        role = "listitem" if name in ("Overview", "CPU", "Memory", "Disk", "GPU", "Network", "Processes", "Thermals", "Drivers", "Settings") else "button"
        pattern = r'widget @w1/main-canvas#(\d+) role=(' + role + r') name="' + re.escape(name) + r'"[^\n]*enabled=true[^\n]*actions=\[[^\]\n]*press'
        matches = re.findall(pattern, self.snapshot())
        if len(matches) != 1:
            raise RuntimeError(f"Expected one enabled press control: {name}; found {len(matches)}")
        return matches[0][0]

    def click(self, name):
        return self.input_command(f"widget-click main-canvas {self.control(name)}")

    def input_command(self, command):
        state = self.command("profile on")
        previous = re.search(r"\binput_latency_n=(\d+)", state)
        prior = int(previous[1]) if previous else 0
        state = self.command(command)
        deadline = time.monotonic() + 5
        while not (measured := re.search(r"\binput_latency_n=(\d+)", state)) or int(measured[1]) <= prior:
            if time.monotonic() >= deadline:
                raise RuntimeError("Input did not reach a responding present")
            time.sleep(.02)
            state = self.command("profile on")
        return state

    def profile(self):
        self.command("profile off")
        self.command("profile on")

    def metrics(self, require_input=True):
        value = self.command("profile on")
        line = next((line for line in value.splitlines() if line.startswith("frame_profile ")), "")
        result = {key: int(value) for key, value in re.findall(r"(\w+)=(\d+)", line)}
        for stage in (("frame_work", "input_latency") if require_input else ("frame_work",)):
            if result.get(stage + "_n", 0) == 0:
                raise RuntimeError(f"No {stage} measurements were captured")
            if result[stage + "_n"] != result[stage + "_window_n"]:
                raise RuntimeError(f"{stage} percentile window lost sample coverage")
        return result

    def close(self):
        if not self.process:
            return
        try:
            if self.win:
                self.win.clean_quit(self.process)
            else:
                with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
                    connection.settimeout(2)
                    connection.connect(str(self.endpoint))
                    connection.sendall(b"quit\n")
                if self.process.wait(timeout=15) != 0:
                    raise RuntimeError("GUI did not quit cleanly")
        finally:
            if self.job:
                self.job.close()
            elif self.process.poll() is None:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait(timeout=5)
            self.process = None
            # This directory was absent before this owned launch. Remove its
            # potentially sensitive snapshots before releasing the temp home.
            if self.ipc.exists() and self.ipc.resolve() == self.binary.parent / ".zig-cache/native-sdk-automation":
                shutil.rmtree(self.ipc)
        self.closed = True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--inspect", action="store_true", help="Print authored controls only; no timing qualification")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    engine = binary.with_name("sd300_engine.dll" if sys.platform == "win32" else "libsd300_engine.dylib" if sys.platform == "darwin" else "libsd300_engine.so")
    collector = binary.with_name("sd300.exe" if sys.platform == "win32" else "sd300")
    report = {"schema": 1, "revision": args.revision, "platform": sys.platform,
        "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(), "automation_enabled": True,
        "engine_sha256": hashlib.sha256(engine.read_bytes()).hexdigest(),
        "collector_sha256": hashlib.sha256(collector.read_bytes()).hexdigest(),
        "method": "Synchronous event work through host present; input receipt to responding present. Queue wait excluded from work. Physical scanout not measured.",
        "cohorts": [], "passed": False}
    session = None
    try:
        with tempfile.TemporaryDirectory(prefix="sd300-interaction-") as directory, ExitStack() as cleanup:
            session = Session(binary, Path(directory))
            cleanup.callback(session.close)
            state = session.start()
            if args.inspect:
                print("\n".join(line for line in state.splitlines() if 'name="CPU"' in line or 'name="Overview"' in line))
                return
            ready_deadline = time.monotonic() + 30
            while 'name="Live"' not in session.command("profile off"):
                if time.monotonic() >= ready_deadline:
                    raise RuntimeError("Monitoring did not become live")
                time.sleep(.25)
            for width, height in ((1180, 760), (950, 760)):
                session.command(f"resize {width} {height}")
                for mode in ("User mode", "Technician mode"):
                    if mode not in session.snapshot():
                        session.click("Technician mode" if mode == "User mode" else "User mode")
                    session.profile()
                    for _ in range(2):
                        for name in ("CPU", "Memory", "Disk", "GPU", "Network", "Processes", "Thermals", "Drivers", "Settings", "Overview"):
                            session.click(name)
                    data = session.metrics()
                    report["cohorts"].append({"kind": "navigation", "width": width, "height": height, "mode": mode, **data})
                    session.click("Processes")
                    session.profile()
                    # Focus traversal exercises repeated keyboard input without
                    # invoking optional diagnostics, exports, repairs or setup.
                    for _ in range(20):
                        session.input_command("widget-key main-canvas tab")
                    report["cohorts"].append({"kind": "keyboard", "width": width, "height": height, "mode": mode, **session.metrics()})
                    session.click("Processes")
                    session.profile()
                    # Distinct live refreshes, with no user input. Snapshot
                    # publication itself is outside synchronous frame work.
                    for _ in range(20):
                        time.sleep(1)
                        session.command("profile on")
                    report["cohorts"].append({"kind": "refresh", "width": width, "height": height, "mode": mode, **session.metrics(require_input=False)})
            report["frame_gate"] = all(row["frame_work_p95_us"] <= 16700 for row in report["cohorts"])
            report["input_gate"] = all(row["input_latency_p95_us"] <= 50000 for row in report["cohorts"] if row["kind"] != "refresh")
            report["refresh_stall_gate"] = all(row["frame_work_total_max_us"] <= 100000 for row in report["cohorts"] if row["kind"] == "refresh")
            report["passed"] = report["frame_gate"] and report["input_gate"] and report["refresh_stall_gate"]
    except BaseException as error:
        report.update(passed=False, failure_type=type(error).__name__, failure=str(error))
        raise
    finally:
        try:
            if session:
                session.close()
                report["clean_shutdown"] = session.closed
        finally:
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    if not report["passed"]:
        raise SystemExit("Interaction performance gates did not pass")


if __name__ == "__main__":
    main()
