"""Qualify the standalone installer's launcher against Linux's real GIO parser."""
import ctypes
import ctypes.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import unittest


ROOT = Path(__file__).resolve().parent.parent
SHELL = os.environ.get("SD300_TEST_SHELL") or shutil.which("bash") or shutil.which("sh")


def shell_path(path):
    if os.name != "nt":
        return str(path)
    value = str(path).replace("\\", "/")
    return "/" + value[0].lower() + value[2:]


@unittest.skipUnless(SHELL, "a POSIX shell is required")
class DesktopEntry(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="sd300-desktop-")
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.template = self.root / "template.desktop"
        self.template.write_text("[Desktop Entry]\nType=Application\nName=SD-300\nExec=@SD300_GUI@\nIcon=sd300\nTerminal=false\n", encoding="utf-8", newline="\n")

    def configure(self, name):
        directory = self.root / name
        directory.mkdir()
        executable = directory / "sd300-gui"
        executable.write_text('#!/bin/sh\nprintf "%s" "$0" > "$SD300_DESKTOP_TEST_RESULT"\n', encoding="utf-8", newline="\n")
        executable.chmod(0o755)
        icon = directory / "app-icon.png"
        icon.write_bytes(b"fixture icon path")
        result = subprocess.run([SHELL, "-c", '. "$1"; sd300_configure_linux_desktop "$2" "$3" "$4"',
            "sh", shell_path(ROOT / "scripts/managed-installers/sd300-installer.sh"), shell_path(self.template),
            shell_path(executable), shell_path(icon)], env=dict(os.environ, SD300_MANAGED_INSTALLER_TEST_ONLY="1"),
            capture_output=True, timeout=10)
        return result, executable, icon

    def test_application_entry_uses_quoted_executable_and_absolute_owned_icon(self):
        result, executable, icon = self.configure("Applications with spaces & Unicode-é")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f'Exec="{shell_path(executable)}"\n', result.stdout.decode())
        self.assertIn(f'Icon={shell_path(icon)}\n', result.stdout.decode())
        self.assertNotIn("Icon=sd300\n", result.stdout.decode())

    @unittest.skipUnless(sys.platform.startswith("linux"), "GIO launch requires the native Linux runtime")
    def test_gio_resolves_icon_and_launches_literal_paths(self):
        gio = ctypes.CDLL(ctypes.util.find_library("gio-2.0") or "libgio-2.0.so.0")
        glib = ctypes.CDLL(ctypes.util.find_library("glib-2.0") or "libglib-2.0.so.0")
        objects = ctypes.CDLL(ctypes.util.find_library("gobject-2.0") or "libgobject-2.0.so.0")
        gio.g_desktop_app_info_new_from_filename.argtypes = [ctypes.c_char_p]
        gio.g_desktop_app_info_new_from_filename.restype = ctypes.c_void_p
        gio.g_app_info_get_icon.argtypes = [ctypes.c_void_p]
        gio.g_app_info_get_icon.restype = ctypes.c_void_p
        gio.g_file_icon_get_file.argtypes = [ctypes.c_void_p]
        gio.g_file_icon_get_file.restype = ctypes.c_void_p
        gio.g_file_get_path.argtypes = [ctypes.c_void_p]
        gio.g_file_get_path.restype = ctypes.c_void_p
        gio.g_app_info_launch.argtypes = [ctypes.c_void_p] * 4
        gio.g_app_info_launch.restype = ctypes.c_int
        glib.g_free.argtypes = [ctypes.c_void_p]
        objects.g_object_unref.argtypes = [ctypes.c_void_p]
        prior = os.environ.get("SD300_DESKTOP_TEST_RESULT")
        try:
            for index, name in enumerate(("Applications with spaces", "literal $cash", "literal `tick`",
                    'literal "quote"', "literal \\slash", "literal %field", "literal &amp",
                    'literal $cash `tick` "quote" \\slash %field &amp')):
                with self.subTest(name=name):
                    result, executable, icon = self.configure(name)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    desktop = self.root / f"configured-{index}.desktop"
                    desktop.write_bytes(result.stdout)
                    app = gio.g_desktop_app_info_new_from_filename(os.fsencode(desktop))
                    self.assertTrue(app, f"GIO rejected the synthetic application entry: {result.stdout!r}")
                    try:
                        icon_file = gio.g_file_icon_get_file(gio.g_app_info_get_icon(app))
                        icon_path = gio.g_file_get_path(icon_file)
                        self.assertTrue(icon_path, "GIO did not resolve a file icon")
                        try:
                            self.assertEqual(ctypes.string_at(icon_path), os.fsencode(icon))
                        finally:
                            glib.g_free(icon_path)
                        marker = self.root / f"launched-{index}.txt"
                        os.environ["SD300_DESKTOP_TEST_RESULT"] = str(marker)
                        self.assertTrue(gio.g_app_info_launch(app, None, None, None), "GIO failed to launch the entry")
                        deadline = time.monotonic() + 5
                        while not marker.exists() and time.monotonic() < deadline:
                            time.sleep(.025)
                        self.assertEqual(marker.read_bytes(), os.fsencode(executable))
                    finally:
                        objects.g_object_unref(app)
        finally:
            if prior is None:
                os.environ.pop("SD300_DESKTOP_TEST_RESULT", None)
            else:
                os.environ["SD300_DESKTOP_TEST_RESULT"] = prior

    def test_invalid_exec_path_is_rejected(self):
        for name in ("invalid=executable", "invalid\nline"):
            if os.name == "nt" and "\n" in name:
                continue
            with self.subTest(name=name):
                result, _, _ = self.configure(name)
                self.assertNotEqual(result.returncode, 0)

    def test_installed_binary_verification_quotes_its_path(self):
        directory = self.root / "Installed application"
        directory.mkdir()
        executable = directory / "sd300"
        executable.write_text("#!/bin/sh\nprintf '%s\\n' 'sd300 @SD300_VERSION@'\n", encoding="utf-8", newline="\n")
        executable.chmod(0o755)
        result = subprocess.run([SHELL, "-c", '. "$1"; sd300_intended_binary=$2; sd300_verify_binary',
            "sh", shell_path(ROOT / "scripts/managed-installers/sd300-installer.sh"), shell_path(executable)],
            env=dict(os.environ, SD300_MANAGED_INSTALLER_TEST_ONLY="1"), capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.decode().strip(), shell_path(executable))


if __name__ == "__main__":
    unittest.main()
