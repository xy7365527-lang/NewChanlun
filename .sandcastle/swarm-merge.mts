/** #1617：主控验收清单驱动的归并入口；与自动领票分离，不自动升级验收。 */
import * as sandcastle from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, mkdirSync, realpathSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { runManagedMerge, validateMergeManifest, type MergeManifest, type MergeCheck } from "./managed-merge.mts";
import { admitMergeInput, appRemoteFacts, type RemoteFactsReader } from "./merge-source.mts";
import { verifyManagedRust } from "./managed-commit.mts";
import { prepareFinalReviewPrompt } from "./review-context.mts";
import { mergeBinding, mergeInstance } from "./docker-acceptance.mts";
import { type DockerCli, MERGE_INSTANCE_ENV, MERGE_RUN_ENV, reclaimOrphanContainers } from "./orphan-container.mts";
import { dockerCliProxyEnv, assertSwarmExecutionReady, type ReadyOptions, assertDockerCliProxyReachable, swarmAgent, swarmSandbox } from "./swarm-route.mts";
import { withLegacyAdmission } from "./drain-control.mts";

/** 旧入口准入：生产固定用 drain-control 的共享根；测试注入临时根。 */
export type LegacyAdmission = <T>(action: () => T | Promise<T>) => Promise<T>;
export interface AcceptedMergeOptions { repo?: string; admission?: LegacyAdmission; acceptance?: ReadyOptions["acceptance"]; docker?: DockerCli }

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

/** 准入事实只认完全相同的 run、清单、仓库、工位与验证配置；缺失、损坏或不符都要重新取准入。 */
function admittedBefore(path: string, fact: unknown): boolean {
  try { return isDeepStrictEqual(JSON.parse(readFileSync(path, "utf8")), fact); }
  catch { return false; }
}

export async function runAcceptedSwarmMerge(manifestPath: string, image: string, logWorker: (e: Record<string, unknown>) => void, options: AcceptedMergeOptions = {}) {
  const repo = realpathSync(options.repo ?? process.cwd());
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8")) as MergeManifest;
  validateMergeManifest(manifest); // 创建工位前拒绝未验收/错误目标。
  // DeepAgents 来源在任何 Git 写入前复核 PASS(head)、交付状态与实时远端；旧 sandcastle 清单不构造远端读取器。
  const facts: RemoteFactsReader = manifest.inputs.some(i => i.source === "deepagents") ? appRemoteFacts() : () => { throw new Error("旧输入不读远端"); };
  for (const input of manifest.inputs) admitMergeInput(repo, input, facts);
  const cwd = join(repo, ".sandcastle", "worktrees", manifest.branch.replaceAll("/", "-"));
  // 宿主控制目录，不挂进工蜂 worktree，不允许工蜂修改验收输入/恢复记录。
  const evidenceDir = join(repo, ".sandcastle", "logs", "merges", manifest.runId);
  // #1742 冻结语义：含旧 sandcastle 来源的归并是手工点名的旧入口。首次副作用前在冻结共享锁内落一份准入事实，
  // 绑定 run、固定清单、仓库、工位与组合验证配置；只有同一事实才能在冻结后续做。目录或检查点存在都不算准入。
  // 仅含 DeepAgents 来源的清单属于新执行器，不受旧入口冻结约束。
  const admissionPath = join(evidenceDir, "admission.json");
  const fact = { v: 1, repo, cwd, checkId: MERGE_CHECK_ID, manifest };
  const start = () => {
    mkdirSync(evidenceDir, { recursive: true });
    if (!existsSync(cwd)) {
      execFileSync("git", ["worktree", "add", "-b", manifest.branch, cwd, manifest.base], { cwd: repo, stdio: "pipe" });
    }
  };
  if (manifest.inputs.some(i => i.source === undefined) && !admittedBefore(admissionPath, fact)) {
    await (options.admission ?? withLegacyAdmission)(() => {
      mkdirSync(evidenceDir, { recursive: true });
      writeFileSync(`${admissionPath}.tmp`, JSON.stringify(fact, null, 2) + "\n", { mode: 0o600 });
      renameSync(`${admissionPath}.tmp`, admissionPath);
      start();
    });
  } else start();
  if (realpathSync(manifestPath).startsWith(realpathSync(cwd) + "/")) throw new Error("验收清单不得位于工蜂 worktree");
  const readiedCheckpoints = new Map<string, string>(); // binding → 准入核对过的镜像 Id
  let routeChecked = false;
  const result = await runManagedMerge({
    cwd, manifest, checkpoint: join(evidenceDir, "checkpoint.json"), checkId: MERGE_CHECK_ID,
    // SIGKILL 后零人工收敛：锁持有者已死且停在冲突阶段时，先自动回收本 run 实例遗留的孤儿工蜂容器，再接管锁（#1742）。
    reclaimOrphans: () => { reclaimOrphanContainers({ cwd, image, runId: manifest.runId, instance: mergeInstance(evidenceDir, manifest.runId, manifest.base), evidenceDir, docker: options.docker }); },
    verify: (head, files) => verifyCombination(cwd, manifest, head, files),
    admit: input => admitMergeInput(repo, input, facts),
    resolveConflict: async ({ head, incoming, files }) => {
      // #1617：仅冲突需要 Docker；此时 MERGE_HEAD 与检查点已保存，失败保留现场。
      // 14/15：每个冲突检查点都过一次门禁，绑定各自的 run+基线+HEAD+入站 SHA；同一检查点在本进程内只过一次，
      // SIGKILL 后重入（新进程）得到相同绑定，按恢复放行；别的 run 或基线被拒绝。
      const binding = mergeBinding({ runId: manifest.runId, base: manifest.base, instance: mergeInstance(evidenceDir, manifest.runId, manifest.base), head, incoming });
      if (!readiedCheckpoints.has(binding)) {
        const admittedId = assertSwarmExecutionReady(image, { rows: ["14", "15"], binding, acceptance: options.acceptance,
          // 占用 used-14/15 / 检查点之前预检；同一进程内成功一次即可
          modelProbe: routeChecked ? undefined : (id) => { assertDockerCliProxyReachable(id); routeChecked = true; } });
        readiedCheckpoints.set(binding, admittedId);
      }
      const launchImage = readiedCheckpoints.get(binding)!;
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
        agent: swarmAgent(), sandbox: swarmSandbox(launchImage, { ...dockerCliProxyEnv(), [MERGE_RUN_ENV]: manifest.runId, [MERGE_INSTANCE_ENV]: mergeInstance(evidenceDir, manifest.runId, manifest.base) }), prompt: final.prompt });
      return run.stdout;
    },
    onCheckpoint: s => logWorker({ phase: "merger", branch: manifest.branch, status: s.phase, head: s.snapshot.head, position: s.position, checkpoint: join(evidenceDir, "checkpoint.json") }),
  });
  console.log(`功能归并完成：${manifest.branch} @ ${result.snapshot.head}；仅待验收，不合 main、不关来源票。`);
  return result;
}
