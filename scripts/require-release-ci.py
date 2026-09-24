#!/usr/bin/env python3
"""Require the newest exact-revision CI run before either publication step."""
import argparse
import json
import re
import subprocess
import time


def latest_candidate_run(runs, sha):
    candidates = [run for run in runs if run.get('headSha') == sha
                  and run.get('event') in ('push', 'workflow_dispatch')]
    return max(candidates, key=lambda run: (run['createdAt'], run['databaseId']), default=None)


def require_success(fetch, sha, timeout_seconds, clock=time.monotonic, sleep=time.sleep):
    deadline = clock() + timeout_seconds
    last_state = object()
    while True:
        run = latest_candidate_run(fetch(), sha)
        state = (run['databaseId'], run['status'], run['conclusion']) if run else None
        if state != last_state:
            print(f"Candidate CI: {state}; {run['url'] if run else sha}", flush=True)
            last_state = state
        if run and run['status'] == 'completed':
            if run['conclusion'] != 'success':
                raise RuntimeError(f"Candidate CI did not pass: {run['conclusion']} ({run['url']}). Publication is blocked.")
            return run
        remaining = deadline - clock()
        if remaining <= 0:
            raise RuntimeError(f"No completed successful CI run for exact revision {sha} before the deadline. "
                               "Run CI on this exact revision through a main push or workflow_dispatch. Publication is blocked.")
        sleep(min(30, remaining))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', required=True)
    parser.add_argument('--sha', required=True)
    parser.add_argument('--timeout-seconds', type=int, default=3600)
    args = parser.parse_args()
    if not re.fullmatch(r'[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+', args.repo):
        parser.error('Invalid repository')
    if not re.fullmatch(r'[0-9a-f]{40}', args.sha):
        parser.error('An exact commit SHA is required')
    if not 1 <= args.timeout_seconds <= 7200:
        parser.error('Timeout must be between 1 and 7200 seconds')

    def fetch():
        output = subprocess.run(
            ['gh', 'run', 'list', '--repo', args.repo, '--workflow', 'ci.yml',
             '--commit', args.sha, '--limit', '100', '--json',
             'databaseId,headSha,event,status,conclusion,url,createdAt'],
            capture_output=True, text=True, encoding='utf-8', timeout=30, check=True)
        return json.loads(output.stdout)

    run = require_success(fetch, args.sha, args.timeout_seconds)
    print(f"Exact-revision CI passed: {run['url']}", flush=True)


if __name__ == '__main__':
    main()
