#!/usr/bin/env node
// pstack-lite：Prime 递归发现 + 诊断分类检查（#1130 验收②⑥）
//
// 用法：
//   node scripts/pstack-lite/check-skills.mjs [--cwd <dir>] [--agent-dir <dir>]
//       [--json] [--expect-unslop] [--self-test]
//
// 默认：用 Prime 自己的 DefaultResourceLoader 做一次完整 reload（与 Prime 启动时同一
// 条发现路径），报告加载到的 Skill 与全部诊断，并把诊断分成两类：
//   - expectedProjectionCollisions：用户级 symlink 投影到项目级同名 Skill 的 collision
//     （本机既定布局 `~/.agents/skills/<name> -> <cwd>/.agents/skills/<name>`，两条路径
//     指向同一文件），不计为 pstack-lite 新错误；
//   - unexpectedDiagnostics：其余所有诊断（真实 shadow / warning / error / 跨文件
//     collision），一律失败。
// unslop 必须只有用户级副本：任何项目级 unslop 或任何 unslop collision 都失败；
// pstack-* 任一 collision 也一律失败（两者都不享受投影例外）。
// --expect-unslop：额外要求用户级全局 unslop 存在；缺失则失败。
// --self-test：不碰真实 Skill 目录，用临时夹具验证 (a) Prime 的 warning / 真实同名
//   shadow（collision）检测与项目级优先语义，(b) symlink 投影分类器把「用户级投影到
//   项目级」判为 expected、把真实 shadow 判为 unexpected、把 unslop/pstack-* 投影判为
//   unexpected。
//
// 本脚本不依赖模型、不运行全仓重放，只做 Skill 发现与诊断检查。
import { existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { homedir, tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";
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

// ---------------------------------------------------------------------------
// 诊断分类：区分「既定 symlink 投影 collision」与「真正需要失败的诊断」
// ---------------------------------------------------------------------------
function isUnderPath(p, root) {
  const normalizedRoot = resolve(root);
  if (p === normalizedRoot) return true;
  const prefix = normalizedRoot.endsWith(sep) ? normalizedRoot : `${normalizedRoot}${sep}`;
  return p.startsWith(prefix);
}

// 用户级 Skill 发现根目录（与 Prime DefaultResourceLoader 的发现路径一致）。
function userSkillRoots(agentDir) {
  return [join(agentDir, "skills"), join(homedir(), ".agents", "skills")];
}

// 项目级 Skill 发现根目录：<cwd>/.prime/agent/skills 与自 cwd 上溯到 git 根的每个
// <dir>/.agents/skills（对应 Prime 的 collectAncestorAgentsSkillDirs，剔除用户级
// ~/.agents/skills）。
function projectSkillRoots(cwd) {
  const roots = [join(cwd, ".prime", "agent", "skills")];
  const userAgentsSkillsDir = resolve(join(homedir(), ".agents", "skills"));
  let dir = resolve(cwd);
  while (true) {
    const skillDir = resolve(join(dir, ".agents", "skills"));
    if (skillDir !== userAgentsSkillsDir) roots.push(skillDir);
    if (existsSync(join(dir, ".git"))) break;
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return roots;
}

function scopeOfPath(p, userRoots, projectRoots) {
  const resolved = resolve(p);
  for (const root of userRoots) {
    if (isUnderPath(resolved, root)) return "user";
  }
  for (const root of projectRoots) {
    if (isUnderPath(resolved, root)) return "project";
  }
  return "unknown";
}

function safeRealpath(p) {
  try {
    return realpathSync(p);
  } catch {
    return null;
  }
}

// 一条 collision 是否属于「用户级 symlink 投影到项目级同名 Skill」。
// 判据：winner/loser 的 canonical 路径相同（同一文件，经 symlink 以两条路径到达），
// 且一条属用户级、一条属项目级。unslop 与 pstack-* 永不享受投影例外。
function isExpectedProjectionCollision(d, userRoots, projectRoots) {
  if (!d || d.type !== "collision") return false;
  const c = d.collision;
  if (!c || c.resourceType !== "skill") return false;
  const name = c.name;
  if (name === "unslop" || name.startsWith("pstack-")) return false;
  const winnerReal = safeRealpath(c.winnerPath);
  const loserReal = safeRealpath(c.loserPath);
  if (!winnerReal || !loserReal || winnerReal !== loserReal) return false;
  const wScope = scopeOfPath(c.winnerPath, userRoots, projectRoots);
  const lScope = scopeOfPath(c.loserPath, userRoots, projectRoots);
  return (wScope === "user" && lScope === "project") || (wScope === "project" && lScope === "user");
}

function classifyDiagnostics(diagnostics, userRoots, projectRoots) {
  const expectedProjectionCollisions = [];
  const unexpectedDiagnostics = [];
  for (const d of diagnostics) {
    if (isExpectedProjectionCollision(d, userRoots, projectRoots)) {
      expectedProjectionCollisions.push(d);
    } else {
      unexpectedDiagnostics.push(d);
    }
  }
  return { expectedProjectionCollisions, unexpectedDiagnostics };
}

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
    console.log(`rawDiagnostics: ${r.diagnostics.raw}`);
    console.log(`expectedProjectionCollisions: ${r.diagnostics.expectedProjectionCollisions.length}`);
    console.log(`unexpectedDiagnostics: ${r.diagnostics.unexpectedDiagnostics.length}`);
    for (const d of r.diagnostics.expectedProjectionCollisions) {
      console.log(`  [expected projection] ${d.collision?.name}${d.path ? " @ " + d.path : ""}`);
    }
    for (const d of r.diagnostics.unexpectedDiagnostics) {
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
    for (const [group, checks] of Object.entries(r.selfTest)) {
      console.log(`self-test[${group}]`);
      for (const t of checks) {
        console.log(`  ${t.ok ? "PASS" : "FAIL"} — ${t.name} (${t.detail})`);
      }
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

  const uRoots = userSkillRoots(agentDir);
  const pRoots = projectSkillRoots(cwd);
  const { expectedProjectionCollisions, unexpectedDiagnostics } = classifyDiagnostics(diagnostics, uRoots, pRoots);

  const report = {
    mode: "check",
    ok: true,
    cwd,
    agentDir,
    skills: { total: skills.length, byScope },
    diagnostics: {
      raw: diagnostics.length,
      expectedProjectionCollisions,
      unexpectedDiagnostics,
    },
    unslop: unslop
      ? { present: true, path: unslop.filePath, scope: unslop.sourceInfo?.scope ?? "?", projectCopy: projectUnslop?.filePath ?? null }
      : { present: false, path: null, scope: null, projectCopy: null },
    selfTest: null,
  };

  if (unexpectedDiagnostics.length > 0) report.ok = false;
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
// 自测 A：真实（非投影）shadow / warning 检测，不碰真实 Skill 目录
// ---------------------------------------------------------------------------
function runShadowSelfTest() {
  const base = mkdtempSync(join(tmpdir(), "pstack-skill-check-"));
  const projectDir = join(base, "project");
  const userDir = join(base, "user");
  const checks = [];
  try {
    // 1) 合法 Skill：应加载且零诊断
    const goodDir = join(projectDir, "goodskill");
    mkdirSync(goodDir, { recursive: true });
    writeFileSync(join(goodDir, "SKILL.md"), "---\nname: goodskill\ndescription: a valid skill\n---\nbody\n");

    // 2) 名字与目录不符：应产生 warning 但仍加载
    const badNameDir = join(projectDir, "badname");
    mkdirSync(badNameDir, { recursive: true });
    writeFileSync(join(badNameDir, "SKILL.md"), "---\nname: other-name\ndescription: name mismatch\n---\nbody\n");

    // 3) 空描述：应产生 warning 且不加载
    const noDescDir = join(projectDir, "nodesc");
    mkdirSync(noDescDir, { recursive: true });
    writeFileSync(join(noDescDir, "SKILL.md"), "---\nname: nodesc\ndescription: \"\"\n---\nbody\n");

    // 4) 同名 shadow：项目级优先于用户级，且产生 collision 诊断（两条真实不同文件）
    const pShadowDir = join(projectDir, "unslop");
    mkdirSync(pShadowDir, { recursive: true });
    writeFileSync(join(pShadowDir, "SKILL.md"), "---\nname: unslop\ndescription: project copy\n---\nproject\n");
    const uShadowDir = join(userDir, "unslop");
    mkdirSync(uShadowDir, { recursive: true });
    writeFileSync(join(uShadowDir, "SKILL.md"), "---\nname: unslop\ndescription: user copy\n---\nuser\n");

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

    // 这些 warning + 真实 shadow 都不享受投影例外，全部应判 unexpected。
    const cls = classifyDiagnostics(diags, [userDir], [projectDir]);
    checks.push({
      name: "真实 shadow + warning 全部判 unexpected（不享受投影例外）",
      ok: cls.expectedProjectionCollisions.length === 0 && cls.unexpectedDiagnostics.length === diags.length && diags.length > 0,
      detail: `expected=${cls.expectedProjectionCollisions.length} unexpected=${cls.unexpectedDiagnostics.length} raw=${diags.length}`,
    });

    return checks;
  } finally {
    rmSync(base, { recursive: true, force: true });
  }
}

// ---------------------------------------------------------------------------
// 自测 B：合成 symlink 投影分类（不依赖模型、不碰真实 Skill 目录）
// ---------------------------------------------------------------------------
function runProjectionSelfTest() {
  const base = mkdtempSync(join(tmpdir(), "pstack-skill-projection-"));
  const projectDir = join(base, "project");
  const userDir = join(base, "user");
  const checks = [];
  const uRoots = [userDir];
  const pRoots = [projectDir];
  try {
    // 1) 项目级真实 Skill + 用户级 symlink 投影 -> 同一文件：应判 expected。
    const projFoo = join(projectDir, "foo");
    mkdirSync(projFoo, { recursive: true });
    writeFileSync(join(projFoo, "SKILL.md"), "---\nname: foo\ndescription: project foo\n---\nproject\n");
    mkdirSync(userDir, { recursive: true });
    symlinkSync(projFoo, join(userDir, "foo"), "dir");
    const projCollision = {
      type: "collision",
      message: 'name "foo" collision',
      path: join(userDir, "foo", "SKILL.md"),
      collision: {
        resourceType: "skill",
        name: "foo",
        winnerPath: join(projFoo, "SKILL.md"),
        loserPath: join(userDir, "foo", "SKILL.md"),
      },
    };
    const rProj = classifyDiagnostics([projCollision], uRoots, pRoots);
    checks.push({
      name: "用户级 symlink 投影 -> expected（不计为新错误）",
      ok: rProj.expectedProjectionCollisions.length === 1 && rProj.unexpectedDiagnostics.length === 0,
      detail: `expected=${rProj.expectedProjectionCollisions.length} unexpected=${rProj.unexpectedDiagnostics.length}`,
    });

    // 2) 真实非投影 shadow（两条不同文件）：必须判 unexpected。
    const projBar = join(projectDir, "bar");
    mkdirSync(projBar, { recursive: true });
    writeFileSync(join(projBar, "SKILL.md"), "---\nname: bar\ndescription: project bar\n---\nproject\n");
    const userBar = join(userDir, "bar");
    mkdirSync(userBar, { recursive: true });
    writeFileSync(join(userBar, "SKILL.md"), "---\nname: bar\ndescription: user bar\n---\nuser\n");
    const realShadow = {
      type: "collision",
      message: 'name "bar" collision',
      path: join(userBar, "SKILL.md"),
      collision: {
        resourceType: "skill",
        name: "bar",
        winnerPath: join(projBar, "SKILL.md"),
        loserPath: join(userBar, "SKILL.md"),
      },
    };
    const rShadow = classifyDiagnostics([realShadow], uRoots, pRoots);
    checks.push({
      name: "真实非投影 shadow -> unexpected",
      ok: rShadow.expectedProjectionCollisions.length === 0 && rShadow.unexpectedDiagnostics.length === 1,
      detail: `expected=${rShadow.expectedProjectionCollisions.length} unexpected=${rShadow.unexpectedDiagnostics.length}`,
    });

    // 3) 投影形态的项目级 unslop：不得被投影例外吞掉，仍判 unexpected。
    const projUnslop = join(projectDir, "unslop");
    mkdirSync(projUnslop, { recursive: true });
    writeFileSync(join(projUnslop, "SKILL.md"), "---\nname: unslop\ndescription: project unslop\n---\nproject\n");
    symlinkSync(projUnslop, join(userDir, "unslop"), "dir");
    const unslopCollision = {
      type: "collision",
      message: 'name "unslop" collision',
      path: join(userDir, "unslop", "SKILL.md"),
      collision: {
        resourceType: "skill",
        name: "unslop",
        winnerPath: join(projUnslop, "SKILL.md"),
        loserPath: join(userDir, "unslop", "SKILL.md"),
      },
    };
    const rUnslop = classifyDiagnostics([unslopCollision], uRoots, pRoots);
    checks.push({
      name: "投影形态的项目级 unslop 仍 -> unexpected",
      ok: rUnslop.expectedProjectionCollisions.length === 0 && rUnslop.unexpectedDiagnostics.length === 1,
      detail: `expected=${rUnslop.expectedProjectionCollisions.length} unexpected=${rUnslop.unexpectedDiagnostics.length}`,
    });

    // 4) 投影形态的 pstack-*：不得被投影例外吞掉，仍判 unexpected。
    const projPstack = join(projectDir, "pstack-how");
    mkdirSync(projPstack, { recursive: true });
    writeFileSync(join(projPstack, "SKILL.md"), "---\nname: pstack-how\ndescription: project pstack-how\n---\nproject\n");
    symlinkSync(projPstack, join(userDir, "pstack-how"), "dir");
    const pstackCollision = {
      type: "collision",
      message: 'name "pstack-how" collision',
      path: join(userDir, "pstack-how", "SKILL.md"),
      collision: {
        resourceType: "skill",
        name: "pstack-how",
        winnerPath: join(projPstack, "SKILL.md"),
        loserPath: join(userDir, "pstack-how", "SKILL.md"),
      },
    };
    const rPstack = classifyDiagnostics([pstackCollision], uRoots, pRoots);
    checks.push({
      name: "投影形态的 pstack-* collision 仍 -> unexpected",
      ok: rPstack.expectedProjectionCollisions.length === 0 && rPstack.unexpectedDiagnostics.length === 1,
      detail: `expected=${rPstack.expectedProjectionCollisions.length} unexpected=${rPstack.unexpectedDiagnostics.length}`,
    });

    return checks;
  } finally {
    rmSync(base, { recursive: true, force: true });
  }
}

function runSelfTest() {
  const shadow = runShadowSelfTest();
  const projection = runProjectionSelfTest();
  const allOk = [...shadow, ...projection].every((c) => c.ok);
  const report = {
    mode: "self-test",
    ok: allOk,
    cwd,
    agentDir,
    skills: null,
    diagnostics: null,
    unslop: null,
    selfTest: { shadow, projection },
  };
  return out(report);
}

// ---------------------------------------------------------------------------
// 入口
// ---------------------------------------------------------------------------
const code = selfTest ? runSelfTest() : await runRealCheck();
process.exit(code);
