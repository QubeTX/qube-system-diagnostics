import hashlib
import importlib.util
import io
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location("qualification", Path(__file__).with_name("qualify-native-resources.py"))
qualification = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualification)


class BaselineArchive(unittest.TestCase):
    def archive(self, member):
        data = io.BytesIO()
        with zipfile.ZipFile(data, "w") as archive:
            archive.writestr(member, b"fixture")
        return data.getvalue()

    def download(self, root, archive, checksum):
        with patch.object(qualification.urllib.request, "urlopen", side_effect=lambda url, timeout: io.BytesIO(checksum if url.endswith(".sha256") else archive)):
            return qualification.baseline_gui(root, "windows-x86_64")

    def test_verified_payload_is_bounded_to_its_stage(self):
        archive = self.archive("sd300-gui.exe")
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "target").mkdir()
            binary, launcher = self.download(root, archive, hashlib.sha256(archive).hexdigest().encode())
            self.assertEqual(binary.read_bytes(), b"fixture")
            self.assertEqual(binary, launcher)
            self.assertTrue(binary.is_relative_to(root))

    def test_mismatch_and_path_escape_are_rejected_before_use(self):
        for member, checksum in [("sd300-gui.exe", b"0" * 64), ("../escaped", None), ("sd300-gui.exe", b"")]:
            with self.subTest(member=member, checksum=checksum), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "target").mkdir()
                archive = self.archive(member)
                with self.assertRaises(RuntimeError):
                    self.download(root, archive, hashlib.sha256(archive).hexdigest().encode() if checksum is None else checksum)
                self.assertFalse((root / "target/resource-baseline-gui/escaped").exists())


if __name__ == "__main__":
    unittest.main()
