"""Measure unchanged candidate bytes with their source harness; no GUI observer."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser()
parser.add_argument("--target", required=True)
parser.add_argument("--seconds", required=True, type=int, choices=[330, 900])
args = parser.parse_args()
root = Path.cwd()
binaries = list((root / "downloaded").rglob("sd300-gui"))
assert len(binaries) == 1, "Ambiguous downloaded GUI"
gui = binaries[0]
cli = gui.with_name("sd300")
engine = gui.with_name("libsd300_engine.dylib" if sys.platform == "darwin" else "libsd300_engine.so")
for path in (gui, cli):
    path.chmod(path.stat().st_mode | 0o111)
hashes = {"sha256": hashlib.sha256(gui.read_bytes()).hexdigest(),
          "engine_sha256": hashlib.sha256(engine.read_bytes()).hexdigest(),
          "collector_sha256": hashlib.sha256(cli.read_bytes()).hexdigest()}
reports = [json.loads(p.read_text(encoding="utf-8")) for p in (root / "original-reports").glob("*.json")]
matched = [report for report in reports if all(report.get(key) == value for key, value in hashes.items())]
assert matched and len({report["revision"] for report in matched}) == 1, "Artifact hashes disagree with native smoke evidence"
revision = matched[0]["revision"]
out = root / "target/candidate-resources"
out.mkdir(parents=True, exist_ok=True)
(out / "identity.json").write_text(json.dumps({"source_run": 35922389957,
    "source_head": "ec5635d6bf8ed13a96d0405f8a762e245739e459", "build_checkout": revision,
    "harness_source": "ec5635d6bf8ed13a96d0405f8a762e245739e459", "target": args.target,
    "before_evidence": "docs/qualification/v4/native-resources-309d2f3.json", **hashes}, indent=2)+"\n", encoding="utf-8")
failures = []
for kind, binary, extra in [("tui", cli, []), ("gui-foreground", gui, []), ("gui-hidden", gui, ["--hidden"])]:
    report_path = out / (kind + ".json")
    harness = root / "candidate/scripts" / ("measure-tui-unix.py" if kind == "tui" else "measure-gui-unix.py")
    command = [sys.executable, str(harness), str(binary), "--seconds", str(args.seconds), "--revision", revision, "--output", str(report_path), *extra]
    if kind.startswith("gui") and sys.platform == "linux":
        command = ["env", "GDK_BACKEND=x11", "xvfb-run", "-a", "dbus-run-session", "--", *command]
    # Each source harness owns finite cleanup. A hard outer failure stops this
    # host, so a potentially remaining child cannot contaminate the next window.
    subprocess.run(command, check=True, timeout=args.seconds + 120)
    report = json.loads(report_path.read_text(encoding="utf-8"))
    for gate in ["cpu_gate", "rss_gate", "clean_shutdown"] + (["terminal_restored"] if kind == "tui" else []):
        if report.get(gate) is not True:
            failures.append(kind + ":" + gate)
if failures:
    raise SystemExit("Candidate resource gates failed: " + ", ".join(failures))
