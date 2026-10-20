/* #1394 revision2：实际页面/Client的状态转换；DOM、timer、传输和分配为具名TestOnly。 */
"use strict";
const fs = require("node:fs"),
  path = require("node:path"),
  vm = require("node:vm"),
  crypto = require("node:crypto"),
  assert = require("node:assert/strict"),
  test = require("node:test"),
  { Module, createRequire } = require("node:module");
const root = path.resolve(__dirname, "../.."),
  helperPath = path.join(__dirname, "test_q3_complete_client.cjs"),
  mod = new Module(helperPath, module),
  req = createRequire(helperPath);
mod.filename = helperPath;
mod.require = (name) => (name === "node:test" ? () => {} : req(name));
mod._compile(fs.readFileSync(helperPath, "utf8"), helperPath);
const H = mod.exports;
const html = fs.readFileSync(
    path.join(root, "s_session/browser/index.html"),
    "utf8",
  ),
  bundle = fs.readFileSync(
    path.join(root, "s_session/browser/tb01c-client.js"),
    "utf8",
  ),
  script = html
    .split("<script>")[1]
    .split("</script>")[0]
    .replace('load("current");', "");
function page(fetch) {
  const elements = new Map(),
    timers = new Map();
  let serial = 0,
    pickers = 0,
    writes = 0;
  function element(tag = "div", id = "") {
    const classes = new Set(id === "btn-live" ? ["hidden"] : []);
    return {
      tag,
      children: [],
      textContent: "",
      innerHTML: "",
      value: "1",
      checked: true,
      disabled: false,
      dataset: {},
      listeners: {},
      open: false,
      classList: {
        toggle(k, force) {
          const yes = force ?? !classes.has(k);
          if (yes) classes.add(k);
          else classes.delete(k);
        },
        contains: (k) => classes.has(k),
      },
      addEventListener(k, fn) {
        this.listeners[k] = fn;
      },
      focus() {},
      scrollIntoView() {},
      replaceChildren(...nodes) {
        this.children = nodes;
      },
      append(...nodes) {
        this.children.push(...nodes);
      },
    };
  }
  const document = {
    getElementById(id) {
      if (!elements.has(id)) elements.set(id, element("div", id));
      return elements.get(id);
    },
    querySelector(id) {
      return this.getElementById(id);
    },
    createElement: (tag) => element(tag),
  };
  const context = vm.createContext({
    document,
    TextEncoder,
    TextDecoder,
    crypto: crypto.webcrypto,
    performance,
    AbortController,
    URLSearchParams,
    structuredClone,
    console,
    fetch,
    setTimeout(cb, ms) {
      const id = ++serial;
      timers.set(id, { cb, ms });
      return id;
    },
    clearTimeout(id) {
      timers.delete(id);
    },
    showSaveFilePicker: async () => {
      pickers++;
      return {
        createWritable: async () => ({
          write: async () => {
            writes++;
          },
          close: async () => {},
          abort: async () => {},
        }),
      };
    },
    SQueryClientContext: {
      protocol: "s-session/3",
      resources: H.CONFIG,
      allocation: H.measurement(),
    },
  });
  vm.runInContext(bundle, context);
  vm.runInContext(script, context);
  return {
    context,
    elements,
    timers,
    pickers: () => pickers,
    writes: () => writes,
  };
}
const tick = () => new Promise((r) => setImmediate(r));
async function settle(p) {
  await vm.runInContext("renderQ3(v2Client.committed,pageEpoch)", p.context);
  await tick();
}
async function exerciseOldDetail(p, detail) {
  const body = detail.children[2],
    save = detail.children[1];
  detail.open = true;
  await detail.listeners.toggle();
  await save.listeners.click();
  for (let i = 0; i < 10; i++) await tick();
  assert(body.children.length > 0, "保留committed详情全字段仍可访问");
  assert(p.pickers() > 0, "保留committed导出入口仍可打开用户选择器");
  assert(p.writes() > 0, "按同一完整图流式导出确实写出内容");
}
test("失败刷新、失败Watch与取消保同committed时旧details/导出保持可用；换cut后旧闭包拒绝", async () => {
  const server = await H.makeServer();
  let fail = false,
    release,
    entered;
  const p = page(async (...args) => {
    if (fail) throw Error("controlled read failure");
    return server.fetch(...args);
  });
  await vm.runInContext('load("current")', p.context);
  await settle(p);
  const before = vm.runInContext("v2Client.committed", p.context),
    oldContext = vm.runInContext("q3ViewContext", p.context),
    detail = p.elements
      .get("objects")
      .children.find((e) => e.tag === "details");
  fail = true;
  await vm.runInContext('load("current")', p.context);
  assert.equal(vm.runInContext("v2Client.committed", p.context), before);
  assert.equal(vm.runInContext("q3ViewContext", p.context), oldContext);
  await exerciseOldDetail(p, detail);
  await vm.runInContext("applyStream()", p.context);
  await exerciseOldDetail(p, detail);
  assert.equal(vm.runInContext("v2Client.committed", p.context), before);
  fail = false;
  const gate = new Promise((r) => (release = r)),
    reached = new Promise((r) => (entered = r));
  server.before = async () => {
    entered();
    await gate;
  };
  const pending = vm.runInContext('load("current")', p.context);
  await reached;
  vm.runInContext("v2Client.cancel()", p.context);
  release();
  await pending;
  await exerciseOldDetail(p, detail);
  assert.equal(vm.runInContext("v2Client.committed", p.context), before);
  server.before = null;
  await vm.runInContext('load("current")', p.context);
  await settle(p);
  assert.notEqual(vm.runInContext("v2Client.committed", p.context), before);
  const pickers = p.pickers();
  await detail.children[1].listeners.click();
  await tick();
  assert.equal(p.pickers(), pickers, "新committed后旧视图export闭包不得放行");
});
test("原Q3 live控件可见、健康poll续排、busy不重入、stop与错误停止，原pollMs不扩默认", async () => {
  const server = await H.makeServer();
  let fail = false;
  const p = page((...args) => {
    if (fail) throw Error("controlled watch failure");
    return server.fetch(...args);
  });
  await vm.runInContext('load("current")', p.context);
  await settle(p);
  const button = p.elements.get("btn-live");
  assert.equal(button.classList.contains("hidden"), false);
  button.listeners.click();
  const scheduled = [...p.timers.entries()].find(
    ([, v]) => v.ms === H.CONFIG.pollMs,
  );
  assert(scheduled);
  p.timers.delete(scheduled[0]);
  const poll = scheduled[1].cb();
  assert.equal(vm.runInContext("v2Busy", p.context), true);
  assert.equal(
    [...p.timers.values()].filter((t) => t.ms === H.CONFIG.pollMs).length,
    0,
  );
  await poll;
  assert.equal(vm.runInContext("v2Busy", p.context), false);
  assert.equal(vm.runInContext("v2Live", p.context), true);
  assert.equal(
    [...p.timers.values()].filter((t) => t.ms === H.CONFIG.pollMs).length,
    1,
  );
  button.listeners.click();
  assert.equal(vm.runInContext("v2Live", p.context), false);
  assert.equal(
    [...p.timers.values()].filter((t) => t.ms === H.CONFIG.pollMs).length,
    0,
  );
  const requests = server.requests.length;
  await scheduled[1].cb();
  assert.equal(
    server.requests.length,
    requests,
    "已停止的旧timer闭包不发新请求",
  );
  button.listeners.click();
  fail = true;
  const next = [...p.timers.entries()].find(
    ([, v]) => v.ms === H.CONFIG.pollMs,
  );
  p.timers.delete(next[0]);
  const committed = vm.runInContext("v2Client.committed", p.context);
  await next[1].cb();
  assert.equal(vm.runInContext("v2Live", p.context), false);
  assert.equal(vm.runInContext("v2Busy", p.context), false);
  assert.equal(
    [...p.timers.values()].filter((t) => t.ms === H.CONFIG.pollMs).length,
    0,
  );
  assert.equal(vm.runInContext("v2Client.committed", p.context), committed);
});
// 与独立红相同的真实函数/validator微任务调度；本测试不依赖scratch原件存在。
const { htmlContext } = require("./test_tb01c_browser.cjs"),
  API = require("../browser/tb01c-client.js");
const fixture = JSON.parse(
  fs.readFileSync(__dirname + "/fixtures/r12_consumer.json"),
).records;
const a1 = JSON.parse(fixture.a1.body),
  a3 = JSON.parse(fixture.a3.body),
  a4 = JSON.parse(fixture.a4.body),
  watch = JSON.parse(fixture.watch3.body);
const delta4 = {
  ...watch,
  generation: "4",
  structure_cut: "cut-4",
  deltas: watch.deltas.filter((d) => d.generation === "4"),
};
// #1414：页面先读发现回执，只有明确 legacy 才读 /api/state；替身按此协议补 /api/discover。
const legacyDiscovery = () =>
  new Response(
    JSON.stringify({ ok: true, protocol_revision: null, producer_epoch: "1" }),
  );
async function run(source = null) {
  const trials = [];
  for (let turns = 0; turns <= 12; turns++) {
    const fetch = async (url) =>
      url === "/api/discover"
        ? legacyDiscovery()
        : {
            ok: true,
            status: 200,
            json: async () => structuredClone(a1),
          };
    let { context, elements } = htmlContext({ TB01C: API, fetch });
    if (source) {
      context = vm.createContext({
        document: context.document,
        TB01C: API,
        fetch,
        structuredClone,
        console,
        setTimeout,
        clearTimeout,
        performance,
      });
      vm.runInContext(source, context);
    }
    const old = vm.runInContext('load("asof","1")', context);
    for (let k = 0; k < turns; k++) await Promise.resolve();
    const atNewStart = vm.runInContext(
      "({epoch:pageEpoch,cut:currentState?.snapshot?.structure_cut||null})",
      context,
    );
    const fresh = vm.runInContext('load("asof","bad")', context);
    await Promise.all([old, fresh]);
    const after = vm.runInContext(
      "({epoch:pageEpoch,cut:currentState?.snapshot?.structure_cut||null})",
      context,
    );
    trials.push({
      turns,
      at_new_start: atNewStart,
      after,
      new_invalid_request_error_present: elements
        .get("mode-indicator")
        .textContent.includes("失败"),
    });
  }
  return trials;
}
async function runStream(source = null) {
  const trials = [];
  for (let turns = 0; turns <= 40; turns++) {
    const fetch = async (url) => ({
      ok: true,
      status: 200,
      json: async () =>
        structuredClone(url.startsWith("/api/delta") ? delta4 : a4),
    });
    let { context } = htmlContext({ TB01C: API, fetch });
    if (source) {
      context = vm.createContext({
        document: context.document,
        TB01C: API,
        fetch,
        structuredClone,
        console,
        setTimeout,
        clearTimeout,
        performance,
      });
      vm.runInContext(source, context);
    }
    context.seed = structuredClone(a3);
    await vm.runInContext("validateState(seed)", context);
    vm.runInContext(
      'rebuildCommittedFromState(seed);syncModeUI("asof",seed.snapshot);renderAllFromState(seed)',
      context,
    );
    const old = vm.runInContext("applyStream()", context);
    for (let k = 0; k < turns; k++) await Promise.resolve();
    const atNewStart = vm.runInContext(
      "({epoch:pageEpoch,cut:currentState?.snapshot?.structure_cut||null,applied:lastAppliedGen})",
      context,
    );
    const fresh = vm.runInContext('load("asof","bad")', context);
    await Promise.all([old, fresh]);
    const after = vm.runInContext(
      "({epoch:pageEpoch,cut:currentState?.snapshot?.structure_cut||null,applied:lastAppliedGen})",
      context,
    );
    trials.push({ turns, at_new_start: atNewStart, after });
  }
  return trials;
}

const probes = { run, runStream };
test("legacy load所有0..12真实微任务调度拒旧校验安装并保最新失败提示", async () => {
  const trials = await probes.run();
  assert.equal(trials.length, 13);
  for (const t of trials)
    if (t.at_new_start.cut === null) {
      assert.equal(t.after.cut, null);
      assert.equal(t.new_invalid_request_error_present, true);
    }
});
test("legacy stream所有0..40真实微任务调度拒旧Delta/cursor安装，健康路径保留", async () => {
  const trials = await probes.runStream();
  assert.equal(trials.length, 41);
  for (const t of trials)
    if (t.at_new_start.applied === "3") assert.equal(t.after.applied, "3");
  const { htmlContext } = require("./test_tb01c_browser.cjs"),
    API = require("../browser/tb01c-client.js"),
    records = JSON.parse(
      fs.readFileSync(__dirname + "/fixtures/r12_consumer.json"),
    ).records,
    a1 = JSON.parse(records.a1.body);
  const { context } = htmlContext({
    TB01C: API,
    fetch: async (url) =>
      url === "/api/discover"
        ? legacyDiscovery()
        : {
            ok: true,
            status: 200,
            json: async () => structuredClone(a1),
          },
  });
  await vm.runInContext('load("asof","1")', context);
  assert.equal(
    vm.runInContext("currentState.snapshot.structure_cut", context),
    "cut-1",
  );
});
