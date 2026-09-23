"""Native GUI process-family measurement without an attached visual observer.

Uses wait4 CPU including waited descendants, quarter-second summed RSS/fd samples,
isolated settings, explicit window-state checks at the boundaries and the owned
GUI quit socket. Linux requires an X11 session and xdotool; CI uses private Xvfb.
"""
import argparse
import ctypes as c
import hashlib
import json
import os
from pathlib import Path
import signal
import socket
import subprocess
import sys
import tempfile
import threading
import time

import psutil
from resource_metrics import sample_family, descriptor_summary

SECTIONS = ["Overview", "CPU", "Memory", "Storage", "GPU", "Network", "Processes", "Thermals", "Drivers"]


def visible_windows(pid):
    if sys.platform != "darwin":
        result = subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(pid)],
                                capture_output=True, timeout=5)
        if result.returncode not in (0, 1) or result.stderr or len(result.stdout) > 65536:
            raise RuntimeError("X11 window-state query failed")
        return len(result.stdout.splitlines())
    cg = c.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
    cf = c.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
    cg.CGWindowListCopyWindowInfo.argtypes, cg.CGWindowListCopyWindowInfo.restype = [c.c_uint32, c.c_uint32], c.c_void_p
    for name, args, result in [
        ("CFArrayGetCount", [c.c_void_p], c.c_long),
        ("CFArrayGetValueAtIndex", [c.c_void_p, c.c_long], c.c_void_p),
        ("CFDictionaryGetValue", [c.c_void_p, c.c_void_p], c.c_void_p),
        ("CFNumberGetValue", [c.c_void_p, c.c_int, c.c_void_p], c.c_bool),
        ("CFRelease", [c.c_void_p], None),
    ]:
        function = getattr(cf, name)
        function.argtypes, function.restype = args, result
    windows = cg.CGWindowListCopyWindowInfo(17, 0)  # On-screen, exclude desktop elements.
    if not windows:
        raise RuntimeError("CoreGraphics window-state query failed")
    keys = [c.c_void_p.in_dll(cg, name).value for name in ("kCGWindowOwnerPID", "kCGWindowLayer")]
    try:
        count = cf.CFArrayGetCount(windows)
        if not 0 <= count <= 16384:
            raise RuntimeError("CoreGraphics window list exceeds its bound")
        found = 0
        for index in range(count):
            row = cf.CFArrayGetValueAtIndex(windows, index)
            values = []
            for key in keys:
                number = cf.CFDictionaryGetValue(row, key)
                value = c.c_int64()
                if not number or not cf.CFNumberGetValue(number, 4, c.byref(value)):
                    raise RuntimeError("CoreGraphics window identity is unavailable")
                values.append(value.value)
            found += values == [pid, 0]  # Excludes menu-bar/tray windows in hidden mode.
        return found
    finally:
        cf.CFRelease(windows)


def bundle_files(binary):
    engine = binary.with_name("libsd300_engine.dylib" if sys.platform == "darwin" else "libsd300_engine.so")
    if not engine.is_file():
        raise RuntimeError("Adjacent GUI engine is missing")
    for candidate in (binary.parent.parent / "bin/sd300", binary.with_name("sd300")):
        if candidate.is_file():
            return engine, candidate.resolve()
    raise RuntimeError("Bundle-relative collector CLI is missing")


def topics(root, cli):
    values = set()
    for child in root.children(recursive=True):
        try:
            args = child.cmdline()
            if Path(child.exe()).resolve() == cli and len(args) == 3 and args[1] == "collect-server":
                values.add(args[2])
        except psutil.NoSuchProcess:
            pass
    return values


def main():
    def interrupted(_signal, _frame):
        raise KeyboardInterrupt("Qualification interrupted")
    signal.signal(signal.SIGTERM, interrupted)
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("--launcher", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--seconds", type=int, default=900)
    parser.add_argument("--section", choices=SECTIONS, default="Thermals")
    parser.add_argument("--hidden", action="store_true")
    parser.add_argument("--legacy-in-process", action="store_true", help="Qualify the pre-v4 in-process collector topology")
    parser.add_argument("--enforce-gates", action="store_true")
    args = parser.parse_args()
    if not 20 <= args.seconds <= 7200:
        parser.error("Duration must be 20..7200 seconds")
    if args.hidden:
        args.section = "Overview"
    binary = args.binary.resolve(strict=True)
    launcher = args.launcher.resolve(strict=True) if args.launcher else binary
    engine, cli = bundle_files(binary)
    digest = lambda path: hashlib.sha256(path.read_bytes()).hexdigest()
    report = {"schema": 2, "revision": args.revision, "platform": sys.platform, "binary": str(binary),
              "sha256": digest(binary), "engine_sha256": digest(engine), "collector_sha256": digest(cli),
              "launcher_sha256": digest(launcher), "build": "release", "mode": "hidden" if args.hidden else "foreground",
              "section": args.section, "legacy_in_process": args.legacy_in_process,
              "cpu_method": "wait4 user+system including waited descendants; startup/shutdown included; 100% = one logical core",
              "memory_method": "250 ms summed live descendant RSS; shared mappings count per process"}
    process, result, known = None, None, {}
    stderr_tail = [b""]
    reader = None
    try:
        with tempfile.TemporaryDirectory(prefix="sd300-gui-resource-") as directory:
            home = Path(directory)
            if sys.platform == "darwin":
                config = home / "Library/Application Support/SD-300"
                endpoint = Path(f"/tmp/sd300-{os.geteuid()}/gui.sock")
                if endpoint.exists():
                    raise RuntimeError("Existing GUI endpoint found; leave that session untouched")
            else:
                config = home / "config/sd300"
                endpoint = home / "runtime/sd300/gui.sock"
                endpoint.parent.parent.mkdir(mode=0o700)
            config.mkdir(parents=True)
            (config / "settings.json").write_text(json.dumps({"schema_version": 1, "gui": {
                "audience_mode": "user", "last_section": SECTIONS.index(args.section), "tray_enabled": args.hidden,
                "close_to_tray": args.hidden, "launch_at_login": False, "reduced_motion": True}}), encoding="utf-8")
            env = dict(os.environ, HOME=directory, XDG_CONFIG_HOME=str(home / "config"), XDG_RUNTIME_DIR=str(home / "runtime"))
            begin = time.monotonic()
            process = subprocess.Popen([str(launcher)] + (["--startup", "--hidden"] if args.hidden and sys.platform == "darwin" else []),
                                       env=env, start_new_session=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
            def drain():
                while chunk := process.stderr.read(4096):
                    stderr_tail[0] = (stderr_tail[0] + chunk)[-8192:]
            reader = threading.Thread(target=drain, daemon=True)
            reader.start()
            root = psutil.Process(process.pid)
            samples = []
            required = set() if args.legacy_in_process else {"slow"} if args.hidden else set() if args.section in ("Overview", "Processes") else {"slow", "activity", "connections", "diagnostics"}
            checked = False
            while time.monotonic() - begin < args.seconds:
                pid, status, usage = os.wait4(process.pid, os.WNOHANG)
                if pid:
                    process.returncode = os.waitstatus_to_exitcode(status)
                    result = usage
                    raise RuntimeError(f"GUI exited during measurement: {process.returncode}")
                samples.append(sample_family(root, known))
                if not checked and time.monotonic() - begin >= 15:
                    if args.hidden and sys.platform != "darwin":
                        # Linux intentionally has no tray startup route. Unmap
                        # only this PID's windows to exercise background mode.
                        subprocess.run(["xdotool", "search", "--onlyvisible", "--pid", str(process.pid), "windowunmap", "%@"],
                                       check=True, capture_output=True, timeout=5)
                        time.sleep(2)
                        report["hidden_method"] = "X11 unmap of owned windows after startup; tray not supported"
                    actual = visible_windows(process.pid)
                    if (actual == 0) != args.hidden:
                        raise RuntimeError("GUI visibility does not match the requested mode")
                    seen = topics(root, cli)
                    report["collector_topics_at_start"] = sorted(seen)
                    if not required <= seen:
                        raise RuntimeError(f"Required collectors are missing: {sorted(required-seen)}")
                    checked = True
                time.sleep(.25)
            elapsed = time.monotonic() - begin
            if (visible_windows(process.pid) == 0) != args.hidden:
                raise RuntimeError("GUI visibility changed during measurement")
            seen = topics(root, cli)
            report["collector_topics_at_end"] = sorted(seen)
            if not required <= seen:
                raise RuntimeError(f"Collectors missing at completion: {sorted(required-seen)}")
            with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
                connection.settimeout(2)
                connection.connect(str(endpoint))
                connection.sendall(b"quit\n")
            deadline = time.monotonic() + 15
            while time.monotonic() < deadline:
                pid, status, usage = os.wait4(process.pid, os.WNOHANG)
                if pid:
                    process.returncode = os.waitstatus_to_exitcode(status)
                    result = usage
                    break
                time.sleep(.05)
            if result is None or process.returncode != 0:
                raise RuntimeError(f"GUI did not shut down cleanly: {process.returncode}")
            if any(p.is_running() for p in known.values()):
                raise RuntimeError("Owned helper remains after GUI shutdown")
            cpu = (result.ru_utime + result.ru_stime) / elapsed * 100
            peak = max(row[0] for row in samples)
            window = max(1, len(samples)//10)
            report.update(measured_seconds=elapsed, samples=len(samples), cpu_percent_one_core=cpu,
                          cpu_gate=cpu <= (1 if args.hidden else 2), rss_gate=peak <= 150, rss_mib_max=peak,
                          **descriptor_summary(samples), process_count_max=max(r[2] for r in samples),
                          observed_process_identities=len(known), clean_shutdown=True,
                          rss_last_window_delta=sum(r[0] for r in samples[-window:])/window-sum(r[0] for r in samples[:window])/window)
    except BaseException as error:
        report.update(passed=False, failure_type=type(error).__name__, failure=str(error))
        report["stderr_tail"] = stderr_tail[0].decode("utf-8", "replace")
        raise
    finally:
        for child in reversed(list(known.values())):
            try:
                if child.is_running():
                    child.kill()
            except psutil.NoSuchProcess:
                pass
        if process is not None and result is None:
            process.kill()
            process.wait(timeout=5)
        if reader:
            reader.join(timeout=2)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        print(json.dumps(report, indent=2))
    if args.enforce_gates and not (report["cpu_gate"] and report["rss_gate"]):
        raise SystemExit("Measured GUI resource gates did not pass")


if __name__ == "__main__":
    main()
