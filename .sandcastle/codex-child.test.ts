import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { appendFileSync, chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  boundedCodex, CodexTranscript, preflightCheckout, runCodexChild, workerEnvironment,
  type ChildOptions,
} from "./codex-child.ts";
import { MODEL, EFFORT, readRouterConfiguration } from "./codex-router-runtime.ts";

const goodResult = { status: "completed", summary: "已完成限定审阅", findings: [], validation: ["只读检查"] };
const sessionId = "12345678-1234-1234-1234-123456789abc";

function git(root: string, ...args: string[]): string {
  return execFileSync("git", ["-C", root, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
}

function fixture() {
  const dir = mkdtempSync(join(tmpdir(), "sandcastle-codex-child-test-"));
  const checkout = join(dir, "worker");
  const controller = join(dir, "controller");
  const bin = join(dir, "bin");
  const home = join(dir, "home");
  const codexHome = join(home, ".codex");
  for (const path of [checkout, controller, bin, codexHome]) mkdirSync(path, { recursive: true });
  const catalog = join(codexHome, "models.json");
  writeFileSync(catalog, JSON.stringify({ models: [{ slug: MODEL, supported_reasoning_levels: [{ effort: EFFORT }] }] }));
  writeFileSync(join(codexHome, "config.toml"), `openai_base_url = "http://127.0.0.1:4202/v1"\nmodel_catalog_json = ${JSON.stringify(catalog)}\n`);
  git(checkout, "init", "-b", "codex/1456-fixture");
  git(checkout, "config", "user.name", "Fixture");
  git(checkout, "config", "user.email", "fixture@example.invalid");
  writeFileSync(join(checkout, "source.txt"), "original\n");
  writeFileSync(join(checkout, ".gitignore"), ".env\n.sandcastle/.env\n");
  git(checkout, "add", ".");
  git(checkout, "commit", "-qm", "fixture");
  const promptFile = join(dir, "task.md");
  writeFileSync(promptFile, "审阅 source.txt；仅报告实际结果。\n");
  const options: ChildOptions = {
    ticket: 1456, parentThreadId: "parent-thread-123", managerAgentId: "/root/sandcastle_manager",
    checkout, controllerCheckout: controller, expectedHead: git(checkout, "rev-parse", "HEAD"),
    mode: "review", promptFile, outputDir: join(dir, "output"), idleTimeoutSeconds: 2, timeoutSeconds: 10,
  };
  const env = { ...process.env, PATH: bin + ":" + process.env.PATH, HOME: home, CODEX_HOME: codexHome };
  function fake(body: string) {
    const file = join(bin, "codex");
    const prologue = `#!${process.execPath}\n`
      + `const fs = require('node:fs'); const path = require('node:path'); const cp = require('node:child_process');\n`
      + `const root = ${JSON.stringify(dir)}; const sid = ${JSON.stringify(sessionId)};\n`
      + `const emit = (value) => process.stdout.write(JSON.stringify(value) + '\\n');\n`
      + `const capture = () => { fs.writeFileSync(path.join(root, 'argv.json'), JSON.stringify(process.argv.slice(2))); fs.writeFileSync(path.join(root, 'worker-env.json'), JSON.stringify(Object.keys(process.env))); };\n`
      + `const session = () => { const p = path.join(process.env.CODEX_HOME, 'sessions', '2026', '09', '14'); fs.mkdirSync(p, { recursive: true }); fs.writeFileSync(path.join(p, 'rollout-fixture-' + sid + '.jsonl'), '{}\\n'); emit({ type: 'thread.started', thread_id: sid }); };\n`
      + `const complete = (result = ${JSON.stringify(goodResult)}) => { emit({ type: 'item.completed', item: { type: 'agent_message', text: JSON.stringify(result) } }); emit({ type: 'turn.completed', usage: {input_tokens: 1, cached_input_tokens: 0, output_tokens: 1} }); };\n`
      + `let stdin = ''; process.stdin.setEncoding('utf8'); process.stdin.on('data', (x) => stdin += x); process.stdin.on('end', () => { fs.writeFileSync(path.join(root, 'received-prompt.md'), stdin); ${body}\n});\n`;
    writeFileSync(file, prologue); chmodSync(file, 0o700);
  }
  return { dir, options, env, fake, clean: () => rmSync(dir, { recursive: true, force: true }) };
}

function assertStoppedAndUnlocked(result: Awaited<ReturnType<typeof runCodexChild>>, options: ChildOptions) {
  assert.ok(result.process_pid);
  assert.throws(() => process.kill(result.process_pid!, 0), /ESRCH/);
  const lock = join(tmpdir(), `sandcastle-codex-child-${createHash("sha256").update(realpathSync(options.checkout)).digest("hex")}.lock`);
  assert.ok(!existsSync(lock));
}

test("SDK bypass 被精确收紧，prompt 不进入 argv，模型和推理档固定", async () => {
  const f = fixture();
  try {
  for (const mode of ["review", "execute"] as const) {
    const provider = await boundedCodex(mode, "/tmp/schema's.json", workerEnvironment(f.env));
    const command = provider.buildPrintCommand({ prompt: "$(touch /tmp/never-run) `pwd`", dangerouslySkipPermissions: true });
    assert.equal(command.stdin, "$(touch /tmp/never-run) `pwd`");
    assert.ok(command.command.startsWith("codex -a never exec --json --sandbox "));
    assert.ok(command.command.includes(mode === "review" ? "--sandbox read-only" : "--sandbox workspace-write"));
    assert.ok(command.command.includes("--ignore-user-config"));
    assert.ok(command.command.includes("'deepseek/deepseek-v4.1-flash'"));
    assert.ok(command.command.includes('model_reasoning_effort="high"'));
    assert.ok(command.command.includes('model_provider="codex-router"'));
    assert.ok(command.command.includes('model_providers.codex-router.base_url="http://127.0.0.1:4202/v1"'));
    assert.ok(command.command.includes('model_providers.codex-router.requires_openai_auth=true'));
    assert.ok(command.command.includes('model_providers.codex-router.supports_websockets=false'));
    assert.ok(command.command.includes('model_catalog_json='));
    assert.ok(command.command.includes("network_access=false"));
    assert.ok(!/dangerously|danger-full-access|merge-to-head|prime-agent|touch/.test(command.command));
    assert.throws(() => provider.buildPrintCommand({ prompt: "x", dangerouslySkipPermissions: false, resumeSession: "old" }), /resume\/fork/);
  }
  } finally { f.clean(); }
});

test("Router 拒绝非本机地址及 caller secret，错误不泄漏输入", async () => {
  const f = fixture();
  try {
    const token = "DO_NOT_EXPOSE_CALLER_CAPABILITY_1234567890";
    for (const baseUrl of ["https://127.0.0.1:4202/v1", "http://example.invalid:4202/v1", "http://127.0.0.1:99999/v1", `http://127.0.0.1:4202/_codex-router/${token}/v1`, "http://user:pass@127.0.0.1:4202/v1", "http://127.0.0.1:4202/v1?key=x"]) {
      writeFileSync(join(f.env.CODEX_HOME, "config.toml"), `openai_base_url = ${JSON.stringify(baseUrl)}\nmodel_catalog_json = ${JSON.stringify(join(f.env.CODEX_HOME, "models.json"))}\n`);
      assert.throws(() => readRouterConfiguration(workerEnvironment(f.env)), (error: unknown) => error instanceof Error && /Router 地址/.test(error.message) && !error.message.includes(token));
    }
    writeFileSync(join(f.env.CODEX_HOME, "config.toml"), `openai_base_url = "http://127.0.0.1:4202/_codex-router/${token}/v1"\nmodel_catalog_json = ${JSON.stringify(join(f.env.CODEX_HOME, "models.json"))}\n`);
    const result = await runCodexChild(f.options, { env: f.env });
    assert.equal(result.status, "failed");
    assert.equal(result.process_pid, null);
    assert.ok(!JSON.stringify(result).includes(token));
  } finally { f.clean(); }
});

test("Router 目录须唯一登记 Flash high；缺配置或坏 TOML 不回退", () => {
  const f = fixture();
  try {
    const env = workerEnvironment(f.env);
    assert.equal(readRouterConfiguration(env).baseUrl, "http://127.0.0.1:4202/v1");
    const model = { slug: MODEL, supported_reasoning_levels: [{ effort: "high" }] };
    for (const models of [[], [model, model], [{ slug: MODEL, supported_reasoning_levels: [{ effort: "xhigh" }] }]]) {
      writeFileSync(join(f.env.CODEX_HOME, "models.json"), JSON.stringify({ models }));
      assert.throws(() => readRouterConfiguration(env), /唯一.*high/);
    }
    writeFileSync(join(f.env.CODEX_HOME, "config.toml"), 'invalid = "UNREVEALED\n');
    assert.throws(() => readRouterConfiguration(env), (error: unknown) => error instanceof Error && !error.message.includes("UNREVEALED"));
    rmSync(join(f.env.CODEX_HOME, "config.toml"));
    assert.throws(() => readRouterConfiguration(env), /无法读取 Router/);
  } finally { f.clean(); }
});

test("认证沿用目录，环境只留白名单，不转发密钥或自动续跑配置", () => {
  const env = workerEnvironment({ PATH: "/bin", HOME: "/home/u", CODEX_HOME: "/auth-location", GH_TOKEN: "fixture", OPENAI_API_KEY: "fixture", OPENAI_BASE_URL: "http://untrusted.invalid", CODEX_THREAD_ID: "another", BASH_ENV: "/unsafe" });
  assert.deepEqual(env, { PATH: "/bin", HOME: "/home/u", CODEX_HOME: "/auth-location", NO_COLOR: "1" });
});

test("保留大小写代理路由环境，但不传 API/GitHub 密钥", () => {
  const routes = {
    HTTP_PROXY: "http://proxy.example.invalid:8080", HTTPS_PROXY: "http://proxy.example.invalid:8081",
    ALL_PROXY: "socks5://proxy.example.invalid:1080", NO_PROXY: "localhost,127.0.0.1",
    http_proxy: "http://proxy.example.invalid:8082", https_proxy: "http://proxy.example.invalid:8083",
    all_proxy: "socks5://proxy.example.invalid:1081", no_proxy: "example.invalid",
  };
  assert.deepEqual(workerEnvironment({ ...routes, GH_TOKEN: "fixture", OPENAI_API_KEY: "fixture" }), { ...routes, NO_COLOR: "1" });
});

test("checkout 守卫拒绝共享 cwd、脏树、错误 HEAD、main、detached HEAD 和项目配置", () => {
  const f = fixture();
  try {
    assert.equal(preflightCheckout(f.options).head, f.options.expectedHead);
    assert.throws(() => preflightCheckout({ ...f.options, controllerCheckout: f.options.checkout }), /独立 checkout/);
    assert.throws(() => preflightCheckout({ ...f.options, expectedHead: "0".repeat(40) }), /expected-head/);
    writeFileSync(join(f.options.checkout, "source.txt"), "dirty");
    assert.throws(() => preflightCheckout(f.options), /不干净/);
    git(f.options.checkout, "restore", "source.txt");
    writeFileSync(join(f.options.checkout, "untracked"), "dirty");
    assert.throws(() => preflightCheckout(f.options), /不干净/);
    rmSync(join(f.options.checkout, "untracked"));
    git(f.options.checkout, "branch", "-m", "main");
    assert.throws(() => preflightCheckout(f.options), /codex\/\*/);
    git(f.options.checkout, "branch", "-m", "codex/1456-fixture");
    git(f.options.checkout, "checkout", "--detach");
    assert.throws(() => preflightCheckout(f.options));
    git(f.options.checkout, "checkout", "codex/1456-fixture");
    mkdirSync(join(f.options.checkout, ".codex"));
    writeFileSync(join(f.options.checkout, ".codex", "config.toml"), "do_not_read = true");
    git(f.options.checkout, "add", ".codex/config.toml");
    git(f.options.checkout, "commit", "-qm", "config fixture");
    assert.throws(() => preflightCheckout({ ...f.options, expectedHead: git(f.options.checkout, "rev-parse", "HEAD") }), /项目 .codex\/config.toml/);
  } finally { f.clean(); }
});

for (const flag of ["assume-unchanged", "skip-worktree"] as const) {
  test(`预检拒绝 ${flag} 隐藏的实际修改，保留原始 index 标志`, () => {
    const f = fixture();
    try {
      git(f.options.checkout, "update-index", `--${flag}`, "source.txt");
      writeFileSync(join(f.options.checkout, "source.txt"), "hidden change");
      assert.equal(git(f.options.checkout, "status", "--porcelain"), "");
      const before = git(f.options.checkout, "ls-files", "-v");
      assert.throws(() => preflightCheckout(f.options), /assume-unchanged\/skip-worktree/);
      assert.equal(git(f.options.checkout, "ls-files", "-v"), before);
      assert.equal(readFileSync(join(f.options.checkout, "source.txt"), "utf8"), "hidden change");
    } finally { f.clean(); }
  });

  test(`后检拒绝工蜂新增 ${flag} 隐藏的修改`, async () => {
    const f = fixture();
    try {
      f.fake(`session(); cp.execFileSync('git', ['update-index', '--${flag}', 'source.txt']); fs.writeFileSync('source.txt', 'hidden change'); complete();`);
      const result = await runCodexChild({ ...f.options, mode: "execute" }, { env: f.env });
      assert.equal(result.status, "failed");
      assert.match(result.failure_reason ?? "", /assume-unchanged\/skip-worktree/);
      assert.equal(git(f.options.checkout, "status", "--porcelain"), "");
      assert.equal(readFileSync(join(f.options.checkout, "source.txt"), "utf8"), "hidden change");
    } finally { f.clean(); }
  });
}

test("core.filemode=false 不能隐藏权限修改，真实配置不变", () => {
  const f = fixture();
  try {
    git(f.options.checkout, "config", "core.filemode", "false");
    chmodSync(join(f.options.checkout, "source.txt"), 0o755);
    assert.equal(git(f.options.checkout, "status", "--porcelain"), "");
    assert.throws(() => preflightCheckout(f.options), /不干净/);
    assert.equal(git(f.options.checkout, "config", "core.filemode"), "false");
  } finally { f.clean(); }
});

test("真实子进程：直接 SDK provider 完成，保存 session/parent 映射，不加载项目 .env", async () => {
  const f = fixture();
  try {
    mkdirSync(join(f.options.checkout, ".sandcastle"));
    writeFileSync(join(f.options.checkout, ".sandcastle", ".env"), "GH_TOKEN=not-a-real-credential\n");
    writeFileSync(f.options.promptFile, `单引号 ' 双引号 " 反引号 \`pwd\` $(touch ${join(f.dir, "injection")})\n`);
    f.fake("capture(); session(); complete();");
    const states: string[] = [];
    const route = "http://proxy.example.invalid:8080";
    const result = await runCodexChild(f.options, { env: { ...f.env, HTTPS_PROXY: route }, onState: (s) => states.push(s.status) });
    assert.equal(result.status, "completed", result.failure_reason ?? "");
    assert.equal(result.parent_thread_id, f.options.parentThreadId);
    assert.equal(result.manager_agent_id, f.options.managerAgentId);
    assert.equal(result.external_session_id, sessionId);
    assert.ok(result.external_session_log && existsSync(result.external_session_log));
    assert.equal(result.worker_kind, "sandcastle_external_codex");
    assert.equal(result.native_subagent, false);
    assert.deepEqual(result.process_exit, { code: 0, signal: null });
    assert.ok(result.process_pid);
    assert.deepEqual(result.result, goodResult);
    assert.equal(states.at(-1), "completed");
    assert.equal(states.filter((s) => s === "completed").length, 1);
    assert.equal(JSON.parse(readFileSync(join(f.options.outputDir, "result.json"), "utf8")).status, "completed");
    assert.deepEqual(JSON.parse(readFileSync(join(f.dir, "argv.json"), "utf8")).slice(0, 6), ["-a", "never", "exec", "--json", "--sandbox", "read-only"]);
    assert.ok(!readFileSync(join(f.dir, "worker-env.json"), "utf8").includes("GH_TOKEN"));
    assert.ok(readFileSync(join(f.dir, "worker-env.json"), "utf8").includes("HTTPS_PROXY"));
    assert.ok(!readFileSync(join(f.options.outputDir, "result.json"), "utf8").includes(route));
    assert.ok(!readFileSync(join(f.options.outputDir, "events.jsonl"), "utf8").includes(route));
    assert.ok(readFileSync(join(f.dir, "received-prompt.md"), "utf8").includes("$(touch"));
    assert.ok(!existsSync(join(f.dir, "injection")));
    assert.equal(git(f.options.checkout, "status", "--porcelain"), "");
    assert.equal(git(f.options.checkout, "rev-parse", "HEAD"), f.options.expectedHead);
  } finally { f.clean(); }
});

test("不会覆盖结果目录，失败预检也留结构化结果且不开进程", async () => {
  const f = fixture();
  try {
    const result = await runCodexChild({ ...f.options, expectedHead: "0".repeat(40) }, { env: f.env });
    assert.equal(result.status, "failed");
    assert.equal(result.process_pid, null);
    const before = readFileSync(join(f.options.outputDir, "result.json"), "utf8");
    await assert.rejects(runCodexChild(f.options, { env: f.env }), /EEXIST/);
    assert.equal(readFileSync(join(f.options.outputDir, "result.json"), "utf8"), before);
  } finally { f.clean(); }
});

for (const [name, body, expected] of [
  ["空输出 exit0", "", /缺少/],
  ["坏 JSONL 后还有完成结果", "process.stdout.write('invalid\\n'); session(); complete();", /事件 JSONL/],
  ["最终结果坏 JSON", "session(); emit({type:'item.completed',item:{type:'agent_message',text:'not-json'}}); emit({type:'turn.completed'});", /最终结果不是/],
  ["error 不能被后续完成掩盖", "session(); emit({type:'error',message:'failed'}); complete();", /Codex 报告 error/],
  ["缺少 turn.completed", "session(); emit({type:'item.completed',item:{type:'agent_message',text:'{}'}});", /缺少/],
  ["有效结果但进程 exit7", "session(); complete(); process.exitCode = 7;", /code=7/],
  ["有效结果但自杀信号", "session(); complete(); process.kill(process.pid, 'SIGTERM');", /signal=SIGTERM/],
  ["session 文件缺失", "emit({type:'thread.started',thread_id:sid}); complete();", /实际 Codex session 日志/],
  ["业务 blocked", "session(); complete({status:'blocked',summary:'缺前置',findings:[],validation:[]});", /报告 blocked/],
  ["只读模式出现改动", "session(); fs.writeFileSync('source.txt', 'changed'); complete();", /checkout 出现修改/],
] as const) {
  test(`异常不成功：${name}`, async () => {
    const f = fixture();
    try {
      f.fake(body);
      const result = await runCodexChild(f.options, { env: f.env });
      assert.equal(result.status, "failed");
      assert.match(result.failure_reason ?? "", expected);
      assert.ok(result.ended_at);
      assert.ok(result.process_exit);
    } finally { f.clean(); }
  });
}

test("execute 只留独立分支差异，不做 commit/push/merge", async () => {
  const f = fixture();
  try {
    f.fake("capture(); session(); fs.writeFileSync('source.txt', 'changed'); complete();");
    const result = await runCodexChild({ ...f.options, mode: "execute" }, { env: f.env });
    assert.equal(result.status, "completed", result.failure_reason ?? "");
    assert.equal(result.codex_sandbox, "workspace-write");
    assert.ok(result.checkout_after?.status.includes("source.txt"));
    assert.equal(result.checkout_after?.head, f.options.expectedHead);
    assert.equal(result.automatic_claim, false);
    assert.equal(result.automatic_push, false);
    assert.equal(result.automatic_merge, false);
  } finally { f.clean(); }
});

test("运行中 HEAD 漂移必失败并保留差异，不自行回滚", async () => {
  const f = fixture();
  try {
    f.fake("session(); fs.writeFileSync('source.txt', 'changed'); cp.execFileSync('git', ['add', 'source.txt']); cp.execFileSync('git', ['commit', '-qm', 'unexpected fixture commit']); complete();");
    const result = await runCodexChild({ ...f.options, mode: "execute" }, { env: f.env });
    assert.equal(result.status, "failed");
    assert.match(result.failure_reason ?? "", /HEAD 或分支发生变化/);
    assert.notEqual(git(f.options.checkout, "rev-parse", "HEAD"), f.options.expectedHead);
  } finally { f.clean(); }
});

test("收到完成文本仍等待真实进程退出", async () => {
  const f = fixture();
  try {
    f.fake("session(); complete(); setTimeout(() => fs.writeFileSync(path.join(root, 'actually-exiting'), 'yes'), 200);");
    const result = await runCodexChild(f.options, { env: f.env, onState: (s) => {
      if (s.status === "completed") assert.ok(existsSync(join(f.dir, "actually-exiting")));
    } });
    assert.equal(result.status, "completed", result.failure_reason ?? "");
  } finally { f.clean(); }
});

test("无输出超时停止进程并失败", async () => {
  const f = fixture();
  try {
    f.fake("setInterval(() => {}, 1000);");
    const result = await runCodexChild({ ...f.options, idleTimeoutSeconds: 0.15 }, { env: f.env });
    assert.equal(result.status, "failed");
    assert.match(result.failure_reason ?? "", /空闲超时/);
    assert.notEqual(result.process_exit?.code, 0);
    assert.throws(() => process.kill(result.process_pid!, 0), /ESRCH/);
  } finally { f.clean(); }
});

test("管理者取消只终止自己的进程组，等待根进程和子进程退出", async () => {
  const f = fixture();
  try {
    f.fake("const descendant = cp.spawn(process.execPath, ['-e', 'setInterval(() => {}, 1000)'], {stdio:'ignore'}); fs.writeFileSync(path.join(root, 'descendant.pid'), String(descendant.pid)); session(); setInterval(() => {}, 1000);");
    const controller = new AbortController();
    const result = await runCodexChild(f.options, { env: f.env, signal: controller.signal, onState: (s) => {
      if (s.external_session_id) controller.abort();
    } });
    assert.equal(result.status, "cancelled");
    assert.notEqual(result.process_exit?.code, 0);
    assert.throws(() => process.kill(result.process_pid!, 0), /ESRCH/);
    const childPid = Number(readFileSync(join(f.dir, "descendant.pid"), "utf8"));
    assert.throws(() => process.kill(childPid, 0), /ESRCH/);
  } finally { f.clean(); }
});

test("预先取消不开工；相同 checkout 并行任务被锁挡住", async () => {
  const f = fixture();
  try {
    const cancelled = new AbortController(); cancelled.abort();
    const before = await runCodexChild({ ...f.options, outputDir: join(f.dir, "before") }, { signal: cancelled.signal, env: f.env });
    assert.equal(before.status, "cancelled"); assert.equal(before.process_pid, null);
    f.fake("session(); setInterval(() => {}, 1000);");
    const controller = new AbortController();
    let second: Promise<Awaited<ReturnType<typeof runCodexChild>>> | undefined;
    const first = await runCodexChild(f.options, { env: f.env, signal: controller.signal, onState: (s) => {
      if (s.external_session_id && !second) {
        second = runCodexChild({ ...f.options, outputDir: join(f.dir, "second") }, { env: f.env });
        void second.finally(() => controller.abort());
      }
    } });
    assert.equal(first.status, "cancelled");
    const denied = await second!;
    assert.equal(denied.status, "failed"); assert.equal(denied.process_pid, null);
    assert.match(denied.failure_reason ?? "", /EEXIST/);
  } finally { f.clean(); }
});

test("流解析拒绝重复 session 及 schema 外内容", async () => {
  const transcript = new CodexTranscript(await boundedCodex("review", "/tmp/schema.json"));
  transcript.accept(JSON.stringify({ type: "thread.started", thread_id: "a" }));
  transcript.accept(JSON.stringify({ type: "thread.started", thread_id: "b" }));
  transcript.accept(JSON.stringify({ type: "turn.completed" }));
  assert.throws(() => transcript.output(), /多个/);
  const extra = new CodexTranscript(await boundedCodex("review", "/tmp/schema.json"));
  extra.accept(JSON.stringify({ type: "thread.started", thread_id: "a" }));
  extra.accept(JSON.stringify({ type: "item.completed", item: { type: "agent_message", text: JSON.stringify({ ...goodResult, extra: true }) } }));
  extra.accept(JSON.stringify({ type: "turn.completed" }));
  assert.throws(() => extra.output(), /结果契约/);
});

for (const point of ["spawn", "session", "terminal"] as const) {
  test(`onState 在 ${point} 同步抛错：停止工蜂、释放锁、持久化 failed`, async () => {
    const f = fixture();
    try {
      f.fake(point === "terminal" ? "session(); complete();" : "session(); setInterval(() => {}, 1000);");
      let injected = false;
      const result = await runCodexChild(f.options, { env: f.env, onState: (state) => {
        const target = point === "spawn" ? state.process_pid !== null
          : point === "session" ? state.external_session_id !== null : state.status === "completed";
        if (target && !injected) { injected = true; throw new Error(`fixture ${point} callback failure`); }
      } });
      assert.ok(injected);
      assert.equal(result.status, "failed");
      assert.match(result.failure_reason ?? "", /callback failure/);
      assertStoppedAndUnlocked(result, f.options);
      assert.equal(JSON.parse(readFileSync(join(f.options.outputDir, "result.json"), "utf8")).status, "failed");
    } finally { f.clean(); }
  });
}

for (const stream of ["stdout", "stderr"] as const) {
  test(`${stream} 日志 ENOSPC：实际子进程被停止，failed 仍可交付`, async () => {
    const f = fixture();
    try {
      f.fake("session(); process.stderr.write('fixture log\\n'); setInterval(() => {}, 1000);");
      let injected = false;
      const result = await runCodexChild(f.options, {
        env: f.env,
        writeLog: (fd, chunk, channel) => {
          if (channel === stream) { injected = true; throw Object.assign(new Error("fixture ENOSPC"), { code: "ENOSPC" }); }
          appendFileSync(fd, chunk);
        },
      });
      assert.ok(injected);
      assert.equal(result.status, "failed");
      assert.match(result.failure_reason ?? "", /ENOSPC/);
      assertStoppedAndUnlocked(result, f.options);
      assert.equal(JSON.parse(readFileSync(join(f.options.outputDir, "result.json"), "utf8")).status, "failed");
    } finally { f.clean(); }
  });
}

test("运行状态文件写失败接入清理；独立 result 记录 failed", async () => {
  const f = fixture();
  try {
    f.fake("session(); setInterval(() => {}, 1000);");
    let injected = false;
    const result = await runCodexChild(f.options, { env: f.env, onState: (state) => {
      if (state.process_pid && !injected) { injected = true; mkdirSync(join(f.options.outputDir, "state.json.tmp")); }
    } });
    assert.equal(result.status, "failed");
    assert.match(result.failure_reason ?? "", /EISDIR/);
    assertStoppedAndUnlocked(result, f.options);
    assert.equal(JSON.parse(readFileSync(join(f.options.outputDir, "result.json"), "utf8")).status, "failed");
  } finally { f.clean(); }
});

test("最终 receipt 无法写入也返回 failed，不通知 completed，已退出并释放锁", async () => {
  const f = fixture();
  try {
    f.fake("session(); complete();");
    let injected = false;
    const statuses: string[] = [];
    const result = await runCodexChild(f.options, { env: f.env, onState: (state) => {
      statuses.push(state.status);
      if (state.process_pid && !injected) { injected = true; mkdirSync(join(f.options.outputDir, "result.json")); }
    } });
    assert.equal(result.status, "failed");
    assert.match(result.failure_reason ?? "", /终态持久化失败/);
    assert.ok(!statuses.includes("completed"));
    assertStoppedAndUnlocked(result, f.options);
  } finally { f.clean(); }
});

test("session finder pending 时取消，查找返回后仍为 cancelled", async () => {
  const f = fixture();
  try {
    f.fake("session(); complete();");
    const controller = new AbortController();
    let release!: (path: string) => void;
    let ready!: () => void;
    const entered = new Promise<void>((resolve) => { ready = resolve; });
    const statuses: string[] = [];
    const run = runCodexChild(f.options, {
      env: f.env, signal: controller.signal, onState: (state) => statuses.push(state.status),
      sessionFinder: async () => { ready(); return new Promise<string>((resolve) => { release = resolve; }); },
    });
    await entered;
    controller.abort();
    release(join(f.env.CODEX_HOME, "sessions", "2026", "09", "14", `rollout-fixture-${sessionId}.jsonl`));
    const result = await run;
    assert.equal(result.status, "cancelled");
    assert.ok(!statuses.includes("completed"));
    assertStoppedAndUnlocked(result, f.options);
    assert.equal(JSON.parse(readFileSync(join(f.options.outputDir, "result.json"), "utf8")).status, "cancelled");
  } finally { f.clean(); }
});

test("CLI stdout 管道关闭不会击穿监督器或遗留 detached 工蜂", async () => {
  const f = fixture();
  try {
    f.fake("setTimeout(() => session(), 100); setInterval(() => {}, 1000);");
    const args = ["--import", "tsx", resolve(".sandcastle/codex-child.mts"),
      "--ticket", "1456", "--parent-thread-id", f.options.parentThreadId,
      "--manager-agent-id", f.options.managerAgentId, "--checkout", f.options.checkout,
      "--expected-head", f.options.expectedHead, "--prompt-file", f.options.promptFile,
      "--output-dir", f.options.outputDir];
    const cli = spawn(process.execPath, args, { env: f.env, stdio: ["ignore", "pipe", "pipe"] });
    let buffer = "";
    let closed = false;
    cli.stdout.on("data", (chunk: Buffer) => {
      buffer += chunk.toString();
      if (!closed && /"process_pid":\d+/.test(buffer)) { closed = true; cli.stdout.destroy(); }
    });
    const timeout = setTimeout(() => cli.kill("SIGKILL"), 10000);
    const code = await new Promise<number | null>((done) => cli.on("close", done));
    clearTimeout(timeout);
    assert.ok(closed);
    assert.equal(code, 1);
    const result = JSON.parse(readFileSync(join(f.options.outputDir, "result.json"), "utf8"));
    assert.equal(result.status, "failed");
    assert.match(result.failure_reason, /EPIPE|输出渠道失败/);
    assertStoppedAndUnlocked(result, f.options);
  } finally { f.clean(); }
});

for (const signal of ["SIGINT", "SIGTERM"] as const) {
  test(`CLI ${signal} 被转为取消，退出码非零且留下结果`, async () => {
    const f = fixture();
    try {
      f.fake("session(); setInterval(() => {}, 1000);");
      const args = ["--import", "tsx", resolve(".sandcastle/codex-child.mts"),
        "--ticket", "1456", "--parent-thread-id", f.options.parentThreadId,
        "--manager-agent-id", f.options.managerAgentId, "--checkout", f.options.checkout,
        "--expected-head", f.options.expectedHead, "--prompt-file", f.options.promptFile,
        "--output-dir", f.options.outputDir];
      const cli = spawn(process.execPath, args, { env: f.env, stdio: ["ignore", "pipe", "pipe"] });
      let buffer = "";
      let sent = false;
      cli.stdout.on("data", (chunk: Buffer) => {
        buffer += chunk.toString();
        if (!sent && buffer.includes(`"external_session_id":"${sessionId}"`)) { sent = true; cli.kill(signal); }
      });
      const timeout = setTimeout(() => cli.kill("SIGKILL"), 10000);
      const code = await new Promise<number | null>((done) => cli.on("close", done));
      clearTimeout(timeout);
      assert.ok(sent); assert.equal(code, 130);
      const result = JSON.parse(readFileSync(join(f.options.outputDir, "result.json"), "utf8"));
      assert.equal(result.status, "cancelled");
      assert.throws(() => process.kill(result.process_pid, 0), /ESRCH/);
    } finally { f.clean(); }
  });
}
