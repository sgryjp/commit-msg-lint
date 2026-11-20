// Run after building the Web target:
// wasm-pack build web --target web --dev --out-dir /tmp/commit-msg-lint-web-pkg
// WASM_PKG_DIR=/tmp/commit-msg-lint-web-pkg node --test web/tests/site-wasm.test.cjs
const test = require("node:test");
const assert = require("node:assert/strict");
const fs = require("node:fs");
const path = require("node:path");
const vm = require("node:vm");
const { pathToFileURL } = require("node:url");

function element(value = "") {
  return {
    value, disabled: false, children: [], listeners: {},
    addEventListener(type, callback) { this.listeners[type] = callback; },
    dispatch(type) { this.listeners[type](); },
    replaceChildren() { this.children = []; },
    append(child) { this.children.push(child); },
  };
}

async function setup() {
  assert.ok(process.env.WASM_PKG_DIR, "set WASM_PKG_DIR to a wasm-pack --target web build");
  const pkg = path.resolve(process.env.WASM_PKG_DIR);
  const wasm = await import(pathToFileURL(path.join(pkg, "commit_msg_lint_web.js")));
  const bytes = fs.readFileSync(path.join(pkg, "commit_msg_lint_web_bg.wasm"));
  const html = fs.readFileSync(path.join(__dirname, "../../site/index.html"), "utf8");
  const script = html.match(/<script type="module">([\s\S]*?)<\/script>/)[1];
  const importLine = 'import init, { defaultConfig, lint } from "./pkg/commit_msg_lint_web.js";';
  assert.ok(script.includes(importLine));
  const elements = {
    message: element("feat: add parser"), config: element(),
    "reset-config": element(), result: element(),
  };
  const document = {
    getElementById: id => elements[id],
    createElement: () => element(),
  };
  await vm.runInNewContext(
    `(async () => { ${script.replace(importLine, "")} })()`,
    { document, init: () => wasm.default({ module_or_path: bytes }),
      defaultConfig: wasm.defaultConfig, lint: wasm.lint },
  );
  return elements;
}

test("demo displays ignore defaults and applies configuration edits and reset", async () => {
  const elements = await setup();
  const config = elements.config;
  const message = elements.message;
  const result = () => elements.result.children.map(child => child.textContent);
  assert.deepEqual(JSON.parse(config.value).ignore,
    { merge: true, revert: true, autosquash: true, "header-patterns": [] });

  message.value = `Merge branch feature\n\n${"x".repeat(100)}`;
  message.dispatch("input");
  assert.deepEqual(result(), ["No problems found."]);

  config.value = '{"version":1,"ignore":{"merge":false}}';
  config.dispatch("input");
  assert.ok(result().some(text => text.includes("[header-format]")));
  assert.ok(result().some(text => text.includes("[body-max-width]")));

  config.value = '{"version":1,"ignore":{"merge":false,"header-patterns":["branch"]}}';
  config.dispatch("input");
  assert.deepEqual(result(), ["No problems found."]);

  message.value = "bad header";
  config.value = '{"version":1,"rules":{"header-format":{"severity":"warning"}}}';
  config.dispatch("input");
  assert.ok(elements.result.children.some(child => child.className === "warning"));
  assert.ok(result().some(text => text.includes("warning[header-format]")));

  message.value = `Merge branch feature\n\n${"x".repeat(100)}`;
  config.value = '{"version":1,"ignore":{"header-patterns":[""]}}';
  config.dispatch("input");
  assert.match(result()[0], /must not contain an empty pattern/);
  assert.equal(elements.result.children[0].className, "error");

  elements["reset-config"].dispatch("click");
  assert.equal(message.value, `Merge branch feature\n\n${"x".repeat(100)}`);
  assert.deepEqual(result(), ["No problems found."]);
});
