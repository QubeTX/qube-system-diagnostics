"""Check managed shell prefix/child contracts without installing the product."""
import os
from pathlib import Path
import shutil
import shlex
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
SHELL = os.environ.get("SD300_TEST_SHELL") or shutil.which("bash") or shutil.which("sh")
PREFIX_KEYS = ("SD300_INSTALL_DIR", "TR300_TUI_INSTALL_DIR", "CARGO_DIST_FORCE_INSTALL_DIR", "CARGO_HOME")


def shell_path(path):
    value = str(path).replace("\\", "/")
    return "/" + value[0].lower() + value[2:] if os.name == "nt" else value


@unittest.skipUnless(SHELL, "a POSIX shell is required")
class ManagedShellDiscovery(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="sd300-shell-discovery-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.home = self.root / "home"
        self.home.mkdir()
        self.env = {key: value for key, value in os.environ.items()
                    if key not in (*PREFIX_KEYS, "TR300_TUI_UNMANAGED_INSTALL", "TR300_TUI_NO_MODIFY_PATH",
                                   "INSTALLER_NO_MODIFY_PATH", "XDG_CONFIG_HOME", "GITHUB_PATH", "ZDOTDIR")}
        self.env.update(HOME=shell_path(self.home), SD300_MANAGED_INSTALLER_TEST_ONLY="1",
                        XDG_DATA_HOME=shell_path(self.root / "data"),
                        XDG_CACHE_HOME=shell_path(self.root / "cache"))

    def run_shell(self, code, overrides=None):
        return subprocess.run([SHELL, "-c", '. "$1"; ' + code, "sh",
            shell_path(ROOT / "scripts/managed-installers/sd300-installer.sh")],
            env=dict(self.env, **(overrides or {})), capture_output=True, text=True, timeout=10)

    def test_default_prefix_is_cargo_home(self):
        result = self.run_shell("sd300_install_prefix")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), shell_path(self.home) + "/.cargo")

    def test_custom_prefix_precedence(self):
        for start, key in enumerate(PREFIX_KEYS):
            with self.subTest(key=key):
                overrides = {name: shell_path(self.root / (name + " with spaces"))
                             for name in PREFIX_KEYS[start:]}
                result = self.run_shell("sd300_install_prefix", overrides)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), overrides[key])

    def test_child_receives_resolved_prefix_and_preserves_parent_environment(self):
        child = self.root / "fake dist installer.sh"
        child.write_text('printf "prefix=%s\\nno_modify=%s\\narg=%s\\n" '
                         '"$TR300_TUI_INSTALL_DIR" "$TR300_TUI_NO_MODIFY_PATH" "$1"\n',
                         encoding="utf-8", newline="\n")
        overrides = {"SD300_INSTALL_DIR": shell_path(self.root / "requested prefix"),
                     "TR300_TUI_INSTALL_DIR": "original-parent-value",
                     "TR300_TUI_NO_MODIFY_PATH": "1", "CHILD_FIXTURE": shell_path(child)}
        result = self.run_shell('sd300_intended_prefix=$(sd300_install_prefix); '
            'sd300_run_dist_installer "$CHILD_FIXTURE" --no-modify-path; '
            'printf "parent=%s\\n" "$TR300_TUI_INSTALL_DIR"', overrides)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), ["prefix=" + overrides["SD300_INSTALL_DIR"],
            "no_modify=1", "arg=--no-modify-path", "parent=original-parent-value"])

    def test_child_environment_stays_unset_in_parent(self):
        child = self.root / "fake dist installer.sh"
        child.write_text('test -n "$TR300_TUI_INSTALL_DIR"\n', encoding="utf-8", newline="\n")
        result = self.run_shell('sd300_intended_prefix=$(sd300_install_prefix); '
            'sd300_run_dist_installer "$CHILD_FIXTURE" || exit $?; '
            'test "${TR300_TUI_INSTALL_DIR+set}" != set', {"CHILD_FIXTURE": shell_path(child)})
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_child_failure_is_preserved(self):
        child = self.root / "failing dist installer.sh"
        child.write_text("exit 17\n", encoding="utf-8", newline="\n")
        result = self.run_shell('sd300_intended_prefix=$(sd300_install_prefix); '
            'sd300_run_dist_installer "$CHILD_FIXTURE"', {"CHILD_FIXTURE": shell_path(child)})
        self.assertEqual(result.returncode, 17)

    def test_unmanaged_mode_is_rejected_before_child_runs(self):
        result = self.run_shell("sd300_install_prefix; printf child-would-run",
            {"TR300_TUI_UNMANAGED_INSTALL": shell_path(self.root / "unmanaged")})
        self.assertNotEqual(result.returncode, 0)
        self.assertNotIn("child-would-run", result.stdout)
        self.assertIn("disables the receipt", result.stderr)
        self.assertIn("SD300_INSTALL_DIR", result.stderr)

    def test_path_opt_outs_and_arguments_are_not_rewritten(self):
        for key in ("TR300_TUI_NO_MODIFY_PATH", "INSTALLER_NO_MODIFY_PATH"):
            with self.subTest(key=key):
                result = self.run_shell('sd300_intended_prefix=$(sd300_install_prefix); '
                    'sd300_set_path_mutation_expectation; printf "%s" "$sd300_path_mutation_allowed"',
                    {key: "1"})
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, "0")
        result = self.run_shell('sd300_intended_prefix=$(sd300_install_prefix); '
            'sd300_set_path_mutation_expectation --no-modify-path; '
            'printf "%s" "$sd300_path_mutation_allowed"')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "0")

    def prepare_profiles(self, prefix_name="Installed 'app $cash `tick` & space"):
        prefix = self.root / prefix_name
        (prefix / "bin").mkdir(parents=True)
        binary = prefix / "bin/sd300"
        binary.write_text("#!/bin/sh\nprintf fixture\n", encoding="utf-8", newline="\n")
        binary.chmod(0o755)
        quoted_bin = shlex.quote(shell_path(prefix / "bin"))
        (prefix / "env").write_text(f'export PATH={quoted_bin}:"$PATH"\n', encoding="utf-8", newline="\n")
        (prefix / "env.fish").write_text(f"set -gx PATH {quoted_bin} $PATH\n", encoding="utf-8", newline="\n")
        state = self.root / "state"
        state.mkdir()
        xdg = self.root / "XDG config with spaces"
        self.env.update(FIXTURE_PREFIX=shell_path(prefix), FIXTURE_STATE=shell_path(state),
                        XDG_CONFIG_HOME=shell_path(xdg))
        self.profile_setup = ('PATH=/usr/bin:/bin; sd300_temp="$FIXTURE_STATE"; '
            'sd300_intended_prefix="$FIXTURE_PREFIX"; sd300_save_user_path_state; '
            'sd300_set_path_mutation_expectation; sd300_capture_user_path_written_state || exit $?; ')
        self.profile_complete = ('sd300_complete_shell_discovery || exit $?; '
            'sd300_capture_user_path_written_state || exit $?; ')
        return prefix, xdg, binary

    def test_new_interactive_bash_finds_binary_without_existing_bashrc(self):
        _, xdg, binary = self.prepare_profiles()
        result = self.run_shell(self.profile_setup + self.profile_complete +
            "bash --noprofile -ic 'command -v sd300'")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), shell_path(binary))
        self.assertTrue((self.home / ".bashrc").is_file())
        self.assertTrue((xdg / "fish/conf.d/sd300.env.fish").is_file())

    def test_native_fish_uses_actual_xdg_config_directory(self):
        fish = shutil.which("fish")
        if not fish:
            if os.environ.get("SD300_REQUIRE_FISH") == "1":
                self.fail("native fish is required for this qualification lane")
            self.skipTest("native fish startup requires an installed fish shell")
        _, xdg, binary = self.prepare_profiles()
        result = self.run_shell(self.profile_setup + self.profile_complete + "true")
        self.assertEqual(result.returncode, 0, result.stderr)
        result = subprocess.run([fish, "-ic", "command -s sd300"], env=dict(self.env),
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), shell_path(binary))
        self.assertTrue((xdg / "fish/conf.d/sd300.env.fish").is_file())

    def test_rollback_removes_only_exact_new_profile_entries(self):
        _, xdg, _ = self.prepare_profiles()
        result = self.run_shell(self.profile_setup + self.profile_complete +
            "sd300_restore_user_path_state")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.home / ".bashrc").exists())
        self.assertFalse(xdg.exists())

    def test_rollback_preserves_concurrent_profile_changes(self):
        _, xdg, _ = self.prepare_profiles()
        result = self.run_shell(self.profile_setup + self.profile_complete +
            'printf "# concurrent user setting\\n" >> "$HOME/.bashrc"; '
            'printf "# concurrent fish setting\\n" >> "$XDG_CONFIG_HOME/fish/conf.d/sd300.env.fish"; '
            "sd300_restore_user_path_state")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("# concurrent user setting", (self.home / ".bashrc").read_text())
        self.assertIn("# concurrent fish setting", (xdg / "fish/conf.d/sd300.env.fish").read_text())
        self.assertIn("concurrently changed", result.stderr)

    def test_existing_profiles_are_not_overwritten_and_roll_back_exactly(self):
        _, xdg, _ = self.prepare_profiles()
        bashrc = self.home / ".bashrc"
        bashrc.write_bytes(b"# keep bash configuration without newline")
        fish_file = xdg / "fish/conf.d/sd300.env.fish"
        fish_file.parent.mkdir(parents=True)
        fish_file.write_bytes(b"# keep fish configuration without newline")
        result = self.run_shell(self.profile_setup + self.profile_complete +
            'cat "$HOME/.bashrc"; printf "\\n---fish---\\n"; '
            'cat "$XDG_CONFIG_HOME/fish/conf.d/sd300.env.fish"; sd300_restore_user_path_state')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(result.stdout.startswith("# keep bash configuration without newline\n---fish---\n"
                                                "# keep fish configuration without newline\n"))
        self.assertEqual(bashrc.read_bytes(), b"# keep bash configuration without newline")
        self.assertEqual(fish_file.read_bytes(), b"# keep fish configuration without newline")

    def test_concurrently_created_bashrc_is_never_overwritten(self):
        self.prepare_profiles()
        result = self.run_shell(self.profile_setup +
            'printf "# created concurrently\\n" > "$HOME/.bashrc"; ' + self.profile_complete +
            "sd300_restore_user_path_state")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.home / ".bashrc").read_text(), "# created concurrently\n")

    def test_profile_completion_is_idempotent(self):
        _, xdg, _ = self.prepare_profiles()
        result = self.run_shell(self.profile_setup + self.profile_complete + self.profile_complete + "true")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.home / ".bashrc").read_text().count("\n. '"), 1)
        self.assertEqual((xdg / "fish/conf.d/sd300.env.fish").read_text().count("\nsource '"), 1)

    def test_profile_opt_out_makes_no_startup_changes(self):
        _, xdg, _ = self.prepare_profiles()
        for key in ("TR300_TUI_NO_MODIFY_PATH", "INSTALLER_NO_MODIFY_PATH"):
            with self.subTest(key=key):
                result = self.run_shell(self.profile_setup + self.profile_complete + "true", {key: "1"})
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertFalse((self.home / ".bashrc").exists())
                self.assertFalse(xdg.exists())

    def test_partial_completion_failure_keeps_bashrc_rollback_owned(self):
        prefix, xdg, _ = self.prepare_profiles()
        (prefix / "env.fish").unlink()
        result = self.run_shell(self.profile_setup +
            'sd300_complete_shell_discovery; status=$?; '
            'test "$status" -ne 0 || exit 99; '
            'sd300_capture_user_path_written_state || exit $?; sd300_restore_user_path_state')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.home / ".bashrc").exists())
        self.assertFalse(xdg.exists())

    def test_temporary_path_entry_does_not_skip_persistent_discovery(self):
        _, _, binary = self.prepare_profiles()
        result = self.run_shell(self.profile_setup +
            'PATH="$FIXTURE_PREFIX/bin:$PATH"; sd300_set_path_mutation_expectation; ' +
            self.profile_complete + 'test "$sd300_path_mutation_allowed" = 1 || exit 99; '
            'PATH=/usr/bin:/bin bash --noprofile -ic \'command -v sd300\'')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), shell_path(binary))

    def test_child_path_removes_only_selected_bin_and_preserves_parent(self):
        prefix, _, _ = self.prepare_profiles()
        child = self.root / "inspect child path.sh"
        child.write_text('printf "child=%s\\n" "$PATH"\n', encoding="utf-8", newline="\n")
        selected_bin = shell_path(prefix / "bin")
        entries = ["", "/usr/bin", selected_bin, "", "/bin", selected_bin, "/unrelated", ""]
        expected = ":".join(entry for entry in entries if entry != selected_bin)
        result = self.run_shell('sd300_intended_prefix="$FIXTURE_PREFIX"; '
            'PATH="$FIXTURE_PATH"; sd300_run_dist_installer "$CHILD_FIXTURE"; '
            'printf "parent=%s\\n" "$PATH"', {"CHILD_FIXTURE": shell_path(child),
                                                 "FIXTURE_PATH": ":".join(entries)})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.splitlines(), ["child=" + expected, "parent=" + ":".join(entries)])

    def test_child_path_opt_out_preserves_all_entries(self):
        prefix, _, _ = self.prepare_profiles()
        child = self.root / "inspect opted out path.sh"
        child.write_text('printf "%s\\n" "$PATH"\n', encoding="utf-8", newline="\n")
        fixture_path = "/usr/bin:" + shell_path(prefix / "bin") + ":/bin:"
        for overrides, argument in (({"TR300_TUI_NO_MODIFY_PATH": "1"}, ""),
                                    ({"INSTALLER_NO_MODIFY_PATH": "1"}, ""), ({}, "--no-modify-path")):
            with self.subTest(overrides=overrides, argument=argument):
                overrides.update(CHILD_FIXTURE=shell_path(child), FIXTURE_PATH=fixture_path)
                result = self.run_shell('sd300_intended_prefix="$FIXTURE_PREFIX"; '
                    'PATH="$FIXTURE_PATH"; sd300_run_dist_installer "$CHILD_FIXTURE" ' + argument, overrides)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout.strip(), fixture_path)


if __name__ == "__main__":
    unittest.main()
