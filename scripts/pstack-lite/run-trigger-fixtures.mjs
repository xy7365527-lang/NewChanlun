#!/usr/bin/env node
// pstack-lite：触发夹具 runner + 会话记录证据分析（#1130 验收③④⑥）
//
// 用法：
//   node scripts/pstack-lite/run-trigger-fixtures.mjs [--fixtures <json>]
//       [--item <id>] [--provider <p>] [--model <m>] [--timeout-ms <n>]
//       [--out <report.json>] [--self-test]
//
// 语义：每个 case 在新 Prime 会话（prime-agent -p）里跑一次，然后从会话记录
// （~/.prime/agent/sessions/<id>.jsonl 或 --session-dir 指定目录）判断 Skill 是否
// 真的被加载。判断依据是「工具调用读取了该 Skill 的目录/SKILL.md 路径」，而不是
// 模型在回答里自报「我加载了 Skill」——assistant 的 text/thinking 一律不算证据。
//
// 正例（expect=loaded）：会话记录里出现该 Skill 目录路径 → PASS，否则 FAIL。
// 反例（expect=skipped）：会话记录里不出现该 Skill 目录路径 → PASS，否则 FAIL。
//
// 仅跑 fixtures.json 里 status=active 且 skill 目录存在的条目；draft 条目只校验
// schema（每项 ≥3 正例 + ≥3 反例），不实跑。
import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { mkdtempSync, cpSync, statSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const __dirname = dirname(new URL(import.meta.url).pathname);

function findPrimeAgentDist() {
  if (process.env.PRIME_AGENT_DIST && existsSync(join(process.env.PRIME_AGENT_DIST, "core", "skills.js"))) {
    return process.env.PRIME_AGENT_DIST;
  }
  const candidates = [
    "/usr/local/lib/node_modules/prime-agent/dist",
    join(homedir(), ".local/lib/node_modules/prime-agent/dist"),
    join(homedir(), ".npm-global/lib/node_modules/prime-agent/dist"),
    join(homedir(), "node_modules/prime-agent/dist"),
  ];
  for (const c of candidates) {
    if (existsSync(join(c, "core", "skills.js"))) return c;
  }
  try {
    const pkg = require.resolve("prime-agent/package.json");
    return join(dirname(pkg), "dist");
  } catch {}
  throw new Error("找不到 prime-agent 的 dist 目录；请设置 PRIME_AGENT_DIST");
}

const DIST = findPrimeAgentDist();
const { DefaultResourceLoader } = await import(join(DIST, "core", "resource-loader.js"));

// ---------------------------------------------------------------------------
// 参数
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const flag = (n) => args.includes(n);
const opt = (n, def) => {
  const i = args.indexOf(n);
  return i >= 0 && args[i + 1] ? args[i + 1] : def;
};
const fixturesPath = resolve(opt("--fixtures", join(__dirname, "..", "..", "docs", "agents", "pstack-lite", "fixtures.json")));
const itemFilter = opt("--item", null);
const provider = process.env.PRIME_AGENT_PROVIDER ?? opt("--provider", "deepseek");
const model = process.env.PRIME_AGENT_MODEL ?? opt("--model", "deepseek-v4-pro");
const timeoutMs = Number(opt("--timeout-ms", "300000"));
const outPath = opt("--out", null);
const selfTest = flag("--self-test");
const primeAgentBin = process.env.PRIME_AGENT_BIN ?? "prime-agent";

const fixtures = JSON.parse(readFileSync(fixturesPath, "utf-8"));
const fixturesDir = dirname(fixturesPath);

// ---------------------------------------------------------------------------
// 证据分析：从会话记录判断 Skill 是否加载（不采信模型自报）
// ---------------------------------------------------------------------------
function analyzeSession(sessionFile, skillDir) {
  const norm = resolve(skillDir);
  const hits = [];
  let loaded = false;
  let parsed = 0;
  let content = "";
  try {
    content = readFileSync(sessionFile, "utf-8");
  } catch {
    return { loaded: false, hits: [], error: "session file unreadable: " + sessionFile };
  }
  for (const line of content.split("\n")) {
    if (!line.trim()) continue;
    let d;
    try {
      d = JSON.parse(line);
    } catch {
      continue;
    }
    if (d.type !== "message") continue;
    parsed++;
    const m = d.message ?? {};
    if (m.role === "assistant") {
      for (const c of m.content ?? []) {
        if (c.type !== "toolCall") continue; // 忽略 text/thinking（模型自报）
        const code = typeof c.arguments?.code === "string" ? c.arguments.code
          : typeof c.arguments?.command === "string" ? c.arguments.command : "";
        const name = typeof c.name === "string" ? c.name : "";
        if (scan(`${code}\n${name}`, line)) continue;
      }
    } else if (m.role === "toolResult") {
      let txt = typeof m.toolName === "string" ? m.toolName : "";
      for (const c of m.content ?? []) {
        if (c.type === "text" && typeof c.text === "string") txt += "\n" + c.text;
      }
      txt += "\n" + (m.details?.stdout ?? "") + "\n" + (m.details?.stderr ?? "");
      scan(txt, line);
    }
  }
  function scan(text, line) {
    if (typeof text === "string" && text.includes(norm)) {
      loaded = true;
      hits.push({ line, snippet: text.slice(Math.max(0, text.indexOf(norm) - 40), text.indexOf(norm) + 80) });
      return true;
    }
    return false;
  }
  return { loaded, hits, parsed };
}

// ---------------------------------------------------------------------------
// schema 校验：每项 ≥3 正例 + ≥3 反例，case 必须有 id/prompt/expect
// ---------------------------------------------------------------------------
function validateFixtures(fx) {
  const errors = [];
  if (!Array.isArray(fx.items) || fx.items.length === 0) errors.push("fixtures.items 必须是非空数组");
  for (const item of fx.items ?? []) {
    const pos = (item.positive ?? []).length;
    const neg = (item.negative ?? []).length;
    if (!item.id) errors.push("item 缺 id");
    if (!item.skill) errors.push(`${item.id}: 缺 skill`);
    if (pos < 3) errors.push(`${item.id}: positive 不足 3 条（当前 ${pos}）`);
    if (neg < 3) errors.push(`${item.id}: negative 不足 3 条（当前 ${neg}）`);
    for (const c of [...(item.positive ?? []), ...(item.negative ?? [])]) {
      if (!c.id || !c.prompt) errors.push(`${item.id}: case 缺 id/prompt`);
      if (c.expect !== "loaded" && c.expect !== "skipped") errors.push(`${item.id}/${c.id}: expect 必须是 loaded|skipped`);
    }
  }
  return errors;
}

// ---------------------------------------------------------------------------
// 会话 runner
// ---------------------------------------------------------------------------
function runCase(fixtureCwd, prompt, caseId) {
  const sessionDir = mkdtempSync(join(tmpdir(), "pstack-trigger-"));
  const r = spawnSync(primeAgentBin, [
    "-p", "--mode", "json",
    "--cwd", fixtureCwd,
    "--provider", provider,
    "--model", model,
    "--session-dir", sessionDir,
    "--",
    prompt,
  ], { encoding: "utf8", timeout: timeoutMs, maxBuffer: 64 * 1024 * 1024 });
  const files = existsSync(sessionDir) ? readdirSync(sessionDir).filter((f) => f.endsWith(".jsonl")) : [];
  const sessionFile = files.length === 1 ? join(sessionDir, files[0]) : null;
  return {
    caseId,
    sessionDir,
    sessionFile,
    status: r.error && r.error.code === "ETIMEDOUT" ? "timeout" : r.status === 0 ? "ok" : `exit-${r.status}`,
    stderr: (r.stderr ?? "").slice(-2000),
    cleanup: () => rmSync(sessionDir, { recursive: true, force: true }),
  };
}

// ---------------------------------------------------------------------------
// 自测：用合成会话记录验证证据分析器（不依赖模型）
// ---------------------------------------------------------------------------
function runSelfTest() {
  const base = mkdtempSync(join(tmpdir(), "pstack-trigger-selftest-"));
  const skillDir = join(base, "proj", ".agents", "skills", "probe-fixture");
  const checks = [];
  try {
    mkdirSync(skillDir, { recursive: true });
    const header = { type: "session", version: 3, id: "s", cwd: join(base, "proj") };
    const userMsg = { type: "message", message: { role: "user", content: [{ type: "text", text: "load probe-fixture" }] } };
    const readCall = {
      type: "message",
      message: {
        role: "assistant",
        content: [{ type: "toolCall", name: "ipython", arguments: { code: `print(Path('${skillDir}/SKILL.md').read_text())` } }],
      },
    };
    const selfReport = {
      type: "message",
      message: {
        role: "assistant",
        content: [{ type: "text", text: "I loaded probe-fixture and got the marker." }],
      },
    };
    const mathCall = {
      type: "message",
      message: { role: "assistant", content: [{ type: "toolCall", name: "ipython", arguments: { code: "print(2+2)" } }] },
    };
    const readResult = {
      type: "message",
      message: { role: "toolResult", toolName: "ipython", content: [{ type: "text", text: "---\nname: probe-fixture\n---\n" }], details: { stdout: skillDir + "/SKILL.md" } },
    };

    const linesOf = (entries) => entries.map((e) => JSON.stringify(e)).join("\n") + "\n";

    // 1) 工具调用读取 SKILL.md → 判定 loaded
    const p1 = join(base, "positive.jsonl");
    writeFileSync(p1, linesOf([header, userMsg, readCall, readResult]));
    const r1 = analyzeSession(p1, skillDir);
    checks.push({ name: "工具调用读 SKILL.md → loaded", ok: r1.loaded === true, detail: `loaded=${r1.loaded}` });

    // 2) 仅模型自报「我加载了」，无工具调用证据 → 判定未加载
    const p2 = join(base, "selfreport.jsonl");
    writeFileSync(p2, linesOf([header, userMsg, selfReport, mathCall]));
    const r2 = analyzeSession(p2, skillDir);
    checks.push({ name: "模型自报不算加载证据 → not loaded", ok: r2.loaded === false, detail: `loaded=${r2.loaded}` });

    // 3) toolResult 的 stdout 含 Skill 路径 → 判定 loaded
    const p3 = join(base, "toolresult.jsonl");
    writeFileSync(p3, linesOf([header, readResult]));
    const r3 = analyzeSession(p3, skillDir);
    checks.push({ name: "toolResult 含 Skill 路径 → loaded", ok: r3.loaded === true, detail: `loaded=${r3.loaded}` });

    const allOk = checks.every((c) => c.ok);
    const report = { mode: "self-test", ok: allOk, checks };
    console.log(JSON.stringify(report, null, 2));
    return allOk ? 0 : 1;
  } finally {
    rmSync(base, { recursive: true, force: true });
  }
}

// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------
if (selfTest) process.exit(runSelfTest());

const schemaErrors = validateFixtures(fixtures);
if (schemaErrors.length > 0) {
  console.error("fixtures schema 校验失败：");
  for (const e of schemaErrors) console.error("  - " + e);
  process.exit(1);
}

const items = itemFilter ? fixtures.items.filter((it) => it.id === itemFilter) : fixtures.items;
if (items.length === 0) {
  console.error(`未找到 item=${itemFilter}`);
  process.exit(1);
}

const report = { mode: "run", ok: true, fixtures: fixturesPath, provider, model, items: [] };
for (const item of items) {
  const itemReport = { id: item.id, skill: item.skill, status: item.status, cases: [], skipped: false, reason: null };
  const skillDir = resolveSkillDir(item);
  const active = item.status === "active" && skillDir && existsSync(skillDir);
  if (!active) {
    itemReport.skipped = true;
    itemReport.reason = skillDir ? `status=${item.status}` : "skill 目录不存在（draft）";
    report.items.push(itemReport);
    continue;
  }

  // 为 item 建临时夹具项目，把 skill 目录复制进 .agents/skills/<skill>/
  const fixtureCwd = mkdtempSync(join(tmpdir(), "pstack-fixture-"));
  const installedSkillDir = join(fixtureCwd, ".agents", "skills", item.skill);
  mkdirSync(installedSkillDir, { recursive: true });
  cpSync(skillDir, installedSkillDir, { recursive: true });

  // 前置：确认该 Skill 在夹具项目里被 Prime 发现且零诊断（registered）
  try {
    const loader = new DefaultResourceLoader({ cwd: fixtureCwd, agentDir: join(homedir(), ".prime", "agent") });
    await loader.reload();
    const reg = loader.getSkills().skills.find((s) => s.name === item.skill);
    itemReport.registered = !!reg;
    itemReport.registrationDiagnostics = loader.getSkills().diagnostics;
    if (!reg) {
      itemReport.skipped = true;
      itemReport.reason = `skill ${item.skill} 在夹具项目里未被 Prime 发现`;
      report.items.push(itemReport);
      rmSync(fixtureCwd, { recursive: true, force: true });
      continue;
    }
  } catch (e) {
    itemReport.skipped = true;
    itemReport.reason = `preflight 失败: ${e.message}`;
    report.items.push(itemReport);
    rmSync(fixtureCwd, { recursive: true, force: true });
    continue;
  }

  const cases = [
    ...item.positive.map((c) => ({ ...c, expect: "loaded" })),
    ...item.negative.map((c) => ({ ...c, expect: "skipped" })),
  ];
  for (const c of cases) {
    const run = runCase(fixtureCwd, c.prompt, c.id);
    let verdict = "ERROR";
    let loaded = null;
    let detail = run.status;
    try {
      if (run.sessionFile) {
        const a = analyzeSession(run.sessionFile, installedSkillDir);
        loaded = a.loaded;
        const expected = c.expect === "loaded";
        verdict = a.loaded === expected ? "PASS" : "FAIL";
        detail = `loaded=${a.loaded} expect=${c.expect}`;
      } else {
        detail = `no session record (status=${run.status}): ${run.stderr.slice(0, 300)}`;
      }
    } finally {
      run.cleanup();
    }
    if (verdict !== "PASS") report.ok = false;
    itemReport.cases.push({ id: c.id, expect: c.expect, loaded, verdict, detail });
  }
  report.items.push(itemReport);
  rmSync(fixtureCwd, { recursive: true, force: true });
}

console.log(JSON.stringify(report, null, 2));
if (outPath) writeFileSync(resolve(outPath), JSON.stringify(report, null, 2), "utf-8");
process.exit(report.ok ? 0 : 1);

function resolveSkillDir(item) {
  if (item.skill_dir) return resolve(fixturesDir, item.skill_dir);
  if (item.scope === "user") return join(homedir(), ".agents", "skills", item.skill);
  return resolve(fixturesDir, "fixtures", item.skill);
}
