"""Bounded native stack attribution, kept separate from resource acceptance."""
from pathlib import Path
import ctypes as c
import math
import re
import subprocess
import tempfile
import time

import psutil


class ThreadInfo(c.Structure):
    # Public proc_threadinfo ABI: these counters are nanoseconds, unlike the
    # Mach absolute ticks returned by proc_taskinfo. See XNU fill_taskthreadinfo.
    _fields_ = [("user_ns", c.c_uint64), ("system_ns", c.c_uint64),
                ("scheduling", c.c_int32 * 8), ("name", c.c_char * 64)]


def sampled_thread_ids(stacks):
    ids = sorted({int(value) for value in re.findall(r"^\s*\d+ Thread_(\d+)\b", stacks, re.MULTILINE)})
    if len(ids) > 1024 or any(not 0 < value < 2**64 for value in ids):
        raise RuntimeError("Native sample thread inventory exceeds its bound")
    return ids


def native_thread_counters(pid, identities, query=None):
    if not 0 < len(identities) <= 1024:
        return None
    if query is None:
        library = c.CDLL("/usr/lib/libproc.dylib", use_errno=True)
        query = library.proc_pidinfo
        query.argtypes = [c.c_int, c.c_int, c.c_uint64, c.c_void_p, c.c_int]
        query.restype = c.c_int
    counters = {}
    for identity in identities:
        info = ThreadInfo()
        # PROC_PIDTHREADID64INFO uses the kernel thread IDs in sample output.
        # It avoids task_for_pid, which psutil threads() needs but CI denies.
        if query(pid, 15, identity, c.byref(info), c.sizeof(info)) == c.sizeof(info):
            counters[identity] = (info.user_ns + info.system_ns) / 1e9
    return counters or None


def thread_counters(process):
    try:
        rows = process.threads()
        if len(rows) > 1024:
            return None
        counters = {row.id: row.user_time + row.system_time for row in rows}
        return counters if all(math.isfinite(value) and value >= 0 for value in counters.values()) else None
    except (psutil.Error, OSError):
        return None


def thread_delta(before, after, elapsed):
    if before is None or after is None or not math.isfinite(elapsed) or elapsed <= 0:
        return {"available": False, "reason": "Thread counters unavailable"}
    common = before.keys() & after.keys()
    valid = {identity: after[identity] - before[identity] for identity in common
             if after[identity] >= before[identity]}
    rows = [{"thread_id": identity, "cpu_seconds": value,
             "cpu_percent_one_core": value / elapsed * 100}
            for identity, value in sorted(valid.items(), key=lambda row: (-row[1], row[0]))]
    return {"available": True, "elapsed_seconds": elapsed, "threads": rows[:32],
            "truncated": len(rows) > 32, "new_threads": len(after.keys() - before.keys()),
            "ended_threads": len(before.keys() - after.keys()), "reset_counters": len(common) - len(valid),
            "note": "Live-thread CPU deltas in the following diagnostic window; excludes unreadable/ended/new/reset threads. Diagnostic only."}


def sample_owned_process(pid):
    process = psutil.Process(pid)
    with tempfile.TemporaryDirectory(prefix="sd300-stack-sample-") as directory:
        output = Path(directory) / "sample.txt"
        result = subprocess.run(
            ["/usr/bin/sample", str(pid), "5", "1", "-file", str(output)],
            stdin=subprocess.DEVNULL, capture_output=True, timeout=20,
        )
        if result.returncode or not output.is_file():
            raise RuntimeError(f"Native sample failed ({result.returncode}): {result.stderr[-4096:]!r}")
        with output.open("rb") as stream:
            data = stream.read(1024 * 1024 + 1)
        if len(data) > 1024 * 1024:
            raise RuntimeError("Native stack sample exceeded its output bound")
        stacks = data.decode("utf-8", "replace")
        identities = sampled_thread_ids(stacks)
        before = native_thread_counters(pid, identities)
        process_before = process.cpu_times()
        begin = time.monotonic()
        time.sleep(5)
        after = native_thread_counters(pid, identities)
        process_after = process.cpu_times()
        elapsed = time.monotonic() - begin
        cpu = thread_delta(before, after, elapsed)
        cpu.update(source="proc_pidinfo PROC_PIDTHREADID64INFO; nanosecond counters",
                   sampled_thread_count=len(identities),
                   readable_before=len(before or {}), readable_after=len(after or {}),
                   process_cpu_percent_one_core=((process_after.user + process_after.system)
                                                 - (process_before.user + process_before.system)) / elapsed * 100)
        return {"duration_seconds": 5, "interval_ms": 1,
                "stacks": stacks,
                "thread_cpu": cpu,
                "note": "Includes waiting stacks; frequency alone is not CPU time. Diagnostic only."}
