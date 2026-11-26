"""Generate a visibly marked test frontend; never modify the production dist."""
import json
from pathlib import Path
import shutil
import uuid

ROOT = Path(__file__).resolve().parents[1]
def require(condition, message):
    if not condition:
        raise SystemExit("Test build safety check failed: " + message)

def main():
    base = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))
    test = json.loads((ROOT / "src-tauri/tauri.test.conf.json").read_text(encoding="utf-8"))
    require(base["identifier"] == "com.tyut.keepalive", "unexpected production identifier")
    require(test["identifier"] == base["identifier"] + ".test", "test identifier not isolated")
    require(test["productName"] == base["productName"] + " Test", "test product name not isolated")
    require(test["mainBinaryName"] == "tyut-keepalive-test", "test executable name not isolated")
    require(test["build"]["frontendDist"] == "../dist-test", "test frontend must be separate")
    expected = uuid.uuid5(uuid.NAMESPACE_DNS, "com.tyut.keepalive.test.msi.x64")
    require(uuid.UUID(test["bundle"]["windows"]["wix"]["upgradeCode"]) == expected, "test MSI upgrade GUID mismatch")
    updater = test["plugins"]["updater"]
    require(test["bundle"]["createUpdaterArtifacts"] is False and updater["pubkey"] == "" and updater["endpoints"] == [], "test updater must be disabled")
    require(updater["requireSignedVersion"] is True and updater["allowDowngrades"] is False and updater["dangerousInsecureTransportProtocol"] is False, "unsafe updater config")
    require("测试版" in test["app"]["windows"][0]["title"], "test window is not visibly marked")

    target = ROOT / "dist-test"
    require(not target.exists(), "dist-test already exists; use a fresh checkout or remove only this generated directory")
    shutil.copytree(ROOT / "dist", target)
    html = target / "index.html"
    source = html.read_text(encoding="utf-8")
    require("TYUT Keepalive" in source, "branding anchor missing")
    source = source.replace("TYUT Keepalive", "TYUT Keepalive Test")
    source = source.replace("校园网络连接助手", "独立测试版 · 与正式版数据隔离")
    source = source.replace("启动时检查一次；仅在你确认后下载、验证签名及安装。未配置可信公钥时停用。", "独立测试版不检查或安装正式更新。")
    html.write_text(source, encoding="utf-8")
    main_js = target / "main.js"
    source = main_js.read_text(encoding="utf-8")
    anchor = "签名更新未启用：尚未配置可信公钥"
    require(anchor in source, "update status anchor missing")
    main_js.write_text(source.replace(anchor, "独立测试版：自动更新已禁用"), encoding="utf-8")
    print("Isolated test configuration and frontend prepared")

if __name__ == "__main__":
    main()
