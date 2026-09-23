"""Shared live-family sampling; optional descriptor access never fabricates zero."""
import os
import psutil


def sample_family(root, known, attribution=None):
    rss = fds = count = 0
    descriptors_available = True
    roles = {}
    for child in [root, *root.children(recursive=True)]:
        try:
            known[(child.pid, child.create_time())] = child
            memory = child.memory_info().rss  # Denied memory invalidates RSS qualification.
            rss += memory
            if attribution is not None:
                role = attribution.observe(child, memory)
                row = roles.setdefault(role, {"rss_mib": 0, "processes": 0})
                row["rss_mib"] += memory / 2**20
                row["processes"] += 1
            count += 1
            try:
                fds += child.num_fds()
            except psutil.AccessDenied:
                descriptors_available = False
        except psutil.NoSuchProcess:
            pass
    if len(known) > 4096:
        raise RuntimeError("Observed process identities exceed the bounded qualification inventory")
    if attribution is not None and rss > attribution.peak:
        attribution.peak, attribution.peak_roles = rss, roles
    return rss / 2**20, fds if descriptors_available else None, count


def descriptor_summary(samples):
    missing = sum(row[1] is None for row in samples)
    observed = max((row[1] for row in samples if row[1] is not None), default=None)
    return {"fd_count_max": None if missing else observed, "observed_fd_count_max": observed,
            "fd_unavailable_samples": missing}


class FamilyAttribution:
    """Bounded role evidence; total wait4 CPU remains the qualification oracle."""
    def __init__(self, root_pid):
        self.root_pid = root_pid
        self.identities = {}
        self.peak = 0
        self.peak_roles = {}

    def observe(self, child, rss):
        identity = child.pid, child.create_time()
        row = self.identities.get(identity)
        if row is None:
            role = "monitor" if child.pid == self.root_pid else "helper"
            if role == "helper":
                try:
                    args = child.cmdline()
                    if len(args) == 3 and args[1] == "collect-server" and args[2] in ("slow", "activity", "connections", "diagnostics", "static", "drivers", "health"):
                        role = "collector:" + args[2]
                    elif args and os.path.basename(args[0]) in ("ping", "ping6", "diskutil", "ioreg", "system_profiler", "route", "netstat", "ss", "smartctl", "launchctl"):
                        role = "helper:" + os.path.basename(args[0])
                except (psutil.AccessDenied, psutil.NoSuchProcess):
                    pass
            if len(self.identities) >= 4096:
                raise RuntimeError("Role attribution exceeds its bounded inventory")
            row = self.identities[identity] = {"role": role, "cpu": None, "rss": 0}
        row["rss"] = max(row["rss"], rss)
        try:
            cpu = child.cpu_times()
            row["cpu"] = cpu.user + cpu.system
        except (psutil.AccessDenied, psutil.NoSuchProcess):
            pass
        return row["role"]

    def report(self):
        roles = {}
        for row in self.identities.values():
            role = roles.setdefault(row["role"], {"observed_cpu_seconds": 0, "rss_mib_peak_per_process": 0, "cpu_unavailable_identities": 0})
            role["observed_cpu_seconds"] += row["cpu"] or 0
            role["cpu_unavailable_identities"] += row["cpu"] is None
            role["rss_mib_peak_per_process"] = max(role["rss_mib_peak_per_process"], row["rss"] / 2**20)
        return {"rss_peak_roles": self.peak_roles, "diagnostic_live_role_cost": roles,
                "diagnostic_note": "Role CPU omits work after the final live poll and children that exit between polls; the total wait4 CPU includes waited descendants"}
