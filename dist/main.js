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
  $("state").textContent = status.state || "-";
  $("account").textContent = status.account || "-";
  $("lastCheck").textContent = status.lastCheck || "-";
  $("lastLogin").textContent = status.lastLogin || "-";
  $("message").textContent = status.message || "-";
}

async function refresh() {
  try {
    render(await invoke("get_status"));
    const enabled = await invoke("is_autostart");
    $("autostart").textContent = enabled ? "开机自启：已开启" : "开机自启：已关闭";
  } catch (error) {
    notice(String(error));
  }
}

listen("status", (event) => render(event.payload));
refresh();
setInterval(refresh, 1000);

$("reconnect").onclick = async () => {
  try { await invoke("reconnect"); notice("已发送重连请求"); }
  catch (error) { notice(String(error)); }
};
$("reset").onclick = async () => {
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
$("quit").onclick = async () => { await invoke("quit"); };
