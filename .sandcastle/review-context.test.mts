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
  estimateTokens,
  hostGit,
  prepareReviewPrompt,
  recordReviewerRunFailure,
  renderTemplate,
  REVIEW_PROMPT_MAX_TOKENS,
  REVIEW_PROMPT_TOO_LARGE_EXIT,
  ReviewPromptTooLargeError,
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
  for (const key of ["DIFF_STAT", "COMMITS", "FILES", "ISSUE", "BRANCH", "TARGET_BRANCH", "ISSUE_NUMBER"]) {
    assert.ok(TEMPLATE.includes(`{{${key}}}`), `模板缺 {{${key}}}`);
  }
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
