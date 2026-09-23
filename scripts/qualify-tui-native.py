"""Run interaction checks and stage profiling sequentially on the native CI host."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
suffix = ".exe" if os.name == "nt" else ""
binary = Path("target/release/sd300" + suffix)
for ascii_mode in (False, True):
    path = args.output / ("pty-ascii.json" if ascii_mode else "pty-unicode.json")
    command = [sys.executable, "scripts/qualify-tui-pty.py", str(binary), "--output", str(path)]
    if ascii_mode:
        command.append("--ascii")
    child = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    try:
        _, errors = child.communicate(timeout=120)
    except subprocess.TimeoutExpired:
        child.terminate()  # Unix handler performs owned-child cleanup; Windows closes its kill job.
        try:
            child.communicate(timeout=15)
        except subprocess.TimeoutExpired:
            child.kill()
            child.communicate()
        raise SystemExit("Terminal qualification exceeded its bound")
    if child.returncode:
        print(errors[-8192:], file=sys.stderr)
        raise SystemExit(child.returncode)
    report = json.loads(path.read_text(encoding="utf-8"))
    print(json.dumps({key: report[key] for key in ("sha256", "platform", "ascii", "startup_chooser_ms", "input_p95_ms", "input_max_ms", "terminal_restored")}))
profile = subprocess.run(["target/release/examples/profile-monitor" + suffix, "15"], capture_output=True, timeout=60)
if profile.returncode:
    print(profile.stderr.decode("utf-8", "replace")[-8192:], file=sys.stderr)
    raise SystemExit(profile.returncode)
(args.output/"stage-profile.json").write_bytes(profile.stdout)
