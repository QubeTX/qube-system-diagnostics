"""Measure the whole owned GUI process family without an attached visual observer.

Uses the same pinned development environment and Job Object accounting as the
TUI harness. The GUI starts suspended and joins the job before any code runs.
Its isolated settings select the section; no pointer observer is attached.
"""
import argparse
import ctypes as c
from contextlib import closing
from ctypes import wintypes as w
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

import psutil

spec = importlib.util.spec_from_file_location("sd300_tui_measure", Path(__file__).with_name("measure-tui-windows.py"))
tui_measure = importlib.util.module_from_spec(spec)
spec.loader.exec_module(tui_measure)
class Job(tui_measure.Job):
    def close(self):
        if self.handle:
            super().close()
            self.handle = None
QUIT_EVENT = "Local\\SD300.Gui.Quit.v1"
SECTIONS = ["Overview", "CPU", "Memory", "Disk", "GPU", "Network", "Processes", "Thermals", "Drivers"]


class Startup(c.Structure):
    _fields_ = [("cb", w.DWORD), ("reserved", w.LPWSTR), ("desktop", w.LPWSTR), ("title", w.LPWSTR),
        ("x", w.DWORD), ("y", w.DWORD), ("width", w.DWORD), ("height", w.DWORD),
        ("xchars", w.DWORD), ("ychars", w.DWORD), ("fill", w.DWORD), ("flags", w.DWORD),
        ("show", w.WORD), ("reserved_size", w.WORD), ("reserved_bytes", c.c_void_p),
        ("stdin", w.HANDLE), ("stdout", w.HANDLE), ("stderr", w.HANDLE)]


class ProcessInfo(c.Structure):
    _fields_ = [("process", w.HANDLE), ("thread", w.HANDLE), ("pid", w.DWORD), ("tid", w.DWORD)]


def kernel():
    api = c.WinDLL("kernel32", use_last_error=True)
    for name, args, ret in [
        ("CreateProcessW", [w.LPCWSTR, w.LPWSTR, c.c_void_p, c.c_void_p, w.BOOL, w.DWORD, c.c_void_p, w.LPCWSTR, c.POINTER(Startup), c.POINTER(ProcessInfo)], w.BOOL),
        ("ResumeThread", [w.HANDLE], w.DWORD),
        ("TerminateProcess", [w.HANDLE, w.UINT], w.BOOL),
        ("WaitForSingleObject", [w.HANDLE, w.DWORD], w.DWORD),
        ("OpenEventW", [w.DWORD, w.BOOL, w.LPCWSTR], w.HANDLE),
        ("SetEvent", [w.HANDLE], w.BOOL),
        ("CloseHandle", [w.HANDLE], w.BOOL),
    ]:
        fn = getattr(api, name)
        fn.argtypes, fn.restype = args, ret
    return api


def spawn_owned(job, argv, env, hidden):
    api = kernel()
    startup = Startup(cb=c.sizeof(Startup), flags=1 if hidden else 0, show=0)
    info = ProcessInfo()
    command = c.create_unicode_buffer(subprocess.list2cmdline([str(a) for a in argv]))
    environment = c.create_unicode_buffer("\0".join(f"{k}={v}" for k, v in sorted(env.items(), key=lambda item: item[0].upper())) + "\0\0")
    if not api.CreateProcessW(str(argv[0]), command, None, None, False, 0x404, environment,
                              str(Path(argv[0]).parent), c.byref(startup), c.byref(info)):
        raise c.WinError(c.get_last_error())
    try:
        job.assign(info.pid)
        if api.ResumeThread(info.thread) == 0xffffffff:
            raise c.WinError(c.get_last_error())
        return psutil.Process(info.pid)
    except BaseException:
        api.TerminateProcess(info.process, 125)
        api.WaitForSingleObject(info.process, 5000)
        raise
    finally:
        api.CloseHandle(info.thread)
        api.CloseHandle(info.process)


def visible_windows(pid):
    api = c.WinDLL("user32", use_last_error=True)
    callback_type = c.WINFUNCTYPE(w.BOOL, w.HWND, w.LPARAM)
    api.EnumWindows.argtypes = [callback_type, w.LPARAM]
    api.EnumWindows.restype = w.BOOL
    api.GetWindowThreadProcessId.argtypes = [w.HWND, c.POINTER(w.DWORD)]
    api.IsWindowVisible.argtypes = [w.HWND]
    api.IsWindowVisible.restype = w.BOOL
    count = 0
    @callback_type
    def visit(handle, _):
        nonlocal count
        owner = w.DWORD()
        api.GetWindowThreadProcessId(handle, c.byref(owner))
        if owner.value == pid and api.IsWindowVisible(handle):
            count += 1
        return True
    if not api.EnumWindows(visit, 0):
        raise c.WinError(c.get_last_error())
    return count


def composite_cli(binary):
    # Match the engine's bundle-relative order; never accept an unrelated
    # installed CLI through a user's managed fallback during qualification.
    for candidate in (binary.parent.parent / "bin" / "sd300.exe", binary.with_name("sd300.exe")):
        if candidate.is_file():
            return candidate.resolve()
    raise RuntimeError("The measured bundle is incomplete: its CLI collector is missing")


def collector_topics(job, cli):
    topics = set()
    for pid in job.pids():
        try:
            process = psutil.Process(pid)
            if Path(process.exe()).resolve() != cli:
                continue
            args = process.cmdline()
            if len(args) == 3 and args[1] == "collect-server":
                topics.add(args[2])
        except psutil.NoSuchProcess:
            pass
    return topics


def required_topics(section, hidden):
    if hidden:
        return {"slow"}
    # Overview and Processes deliberately use only the in-process fast lane
    # and a short-lived static worker. Detailed pages enable all probe lanes.
    return set() if section in ("Overview", "Processes") else {"slow", "activity", "connections", "diagnostics"}


def ensure_no_existing_gui():
    api = kernel()
    handle = api.OpenEventW(0x100000, False, QUIT_EVENT)
    if handle:
        api.CloseHandle(handle)
        raise RuntimeError("An SD-300 GUI lifecycle endpoint already exists; leave the existing application untouched")
    if c.get_last_error() != 2:
        raise RuntimeError("Could not safely establish that the GUI lifecycle endpoint is absent")
    if any(p.info["name"] and p.info["name"].lower() == "sd300-gui.exe" for p in psutil.process_iter(["name"])):
        raise RuntimeError("An SD-300 GUI process already exists; do not benchmark or stop another session")


def clean_quit(process):
    if not process.is_running():
        raise RuntimeError("The GUI exited before its requested clean shutdown")
    api = kernel()
    handle = api.OpenEventW(2, False, QUIT_EVENT)
    if not handle:
        raise RuntimeError("The owned GUI did not expose its lifecycle quit endpoint")
    try:
        if not api.SetEvent(handle):
            raise c.WinError(c.get_last_error())
    finally:
        api.CloseHandle(handle)
    code = process.wait(timeout=15)
    if code != 0:
        raise RuntimeError(f"GUI exited with code {code}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--seconds", type=int, default=900)
    parser.add_argument("--warmup", type=int, default=15)
    parser.add_argument("--section", choices=SECTIONS, default="Overview")
    parser.add_argument("--hidden", action="store_true")
    parser.add_argument("--legacy-in-process", action="store_true", help="Qualify the pre-v4 in-process collector topology")
    parser.add_argument("--enforce-gates", action="store_true")
    args = parser.parse_args()
    if not 5 <= args.seconds <= 7200 or not 0 <= args.warmup <= 300:
        parser.error("Duration must be 5..7200 seconds and warmup 0..300")
    if args.hidden and args.section != "Overview":
        parser.error("Hidden qualification measures Overview summaries")
    binary = args.binary.resolve(strict=True)
    engine = binary.with_name("sd300_engine.dll")
    if not engine.is_file():
        parser.error("The measured artifact must include its adjacent engine DLL")
    cli = composite_cli(binary)
    ensure_no_existing_gui()
    report = {"schema":2, "binary":str(binary), "sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
        "engine_sha256":hashlib.sha256(engine.read_bytes()).hexdigest(), "revision":args.revision,
        "collector_sha256":hashlib.sha256(cli.read_bytes()).hexdigest(),
        "build":"release", "mode":"hidden" if args.hidden else "foreground", "section":args.section,
        "legacy_in_process":args.legacy_in_process,
        "warmup_seconds":args.warmup, "method":"Job Object user+kernel CPU, including terminated descendants; 100% = one logical core",
        "memory_method":"250 ms sum of live job processes; RSS conservatively counts shared mappings per process"}
    job = Job()
    try:
        with tempfile.TemporaryDirectory(prefix="sd300-gui-measure-") as directory, closing(job):
            settings = Path(directory) / "SD-300"
            settings.mkdir()
            (settings / "settings.json").write_text(json.dumps({"schema_version":1,"gui":{
                "audience_mode":"user", "last_section":SECTIONS.index(args.section), "tray_enabled":args.hidden,
                "close_to_tray":args.hidden, "launch_at_login":False, "reduced_motion":True}}), encoding="utf-8")
            env = dict(os.environ, APPDATA=directory, LOCALAPPDATA=directory)
            process = spawn_owned(job, [binary] + (["--startup", "--hidden"] if args.hidden else []), env, args.hidden)
            time.sleep(args.warmup)
            if not process.is_running():
                raise RuntimeError("The GUI exited during warmup")
            windows = visible_windows(process.pid)
            if (args.hidden and windows != 0) or (not args.hidden and windows == 0):
                raise RuntimeError("The measured GUI did not enter its requested visibility state")
            topics = collector_topics(job, cli)
            required = set() if args.legacy_in_process else required_topics(args.section, args.hidden)
            if not required <= topics:
                raise RuntimeError(f"Required collector workers are absent after warmup: {sorted(required-topics)}")
            report["collector_topics_at_start"] = sorted(topics)
            report["required_collector_topics"] = sorted(required)
            before = job.accounting()
            begin = time.monotonic()
            previous_cpu, previous_time = (before.user + before.kernel) / 1e7, begin
            samples, names = [], set()
            role_cache, peak_roles, peak_bytes = {}, {}, -1
            while time.monotonic() - begin < args.seconds:
                time.sleep(.25)
                if not process.is_running():
                    raise RuntimeError("The GUI exited during measurement")
                now, account = time.monotonic(), job.accounting()
                cpu = (account.user + account.kernel) / 1e7
                live = job.pids()
                rss = private = handles = 0
                roles = {}
                for pid in live:
                    try:
                        child = psutil.Process(pid)
                        memory = child.memory_info()
                        rss += memory.rss
                        private += memory.private
                        handles += child.num_handles()
                        names.add(child.name())
                        identity = (pid, child.create_time())
                        role = role_cache.get(identity)
                        if role is None:
                            role = "gui" if pid == process.pid else "helper"
                            arguments = child.cmdline()
                            if len(arguments) == 3 and arguments[1] == "collect-server" and arguments[2] in {"slow", "activity", "connections", "diagnostics", "static", "health", "drivers"}:
                                role = "collector:" + arguments[2]
                            if len(role_cache) >= 4096:
                                raise RuntimeError("Observed process identities exceed their bound")
                            role_cache[identity] = role
                        row = roles.setdefault(role, {"rss_mib":0, "private_mib":0, "processes":0})
                        row["rss_mib"] += memory.rss/2**20
                        row["private_mib"] += memory.private/2**20
                        row["processes"] += 1
                    except psutil.NoSuchProcess:
                        pass
                samples.append(((cpu-previous_cpu)/(now-previous_time)*100, rss/2**20, private/2**20, handles, len(live)))
                if rss > peak_bytes:
                    peak_bytes, peak_roles = rss, roles
                    report["rss_peak_elapsed_seconds"] = now-begin
                previous_cpu, previous_time = cpu, now
            after = job.accounting()
            elapsed = time.monotonic() - begin
            cpu = (after.user + after.kernel - before.user - before.kernel) / 1e7 / elapsed * 100
            peak_rss, peak_private = max(row[1] for row in samples), max(row[2] for row in samples)
            window = max(1, len(samples)//10)
            mean = lambda rows, index: sum(row[index] for row in rows)/len(rows)
            report.update(measured_seconds=elapsed, samples=len(samples), cpu_percent_one_core=cpu,
                cpu_interval_p95=sorted(row[0] for row in samples)[int(.95*(len(samples)-1))],
                rss_mib_max=peak_rss, private_mib_max=peak_private,
                rss_peak_roles=peak_roles,
                rss_last_window_delta=mean(samples[-window:],1)-mean(samples[:window],1),
                private_last_window_delta=mean(samples[-window:],2)-mean(samples[:window],2),
                handles_max=max(row[3] for row in samples), process_count_max=max(row[4] for row in samples),
                processes_created=after.total-before.total, process_names=sorted(names),
                cpu_gate=cpu <= (1 if args.hidden else 2), rss_gate=peak_rss <= 150, private_gate=peak_private <= 300)
            topics = collector_topics(job, cli)
            report["collector_topics_at_end"] = sorted(topics)
            if not required <= topics:
                raise RuntimeError(f"Required collector workers are absent at completion: {sorted(required-topics)}")
            windows = visible_windows(process.pid)
            if (args.hidden and windows != 0) or (not args.hidden and windows == 0):
                raise RuntimeError("The GUI visibility changed during measurement")
            clean_quit(process)
            deadline = time.monotonic()+2
            while job.pids() and time.monotonic() < deadline:
                time.sleep(.05)
            if job.pids():
                raise RuntimeError("Owned helper processes remained after the GUI exited")
            report["clean_shutdown"] = True
    except BaseException as error:
        report.update(passed=False, failure_type=type(error).__name__, failure=str(error))
        raise
    finally:
        job.close()
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report,indent=2)+"\n", encoding="utf-8")
        print(json.dumps(report,indent=2))
    if args.enforce_gates and not all(report[key] for key in ("cpu_gate", "rss_gate", "private_gate", "clean_shutdown")):
        raise SystemExit("Measured GUI resource gates did not pass")


if __name__ == "__main__":
    main()
