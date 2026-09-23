"""Shared live-family sampling; optional descriptor access never fabricates zero."""
import psutil


def sample_family(root, known):
    rss = fds = count = 0
    descriptors_available = True
    for child in [root, *root.children(recursive=True)]:
        try:
            known[(child.pid, child.create_time())] = child
            rss += child.memory_info().rss  # Denied memory invalidates RSS qualification.
            count += 1
            try:
                fds += child.num_fds()
            except psutil.AccessDenied:
                descriptors_available = False
        except psutil.NoSuchProcess:
            pass
    if len(known) > 4096:
        raise RuntimeError("Observed process identities exceed the bounded qualification inventory")
    return rss / 2**20, fds if descriptors_available else None, count


def descriptor_summary(samples):
    missing = sum(row[1] is None for row in samples)
    observed = max((row[1] for row in samples if row[1] is not None), default=None)
    return {"fd_count_max": None if missing else observed, "observed_fd_count_max": observed,
            "fd_unavailable_samples": missing}
