"""Assess resource reports without changing their original measurements/verdicts."""
import math


def resource_verdict(report, version, *, frontend, hidden=False, windows=False, seconds=900):
    if frontend not in ("tui", "gui") or (frontend == "tui" and hidden):
        raise ValueError("Invalid resource measurement mode")
    exception = version == "4.0.0"
    limits = {
        "cpu_percent_one_core": (3 if hidden else 4) if exception else (1 if hidden else 2),
        "rss_mib_max": 200 if exception else 150,
    }
    if windows:
        limits["private_mib_max"] = 300
    failures = []

    def finite(value):
        return type(value) in (int, float) and math.isfinite(value) and value >= 0

    for field, limit in limits.items():
        value = report.get(field)
        if not finite(value) or value > limit:
            failures.append(field)
    elapsed = report.get("measured_seconds")
    if not finite(elapsed) or elapsed < seconds:
        failures.append("measurement_duration")
    samples = report.get("samples")
    if type(samples) is not int or samples < 1:
        failures.append("samples")
    if report.get("build") != "release":
        failures.append("release_build")
    if report.get("passed") is False or report.get("failure") or report.get("diagnostic_only"):
        failures.append("functional_or_diagnostic_failure")
    original = {k: v for k, v in report.items() if k.endswith("_gate")}
    cpu_key = "foreground_cpu_gate" if frontend == "tui" and windows else "cpu_gate"
    for key in [cpu_key, "rss_gate"] + (["private_gate"] if windows else []):
        if type(report.get(key)) is not bool:
            failures.append("missing_original_" + key)
    if frontend == "tui":
        if report.get("terminal_restored") is not True:
            failures.append("terminal_restored")
        # The Windows ConPTY harness proves bounded shutdown before restoration.
        if not windows and report.get("clean_shutdown") is not True:
            failures.append("clean_shutdown")
    elif report.get("clean_shutdown") is not True:
        failures.append("clean_shutdown")
    return {
        "policy": "v4.0.0-operator-resource-ceilings" if exception else "original-resource-targets",
        "limits": limits,
        "original_gates": original,
        "failures": failures,
        "passed": not failures,
    }
