"""Build an immutable baseline, then measure before/after sequentially on this native runner."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tarfile
import urllib.request
import zipfile

import psutil

BASELINE = "f83ae429490aecb165921037b2f9ed994327634f"
BASELINE_GUI_SOURCE = "d4896546f190c0e5afe176f36544dca7aa806227"


class CommandFailed(RuntimeError):
    """A completed child failed; distinct from an unbounded or interrupted run."""


def baseline_gui(root, target):
    name = "sd300-gui-macos-universal.zip" if target.startswith("macos-") else f"sd300-gui-{target}." + ("zip" if target.startswith("windows-") else "tar.xz")
    directory = root / "target/resource-baseline-gui"
    directory.mkdir()
    url = "https://github.com/QubeTX/qube-system-diagnostics/releases/download/v3.1.3/" + name
    with urllib.request.urlopen(url + ".sha256", timeout=30) as response:
        sidecar = response.read(4097)
    if len(sidecar) > 4096:
        raise RuntimeError("Baseline checksum sidecar exceeds its bound")
    fields = sidecar.decode("ascii").split()
    digest = fields[0] if fields else ""
    if len(digest) != 64 or any(c not in "0123456789abcdefABCDEF" for c in digest):
        raise RuntimeError("Invalid baseline checksum sidecar")
    archive = directory / name
    measured = hashlib.sha256()
    total = 0
    with urllib.request.urlopen(url, timeout=30) as response, archive.open("xb") as output:
        while chunk := response.read(1024 * 1024):
            total += len(chunk)
            if total > 200 * 1024 * 1024:
                raise RuntimeError("Baseline archive exceeds its bound")
            measured.update(chunk)
            output.write(chunk)
    if measured.hexdigest() != digest.lower():
        raise RuntimeError("Baseline archive checksum mismatch")
    payload = directory / "payload"
    payload.mkdir()
    if name.endswith(".zip"):
        with zipfile.ZipFile(archive) as source:
            if sum(info.file_size for info in source.infolist()) > 400 * 1024 * 1024:
                raise RuntimeError("Baseline expanded archive exceeds its bound")
            for info in source.infolist():
                if not (payload / info.filename).resolve().is_relative_to(payload.resolve()):
                    raise RuntimeError("Baseline ZIP member escapes its root")
            source.extractall(payload)
    else:
        with tarfile.open(archive) as source:
            if sum(info.size for info in source.getmembers()) > 400 * 1024 * 1024:
                raise RuntimeError("Baseline expanded archive exceeds its bound")
            source.extractall(payload, filter="data")
    if target.startswith("macos-"):
        binary = payload / "SD-300.app/Contents/MacOS/sd300-gui"
    elif target.startswith("windows-"):
        binary = payload / "sd300-gui.exe"
    else:
        binary = payload / "sd300/libexec/sd300-gui"
    binary.chmod(0o755)
    launcher = payload / "sd300/bin/sd300-gui" if target.startswith("linux-") else binary
    return binary, launcher


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
        raise CommandFailed(f"Native qualification command exited {code}")


def measure(command, seconds, report, *, baseline=False):
    try:
        bounded(command, seconds)
    except CommandFailed:
        # A broken immutable baseline must remain a failed observation, but
        # must not prevent measuring the replacement. Only accept an explicit,
        # bounded failure report from the completed baseline harness. Timeouts,
        # missing/malformed reports and every candidate failure still propagate.
        if not baseline or not report.is_file() or report.stat().st_size > 2 * 1024 * 1024:
            raise
        result = json.loads(report.read_text(encoding="utf-8"))
        if (result.get("schema") != 2 or result.get("passed") is not False
                or result.get("revision") != "v3.1.3" or result.get("legacy_in_process") is not True
                or not isinstance(result.get("failure"), str) or not result["failure"]):
            raise
        print(f"Immutable baseline failed; retain {report.name} and continue candidate measurement.", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path)
    parser.add_argument("--seconds", type=int, choices=[330, 900], required=True)
    parser.add_argument("--target", choices=["windows-x86_64", "macos-arm64", "macos-x86_64", "linux-gnu-x86_64", "linux-gnu-arm64", "linux-musl-x86_64"], required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    os.chdir(root)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    # The isolated Alpine runner mounts a checkout owned by the host UID. Scope
    # trust to this resolved checkout for these commands, not global Git state.
    git = ["git", "-c", f"safe.directory={root}"]
    candidate = subprocess.check_output([*git, "rev-parse", "HEAD"], text=True).strip()
    baseline = root / "target/resource-baseline-source"
    # A retained worktree may contain an interrupted earlier attempt: never
    # reset or delete it silently. A fresh hosted runner creates this once.
    if baseline.exists():
        raise RuntimeError(f"Baseline stage already exists: {baseline}")
    bounded([*git, "fetch", "--depth=1", "origin", BASELINE], 120)
    bounded([*git, "fetch", "--depth=1", "origin", "tag", "v3.1.3"], 120)
    public_source = subprocess.check_output([*git, "rev-parse", "v3.1.3^{commit}"], text=True).strip()
    if public_source != BASELINE_GUI_SOURCE:
        raise RuntimeError("Immutable public GUI baseline tag changed")
    bounded([*git, "diff", "--quiet", BASELINE_GUI_SOURCE, BASELINE, "--", "src", "gui", "gui-engine", "scripts", "Cargo.toml", "Cargo.lock", "build.rs", "rust-toolchain.toml"], 30)
    (output / "comparison-identity.json").write_text(json.dumps({"candidate": candidate, "baseline_cli_source": BASELINE,
        "baseline_gui_source": BASELINE_GUI_SOURCE, "baseline_gui_tag": "v3.1.3", "baseline_product_code_matches": True}, indent=2) + "\n", encoding="utf-8")
    bounded([*git, "worktree", "add", "--detach", str(baseline), BASELINE], 60)
    bounded(["cargo", "build", "--release", "--locked", "--manifest-path", str(baseline / "Cargo.toml")], 900)
    before_gui, before_launcher = baseline_gui(root, args.target)
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
    gui_harness = root / "scripts" / ("measure-gui-windows.py" if windows else "measure-gui-unix.py")
    after_gui = root / "target/native-gui-stage" / args.target / "app/zig-out/bin" / ("sd300-gui.exe" if windows else "sd300-gui")
    for label, binary, launcher, source, revision in [
        ("before", before_gui, before_launcher, baseline, "v3.1.3"),
        ("after", after_gui, after_gui, root, candidate),
    ]:
        shutil.copy2(source / "target/release" / executable, binary.with_name(executable))
        for hidden in (False, True):
            report_path = output / f"{label}-gui-{'hidden' if hidden else 'foreground'}.json"
            command = [sys.executable, str(gui_harness), str(binary), "--output", str(report_path),
                       "--revision", revision, "--seconds", str(args.seconds), "--section", "Overview" if hidden else "Thermals"]
            if not windows:
                command += ["--launcher", str(launcher)]
            if hidden:
                command.append("--hidden")
            if label == "before":
                command.append("--legacy-in-process")
            if args.target.startswith("linux-"):
                command = ["env", "GDK_BACKEND=x11", "xvfb-run", "-a", "dbus-run-session", "--", *command]
            measure(command, args.seconds + 120, report_path, baseline=label == "before")
            gui_report = json.loads(report_path.read_text(encoding="utf-8"))
            if label == "after":
                gui_gates = ["cpu_gate", "rss_gate", "clean_shutdown"] + (["private_gate"] if windows else [])
                failures += [f"gui-{'hidden' if hidden else 'foreground'}:{gate}" for gate in gui_gates if gui_report.get(gate) is not True]
    if failures:
        raise RuntimeError("Candidate resource gates failed: " + ", ".join(failures))


if __name__ == "__main__":
    main()
