import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class LinuxSupportTest(unittest.TestCase):
    def test_linux_scripts_are_valid_shell(self):
        for name in ("start-linux.sh", "install-linux-autostart.sh"):
            with self.subTest(name=name):
                script = ROOT / name
                self.assertTrue(script.is_file(), f"缺少 {name}")
                result = subprocess.run(["bash", "-n", str(script)], capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)

    def test_daemon_does_not_use_windows_flags_on_linux(self):
        source = (ROOT / "AutoNet.py").read_text(encoding="utf-8")
        self.assertIn('if os.name != "nt":', source)


if __name__ == "__main__":
    unittest.main()
