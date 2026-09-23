"""Build an immutable baseline, then measure before/after sequentially on this native runner."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

import psutil

BASELINE = "f83ae429490aecb165921037b2f9ed994327634f"


def bounded(command, seconds):
    child = subprocess.Popen(command)
    try:
        code = child.wait(timeout=seconds)
    except subprocess.TimeoutExpired:
        try:
            descendants = psutil.Process(child.pid).children(recursive=True)
        except psutil.NoSuchProcess:
            descendants = []
        child.terminate()
        try:
            child.wait(timeout=15)
        except subprocess.TimeoutExpired:
            child.kill()
            child.wait(timeout=5)
        for descendant in reversed(descendants):
            try:
                descendant.kill()
            except psutil.NoSuchProcess:
                pass
        raise RuntimeError("Native resource qualification exceeded its deadline")
    if code:
        raise RuntimeError(f"Native qualification command exited {code}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--seconds", type=int, choices=[330, 900], required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    os.chdir(root)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    candidate = subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip()
    baseline = root / "target/resource-baseline-source"
    # A retained worktree may contain an interrupted earlier attempt: never
    # reset or delete it silently. A fresh hosted runner creates this once.
    if baseline.exists():
        raise RuntimeError(f"Baseline stage already exists: {baseline}")
    bounded(["git", "fetch", "--depth=1", "origin", BASELINE], 120)
    bounded(["git", "worktree", "add", "--detach", str(baseline), BASELINE], 60)
    bounded(["cargo", "build", "--release", "--locked", "--manifest-path", str(baseline / "Cargo.toml")], 900)
    windows = os.name == "nt"
    harness = root / "scripts" / ("measure-tui-windows.py" if windows else "measure-tui-unix.py")
    executable = "sd300.exe" if windows else "sd300"
    for label, source, revision in [("before", baseline, BASELINE), ("after", root, candidate)]:
        report = output / f"{label}-tui.json"
        command = [sys.executable, str(harness), str(source / "target/release" / executable),
                   "--seconds", str(args.seconds), "--output", str(report), "--revision", revision]
        bounded(command, args.seconds + 120)
    report = json.loads((output / "after-tui.json").read_text(encoding="utf-8"))
    gates = ["foreground_cpu_gate" if windows else "cpu_gate", "rss_gate", "terminal_restored"]
    if windows:
        gates.append("private_gate")
    failures = [gate for gate in gates if report.get(gate) is not True]
    if failures:
        raise RuntimeError("Candidate resource gates failed: " + ", ".join(failures))


if __name__ == "__main__":
    main()
