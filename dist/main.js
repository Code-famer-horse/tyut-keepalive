import { normalizeTheme, readTheme, saveTheme, describeState, addHistory } from "./ui-state.js";
const $ = id => document.getElementById(id);
const pages = ["overview", "logs", "settings", "about"];
const controls = ["reconnect", "saveCredentials", "reset", "autostart", "hide", "quit"];
const tauri = window.__TAURI__;
const invoke = tauri?.core?.invoke;
const listen = tauri?.event?.listen;
let ready = false;
let autostartKnown = false;
let busy = false;
let statusRevision = 0;
let requestRevision = 0;
let history = [];
function notice(message, error = false) {
  const el = $("notice");
  el.textContent = String(message || "");
  el.dataset.error = String(error);
  el.hidden = !message;
}
function setPage(page) {
  if (!pages.includes(page)) return;
  for (const name of pages) $("page-" + name).hidden = name !== page;
  for (const button of document.querySelectorAll("[data-page]")) {
    if (button.dataset.page === page) button.setAttribute("aria-current", "page");
    else button.removeAttribute("aria-current");
  }
  if (page === "logs") renderHistory();
}
for (const button of document.querySelectorAll("[data-page]")) {
  button.addEventListener("click", () => setPage(button.dataset.page));
}
$("setup").addEventListener("click", () => {
  setPage("settings");
  $("accountInput").focus();
});
function applyTheme(theme) {
  const value = normalizeTheme(theme);
  document.documentElement.dataset.theme = value;
  $("theme").value = value;
}
let storage;
try { storage = window.localStorage; } catch { storage = null; }
applyTheme(readTheme(storage));
$("theme").addEventListener("change", () => {
  applyTheme($("theme").value);
  if (!saveTheme(storage, $("theme").value)) notice("主题已切换，但无法保存偏好设置", true);
});
function updateControls() {
  for (const id of controls) $(id).disabled = !ready || busy || (id === "autostart" && !autostartKnown);
}
function render(status) {
  if (!status || typeof status !== "object") return;
  const [label, tone] = describeState(status.state);
  $("connectionCard").dataset.tone = tone;
  $("state").textContent = label;
  $("account").textContent = status.account || "未配置";
  $("lastCheck").textContent = status.lastCheck || "—";
  $("lastLogin").textContent = status.lastLogin || "—";
  $("message").textContent = status.message || "—";
}
function renderHistory() {
  const filter = $("logFilter").value;
  const entries = history.filter(entry => filter === "all" || entry.tone === filter);
  const fragment = document.createDocumentFragment();
  for (const entry of entries) {
    const li = document.createElement("li");
    const time = document.createElement("time");
    time.dateTime = entry.receivedAt;
    time.textContent = new Date(entry.receivedAt).toLocaleString();
    const label = document.createElement("strong");
    label.textContent = entry.label;
    label.dataset.tone = entry.tone;
    const message = document.createElement("span");
    message.textContent = entry.message;
    li.append(time, label, message);
    fragment.append(li);
  }
  $("historyList").replaceChildren(fragment);
  $("historyEmpty").hidden = entries.length > 0;
}
$("logFilter").addEventListener("change", renderHistory);
$("clearHistory").addEventListener("click", () => { history = []; renderHistory(); });
async function refreshStatus() {
  const request = ++requestRevision;
  const eventRevision = statusRevision;
  const snapshot = await invoke("get_status");
  if (request === requestRevision && eventRevision === statusRevision) render(snapshot);
}
let autostartRevision = 0;
async function refreshAutostart() {
  const revision = ++autostartRevision;
  try {
    const enabled = await invoke("is_autostart");
    if (revision !== autostartRevision) return;
    $("autostart").checked = Boolean(enabled);
    $("autostartText").textContent = enabled ? "系统设置：已开启" : "系统设置：已关闭";
    autostartKnown = true;
  } catch {
    if (revision !== autostartRevision) return;
    $("autostartText").textContent = "无法读取自启设置";
    autostartKnown = false;
  } finally { updateControls(); }
}
async function refresh() {
  if (!ready || busy) return;
  try { await Promise.all([refreshStatus(), refreshAutostart()]); }
  catch (error) { notice("读取状态失败：" + String(error), true); }
}
async function action(fn) {
  if (!ready || busy) return;
  busy = true;
  updateControls();
  try { await fn(); } catch (error) { notice(String(error), true); }
  finally { busy = false; updateControls(); }
}
$("reconnect").addEventListener("click", () => action(async () => {
  await invoke("reconnect");
  notice("已发送重连请求");
}));
$("credentials").addEventListener("submit", event => {
  event.preventDefault();
  action(async () => {
    const account = $("accountInput").value.trim();
    const password = $("passwordInput").value;
    if (!account || !password) { notice("请输入账号和密码", true); return; }
    try { await invoke("setup_credentials", { account, password }); }
    finally { $("passwordInput").value = ""; }
    $("accountInput").value = "";
    setPage("overview");
    notice("凭据已保存，等待后台认证");
  });
});
$("reset").addEventListener("click", () => action(async () => {
  if (!window.confirm("清除本机保存的校园网凭据？此操作不会主动注销当前网络。")) return;
  await invoke("reset_credentials");
  $("credentials").reset();
  notice("凭据已删除，请重新设置");
}));
$("autostart").addEventListener("change", () => action(async () => {
  const enabled = $("autostart").checked;
  ++autostartRevision;
  try {
    const actual = await invoke("set_autostart", { enabled });
    $("autostart").checked = Boolean(actual);
    $("autostartText").textContent = actual ? "系统设置：已开启" : "系统设置：已关闭";
    notice("开机自启设置已更新");
  } catch (error) {
    await refreshAutostart();
    throw error;
  }
}));
$("hide").addEventListener("click", () => action(async () => {
  await invoke("hide_window");
  notice("已隐藏窗口，后台服务继续运行");
}));
$("quit").addEventListener("click", () => action(async () => {
  if (window.confirm("彻底退出并停止后台保活？")) await invoke("quit");
}));
document.addEventListener("visibilitychange", () => { if (!document.hidden) refresh(); });
window.addEventListener("focus", refresh);
async function init() {
  if (!invoke || !listen) {
    $("serviceText").textContent = "后台未连接";
    notice("当前未连接 Tauri 后台，界面可预览，网络操作不可用。", true);
    return;
  }
  try {
    await listen("status", event => {
      statusRevision += 1;
      render(event.payload);
      history = addHistory(history, event.payload, new Date().toISOString());
      if (!$("page-logs").hidden) renderHistory();
    });
    await refreshStatus();
    ready = true;
    $("serviceText").textContent = "后台服务已连接";
    updateControls();
    await refreshAutostart();
    try {
      const version = await tauri.app?.getVersion?.();
      if (version) $("appVersion").textContent = "v" + version + " · 开发分支";
    } catch { /* keep the truthful generic development label */ }
  } catch (error) {
    ready = false;
    updateControls();
    $("serviceText").textContent = "后台连接失败";
    notice("初始化失败：" + String(error), true);
  }
}
init();
