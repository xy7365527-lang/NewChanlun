import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test, type TestContext } from "node:test";
import { preflightDelivery, type DeliveryManifest, type ProductChange } from "./delivery-bundle.ts";

const hash = (value: string | Buffer) => createHash("sha256").update(value).digest("hex");
const reportRoot = ".chanlun/review-results/issue1456";

function fixture(t: TestContext, docsOnly = false) {
  const directory = mkdtempSync(join(tmpdir(), "delivery-bundle-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const repo = join(directory, "repo");
  mkdirSync(repo);
  const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  const write = (path: string, value: string) => {
    mkdirSync(dirname(join(repo, path)), { recursive: true });
    writeFileSync(join(repo, path), value);
  };
  const commit = () => { git("add", "."); git("commit", "-qm", "fixture"); return git("rev-parse", "HEAD"); };
  git("init", "-q");
  git("config", "user.email", "fixture@example.invalid");
  git("config", "user.name", "交付预检测试");
  git("config", "core.filemode", "true");
  git("config", "commit.gpgsign", "false");
  const before = "module.exports = 1;\n";
  const after = "module.exports = 2;\n";
  write("app.js", before);
  write("unchanged.js", "module.exports = 'unchanged';\n");
  const base = commit();
  write(docsOnly ? "README.md" : "app.js", docsOnly ? "# 更新使用说明\n" : after);
  // 至少执行一次真实语法检查；预检本身不代跑被声明的检查。
  const compileOutput = execFileSync(process.execPath, ["--check", join(repo, "app.js")], { encoding: "utf8" });
  write(`${reportRoot}/compile.md`, `已执行 node --check app.js，exit=0\n${compileOutput}`);
  write(`${reportRoot}/review.md`, "审查已检查 app.js 的变更与调用方。\n");
  write(`${reportRoot}/scan.md`, "扫描运行原始结果，固定测试材料。\n");
  write(`${reportRoot}/triage.md`, "扫描所报值是固定测试校验和，无未修复产品缺陷；例外尚未批准。\n");
  const head = commit();
  const productDiff: ProductChange[] = docsOnly ? [] : [{
    path: "app.js", beforeSha256: hash(before), afterSha256: hash(after), beforeMode: "100644", afterMode: "100644",
  }];
  const artifact = (path: string) => ({ path, sha256: hash(readFileSync(join(repo, path))) });
  const reports = ["compile", "review", "scan", "triage"].map(name => artifact(`${reportRoot}/${name}.md`));
  const manifest: DeliveryManifest = {
    schemaVersion: 1, ticket: 1456, base, head, productDiff,
    requiredReports: reports.map(item => item.path), reports,
    reviews: [{ report: reports[1], covers: structuredClone(productDiff) }],
    requiredChecks: ["syntax"],
    checks: [{
      id: "syntax", kind: "compile", status: "passed", evidence: reports[0], covers: structuredClone(productDiff),
      command: "node --check app.js", executedAt: new Date().toISOString(), exitCode: 0,
    }],
    scan: { status: "passed", scannedHead: head, evidence: reports[2], executedAt: new Date().toISOString(), exitCode: 0 },
  };
  const disclose = () => {
    manifest.scan.disclosure = { reason: "已分诊固定测试值误报，请单独批准当前扫描状态", triage: reports[3], confirmedUnfixedProductFindings: 0, tailChanges: [] };
  };
  return { repo, directory, git, write, commit, manifest, artifact, disclose };
}

test("完整交付只得到可请示状态，预检不写 Git/工作树也不产生合入授权", t => {
  const f = fixture(t);
  const before = f.git("status", "--porcelain");
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "READY_FOR_APPROVAL", JSON.stringify(result));
  assert.equal(result.mergeAuthorized, false);
  assert.equal(result.requiresMergeApproval, true);
  assert.equal(f.git("rev-parse", "HEAD"), f.manifest.head);
  assert.equal(f.git("status", "--porcelain"), before);
});

test("漏列必需实体报告，与声明了但未入仓的文件，分别报出缺口", t => {
  const f = fixture(t);
  f.manifest.requiredReports.push(`${reportRoot}/missing-review.md`);
  f.write(`${reportRoot}/untracked.md`, "未入仓的报告");
  f.manifest.reports.push(f.artifact(`${reportRoot}/untracked.md`));
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.diagnostics.some(item => item.code === "MISSING_REPORT_DECLARATION"));
  assert.ok(result.diagnostics.some(item => item.code === "REPORT_NOT_IN_HEAD"));
  assert.ok(result.diagnostics.some(item => item.code === "DIRTY_WORKTREE"));
});

test("绝对路径、父目录路径和已跟踪符号链接不算报告入仓", t => {
  const f = fixture(t);
  const outside = join(f.directory, "outside.md");
  writeFileSync(outside, "外部报告");
  const linked = `${reportRoot}/linked.md`;
  symlinkSync(outside, join(f.repo, linked));
  f.manifest.head = f.commit();
  f.manifest.reports.push({ path: outside, sha256: hash("外部报告") }, { path: "../outside.md", sha256: hash("外部报告") }, { path: linked, sha256: hash("外部报告") });
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.equal(result.diagnostics.filter(item => item.code === "INVALID_REPORT_PATH").length, 2);
  assert.ok(result.diagnostics.some(item => item.code === "REPORT_NOT_REGULAR_FILE"));
});

test("已跟踪报告的错误 SHA-256 不能通过", t => {
  const f = fixture(t);
  f.manifest.reports[0].sha256 = "0".repeat(64);
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "REPORT_HASH_MISMATCH"));
});

test("尚未提交的代码改动和 manifest 指向旧 HEAD 均被阻断", t => {
  const f = fixture(t);
  f.write("app.js", "module.exports = 3;\n");
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "DIRTY_WORKTREE"));
  f.commit();
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "HEAD_DRIFT"));
});

test("审查后的代码尾部提交同时使审查和已跑检查过期", t => {
  const f = fixture(t);
  f.write("app.js", "module.exports = 3;\n");
  f.manifest.head = f.commit();
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  for (const code of ["PRODUCT_DIFF_MISMATCH", "REVIEW_COVERAGE_MISSING", "CHECK_SOURCE_DRIFT", "SCAN_PRODUCT_DRIFT"]) {
    assert.ok(result.diagnostics.some(item => item.code === code), code);
  }
});

test("Git assume-unchanged 不得隐藏产品工作树内容漂移", t => {
  const f = fixture(t);
  f.git("update-index", "--assume-unchanged", "app.js");
  f.write("app.js", "module.exports = 999;\n");
  assert.equal(f.git("status", "--porcelain"), "");
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.diagnostics.some(item => item.code === "PRODUCT_WORKTREE_DRIFT"));
});

test("不在交付 diff 内的隐藏文件也不能凭干净 status 放行", t => {
  const f = fixture(t);
  f.git("update-index", "--skip-worktree", "unchanged.js");
  f.write("unchanged.js", "module.exports = 'hidden mutation';\n");
  assert.equal(f.git("status", "--porcelain"), "");
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.diagnostics.some(item => item.code === "INDEX_FLAGS_HIDE_DIRTY"));
});

test("core.filemode=false 不能隐藏 diff 外文件的可执行位漂移", t => {
  const f = fixture(t);
  f.git("config", "core.filemode", "false");
  chmodSync(join(f.repo, "unchanged.js"), 0o755);
  assert.equal(f.git("status", "--porcelain"), "");
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.diagnostics.some(item => item.code === "DIRTY_WORKTREE"));
});

test("diff.ignoreSubmodules=all 不能把 gitlink 变更藏出交付差异", t => {
  const f = fixture(t);
  mkdirSync(join(f.repo, "vendor"));
  f.git("update-index", "--add", "--cacheinfo", "160000", f.manifest.base, "vendor");
  f.git("commit", "-qm", "add gitlink fixture");
  f.manifest.head = f.git("rev-parse", "HEAD");
  f.git("config", "diff.ignoreSubmodules", "all");
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.diagnostics.some(item => item.code === "UNSUPPORTED_PRODUCT_FILE" && item.path === "vendor"));
});

test("本地 replace refs 不能替换最终 commit 的实际树来隐藏源码", t => {
  const f = fixture(t);
  const reviewedHead = f.manifest.head;
  f.write("hidden.js", "module.exports = 'unreviewed';\n");
  f.manifest.head = f.commit();
  f.git("replace", f.manifest.head, reviewedHead);
  f.git("read-tree", "--reset", "-u", reviewedHead);
  assert.equal(f.git("status", "--porcelain"), "");
  f.manifest.scan.scannedHead = f.manifest.head;
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.productDiff.some(item => item.path === "hidden.js"));
  assert.ok(result.diagnostics.some(item => item.code === "REVIEW_COVERAGE_MISSING" && item.path === "hidden.js"));
});

test("新增构建配置即使未写入产品清单也须纳入审查", t => {
  const f = fixture(t);
  f.write("build-config.json", '{"target":"production"}\n');
  f.manifest.head = f.commit();
  const result = preflightDelivery(f.repo, f.manifest);
  assert.ok(result.productDiff.some(item => item.path === "build-config.json"));
  assert.ok(result.diagnostics.some(item => item.code === "REVIEW_COVERAGE_MISSING" && item.path === "build-config.json"));
});

test("已入仓但未列出的报告也会被发现", t => {
  const f = fixture(t);
  f.write(`${reportRoot}/omitted.md`, "必须交付的实体报告\n");
  f.manifest.head = f.commit();
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "UNDECLARED_CHANGED_REPORT"));
});

test("存在报告不等于审查已覆盖最终代码", t => {
  const f = fixture(t);
  f.manifest.reviews[0].covers = [];
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "REVIEW_COVERAGE_MISSING"));
});

test("原审查与尾部审查可按连续字节连接，断裂链不能覆盖最终代码", t => {
  const f = fixture(t);
  const finalSource = "module.exports = 3;\n";
  const original = structuredClone(f.manifest.productDiff[0]);
  f.write("app.js", finalSource);
  execFileSync(process.execPath, ["--check", join(f.repo, "app.js")]);
  f.write(`${reportRoot}/tail-review.md`, "仅复核值由 2 改成 3 的尾部差异。\n");
  const tailReport = f.artifact(`${reportRoot}/tail-review.md`);
  f.manifest.reports.push(tailReport);
  f.manifest.head = f.commit();
  f.manifest.productDiff[0].afterSha256 = hash(finalSource);
  f.manifest.checks[0].covers = structuredClone(f.manifest.productDiff);
  f.manifest.scan.scannedHead = f.manifest.head;
  const tail = { ...original, beforeSha256: original.afterSha256, afterSha256: hash(finalSource) };
  f.manifest.reviews.push({ report: tailReport, covers: [tail] });
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "READY_FOR_APPROVAL", JSON.stringify(result));
  tail.beforeSha256 = hash("并不存在于原审查终点的代码");
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "REVIEW_COVERAGE_MISSING"));
});

test("必需检查的漏项、未跑、失败、伪通过均给出独立诊断", t => {
  const f = fixture(t);
  f.manifest.requiredChecks.push("missing");
  f.manifest.checks.push({ id: "not-run", kind: "test", status: "not_run", covers: [] });
  f.manifest.checks.push({ ...f.manifest.checks[0], id: "failed", status: "failed", exitCode: 1 });
  f.manifest.checks[0].executedAt = undefined;
  const result = preflightDelivery(f.repo, f.manifest);
  for (const code of ["REQUIRED_CHECK_MISSING", "CHECK_NOT_RUN", "CHECK_FAILED", "CHECK_EXECUTION_EVIDENCE_MISSING"]) {
    assert.ok(result.diagnostics.some(item => item.code === code), code);
  }
});

test("产品变更不能删除编译必检项来获得就绪状态", t => {
  const f = fixture(t);
  f.manifest.requiredChecks = [];
  f.manifest.checks = [];
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "COMPILE_CHECK_REQUIRED"));
});

test("编译豁免由实际差异判断，产品差异不能伪称纯文档", t => {
  const f = fixture(t);
  f.manifest.checks[0].status = "exempt";
  f.manifest.checks[0].reason = "纯文档";
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "INVALID_CHECK_EXEMPTION"));
  const docs = fixture(t, true);
  docs.manifest.checks[0].status = "exempt";
  docs.manifest.checks[0].reason = "实际只改 README 和实体报告";
  assert.equal(preflightDelivery(docs.repo, docs.manifest).status, "READY_FOR_APPROVAL");
});

test("可执行位变化不能借文档后缀绕过代码差异检查", t => {
  const f = fixture(t, true);
  chmodSync(join(f.repo, "README.md"), 0o755);
  f.manifest.head = f.commit();
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.productDiff.some(item => item.path === "README.md" && item.afterMode === "100755"));
});

test("构建 txt 与可执行 MDX 不能凭后缀获得文档编译豁免", t => {
  const f = fixture(t, true);
  f.write("requirements.txt", "runtime-package==1.0\n");
  f.write("CMakeLists.txt", "add_executable(app main.c)\n");
  f.write("components/page.mdx", "export const value = process.env.SECRET;\n");
  f.manifest.head = f.commit();
  f.manifest.checks[0].status = "exempt";
  f.manifest.checks[0].reason = "文件有文档后缀";
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "BLOCKED");
  assert.deepEqual(result.productDiff.map(item => item.path).sort(), ["CMakeLists.txt", "components/page.mdx", "requirements.txt"]);
  assert.ok(result.diagnostics.some(item => item.code === "INVALID_CHECK_EXEMPTION"));
});

test("红扫描只有实体分诊和显式披露齐全才得到待批准例外", t => {
  const f = fixture(t);
  f.manifest.scan.status = "failed";
  f.manifest.scan.exitCode = 1;
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "SCAN_EXCEPTION_NOT_DISCLOSED"));
  f.disclose();
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "READY_WITH_EXPLICIT_EXCEPTION", JSON.stringify(result));
  assert.equal(result.mergeAuthorized, false);
  assert.match(result.exceptions[0].message, /例外尚未批准/);
});

test("未执行扫描或已确认产品问题不能借扫描误报披露放行", t => {
  const f = fixture(t);
  f.disclose();
  f.manifest.scan.status = "not_run";
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "SCAN_NOT_RUN"));
  f.manifest.scan.status = "failed";
  f.manifest.scan.exitCode = 1;
  f.manifest.scan.disclosure!.confirmedUnfixedProductFindings = 1;
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "UNFIXED_PRODUCT_FINDINGS"));
});

test("纯报告尾部保留代码检查，但准确披露最终 head 尚未扫描", t => {
  const f = fixture(t);
  f.write(`${reportRoot}/final.md`, "汇总现有证据，没有新增产品事实。\n");
  f.manifest.reports.push(f.artifact(`${reportRoot}/final.md`));
  f.manifest.head = f.commit();
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "SCAN_EXCEPTION_NOT_DISCLOSED"));
  f.disclose();
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "SCAN_TAIL_UNASSESSED"));
  const finalReport = `${reportRoot}/final.md`;
  f.manifest.scan.disclosure!.tailChanges.push({
    path: finalReport, beforeSha256: null, afterSha256: f.artifact(finalReport).sha256, beforeMode: null, afterMode: "100644",
  });
  const result = preflightDelivery(f.repo, f.manifest);
  assert.equal(result.status, "READY_WITH_EXPLICIT_EXCEPTION", JSON.stringify(result));
  assert.equal(result.diagnostics.length, 0);
});

test("合入机械收据无需新生报告，新发现必须转回修复", t => {
  const f = fixture(t);
  f.manifest.postMergeReceipt = { newFindings: [] };
  assert.equal(preflightDelivery(f.repo, f.manifest).status, "READY_FOR_APPROVAL");
  f.manifest.postMergeReceipt.newFindings.push("发现未提交的实体原始报告");
  assert.ok(preflightDelivery(f.repo, f.manifest).diagnostics.some(item => item.code === "POST_MERGE_FINDINGS_REQUIRE_REPAIR"));
});

test("删除的产品文件也按前像被审查，不能以最终文件不存在跳过", t => {
  const f = fixture(t);
  rmSync(join(f.repo, "app.js"));
  f.manifest.head = f.commit();
  const result = preflightDelivery(f.repo, f.manifest);
  assert.ok(result.productDiff.some(item => item.path === "app.js" && item.afterSha256 === null));
  assert.ok(result.diagnostics.some(item => item.code === "REVIEW_COVERAGE_MISSING"));
});

test("manifest 多出的自动批准字段被拒绝，不执行任何附带操作", t => {
  const f = fixture(t);
  const result = preflightDelivery(f.repo, { ...f.manifest, mergeAuthorized: true });
  assert.equal(result.status, "BLOCKED");
  assert.ok(result.diagnostics.some(item => item.code === "INVALID_MANIFEST"));
  assert.equal(f.git("rev-parse", "HEAD"), f.manifest.head);
});
