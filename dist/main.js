const $ = (id) => document.getElementById(id);
const notice = (text) => { $("notice").textContent = text; };

const tauri = window.__TAURI__;
if (!tauri?.core?.invoke || !tauri?.event?.listen) {
  notice("Tauri API 未就绪，请重启应用");
  throw new Error("Tauri global API unavailable");
}
const invoke = tauri.core.invoke;
const listen = tauri.event.listen;

function render(status) {
  const labels = {
    initializing: "正在初始化", "need-setup": "需要设置凭据",
    "credential-error": "凭据存储异常", checking: "正在检查网络",
    "logging-in": "正在认证", online: "已连接", offline: "等待重试",
    "invalid-credentials": "密码错误",
  };
  $("state").textContent = labels[status.state] || status.state || "-";
  $("account").textContent = status.account || "-";
  $("lastCheck").textContent = status.lastCheck || "-";
  $("lastLogin").textContent = status.lastLogin || "-";
  $("message").textContent = status.message || "-";
}

let statusRevision = 0;
async function refresh() {
  try {
    const revision = statusRevision;
    const snapshot = await invoke("get_status");
    if (revision === statusRevision) render(snapshot);
    const enabled = await invoke("is_autostart");
    $("autostart").textContent = enabled ? "开机自启：已开启" : "开机自启：已关闭";
  } catch (error) {
    notice(String(error));
  }
}

async function init() {
  await listen("status", (event) => {
    statusRevision += 1;
    render(event.payload);
  });
  await refresh();
}
init().catch(error => notice(String(error)));
document.addEventListener("visibilitychange", () => {
  if (!document.hidden) refresh();
});
window.addEventListener("focus", refresh);

$("reconnect").onclick = async () => {
  try { await invoke("reconnect"); notice("已发送重连请求"); }
  catch (error) { notice(String(error)); }
};
$("reset").onclick = async () => {
  if (!window.confirm("清除本机保存的校园网凭据？此操作不会主动注销当前网络。")) return;
  try { await invoke("reset_credentials"); notice("凭据已删除，请重新设置"); }
  catch (error) { notice(String(error)); }
};
$("setup").onclick = () => { $("credentials").hidden = !$("credentials").hidden; };
$("credentials").onsubmit = async (event) => {
  event.preventDefault();
  try {
    await invoke("setup_credentials", {
      account: $("accountInput").value,
      password: $("passwordInput").value,
    });
    $("credentials").hidden = true;
    $("accountInput").value = "";
    $("passwordInput").value = "";
    notice("凭证已保存");
  } catch (error) { notice(String(error)); }
};
$("autostart").onclick = async () => {
  try {
    const enabled = await invoke("is_autostart");
    await invoke("set_autostart", { enabled: !enabled });
    notice("开机自启设置已更新");
    refresh();
  } catch (error) { notice(String(error)); }
};
$("hide").onclick = async () => {
  try { await invoke("hide_window"); notice("已隐藏到后台；托盘可唤回，无托盘时重新启动本程序即可显示"); }
  catch (error) { notice(String(error)); }
};
$("quit").onclick = async () => {
  if (window.confirm("彻底退出并停止后台保活？")) await invoke("quit");
};
