"""Validate the generated draft release manifest; runtime updater verifies cryptography."""
import base64
import json
from pathlib import Path
import sys
from urllib.parse import urlsplit

def require(condition, message):
    if not condition:
        raise SystemExit("Updater manifest check failed: " + message)

def check(data, tag):
    version = tag.removeprefix("v")
    require(data.get("version", "").removeprefix("v") == version, "manifest version differs from tag")
    platforms = data["platforms"]
    for target, bundle in [("windows-x86_64", "msi"), ("darwin-aarch64", "app"), ("linux-x86_64", "appimage")]:
        candidates = [platforms[k] for k in (target, target + "-" + bundle) if k in platforms]
        require(bool(candidates), "missing " + target)
        for item in candidates:
            url = urlsplit(item["url"])
            prefix = "/Code-famer-horse/tyut-keepalive/releases/download/" + tag + "/"
            require(url.scheme == "https" and url.netloc == "github.com" and url.path.startswith(prefix), "untrusted asset URL")
            signature = base64.b64decode(item["signature"], validate=True).decode("utf-8")
            comments = [line for line in signature.splitlines() if line.startswith("trusted comment:")]
            require(len(comments) == 1 and ("version:" + version) in comments[0].split("\t"), "missing matching signed version")
    print("Updater manifest covers all three release platforms and declares matching signed versions")

if __name__ == "__main__":
    check(json.loads(Path(sys.argv[1]).read_text(encoding="utf-8")), sys.argv[2])
