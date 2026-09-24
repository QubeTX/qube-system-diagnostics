#!/usr/bin/env python3
import contextlib
import importlib.util
import io
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('release_ci', Path(__file__).with_name('require-release-ci.py'))
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)
SHA = 'a' * 40


def run(number=1, status='completed', conclusion='success', event='push', sha=SHA):
    return dict(databaseId=number, headSha=sha, event=event, status=status,
                conclusion=conclusion, url=f'https://example.invalid/runs/{number}',
                createdAt=f'2026-09-24T00:00:{number:02d}Z')


class ReleaseCiGateTests(unittest.TestCase):
    def require(self, batches, timeout=60):
        now = [0]
        remaining = iter(batches)
        last = [[]]

        def fetch():
            last[0] = next(remaining, last[0])
            return last[0]

        def sleep(seconds):
            now[0] += seconds

        with contextlib.redirect_stdout(io.StringIO()):
            return gate.require_success(fetch, SHA, timeout, lambda: now[0], sleep)

    def test_matching_push_passes(self):
        self.assertEqual(self.require([[run()]])['databaseId'], 1)

    def test_manual_exact_revision_passes(self):
        self.assertEqual(self.require([[run(event='workflow_dispatch')]])['databaseId'], 1)

    def test_pr_merge_ref_is_not_exact_revision_proof(self):
        with self.assertRaisesRegex(RuntimeError, 'deadline'):
            self.require([[run(event='pull_request')]], timeout=1)

    def test_different_revision_cannot_unlock_publication(self):
        with self.assertRaisesRegex(RuntimeError, 'deadline'):
            self.require([[run(sha='b' * 40)]], timeout=1)

    def test_newer_failure_cannot_fall_back_to_old_success(self):
        with self.assertRaisesRegex(RuntimeError, 'did not pass: failure'):
            self.require([[run(), run(2, conclusion='failure')]])

    def test_pending_newest_run_waits_even_with_old_success(self):
        self.assertEqual(self.require([[run(), run(2, 'in_progress', '')], [run(2)]])['databaseId'], 2)

    def test_late_failure_blocks_publication(self):
        with self.assertRaisesRegex(RuntimeError, 'did not pass: failure'):
            self.require([[run(status='in_progress', conclusion='')], [run(conclusion='failure')]])

    def test_no_run_has_a_finite_deadline(self):
        with self.assertRaisesRegex(RuntimeError, 'deadline'):
            self.require([[]], timeout=1)

    def test_canceled_skipped_and_timed_out_do_not_pass(self):
        for conclusion in ('cancelled', 'skipped', 'timed_out', 'neutral', 'action_required'):
            with self.subTest(conclusion=conclusion), self.assertRaisesRegex(RuntimeError, 'did not pass'):
                self.require([[run(conclusion=conclusion)]])


if __name__ == '__main__':
    unittest.main()
