"""Release TUI process-tree measurement through a real ConPTY, without a visual observer.

Dev-only dependencies: psutil==7.2.2, pywinpty==3.0.5. CPU accounting includes
terminated descendants using a Windows job, not just the currently visible PID.
The idle command shell is placed in the job BEFORE launching the measured binary.
No network diagnostics, helper installation, repairs or privileged reads are requested.
"""
import argparse
import ctypes as c
from ctypes import wintypes as w
import hashlib
import json
import os
from pathlib import Path
import tempfile
import threading
import time

import psutil
from winpty import Backend, PtyProcess


class Accounting(c.Structure):
    _fields_ = [(n, c.c_int64) for n in ("user", "kernel", "period_user", "period_kernel")] + [
        (n, w.DWORD) for n in ("faults", "total", "active", "terminated")]


class Job:
    def __init__(self):
        self.api = c.WinDLL("kernel32", use_last_error=True)
        for name, args, ret in [
            ("CreateJobObjectW", [c.c_void_p, w.LPCWSTR], w.HANDLE),
            ("OpenProcess", [w.DWORD, w.BOOL, w.DWORD], w.HANDLE),
            ("AssignProcessToJobObject", [w.HANDLE, w.HANDLE], w.BOOL),
            ("QueryInformationJobObject", [w.HANDLE, c.c_int, c.c_void_p, w.DWORD, c.c_void_p], w.BOOL),
            ("TerminateJobObject", [w.HANDLE, w.UINT], w.BOOL),
            ("CloseHandle", [w.HANDLE], w.BOOL),
        ]:
            fn = getattr(self.api, name)
            fn.argtypes, fn.restype = args, ret
        self.handle = self.api.CreateJobObjectW(None, None)
        if not self.handle:
            raise c.WinError(c.get_last_error())

    def assign(self, pid):
        handle = self.api.OpenProcess(0x1101, False, pid)
        if not handle:
            raise c.WinError(c.get_last_error())
        try:
            if not self.api.AssignProcessToJobObject(self.handle, handle):
                raise c.WinError(c.get_last_error())
        finally:
            self.api.CloseHandle(handle)

    def accounting(self):
        value = Accounting()
        if not self.api.QueryInformationJobObject(self.handle, 1, c.byref(value), c.sizeof(value), None):
            raise c.WinError(c.get_last_error())
        return value

    def pids(self):
        # A fixed bound prevents a runaway child tree from growing the harness.
        class List(c.Structure):
            _fields_ = [("assigned", w.DWORD), ("count", w.DWORD), ("ids", c.c_size_t * 512)]
        value = List()
        if not self.api.QueryInformationJobObject(self.handle, 3, c.byref(value), c.sizeof(value), None):
            raise c.WinError(c.get_last_error())
        if value.assigned > 512:
            raise RuntimeError("Owned process count exceeded the qualification bound")
        return list(value.ids[:value.count])

    def close(self):
        self.api.TerminateJobObject(self.handle, 125)
        self.api.CloseHandle(self.handle)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seconds", type=int, default=60)
    parser.add_argument("--warmup", type=int, default=15)
    parser.add_argument("--columns", type=int, default=80)
    parser.add_argument("--rows", type=int, default=24)
    parser.add_argument("--revision", required=True)
    args = parser.parse_args()
    if not 5 <= args.seconds <= 7200 or not 0 <= args.warmup <= 300:
        parser.error("Measurement duration must be 5..7200 seconds, warmup 0..300")
    binary = args.binary.resolve(strict=True)
    # cmd only receives this validated executable path and fixed arguments.
    if any(ch in str(binary) for ch in '"%&|<>^\r\n'):
        parser.error("Executable path contains command-shell metacharacters")
    job = Job()
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="sd300-tui-measure-") as settings:
            env = os.environ.copy()
            env["APPDATA"] = settings
            env["LOCALAPPDATA"] = settings
            process = PtyProcess.spawn([str(Path(os.environ["SystemRoot"]) / "System32/cmd.exe"), "/d", "/q"],
                env=env, dimensions=(args.rows, args.columns), backend=Backend.ConPTY)
            job.assign(process.pid)
            terminal = {"tail": "", "characters": 0}
            lock = threading.Lock()

            def drain():
                try:
                    while process.isalive():
                        text = process.read(65536)
                        with lock:
                            terminal["tail"] = (terminal["tail"] + text)[-16384:]
                            terminal["characters"] += len(text)
                except (EOFError, OSError):
                    pass

            reader = threading.Thread(target=drain, daemon=True)
            reader.start()
            process.write(f'"{binary}" --user\r\n')
            time.sleep(args.warmup)
            before = job.accounting()
            begin = time.monotonic()
            previous_cpu = (before.user + before.kernel) / 10_000_000
            previous_wall = begin
            rows = []
            names = set()
            role_cost = {}
            cpu_by_pid = {}
            def monitor_alive():
                for pid in job.pids():
                    try:
                        if psutil.Process(pid).name().lower() == binary.name.lower():
                            return True
                    except psutil.NoSuchProcess:
                        pass
                return False
            for pid in job.pids():
                try:
                    child = psutil.Process(pid)
                    times = child.cpu_times()
                    cpu_by_pid[(pid, child.create_time())] = times.user + times.system
                except psutil.NoSuchProcess:
                    pass
            while time.monotonic() - begin < args.seconds:
                time.sleep(.25)
                if not process.isalive():
                    raise RuntimeError("The terminal closed during measurement")
                now = time.monotonic()
                account = job.accounting()
                cpu = (account.user + account.kernel) / 10_000_000
                rss = private = handles = 0
                live = job.pids()
                next_cpu_by_pid = {}
                monitor_seen = False
                for pid in live:
                    try:
                        child = psutil.Process(pid)
                        name = child.name()
                        names.add(name)
                        monitor_seen |= name.lower() == binary.name.lower()
                        role = name.lower()
                        if role == binary.name.lower():
                            arguments = child.cmdline()
                            role = "sd300:tui"
                            if "collect-server" in arguments:
                                topic = arguments[arguments.index("collect-server") + 1]
                                role = "sd300:" + (topic if topic in {"slow", "activity", "connections", "diagnostics", "static", "health", "drivers"} else "worker")
                        times = child.cpu_times()
                        identity = (pid, child.create_time())
                        observed_cpu = times.user + times.system
                        delta = max(0, observed_cpu - cpu_by_pid.get(identity, 0))
                        next_cpu_by_pid[identity] = observed_cpu
                        stats = role_cost.setdefault(role, {"observed_cpu_seconds":0, "rss_mib_peak_per_process":0})
                        stats["observed_cpu_seconds"] += delta
                        memory = child.memory_info()
                        stats["rss_mib_peak_per_process"] = max(stats["rss_mib_peak_per_process"], memory.rss / 2**20)
                        rss += memory.rss
                        private += memory.private
                        handles += child.num_handles()
                    except psutil.NoSuchProcess:
                        pass
                cpu_by_pid = next_cpu_by_pid
                if not monitor_seen:
                    with lock:
                        tail = terminal["tail"]
                    raise RuntimeError(f"No running monitor in the owned job; last terminal output: {tail[-2000:]}")
                rows.append(dict(cpu=100 * (cpu - previous_cpu) / (now - previous_wall),
                    rss=rss, private=private, handles=handles, processes=len(live)))
                previous_cpu, previous_wall = cpu, now
            elapsed = time.monotonic() - begin
            after = job.accounting()
            overall = 100 * ((after.user + after.kernel - before.user - before.kernel) / 10_000_000) / elapsed
            process.write("q")
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                if not monitor_alive():
                    break
                time.sleep(.05)
            else:
                raise RuntimeError("Monitor did not shut down within fifteen seconds")
            process.write("echo SD300_TERMINAL_RESTORED\r\n")
            time.sleep(.2)
            with lock:
                restored = "SD300_TERMINAL_RESTORED" in terminal["tail"] and "\x1b[?1049l" in terminal["tail"]
            result = dict(schema=1, binary=str(binary), sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
                revision=args.revision, build="release", runtime="Windows ConPTY", frontend="tui",
                dimensions=[args.columns,args.rows], warmup_seconds=args.warmup, measured_seconds=elapsed,
                method="Job Object user+kernel CPU, including terminated descendants; 100% = one logical core",
                memory_method="250 ms sum of live job processes; RSS conservatively counts shared mappings per process",
                cpu_percent_one_core=overall, cpu_interval_p95=sorted(r["cpu"] for r in rows)[int(.95*(len(rows)-1))],
                rss_mib_max=max(r["rss"] for r in rows)/2**20, private_mib_max=max(r["private"] for r in rows)/2**20,
                handles_max=max(r["handles"] for r in rows), process_count_max=max(r["processes"] for r in rows),
                process_names=sorted(names), processes_created=after.total-before.total,
                diagnostic_live_role_cost=role_cost,
                diagnostic_note="Role CPU samples omit processes that exit between polls; the total Job Object CPU above includes them",
                terminal_restored=restored, samples=len(rows), foreground_cpu_gate=overall<=2,
                rss_gate=max(r["rss"] for r in rows)<=150*2**20, private_gate=max(r["private"] for r in rows)<=300*2**20)
            args.output.parent.mkdir(parents=True, exist_ok=True)
            args.output.write_text(json.dumps(result, indent=2)+"\n", encoding="utf-8")
            print(json.dumps(result, indent=2))
            process.write("exit\r\n")
            time.sleep(.2)
    finally:
        job.close()
        if process:
            process.close(force=True)


if __name__ == "__main__":
    main()
