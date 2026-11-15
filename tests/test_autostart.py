import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "Register-AutoNetAutostart.ps1"


class AutostartTest(unittest.TestCase):
    def test_script_exists_and_parses(self):
        self.assertTrue(SCRIPT.is_file(), "缺少自启注册脚本")
        result = subprocess.run(
            ["powershell.exe", "-NoProfile", "-Command",
             "$e=$null; [System.Management.Automation.Language.Parser]::ParseFile('Register-AutoNetAutostart.ps1',[ref]$null,[ref]$e)|Out-Null; if($e){$e; exit 1}"],
            cwd=ROOT, capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_existing_task_is_not_overwritten(self):
        self.assertTrue(SCRIPT.is_file(), "缺少自启注册脚本")
        source = SCRIPT.read_text(encoding="utf-8-sig")
        self.assertIn("Get-ScheduledTask", source)
        self.assertNotIn("-Force", source)
        self.assertIn("Interactive", source)
        self.assertIn("-AtLogOn", source)


if __name__ == "__main__":
    unittest.main()
