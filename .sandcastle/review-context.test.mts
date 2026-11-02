/**
 * #1588 ③ 评审提示构造与体积闸测试。
 * 夹具：临时 git 仓 + 超大 diff 分支（全量 diff 远超上限，复现 #1406 形态）。
 * 跑法：npx tsx --test .sandcastle/review-context.test.mts
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  buildReviewPrompt,
  collectReviewSummary,
  estimateTokens,
  formatIssueJson,
  type GitRunner,
  hostGit,
  prepareReviewPrompt,
  prepareFinalReviewPrompt,
  recordReviewerRunFailure,
  renderTemplate,
  resolveReviewBase,
  REVIEW_PROMPT_MAX_TOKENS,
  REVIEW_PROMPT_TOO_LARGE_EXIT,
  ReviewPromptTooLargeError,
  runGuardedReview,
} from "./review-context.mts";

const TEMPLATE = readFileSync(new URL("./review-prompt.md", import.meta.url), "utf8");
const BRANCH = "sandcastle/issue-9999";
const MARKER = "HUGE_DIFF_BODY_MARKER";

/** 临时仓：main 一个基线文件；分支 3 个提交，各加一个 ~400k 字符的大文件（diff 正文带 MARKER）。 */
function hugeDiffRepo(): string {
  const dir = mkdtempSync(join(tmpdir(), "nc-review-ctx-"));
  const git = (...args: string[]) =>
    execFileSync("git", args, {
      cwd: dir,
      encoding: "utf8",
      env: { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@t", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@t" },
    });
  git("init", "-q", "-b", "main");
  writeFileSync(join(dir, "README.md"), "base\n");
  git("add", ".");
  git("commit", "-q", "-m", "base");
  git("checkout", "-q", "-b", BRANCH);
  for (let c = 1; c <= 3; c++) {
    const line = `${MARKER}_${c} ${"x".repeat(70)}\n`;
    writeFileSync(join(dir, `big_${c}.rs`), line.repeat(Math.ceil(400_000 / line.length)));
    git("add", ".");
    git("commit", "-q", "-m", `feat: #9999——第 ${c} 个大改动 !\`echo injected\``);
  }
  git("checkout", "-q", "main");
  return dir;
}

const fetchIssue = (n: number) => JSON.stringify({ title: `测试票 ${n}`, body: "正文含 {{BRANCH}} 与 !`rm -rf /` 字样" });

test("超大 diff 夹具：提示只含摘要、不含 diff 正文，体积在上限内", () => {
  const dir = hugeDiffRepo();
  const git = hostGit(dir);
  const fullDiff = git(["diff", `main...${BRANCH}`]);
  // 夹具确实复现了旧形态的爆点：整段 diff 远超上限。
  assert.ok(estimateTokens(fullDiff) > REVIEW_PROMPT_MAX_TOKENS * 4, `夹具 diff 仅 ~${estimateTokens(fullDiff)} tokens`);

  const built = buildReviewPrompt({ template: TEMPLATE, branch: BRANCH, target: "main", issue: 9999, git, fetchIssue });
  assert.ok(!built.prompt.includes(MARKER), "提示里出现了 diff 正文");
  assert.ok(built.estimatedTokens <= REVIEW_PROMPT_MAX_TOKENS);
  assert.equal(built.estimatedTokens, estimateTokens(built.prompt));
  assert.ok(built.chars < 20_000, `摘要提示 ${built.chars} 字符，异常偏大`);
  // 摘要三件齐全：文件清单（name-status）、--stat、提交列表；原始 issue 在。
  for (const c of [1, 2, 3]) {
    assert.match(built.prompt, new RegExp(`A\\s+big_${c}\\.rs`));
    assert.match(built.prompt, new RegExp(`big_${c}\\.rs \\| +\\d+ \\+`));
    assert.ok(built.prompt.includes(`第 ${c} 个大改动`));
  }
  assert.match(built.prompt, /3 files changed/);
  assert.ok(built.prompt.includes("测试票 9999"));
  // 按需读 diff 的指引在；占位符全部替换，提交标题 / issue 正文里的 {{..}} 与 !`..` 原样保留（单趟替换、inline 不展开）。
  assert.ok(built.prompt.includes(`git diff main...${BRANCH} -- <路径>`));
  assert.ok(built.prompt.includes("正文含 {{BRANCH}} 与 !`rm -rf /` 字样"));
  assert.ok(built.prompt.includes("!`echo injected`"));
  assert.ok(!/\{\{[A-Z_]+\}\}/.test(built.prompt.replace(fetchIssue(9999), "")), "模板占位符未替换完");
});

test("模板回归锁：review-prompt.md 不再用 `!`git diff ...`` 把整段 diff 展开进提示", () => {
  assert.ok(!/!`git diff(?![^`]*--(stat|name-status|name-only))[^`]*`/.test(TEMPLATE));
  assert.ok(!TEMPLATE.includes("!`"), "模板含 sandcastle shell 展开；评审提示须在宿主侧渲染");
  for (const key of ["DIFF_STAT", "COMMITS", "FILES", "ISSUE", "BRANCH", "TARGET_BRANCH", "BASE", "ISSUE_NUMBER"]) {
    assert.ok(TEMPLATE.includes(`{{${key}}}`), `模板缺 {{${key}}}`);
  }
  // N2：issue 原文用 <issue> 标签包裹，不放 ``` 围栏里。
  assert.ok(TEMPLATE.includes("<issue>\n{{ISSUE}}\n</issue>"));
  assert.ok(!/```\s*\n\{\{ISSUE\}\}/.test(TEMPLATE));
  // N3：读 diff 的命令用钉死的基准 {{BASE}}，不用会落后的本地 {{TARGET_BRANCH}}。
  assert.ok(!/git (diff|log)[^`\n]*\{\{TARGET_BRANCH\}\}\.\./.test(TEMPLATE), "读 diff 命令仍用 {{TARGET_BRANCH}} 作基准");
});

/** 不在 <issue>…</issue> 块内、以 ``` 开头的行数（围栏配平 ⇔ 偶数）。 */
function fenceLinesOutsideIssue(prompt: string): number {
  const start = prompt.indexOf("\n<issue>\n");
  const end = prompt.indexOf("\n</issue>\n");
  assert.ok(start >= 0 && end > start, "<issue> 块缺失或错位");
  const outside = prompt.slice(0, start) + prompt.slice(end + "\n</issue>\n".length);
  return outside.split("\n").filter((l) => l.startsWith("```")).length;
}

test("N2：issue 正文含 ``` 围栏与 </issue> 字样时，提示结构不被破坏", () => {
  const body = [
    "复现：",
    "```bash",
    "cargo test --locked",
    "```",
    "未闭合的围栏：",
    "```",
    "还有一行字面的 </issue> 与 </ISSUE >，以及 {{BRANCH}}",
  ].join("\n");
  const built = buildReviewPrompt({
    template: TEMPLATE,
    branch: BRANCH,
    target: "main",
    issue: 9999,
    git: hostGit(hugeDiffRepo()),
    fetchIssue: () => formatIssueJson(JSON.stringify({ title: "围栏票", body })),
  });
  const p = built.prompt;
  // 标签只开闭各一次（正文里的 </issue> 已改写，不会提前闭合）。
  assert.equal(p.split("\n").filter((l) => l === "<issue>").length, 1);
  assert.equal(p.match(/<\/issue>/gi)?.length, 1);
  assert.ok(p.includes("<\\/issue> 与 <\\/ISSUE >"));
  // 正文原样（围栏、未闭合围栏都在块内），块外围栏配平、后续章节完好。
  const inner = p.slice(p.indexOf("\n<issue>\n") + 9, p.indexOf("\n</issue>\n"));
  assert.ok(inner.startsWith("标题：围栏票\n\n复现：\n```bash\ncargo test --locked\n```"));
  assert.equal(fenceLinesOutsideIssue(p) % 2, 0);
  assert.ok(p.indexOf("\n</issue>\n") < p.indexOf("\n# 读 diff 的方法\n"));
  assert.match(p, /## 文件清单[^\n]*\n\n```\n(A\s+big_\d\.rs\n){3}```\n/);
});

test("formatIssueJson：gh 单行 JSON 还原为可读的标题 + 正文", () => {
  const raw = JSON.stringify({ body: "第一行\n```\ncode\n```", title: "T" });
  assert.ok(!raw.includes("\n"));
  assert.equal(formatIssueJson(raw), "标题：T\n\n第一行\n```\ncode\n```");
});

function mockChannels(opts: { notifyThrows?: boolean } = {}) {
  const logged: Record<string, unknown>[] = [];
  const notified: Array<{ issue: number; body: string }> = [];
  return {
    logged,
    notified,
    channels: {
      logWorker: (e: Record<string, unknown>) => logged.push(e),
      notify: (issue: number, body: string) => {
        if (opts.notifyThrows) throw new Error("gh 不可达");
        notified.push({ issue, body });
      },
    },
  };
}

test("超限：显式失败（专用非零退出码）+ 结构化失败记录 + 通知被调用", () => {
  const dir = hugeDiffRepo();
  const m = mockChannels();
  const opts = { template: TEMPLATE, branch: BRANCH, target: "main", issue: 9999, git: hostGit(dir), fetchIssue, maxTokens: 500 };
  assert.throws(
    () => prepareReviewPrompt(opts, m.channels),
    (e: unknown) => {
      assert.ok(e instanceof ReviewPromptTooLargeError);
      assert.equal(e.exitCode, REVIEW_PROMPT_TOO_LARGE_EXIT);
      assert.notEqual(e.exitCode, 0);
      assert.equal(e.limitTokens, 500);
      assert.ok(e.estimatedTokens > 500);
      return true;
    },
  );
  assert.equal(m.notified.length, 1);
  assert.equal(m.notified[0]!.issue, 9999);
  assert.match(m.notified[0]!.body, /FAIL-loud/);
  assert.ok(m.notified[0]!.body.includes(BRANCH));
  assert.ok(m.notified[0]!.body.includes("SANDCASTLE_REVIEW_ONLY="));
  assert.equal(m.logged.length, 1);
  assert.deepEqual(
    { ...m.logged[0], est_tokens: undefined, chars: undefined },
    {
      ticket: 9999,
      branch: BRANCH,
      phase: "reviewer",
      status: "failed",
      reason: "prompt_too_large",
      est_tokens: undefined,
      limit_tokens: 500,
      chars: undefined,
      notified: true,
    },
  );
});

test("超限且通知失败：仍抛原错，记录里 notified=false", () => {
  const m = mockChannels({ notifyThrows: true });
  const git = hostGit(hugeDiffRepo());
  assert.throws(
    () => prepareReviewPrompt({ template: TEMPLATE, branch: BRANCH, target: "main", issue: 1, git, fetchIssue, maxTokens: 10 }, m.channels),
    ReviewPromptTooLargeError,
  );
  assert.equal(m.logged.length, 1);
  assert.equal(m.logged[0]!.notified, false);
  assert.equal(m.logged[0]!.reason, "prompt_too_large");
});

test("未超限：不通知、不写失败记录", () => {
  const m = mockChannels();
  const built = prepareReviewPrompt(
    { template: TEMPLATE, branch: BRANCH, target: "main", issue: 2, git: hostGit(hugeDiffRepo()), fetchIssue },
    m.channels,
  );
  assert.ok(built.prompt.length > 0);
  assert.equal(m.logged.length, 0);
  assert.equal(m.notified.length, 0);
});

test("运行期失败：落 run_error 记录并通知，票面评论不贴原始错误串", () => {
  const m = mockChannels();
  recordReviewerRunFailure(7, "sandcastle/issue-7", new Error("Prompt is too long /secret/path"), m.channels);
  assert.equal(m.logged.length, 1);
  assert.equal(m.logged[0]!.status, "failed");
  assert.equal(m.logged[0]!.reason, "run_error");
  assert.match(String(m.logged[0]!.error), /Prompt is too long/);
  assert.equal(m.notified.length, 1);
  assert.ok(!m.notified[0]!.body.includes("/secret/path"));
});

test("renderTemplate：缺值即报错；单趟替换不二次展开", () => {
  assert.throws(() => renderTemplate("a {{X}} b {{Y}}", { X: "1" }), /\{\{Y\}\}/);
  assert.equal(renderTemplate("{{X}}-{{Y}}", { X: "{{Y}}", Y: "2" }), "{{Y}}-2");
  assert.equal(renderTemplate("{{X}}", { X: "$& $1" }), "$& $1");
});

test("estimateTokens：ASCII 4 字符一 token，中文按字计（保守）", () => {
  assert.equal(estimateTokens(""), 0);
  assert.equal(estimateTokens("abcd"), 1);
  assert.equal(estimateTokens("abcde"), 2);
  assert.equal(estimateTokens("中文"), 2);
  assert.equal(estimateTokens("abcd中文"), 3);
});

// ── N1：渲染期非超限失败（gh / git / 基准解析）同样落 reviewer/failed + 票面评论 ──────────────

const failingGit: GitRunner = () => {
  throw new Error("fatal: git diff 失败 /secret/host/path");
};

function guardedOpts(over: Partial<Parameters<typeof prepareReviewPrompt>[0]> = {}) {
  return { template: TEMPLATE, branch: BRANCH, target: "main", issue: 9999, git: hostGit(hugeDiffRepo()), fetchIssue, ...over };
}

for (const [label, over] of [
  ["git 失败", { git: failingGit }],
  [
    "gh issue view 失败",
    {
      fetchIssue: () => {
        throw new Error("gh: HTTP 502 /secret/host/path");
      },
    },
  ],
  [
    "diff 基准解析失败",
    {
      resolveBase: () => {
        throw new Error("fetch origin main 失败 /secret/host/path");
      },
    },
  ],
] as const) {
  test(`N1 渲染期${label}：落 prepare_error 记录 + 票面评论（不贴原始错误串），原样上抛，工蜂不启动`, async () => {
    const m = mockChannels();
    let ran = false;
    await assert.rejects(
      runGuardedReview(guardedOpts(over), m.channels, async () => {
        ran = true;
      }),
      /\/secret\/host\/path/,
    );
    assert.equal(ran, false);
    assert.equal(m.logged.length, 1);
    assert.equal(m.logged[0]!.phase, "reviewer");
    assert.equal(m.logged[0]!.status, "failed");
    assert.equal(m.logged[0]!.reason, "prepare_error");
    assert.equal(m.logged[0]!.notified, true);
    assert.match(String(m.logged[0]!.error), /\/secret\/host\/path/);
    assert.equal(m.notified.length, 1);
    assert.equal(m.notified[0]!.issue, 9999);
    assert.match(m.notified[0]!.body, /独立评审未启动/);
    assert.ok(m.notified[0]!.body.includes("reason=prepare_error"));
    assert.ok(m.notified[0]!.body.includes(`SANDCASTLE_REVIEW_ONLY=${BRANCH}:9999`));
    assert.ok(!m.notified[0]!.body.includes("/secret/host/path"));
  });
}

test("N1 渲染期失败且 gh 通知也失败：仍落 jsonl（notified=false），抛渲染期原错", async () => {
  const m = mockChannels({ notifyThrows: true });
  await assert.rejects(runGuardedReview(guardedOpts({ git: failingGit }), m.channels, async () => {}), /git diff 失败/);
  assert.equal(m.logged.length, 1);
  assert.equal(m.logged[0]!.reason, "prepare_error");
  assert.equal(m.logged[0]!.notified, false);
});

test("N1 运行期失败：reason=run_error，只记一次", async () => {
  const m = mockChannels();
  await assert.rejects(
    runGuardedReview(guardedOpts(), m.channels, async () => {
      throw new Error("agent exit 137");
    }),
    /agent exit 137/,
  );
  assert.equal(m.logged.length, 1);
  assert.equal(m.logged[0]!.reason, "run_error");
  assert.match(m.notified[0]!.body, /独立评审工蜂运行失败/);
});

test("N1 超限：只记 prompt_too_large 一条（不再叠一条 prepare_error），抛 ReviewPromptTooLargeError", async () => {
  const m = mockChannels();
  await assert.rejects(runGuardedReview(guardedOpts({ maxTokens: 10 }), m.channels, async () => {}), ReviewPromptTooLargeError);
  assert.deepEqual(
    m.logged.map((e) => e.reason),
    ["prompt_too_large"],
  );
  assert.equal(m.notified.length, 1);
});

test("N1 成功：返回 run 结果，不写失败记录", async () => {
  const m = mockChannels();
  const out = await runGuardedReview(guardedOpts(), m.channels, async (built) => built.base);
  assert.equal(out, "main");
  assert.equal(m.logged.length, 0);
  assert.equal(m.notified.length, 0);
});

// ── N3：diff 基准 = 现 fetch 的 origin/main 提交（宿主 main 落后时不混入别线提交） ──────────

const GIT_ENV = { ...process.env, GIT_AUTHOR_NAME: "t", GIT_AUTHOR_EMAIL: "t@t", GIT_COMMITTER_NAME: "t", GIT_COMMITTER_EMAIL: "t@t" };

function gitIn(dir: string) {
  return (...args: string[]) => execFileSync("git", args, { cwd: dir, encoding: "utf8", env: GIT_ENV, stdio: "pipe" }).trim();
}

function commitFile(dir: string, file: string, msg: string): string {
  const g = gitIn(dir);
  writeFileSync(join(dir, file), `${msg}\n`);
  g("add", file);
  g("commit", "-q", "-m", msg);
  return g("rev-parse", "HEAD");
}

/**
 * origin（裸仓）← host（宿主，main 停在 A）；other 克隆往 origin/main 推了别线提交 B，并从 B 分出 feature。
 * host 只取了 feature 分支，本地 main 与 refs/remotes/origin/main 都还停在 A（= 宿主 main 落后）。
 */
function staleHostFixture() {
  const root = mkdtempSync(join(tmpdir(), "nc-review-base-"));
  const origin = join(root, "origin.git");
  const host = join(root, "host");
  const other = join(root, "other");
  execFileSync("git", ["init", "-q", "--bare", "-b", "main", origin], { stdio: "pipe" });
  execFileSync("git", ["clone", "-q", origin, host], { env: GIT_ENV, stdio: "pipe" });
  gitIn(host)("checkout", "-q", "-b", "main");
  const a = commitFile(host, "base.txt", "base A");
  gitIn(host)("push", "-q", "origin", "main");
  execFileSync("git", ["clone", "-q", origin, other], { env: GIT_ENV, stdio: "pipe" });
  const b = commitFile(other, "other.txt", "别线已合入提交 B");
  gitIn(other)("push", "-q", "origin", "main");
  gitIn(other)("checkout", "-q", "-b", "feature");
  commitFile(other, "feature.txt", "feat: 本票提交");
  gitIn(other)("push", "-q", "origin", "feature");
  gitIn(host)("fetch", "-q", "origin", "+refs/heads/feature:refs/heads/feature");
  return { root, origin, host, other, a, b };
}

test("N3 复现：宿主 main 落后时，旧做法 main...feature 把别线提交混进摘要", () => {
  const f = staleHostFixture();
  assert.equal(gitIn(f.host)("rev-parse", "main"), f.a);
  const stale = collectReviewSummary(hostGit(f.host), "main", "feature");
  assert.match(stale.commits, /别线已合入提交 B/);
  assert.match(stale.files, /other\.txt/);
});

test("N3：resolveReviewBase 取现 fetch 的 origin/main SHA；摘要只含本分支提交；不改宿主检出", () => {
  const f = staleHostFixture();
  const logs: string[] = [];
  const git = hostGit(f.host);
  const built = buildReviewPrompt({
    template: TEMPLATE,
    branch: "feature",
    target: "main",
    resolveBase: () => resolveReviewBase(git, { log: (m) => logs.push(m) }),
    issue: 1,
    git,
    fetchIssue,
  });
  assert.equal(built.base, f.b);
  assert.ok(built.prompt.includes("feat: 本票提交"));
  assert.ok(!built.prompt.includes("别线已合入提交 B"));
  assert.ok(!built.prompt.includes("other.txt"));
  assert.ok(built.prompt.includes(`git diff ${f.b}...feature -- <路径>`));
  assert.match(logs.join("\n"), /落后 origin\/main 1 个提交/);
  // 宿主本地 main 与工作区不动（刷新宿主 main 是 claim-loop refreshHostMain 的职责）。
  assert.equal(gitIn(f.host)("rev-parse", "main"), f.a);
  assert.equal(gitIn(f.host)("status", "--porcelain"), "");
});

test("N3：分支分自旧 main 时，以新 origin/main 为基准同样只含本分支提交", () => {
  const f = staleHostFixture();
  const g = gitIn(f.host);
  g("checkout", "-q", "-b", "old-feature", "main");
  commitFile(f.host, "old.txt", "feat: 旧基线上的本票提交");
  g("checkout", "-q", "main");
  const git = hostGit(f.host);
  const base = resolveReviewBase(git, { log: () => {} });
  const s = collectReviewSummary(git, base, "old-feature");
  assert.equal(s.commits.split("\n").length, 1);
  assert.match(s.commits, /旧基线上的本票提交/);
  assert.ok(!s.files.includes("other.txt"));
});

test("N3：宿主 main 与 origin/main 一致时不告警", () => {
  const f = staleHostFixture();
  gitIn(f.host)("pull", "-q", "--ff-only", "origin", "main");
  const logs: string[] = [];
  assert.equal(resolveReviewBase(hostGit(f.host), { log: (m) => logs.push(m) }), f.b);
  assert.deepEqual(logs, []);
});

test("N3：宿主 main 含 origin/main 没有的提交 → FAIL-loud，不猜基准", () => {
  const f = staleHostFixture();
  commitFile(f.host, "local.txt", "本地未推提交");
  assert.throws(() => resolveReviewBase(hostGit(f.host), { log: () => {} }), /宿主 main 含 1 个 origin\/main 没有的提交/);
});

test("N3：fetch 持续失败 → 指数退避重试后 FAIL-loud（经 runGuardedReview 落 prepare_error）", async () => {
  const f = staleHostFixture();
  gitIn(f.host)("remote", "set-url", "origin", join(f.root, "不存在的远端.git"));
  const sleeps: number[] = [];
  const git = hostGit(f.host);
  const m = mockChannels();
  await assert.rejects(
    runGuardedReview(
      {
        template: TEMPLATE,
        branch: "feature",
        target: "main",
        resolveBase: () => resolveReviewBase(git, { attempts: 3, baseDelayMs: 10, sleep: (ms) => sleeps.push(ms), log: () => {} }),
        issue: 1,
        git,
        fetchIssue,
      },
      m.channels,
      async () => {},
    ),
    /git fetch origin main 连续 3 次失败/,
  );
  assert.deepEqual(sleeps, [10, 20]);
  assert.equal(m.logged.length, 1);
  assert.equal(m.logged[0]!.reason, "prepare_error");
});


test("#1617 最终提示恰好 60000 tokens：完整原文保留、不通知", () => {
  const m = mockChannels();
  const prompt = "x".repeat(REVIEW_PROMPT_MAX_TOKENS * 4);
  const built = prepareFinalReviewPrompt(prompt, "exact-base", { issue: 1617, branch: BRANCH }, m.channels);
  assert.equal(built.estimatedTokens, 60_000);
  assert.equal(built.prompt, prompt);
  assert.equal(built.base, "exact-base");
  assert.equal(m.logged.length, 0);
  assert.equal(m.notified.length, 0);
});

test("#1617 最终诊断超限：只记一次失败，不启动工蜂、不截正文", async () => {
  const m = mockChannels();
  let started = false;
  const basePrompt = "x".repeat(59_900 * 4);
  const finalPrompt = basePrompt + "\n本次评审必须读取精确候选 commit " + "a".repeat(40) + "\n" + "e".repeat(24_000);
  assert.ok(estimateTokens(basePrompt) < REVIEW_PROMPT_MAX_TOKENS);
  assert.ok(estimateTokens(finalPrompt) > REVIEW_PROMPT_MAX_TOKENS);
  await assert.rejects(runGuardedReview(guardedOpts(), m.channels, async () => {
    prepareFinalReviewPrompt(finalPrompt, "base", { issue: 1617, branch: BRANCH }, m.channels);
    started = true;
  }), ReviewPromptTooLargeError);
  assert.equal(started, false);
  assert.deepEqual(m.logged.map(e => e.reason), ["prompt_too_large"]);
  assert.equal(m.logged[0]!.chars, finalPrompt.length);
  assert.equal(m.logged[0]!.est_tokens, estimateTokens(finalPrompt));
  assert.equal(m.notified.length, 1);
});

test("#1617 后续修复轮诊断增大时重新计量，上一轮通过不豁免", () => {
  const m = mockChannels();
  const opts = { issue: 1617, branch: BRANCH };
  const base = "x".repeat(59_900 * 4);
  prepareFinalReviewPrompt(base + "first head", "base", opts, m.channels);
  assert.throws(() => prepareFinalReviewPrompt(base + "next head" + "e".repeat(24_000), "base", opts, m.channels), ReviewPromptTooLargeError);
  assert.equal(m.logged.length, 1);
  assert.equal(m.notified.length, 1);
});
