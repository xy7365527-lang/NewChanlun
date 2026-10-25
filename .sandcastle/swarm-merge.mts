/** #1617：主控验收清单驱动的归并入口；与自动领票分离，不自动升级验收。 */
import * as sandcastle from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, mkdirSync, realpathSync } from "node:fs";
import { join } from "node:path";
import { runManagedMerge, validateMergeManifest, type MergeManifest, type MergeCheck } from "./managed-merge.mts";
import { verifyManagedRust } from "./managed-commit.mts";
import { prepareFinalReviewPrompt } from "./review-context.mts";
import { assertSwarmExecutionReady, assertDockerCliProxyReachable, swarmAgent, swarmSandbox } from "./swarm-route.mts";

export const MERGE_CHECK_ID = "combination-v1:git-diff-check,cargo-check-locked,tsc-noEmit";
export async function verifyCombination(cwd: string, manifest: MergeManifest, head: string, files: string[]): Promise<MergeCheck> {
  const evidence: unknown[] = [];
  try {
    const diff = execFileSync("git", ["diff", "--check", manifest.base, head], { cwd, encoding: "utf8", stdio: "pipe" });
    evidence.push({ command: "git diff --check", base: manifest.base, head, stdout: diff });
    const rust = verifyManagedRust({ cwd, branch: manifest.branch, head, issue: manifest.inputs[0]!.issue, phase: "reviewer" },
      { head, committed: true, report: { status: "ready", files, verification: [] } });
    evidence.push(rust);
    if (rust.status === "failed") throw new Error("精确组合 Rust 验证失败");
    if (files.some(f => /\.(m?ts|tsx)$/.test(f) || /(^|\/)(package(-lock)?\.json|tsconfig[^/]*\.json)$/.test(f))) {
      const stdout = execFileSync("npx", ["--no-install", "tsc", "-p", "tsconfig.sandcastle.json", "--noEmit"], { cwd, encoding: "utf8", stdio: "pipe", timeout: 600_000 });
      evidence.push({ command: "npx --no-install tsc -p tsconfig.sandcastle.json --noEmit", head, stdout });
    }
    return { head, checkId: MERGE_CHECK_ID, status: "passed", evidence };
  } catch (error) {
    const e = error as { stdout?: unknown; stderr?: unknown; message?: unknown };
    evidence.push({ stdout: String(e.stdout ?? ""), stderr: String(e.stderr ?? e.message ?? error) });
    return { head, checkId: MERGE_CHECK_ID, status: "failed", evidence };
  }
}

export async function runAcceptedSwarmMerge(manifestPath: string, image: string, logWorker: (e: Record<string, unknown>) => void) {
  const repo = realpathSync(process.cwd());
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as MergeManifest;
  validateMergeManifest(manifest); // 创建工位前拒绝未验收/错误目标。
  const cwd = join(repo, ".sandcastle", "worktrees", manifest.branch.replaceAll("/", "-"));
  // 宿主控制目录，不挂进工蜂 worktree，不允许工蜂修改验收输入/恢复记录。
  const evidenceDir = join(repo, ".sandcastle", "logs", "merges", manifest.runId);
  mkdirSync(evidenceDir, { recursive: true });
  if (!existsSync(cwd)) {
    execFileSync("git", ["worktree", "add", "-b", manifest.branch, cwd, manifest.base], { cwd: repo, stdio: "pipe" });
  }
  if (realpathSync(manifestPath).startsWith(realpathSync(cwd) + "/")) throw new Error("验收清单不得位于工蜂 worktree");
  let conflictExecutionReady = false;
  const result = await runManagedMerge({
    cwd, manifest, checkpoint: join(evidenceDir, "checkpoint.json"), checkId: MERGE_CHECK_ID,
    verify: (head, files) => verifyCombination(cwd, manifest, head, files),
    resolveConflict: async ({ head, incoming, files }) => {
      // #1617：仅冲突需要 Docker；此时 MERGE_HEAD 与检查点已保存，失败保留现场。
      if (!conflictExecutionReady) {
        assertSwarmExecutionReady(image);
        assertDockerCliProxyReachable(image);
        conflictExecutionReady = true;
      }
      const final = prepareFinalReviewPrompt(
        `你只解决当前功能分支的这一次 merge 冲突。原 HEAD=${head}，输入=${incoming}。\n` +
        `只允许编辑以下冻结路径：${JSON.stringify(files)}。不要 git add/commit/merge/abort/reset，不 push、不改 main、不关票，不修改索引/受保护路径。\n` +
        `读取双方语义后解决冲突，不作范围外重构。本机不运行 cargo/rustc；宿主封包后验证精确组合 SHA。\n` +
        `只输出一个 <sandcastle-result>{"status":"ready","files":["实际编辑路径"],"verification":[]}</sandcastle-result>；` +
        `无法在路径边界内解决时 status=blocked。最后附 <promise>COMPLETE</promise>。`,
        manifest.base, { issue: manifest.inputs[0]!.issue, branch: manifest.branch },
        { logWorker, notify: () => { /* 归并不是单张票，失败由宿主归并记录交主控。 */ } },
      );
      // head 策略仅挂已有工位；容器随调用回收，宿主工位/冲突现场不自动清理。
      const run = await sandcastle.run({ cwd, branchStrategy: { type: "head" }, name: "merger-conflict", maxIterations: 1,
        agent: swarmAgent(), sandbox: swarmSandbox(image), prompt: final.prompt });
      return run.stdout;
    },
    onCheckpoint: s => logWorker({ phase: "merger", branch: manifest.branch, status: s.phase, head: s.snapshot.head, position: s.position, checkpoint: join(evidenceDir, "checkpoint.json") }),
  });
  console.log(`功能归并完成：${manifest.branch} @ ${result.snapshot.head}；仅待验收，不合 main、不关来源票。`);
  return result;
}
