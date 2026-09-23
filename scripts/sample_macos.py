"""Bounded native stack attribution, kept separate from resource acceptance."""
from pathlib import Path
import math
import subprocess
import tempfile
import time

import psutil


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
            "note": "Live-thread CPU deltas during the stack sample; excludes ended/new/reset threads. Diagnostic only."}


def sample_owned_process(pid):
    process = psutil.Process(pid)
    with tempfile.TemporaryDirectory(prefix="sd300-stack-sample-") as directory:
        output = Path(directory) / "sample.txt"
        before = thread_counters(process)
        begin = time.monotonic()
        result = subprocess.run(
            ["/usr/bin/sample", str(pid), "5", "1", "-file", str(output)],
            stdin=subprocess.DEVNULL, capture_output=True, timeout=20,
        )
        after = thread_counters(process)
        elapsed = time.monotonic() - begin
        if result.returncode or not output.is_file():
            raise RuntimeError(f"Native sample failed ({result.returncode}): {result.stderr[-4096:]!r}")
        with output.open("rb") as stream:
            data = stream.read(1024 * 1024 + 1)
        if len(data) > 1024 * 1024:
            raise RuntimeError("Native stack sample exceeded its output bound")
        return {"duration_seconds": 5, "interval_ms": 1,
                "stacks": data.decode("utf-8", "replace"),
                "thread_cpu": thread_delta(before, after, elapsed),
                "note": "Includes waiting stacks; frequency alone is not CPU time. Diagnostic only."}
