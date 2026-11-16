const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");

const html = fs.readFileSync(path.join(__dirname, "../site/index.html"), "utf8");
const script = html.match(/<script type="module">([\s\S]*?)<\/script>/)[1];
const importLine = 'import init, { defaultConfig, lint } from "./pkg/commit_msg_lint_web.js";';

function element(value = "") {
  return {
    value,
    disabled: false,
    children: [],
    listeners: {},
    addEventListener(type, callback) { this.listeners[type] = callback; },
    dispatch(type) { this.listeners[type](); },
    replaceChildren() { this.children = []; },
    append(child) { this.children.push(child); },
  };
}

async function setup() {
  assert.ok(script.includes(importLine), "the page imports the generated WASM API");
  const elements = {
    message: element("feat: add parser"),
    config: element(),
    "reset-config": element(),
    result: element(),
  };
  elements.config.disabled = true;
  elements["reset-config"].disabled = true;
  const calls = [];
  let resolveInit;
  let initialized = false;
  const init = () => new Promise((resolve) => {
    resolveInit = () => { initialized = true; resolve(); };
  });
  const defaults = '{\n  "version": 1\n}\n';
  const defaultConfig = () => {
    assert.ok(initialized, "defaults are retrieved only after WASM initialization");
    return defaults;
  };
  const lint = (message, config) => {
    calls.push([message, config]);
    if (config === "invalid") throw new Error("invalid configuration");
    return JSON.stringify(config === "custom" ? [
      { line: 1, severity: "warning", rule: "example", message: "custom diagnostic" },
    ] : []);
  };
  const document = {
    getElementById: (id) => elements[id],
    createElement: () => element(),
  };
  const execution = vm.runInNewContext(
    `(async () => { ${script.replace(importLine, "")} })()`,
    { document, init, defaultConfig, lint },
  );
  assert.equal(elements.config.disabled, true);
  assert.equal(elements["reset-config"].disabled, true);
  assert.equal(calls.length, 0);
  resolveInit();
  await execution;
  return { elements, calls, defaults };
}

test("inputs have accessible labels and reset is a button", () => {
  assert.match(html, /<label for="message">Commit message<\/label>/);
  assert.match(html, /<label for="config">Configuration \(JSONC, optional\)<\/label>/);
  assert.match(html, /<textarea id="config"[^>]* disabled><\/textarea>/);
  assert.match(html, /<button id="reset-config" type="button" disabled>Reset configuration to defaults<\/button>/);
});

test("initialization displays generated defaults and renders diagnostics", async () => {
  const { elements, calls, defaults } = await setup();
  assert.equal(elements.config.value, defaults);
  assert.equal(elements.config.disabled, false);
  assert.equal(elements["reset-config"].disabled, false);
  assert.deepEqual(calls, [["feat: add parser", defaults]]);
  assert.equal(elements.result.children[0].textContent, "No problems found.");
});

test("edits and empty configuration are linted, and reset preserves the message", async () => {
  const { elements, calls, defaults } = await setup();
  elements.message.value = "fix: keep my message";
  elements.message.dispatch("input");
  elements.config.value = "custom";
  elements.config.dispatch("input");
  assert.equal(elements.result.children[0].textContent, "1: warning[example] custom diagnostic");
  elements.config.value = "";
  elements.config.dispatch("input");
  assert.deepEqual(calls.at(-1), ["fix: keep my message", ""]);
  elements.config.value = "invalid";
  elements.config.dispatch("input");
  assert.match(elements.result.children[0].textContent, /invalid configuration/);
  elements["reset-config"].dispatch("click");
  assert.equal(elements.config.value, defaults);
  assert.equal(elements.message.value, "fix: keep my message");
  assert.deepEqual(calls.at(-1), ["fix: keep my message", defaults]);
  assert.equal(elements.result.children[0].textContent, "No problems found.");
});
