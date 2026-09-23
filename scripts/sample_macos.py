"""Bounded native stack attribution, kept separate from resource acceptance."""
from pathlib import Path
import subprocess
import tempfile


def sample_owned_process(pid):
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
        return {"duration_seconds": 5, "interval_ms": 1,
                "stacks": data.decode("utf-8", "replace"),
                "note": "Includes waiting stacks; frequency alone is not CPU time. Diagnostic only."}
