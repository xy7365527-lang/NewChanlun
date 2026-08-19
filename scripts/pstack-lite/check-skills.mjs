#!/usr/bin/env node
// pstack-lite：Prime 递归发现 + 零 warning 检查（#1130 验收②⑥）
//
// 用法：
//   node scripts/pstack-lite/check-skills.mjs [--cwd <dir>] [--agent-dir <dir>]
//       [--json] [--expect-unslop] [--self-test]
//
// 默认：用 Prime 自己的 DefaultResourceLoader 做一次完整 reload（与 Prime 启动时同一
// 条发现路径），报告加载到的 Skill 与全部诊断（warning / collision / error）。
// 诊断为空即「零 warning」通过；存在诊断即失败（exit 1）。
// --expect-unslop：额外要求用户级全局 unslop 存在；缺失则失败。
// --self-test：不碰真实 Skill 目录，用临时夹具验证 Prime 的 warning 检测与同名
//   shadow（collision）检测确实生效，并断言本项目级优先于用户级的遮蔽语义。
//
// 本脚本不依赖模型、不运行全仓重放，只做 Skill 发现与诊断检查。
import { existsSync } from "node:fs";
import { mkdtempSync, rmSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);

// ---------------------------------------------------------------------------
// 定位 prime-agent 的 dist 目录（安装路径因机器而异，做多候选回退）
// ---------------------------------------------------------------------------
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
const { loadSkills } = await import(join(DIST, "core", "skills.js"));

// ---------------------------------------------------------------------------
// 参数解析
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const flag = (name) => args.includes(name);
const opt = (name, def) => {
  const i = args.indexOf(name);
  return i >= 0 && args[i + 1] ? args[i + 1] : def;
};
const json = flag("--json");
const expectUnslop = flag("--expect-unslop");
const selfTest = flag("--self-test");
const cwd = resolve(opt("--cwd", process.cwd()));
const agentDir = resolve(opt("--agent-dir", join(homedir(), ".prime", "agent")));

function out(report) {
  if (json) {
    console.log(JSON.stringify(report, null, 2));
  } else {
    renderText(report);
  }
  return report.ok ? 0 : 1;
}

function renderText(r) {
  console.log(`cwd: ${r.cwd}`);
  console.log(`agentDir: ${r.agentDir}`);
  if (r.skills) {
    console.log(`skills: ${r.skills.total} (project ${r.skills.byScope.project ?? 0}, user ${r.skills.byScope.user ?? 0})`);
  }
  if (r.diagnostics) {
    console.log(`diagnostics: ${r.diagnostics.length}`);
    for (const d of r.diagnostics) {
      console.log(`  [${d.type}] ${d.message}${d.path ? " @ " + d.path : ""}`);
    }
  }
  if (r.unslop) {
    console.log(`unslop: ${r.unslop.present ? "present" : "absent"}${r.unslop.present ? " @ " + r.unslop.path + " (scope=" + r.unslop.scope + ")" : ""}`);
    if (r.unslop.projectCopy) {
      console.log(`  !! 项目级同名 unslop 存在 @ ${r.unslop.projectCopy} —— 会遮蔽用户级全局副本`);
    }
  }
  if (r.selfTest) {
    for (const t of r.selfTest) {
      console.log(`  self-test[${t.name}]: ${t.ok ? "PASS" : "FAIL"} — ${t.detail}`);
    }
  }
}

// ---------------------------------------------------------------------------
// 默认：真实发现检查（与 Prime 同一条加载路径）
// ---------------------------------------------------------------------------
async function runRealCheck() {
  const loader = new DefaultResourceLoader({ cwd, agentDir });
  await loader.reload();
  const { skills, diagnostics } = loader.getSkills();

  const byScope = {};
  for (const s of skills) {
    const sc = s.sourceInfo?.scope ?? "?";
    byScope[sc] = (byScope[sc] ?? 0) + 1;
  }

  const unslop = skills.find((s) => s.name === "unslop");
  const projectUnslop = skills.find((s) => s.name === "unslop" && s.sourceInfo?.scope === "project");

  const report = {
    mode: "check",
    ok: true,
    cwd,
    agentDir,
    skills: { total: skills.length, byScope },
    diagnostics,
    unslop: unslop
      ? { present: true, path: unslop.filePath, scope: unslop.sourceInfo?.scope ?? "?", projectCopy: projectUnslop?.filePath ?? null }
      : { present: false, path: null, scope: null, projectCopy: null },
    selfTest: null,
  };

  if (diagnostics.length > 0) report.ok = false;
  if (projectUnslop) report.ok = false; // 同名 shadow：项目级 unslop 遮蔽用户级全局副本
  if (expectUnslop && !unslop) report.ok = false;

  if (!json) {
    if (expectUnslop && !unslop) {
      console.log("--expect-unslop 要求用户级全局 unslop 存在，但未发现（可能仅安装在主机 ~/.agents/skills/，沙盒内不可见）。");
    }
  }
  return out(report);
}

// ---------------------------------------------------------------------------
// 自测：不碰真实 Skill 目录，验证 warning / 空描述 / 同名 shadow 检测
// ---------------------------------------------------------------------------
function runSelfTest() {
  const base = mkdtempSync(join(tmpdir(), "pstack-skill-check-"));
  const projectDir = join(base, "project");
  const userDir = join(base, "user");
  const checks = [];
  try {
    // 1) 合法 Skill：应加载且零诊断
    const goodDir = join(projectDir, "goodskill");
    mkdirp(goodDir);
    writeFile(join(goodDir, "SKILL.md"), "---\nname: goodskill\ndescription: a valid skill\n---\nbody\n");

    // 2) 名字与目录不符：应产生 warning 但仍加载
    const badNameDir = join(projectDir, "badname");
    mkdirp(badNameDir);
    writeFile(join(badNameDir, "SKILL.md"), "---\nname: other-name\ndescription: name mismatch\n---\nbody\n");

    // 3) 空描述：应产生 warning 且不加载
    const noDescDir = join(projectDir, "nodesc");
    mkdirp(noDescDir);
    writeFile(join(noDescDir, "SKILL.md"), "---\nname: nodesc\ndescription: \"\"\n---\nbody\n");

    // 4) 同名 shadow：项目级优先于用户级，且产生 collision 诊断
    const pShadowDir = join(projectDir, "unslop");
    mkdirp(pShadowDir);
    writeFile(join(pShadowDir, "SKILL.md"), "---\nname: unslop\ndescription: project copy\n---\nproject\n");
    const uShadowDir = join(userDir, "unslop");
    mkdirp(uShadowDir);
    writeFile(join(uShadowDir, "SKILL.md"), "---\nname: unslop\ndescription: user copy\n---\nuser\n");

    // 项目级在前、用户级在后，与 Prime 实际解析顺序一致（项目 > 用户）。
    const r = loadSkills({ cwd: base, agentDir, skillPaths: [projectDir, userDir], includeDefaults: false });
    const names = new Set(r.skills.map((s) => s.name));
    const diags = r.diagnostics;

    checks.push({
      name: "合法 Skill 零诊断加载",
      ok: names.has("goodskill"),
      detail: `loaded=${names.has("goodskill")}`,
    });
    checks.push({
      name: "名字与目录不符产生 warning",
      ok: diags.some((d) => d.type === "warning" && /does not match parent directory/.test(d.message)),
      detail: "badname warning",
    });
    checks.push({
      name: "空描述产生 warning 且不加载",
      ok: diags.some((d) => d.type === "warning" && /description is required/.test(d.message)) && !names.has("nodesc"),
      detail: `nodesc loaded=${names.has("nodesc")}`,
    });
    const collision = diags.find((d) => d.type === "collision" && d.collision?.name === "unslop");
    checks.push({
      name: "同名 shadow 产生 collision（项目级胜出）",
      ok: !!collision && collision.collision.winnerPath.includes("/project/") && collision.collision.loserPath.includes("/user/"),
      detail: collision ? `winner=${collision.collision.winnerPath} loser=${collision.collision.loserPath}` : "no collision",
    });
    checks.push({
      name: "shadow 后仅保留项目级副本",
      ok: names.has("unslop") && r.skills.find((s) => s.name === "unslop")?.filePath.includes("/project/"),
      detail: "winner is project copy",
    });

    const allOk = checks.every((c) => c.ok);
    const report = {
      mode: "self-test",
      ok: allOk,
      cwd,
      agentDir,
      skills: null,
      diagnostics: null,
      unslop: null,
      selfTest: checks,
    };
    if (!json) {
      console.log("self-test 临时目录（已清理）：" + base);
    }
    return out(report);
  } finally {
    rmSync(base, { recursive: true, force: true });
  }
}

function mkdirp(dir) {
  const { mkdirSync } = require("node:fs");
  mkdirSync(dir, { recursive: true });
}
function writeFile(p, content) {
  const { writeFileSync } = require("node:fs");
  writeFileSync(p, content, "utf-8");
}

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------
const code = selfTest ? runSelfTest() : await runRealCheck();
process.exit(code);
