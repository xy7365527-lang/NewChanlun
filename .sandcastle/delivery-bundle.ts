import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, lstatSync, readFileSync, realpathSync } from "node:fs";
import { isAbsolute, resolve } from "node:path";
import { z } from "zod";

const digest = z.string().regex(/^[a-f0-9]{64}$/);
const commit = z.string().regex(/^[a-f0-9]{40}$/);
const artifact = z.object({ path: z.string().min(1), sha256: digest }).strict();
const change = z.object({
  path: z.string().min(1),
  beforeSha256: digest.nullable(),
  afterSha256: digest.nullable(),
  beforeMode: z.string().regex(/^[0-7]{6}$/).nullable(),
  afterMode: z.string().regex(/^[0-7]{6}$/).nullable(),
}).strict();

/** #1456：manifest 放在仓外，避免把自身的最终 commit/hash 写回自身。 */
export const deliveryManifestSchema = z.object({
  schemaVersion: z.literal(1),
  ticket: z.number().int().positive(),
  base: commit,
  head: commit,
  productDiff: z.array(change),
  requiredReports: z.array(z.string().min(1)),
  reports: z.array(artifact),
  noReportsReason: z.string().min(1).optional(),
  reviews: z.array(z.object({ report: artifact, covers: z.array(change) }).strict()),
  requiredChecks: z.array(z.string().min(1)),
  checks: z.array(z.object({
    id: z.string().min(1),
    kind: z.enum(["compile", "test", "other"]),
    status: z.enum(["passed", "failed", "not_run", "exempt"]),
    evidence: artifact.optional(),
    covers: z.array(change),
    command: z.string().min(1).optional(),
    executedAt: z.string().datetime({ offset: true }).optional(),
    exitCode: z.number().int().optional(),
    reason: z.string().min(1).optional(),
  }).strict()),
  scan: z.object({
    status: z.enum(["passed", "failed", "not_run"]),
    scannedHead: commit.optional(),
    evidence: artifact.optional(),
    executedAt: z.string().datetime({ offset: true }).optional(),
    exitCode: z.number().int().optional(),
    disclosure: z.object({
      reason: z.string().min(1),
      triage: artifact,
      confirmedUnfixedProductFindings: z.number().int().nonnegative(),
      // 旧扫描到最终 head 的逐文件位置/内容核对；文档也可能含真实密钥。
      tailChanges: z.array(change),
    }).strict().optional(),
  }).strict(),
  // 收据只是已有合入的机械事实。新发现不能改名为收据来绕开第 6 条。
  postMergeReceipt: z.object({ newFindings: z.array(z.string().min(1)) }).strict().optional(),
}).strict();

export type DeliveryManifest = z.infer<typeof deliveryManifestSchema>;
export type ProductChange = z.infer<typeof change>;
export type DeliveryDiagnostic = { code: string; message: string; path?: string };
export type DeliveryResult = {
  status: "BLOCKED" | "READY_FOR_APPROVAL" | "READY_WITH_EXPLICIT_EXCEPTION";
  ticket?: number;
  base?: string;
  head?: string;
  requiresMergeApproval: true;
  mergeAuthorized: false;
  productDiff: ProductChange[];
  diagnostics: DeliveryDiagnostic[];
  exceptions: DeliveryDiagnostic[];
};

const sha256 = (value: Buffer) => createHash("sha256").update(value).digest("hex");
const sameChange = (a: ProductChange, b: ProductChange) =>
  a.path === b.path && a.beforeSha256 === b.beforeSha256 && a.afterSha256 === b.afterSha256 &&
  a.beforeMode === b.beforeMode && a.afterMode === b.afterMode;

function safePath(path: string): boolean {
  return !isAbsolute(path) && !path.includes("\\") && !path.includes("\0") &&
    path.split("/").every(part => part !== "" && part !== "." && part !== "..");
}

/** 保守判定：仅明确文档格式可豁免；配置、脚本、锁文件及未知格式均需审查/编译。 */
export function isDocumentationPath(path: string): boolean {
  // .txt 含 requirements/CMake 构建输入，MDX 可执行组件，均不普遍豁免。
  return /\.(md|rst|adoc)$/i.test(path);
}

function reviewChainCovers(actual: ProductChange, reviews: DeliveryManifest["reviews"]): boolean {
  const state = (hash: string | null, mode: string | null) => `${hash}:${mode}`;
  const frontier = new Set([state(actual.beforeSha256, actual.beforeMode)]);
  const target = state(actual.afterSha256, actual.afterMode);
  const edges = reviews.flatMap(review => review.covers).filter(edge => edge.path === actual.path);
  for (const current of frontier) {
    for (const edge of edges) {
      if (state(edge.beforeSha256, edge.beforeMode) !== current) continue;
      const next = state(edge.afterSha256, edge.afterMode);
      if (next === target) return true;
      frontier.add(next);
    }
  }
  return false;
}

/** 只读预检。它校验已声明证据的完整性，不替代独立审查，也不执行合入。 */
export function preflightDelivery(repo: string, input: unknown): DeliveryResult {
  const result: DeliveryResult = {
    status: "BLOCKED", requiresMergeApproval: true, mergeAuthorized: false,
    productDiff: [], diagnostics: [], exceptions: [],
  };
  const fail = (code: string, message: string, path?: string) => {
    result.diagnostics.push({ code, message, ...(path === undefined ? {} : { path }) });
  };
  const parsed = deliveryManifestSchema.safeParse(input);
  if (!parsed.success) {
    for (const issue of parsed.error.issues) {
      fail("INVALID_MANIFEST", `${issue.path.join(".")}: ${issue.message}`);
    }
    return result;
  }
  const manifest = parsed.data;
  Object.assign(result, { ticket: manifest.ticket, base: manifest.base, head: manifest.head });
  const git = (...args: string[]) => execFileSync("git", ["--no-replace-objects", "--literal-pathspecs", "-c", "core.filemode=true", "-c", "core.fsmonitor=false", "-C", repo, ...args], {
    encoding: "buffer", stdio: ["ignore", "pipe", "pipe"], maxBuffer: 64 * 1024 * 1024,
    env: { ...process.env, GIT_OPTIONAL_LOCKS: "0" },
  });
  const text = (...args: string[]) => git(...args).toString("utf8").trim();
  try {
    if (realpathSync(repo) !== realpathSync(text("rev-parse", "--show-toplevel"))) {
      fail("NOT_REPOSITORY_ROOT", "repo 必须指向 Git 工作树根目录。");
      return result;
    }
    const repositoryRoot = realpathSync(repo);
    for (const ref of [manifest.base, manifest.head]) {
      if (text("rev-parse", "--verify", `${ref}^{commit}`) !== ref) throw new Error(`无效 commit: ${ref}`);
    }
    if (text("rev-parse", "HEAD") !== manifest.head) fail("HEAD_DRIFT", "当前 HEAD 与 manifest.head 不同；重新冻结最终交付。");
    try { git("merge-base", "--is-ancestor", manifest.base, manifest.head); }
    catch { fail("BASE_NOT_ANCESTOR", "base 不是交付 head 的祖先；核对实际比较基线。"); }
    const dirty = git("status", "--porcelain=v1", "-z", "--untracked-files=all", "--ignore-submodules=none").toString("utf8");
    if (dirty) fail("DIRTY_WORKTREE", `工作树或索引有未冻结改动：${dirty.split("\0").filter(Boolean).join("; ")}`);
    const hidden = git("ls-files", "-v", "-z").toString("utf8").split("\0")
      .filter(entry => /^[a-zS] /.test(entry));
    if (hidden.length) fail("INDEX_FLAGS_HIDE_DIRTY", `索引标志可隐藏工作树改动；先检查并清除 assume-unchanged/skip-worktree 后重新冻结：${hidden.join("; ")}`);

    const treeEntry = (ref: string, path: string) => {
      const entry = git("ls-tree", "-z", ref, "--", path).toString("utf8").split("\0").filter(Boolean);
      return entry.find(line => line.slice(line.indexOf("\t") + 1) === path);
    };
    const blobHash = (ref: string, path: string): string | null => {
      const entry = treeEntry(ref, path);
      if (!entry) return null;
      if (!/^[0-7]{6} blob /.test(entry)) {
        fail("UNSUPPORTED_PRODUCT_FILE", "差异包含子模块或非文件 Git 对象；须另行审查固定目标，此预检不作自动就绪判断。", path);
        return null;
      }
      return sha256(git("show", `${ref}:${path}`));
    };
    const changes = (base: string, head: string): ProductChange[] =>
      git("diff", "--no-renames", "--ignore-submodules=none", "--name-only", "-z", base, head, "--").toString("utf8")
        .split("\0").filter(Boolean).map(path => ({
          path, beforeSha256: blobHash(base, path), afterSha256: blobHash(head, path),
          beforeMode: treeEntry(base, path)?.slice(0, 6) ?? null,
          afterMode: treeEntry(head, path)?.slice(0, 6) ?? null,
        }));

    const unique = (paths: string[], label: string) => {
      const seen = new Set<string>();
      for (const path of paths) {
        if (seen.has(path)) fail("DUPLICATE_ENTRY", `${label} 重复：${path}`);
        seen.add(path);
      }
    };
    unique(manifest.reports.map(item => item.path), "reports");
    unique(manifest.requiredReports, "requiredReports");
    unique(manifest.productDiff.map(item => item.path), "productDiff");
    unique(manifest.requiredChecks, "requiredChecks");
    unique(manifest.checks.map(item => item.id), "checks");
    const reportPaths = new Set(manifest.reports.map(item => item.path));
    const checkedArtifacts = new Set<string>();
    const verifyArtifact = (item: z.infer<typeof artifact>, mustBeReport = true) => {
      if (mustBeReport && !reportPaths.has(item.path)) fail("UNDECLARED_REPORT", "证据实体遗漏在 reports 清单之外。", item.path);
      const key = `${item.path}\0${item.sha256}`;
      if (checkedArtifacts.has(key)) return;
      checkedArtifacts.add(key);
      if (!safePath(item.path)) { fail("INVALID_REPORT_PATH", "实体报告必须使用仓内相对路径，禁止外部路径或父目录跳转。", item.path); return; }
      const entry = treeEntry(manifest.head, item.path);
      if (!entry) { fail("REPORT_NOT_IN_HEAD", "报告不存在于指定 head；请把实体入 Git 后重建 manifest。", item.path); return; }
      if (!/^100(644|755) blob /.test(entry)) { fail("REPORT_NOT_REGULAR_FILE", "报告须为普通文件，不能用符号链接/目录代替入仓实体。", item.path); return; }
      const absolute = resolve(repositoryRoot, item.path);
      try {
        if (!lstatSync(absolute).isFile() || realpathSync(absolute) !== absolute) {
          fail("REPORT_NOT_REGULAR_FILE", "报告工作树路径不能经过符号链接。", item.path); return;
        }
        if (sha256(readFileSync(absolute)) !== item.sha256 || blobHash(manifest.head, item.path) !== item.sha256) {
          fail("REPORT_HASH_MISMATCH", "报告 SHA-256 与已提交实体或当前文件不一致。", item.path);
        }
      } catch { fail("REPORT_MISSING", "报告工作树实体缺失或不可读。", item.path); }
    };
    for (const required of manifest.requiredReports) {
      if (!reportPaths.has(required)) fail("MISSING_REPORT_DECLARATION", "必需实体报告未列入 reports。", required);
    }
    if (!manifest.reports.length && !manifest.noReportsReason) fail("REPORT_DECLARATION_REQUIRED", "须列出实体报告，或明确声明无报告及理由。");
    for (const item of manifest.reports) verifyArtifact(item);

    // 报告只在专属目录且为数据格式时不计产品；不能把源码改名为“报告”逃避覆盖。
    const isReportData = (path: string) =>
      /^\.chanlun\/review-results\//.test(path) && reportPaths.has(path) && /\.(json|csv|log|sarif)$/i.test(path);
    const isProduct = (item: ProductChange) =>
      ![item.beforeMode, item.afterMode].every(mode => mode === null || mode === "100644") ||
      (!isDocumentationPath(item.path) && !isReportData(item.path));
    const actualDiff = changes(manifest.base, manifest.head);
    for (const item of actualDiff) {
      if (/^\.chanlun\/review-results\//.test(item.path) && item.afterMode !== null && !reportPaths.has(item.path)) {
        fail("UNDECLARED_CHANGED_REPORT", "本次新增/修改的入仓报告未纳入交付清单。", item.path);
      }
    }
    result.productDiff = actualDiff.filter(isProduct);
    for (const actual of result.productDiff) {
      const absolute = resolve(repositoryRoot, actual.path);
      if (actual.afterMode !== null && !/^100(644|755)$/.test(actual.afterMode)) {
        fail("UNSUPPORTED_PRODUCT_FILE", "交付产品包含符号链接/子模块；须另行审查其目标与固定版本，此预检不作自动就绪判断。", actual.path);
      } else if (actual.afterMode === null) {
        if (existsSync(absolute)) fail("PRODUCT_WORKTREE_DRIFT", "已删除产品路径在工作树中重新出现。", actual.path);
      } else {
        try {
          const stat = lstatSync(absolute);
          const mode = stat.mode & 0o111 ? "100755" : "100644";
          if (!stat.isFile() || realpathSync(absolute) !== absolute || mode !== actual.afterMode || sha256(readFileSync(absolute)) !== actual.afterSha256) {
            fail("PRODUCT_WORKTREE_DRIFT", "产品工作树内容/权限与指定 head 不同（包括 Git 标记跳过的文件）。", actual.path);
          }
        } catch { fail("PRODUCT_WORKTREE_DRIFT", "产品工作树文件缺失或不可读。", actual.path); }
      }
      if (!manifest.productDiff.some(item => sameChange(item, actual))) fail("PRODUCT_DIFF_MISMATCH", "产品差异遗漏或字节已漂移；按实际 base/head 重新冻结。", actual.path);
    }
    for (const declared of manifest.productDiff) {
      if (!result.productDiff.some(item => sameChange(item, declared))) fail("PRODUCT_DIFF_MISMATCH", "声明的产品差异不属于当前 base/head。", declared.path);
    }
    for (const review of manifest.reviews) verifyArtifact(review.report);
    for (const actual of result.productDiff) {
      if (!reviewChainCovers(actual, manifest.reviews)) {
        fail("REVIEW_COVERAGE_MISSING", "最终代码/构建差异未被实体评审按相同 SHA-256 覆盖；补评这段尾部差异。", actual.path);
      }
    }
    for (const id of manifest.requiredChecks) {
      if (!manifest.checks.some(check => check.id === id)) fail("REQUIRED_CHECK_MISSING", `必需检查未提供结果：${id}`);
    }
    if (result.productDiff.length && !manifest.checks.some(check => check.kind === "compile" && manifest.requiredChecks.includes(check.id))) {
      fail("COMPILE_CHECK_REQUIRED", "实际差异包含代码/构建文件，必需检查中须包含编译检查。");
    }
    for (const check of manifest.checks) {
      if (check.status === "not_run") { fail("CHECK_NOT_RUN", `${check.id} 尚未执行。`); continue; }
      if (check.status === "exempt") {
        if (check.kind !== "compile" || result.productDiff.length || !check.reason) {
          fail("INVALID_CHECK_EXEMPTION", `${check.id}：只有实际纯文档/报告差异可注明理由豁免编译。`);
        }
        continue;
      }
      if (!check.evidence || !check.executedAt || !check.command || check.exitCode === undefined) {
        fail("CHECK_EXECUTION_EVIDENCE_MISSING", `${check.id} 缺已执行的命令、时间、退出码或入仓证据。`);
      }
      if (check.evidence) verifyArtifact(check.evidence);
      if (check.status === "failed" || check.exitCode !== 0) fail("CHECK_FAILED", `${check.id} 未通过（退出码 ${check.exitCode ?? "缺失"}）；修复后补实际执行结果。`);
      for (const actual of result.productDiff) {
        if (!check.covers.some(item => sameChange(item, actual))) fail("CHECK_SOURCE_DRIFT", `${check.id} 未绑定最终产品字节；核对检查后新增的代码差异。`, actual.path);
      }
    }

    const scan = manifest.scan;
    if (scan.disclosure) {
      verifyArtifact(scan.disclosure.triage);
      if (scan.disclosure.confirmedUnfixedProductFindings > 0) fail("UNFIXED_PRODUCT_FINDINGS", "存在已确认、未修复的产品问题；先修复，不能当误报例外放行。");
    }
    if (scan.status === "not_run") fail("SCAN_NOT_RUN", "扫描尚未执行，不能用披露代替执行。");
    else {
      if (!scan.evidence || !scan.executedAt || scan.exitCode === undefined || !scan.scannedHead) {
        fail("SCAN_EXECUTION_EVIDENCE_MISSING", "扫描缺实际 commit、执行时间、退出码或入仓证据。");
      }
      if (scan.evidence) verifyArtifact(scan.evidence);
      if (scan.status === "passed" && scan.exitCode !== 0) fail("SCAN_STATUS_CONFLICT", "扫描声明通过但退出码非 0。");
      if (scan.status === "failed" && scan.exitCode === 0) fail("SCAN_STATUS_CONFLICT", "扫描声明失败但退出码为 0。");
      let scanHeadDrift = false;
      if (scan.scannedHead && scan.scannedHead !== manifest.head) {
        scanHeadDrift = true;
        try {
          git("merge-base", "--is-ancestor", scan.scannedHead, manifest.head);
          const scanTail = changes(scan.scannedHead, manifest.head);
          if (scanTail.some(isProduct)) {
            fail("SCAN_PRODUCT_DRIFT", "扫描后存在代码/构建差异；须重新执行扫描。");
          }
          for (const item of scanTail) {
            if (!scan.disclosure?.tailChanges.some(assessed => sameChange(assessed, item))) {
              fail("SCAN_TAIL_UNASSESSED", "旧扫描未覆盖此尾部文件；须在实体分诊中核对具体位置/内容，并用相同 SHA-256 声明覆盖。", item.path);
            }
          }
        } catch { fail("SCAN_HEAD_INVALID", "扫描 commit 不存在或不是交付 head 的祖先。"); }
      }
      if (scan.status === "failed" || scanHeadDrift) {
        if (!scan.disclosure) fail("SCAN_EXCEPTION_NOT_DISCLOSED", "扫描红灯或仅覆盖报告尾部之前的 commit；须附实体分诊并明确披露待批准的例外。");
        else {
          result.exceptions.push({ code: "SCAN_EXCEPTION_REQUIRES_APPROVAL", message:
            `${scan.disclosure.reason}；实际扫描 ${scan.scannedHead ?? "未提供"}，交付 ${manifest.head}。例外尚未批准。` });
        }
      }
    }
    if (manifest.postMergeReceipt?.newFindings.length) {
      fail("POST_MERGE_FINDINGS_REQUIRE_REPAIR", "合入后的新发现须回到修复/实体报告流程，不能记作机械收据消除交付义务。");
    }
    // 捕捉检查期间的普通 ref/工作树漂移；此工具不声称提供跨进程原子锁。
    if (text("rev-parse", "HEAD") !== manifest.head || git("status", "--porcelain=v1", "-z", "--untracked-files=all", "--ignore-submodules=none").toString("utf8") !== dirty) {
      fail("CONCURRENT_DRIFT", "预检期间 HEAD/工作树发生变化，请冻结后重跑。");
    }
  } catch (error) {
    fail("REPOSITORY_CHECK_FAILED", error instanceof Error ? error.message : String(error));
  }
  if (!result.diagnostics.length) result.status = result.exceptions.length ? "READY_WITH_EXPLICIT_EXCEPTION" : "READY_FOR_APPROVAL";
  return result;
}
