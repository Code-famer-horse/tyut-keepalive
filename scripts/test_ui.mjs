import test from "node:test";
import assert from "node:assert/strict";
import { normalizeTheme, readTheme, saveTheme, describeState, addHistory, HISTORY_LIMIT } from "../dist/ui-state.js";

test("theme supports three modes and handles corrupt preferences", () => {
  for (const theme of ["system", "light", "dark"]) assert.equal(normalizeTheme(theme), theme);
  for (const value of [null, undefined, "unknown", "__proto__"]) assert.equal(normalizeTheme(value), "system");
  const values = new Map();
  const storage = { getItem: key => values.get(key), setItem: (key, value) => values.set(key, value) };
  assert.equal(readTheme(storage), "system");
  assert.equal(saveTheme(storage, "dark"), true);
  assert.equal(readTheme(storage), "dark");
  assert.equal(readTheme(null), "system");
  assert.equal(saveTheme(null, "light"), false);
});
test("every backend connection state has a localized presentation", () => {
  for (const state of ["initializing", "need-setup", "credential-error", "checking", "logging-in", "online", "offline", "invalid-credentials"]) {
    assert.notEqual(describeState(state)[0], "未知状态");
  }
  assert.equal(describeState("online")[1], "success");
  assert.equal(describeState("invalid-credentials")[1], "danger");
  assert.equal(describeState("__proto__")[0], "未知状态");
});
test("UI history records actual transitions, deduplicates and bounds memory", () => {
  const timestamp = "2026-11-26T00:00:00Z";
  const event = { state: "online", message: "网络正常", account: "never copy account" };
  const first = addHistory([], event, timestamp);
  assert.equal(first.length, 1);
  assert.equal(Object.hasOwn(first[0], "account"), false);
  assert.equal(addHistory(first, event, timestamp), first);
  let history = first;
  for (let i = 0; i <= HISTORY_LIMIT; i++) history = addHistory(history, { state: "offline", message: String(i) }, timestamp);
  assert.equal(history.length, HISTORY_LIMIT);
  assert.equal(history[0].message, String(HISTORY_LIMIT));
});
