export const THEMES = Object.freeze(["system", "light", "dark"]);
export function normalizeTheme(value) { return THEMES.includes(value) ? value : "system"; }
export function readTheme(storage) {
  try { return normalizeTheme(storage.getItem("tyut.theme")); } catch { return "system"; }
}
export function saveTheme(storage, theme) {
  try { storage.setItem("tyut.theme", normalizeTheme(theme)); return true; } catch { return false; }
}
export const STATES = Object.freeze({
  initializing: ["正在初始化", "neutral"],
  "need-setup": ["需要设置凭据", "neutral"],
  "credential-error": ["凭据存储异常", "danger"],
  checking: ["正在检查网络", "active"],
  "logging-in": ["正在认证", "active"],
  online: ["校园网已连接", "success"],
  offline: ["等待自动重试", "warning"],
  "invalid-credentials": ["密码错误", "danger"],
  paused: ["自动保活已暂停", "neutral"],
});
export function describeState(state) {
  return Object.hasOwn(STATES, state) ? STATES[state] : ["未知状态", "neutral"];
}
// Bounded in-memory UI history, never persisted and never includes credentials.
export const HISTORY_LIMIT = 100;
export function addHistory(history, status, receivedAt) {
  const [label, tone] = describeState(status.state);
  const entry = { state: status.state, label, tone, message: String(status.message || ""), receivedAt };
  const prior = history[0];
  if (prior && prior.state === entry.state && prior.message === entry.message) return history;
  return [entry, ...history].slice(0, HISTORY_LIMIT);
}
