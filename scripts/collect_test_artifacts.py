"""Collect only expected installer files; hash each and record exact commit."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
def require(condition, message):
    if not condition:
        raise SystemExit("Test artifact check failed: " + message)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", choices=["windows", "linux"], required=True)
    parser.add_argument("--target", required=True)
    args = parser.parse_args()
    expected = {"windows": "x86_64-pc-windows-msvc", "linux": "x86_64-unknown-linux-gnu"}
    require(args.target == expected[args.platform], "wrong architecture")
    bundle = ROOT / "src-tauri/target" / args.target / "release/bundle"
    patterns = [("msi", "*.msi")] if args.platform == "windows" else [("deb", "*.deb"), ("appimage", "*.AppImage")]
    out = ROOT / "test-artifacts"
    out.mkdir(exist_ok=False)
    files = []
    for directory, pattern in patterns:
        candidates = sorted((bundle / directory).glob(pattern))
        require(len(candidates) == 1, "expected exactly one " + pattern)
        package = candidates[0]
        require(package.is_file() and package.stat().st_size > 0, "empty package")
        require("test" in package.name.lower(), "installer filename lacks test marker")
        if directory == "deb":
            name = subprocess.check_output(["dpkg-deb", "-f", str(package), "Package"], text=True).strip()
            require(name == "tyut-keepalive-test", "unexpected Debian package name: " + name)
            listing = subprocess.check_output(["dpkg-deb", "-c", str(package)], text=True)
            require("./usr/bin/tyut-keepalive-test" in listing, "test binary missing from DEB")
        target = out / package.name
        shutil.copy2(package, target)
        files.append(target)
    commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    require(commit == os.environ.get("GITHUB_SHA", commit), "checkout mismatch")
    metadata = {"channel": "test-only", "commit": commit, "platform": args.platform,
                "target": args.target, "version": "0.4.0", "identifier": "com.tyut.keepalive.test",
                "updater": "disabled", "systemCodeSigning": "not configured"}
    (out / "BUILD-INFO.json").write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")
    files.append(out / "BUILD-INFO.json")
    with (out / "SHA256SUMS.txt").open("w", encoding="utf-8", newline="\n") as checks:
        for path in sorted(files):
            with path.open("rb") as file:
                digest = hashlib.file_digest(file, "sha256").hexdigest()
            checks.write(f"{digest}  {path.name}\n")
            print(f"{digest}  {path.name}")

if __name__ == "__main__":
    main()
