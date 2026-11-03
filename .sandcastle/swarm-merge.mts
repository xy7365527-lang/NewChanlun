/** #1617：主控验收清单驱动的归并入口；与自动领票分离，不自动升级验收。 */
import * as sandcastle from "@ai-hero/sandcastle";
import type { SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, mkdirSync, realpathSync, renameSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { isDeepStrictEqual } from "node:util";
import { MERGE_CHECK_ID, runManagedMerge, validateMergeManifest, type MergeManifest, type MergeCheck, type MergeInput, type MergeState } from "./managed-merge.mts";
import { admitMergeInput, appRemoteFacts, type RemoteFactsReader } from "./merge-source.mts";
import { verifyManagedRust } from "./managed-commit.mts";
import { prepareFinalReviewPrompt } from "./review-context.mts";
import {
  ROUND_FILES, assertBoundPath, branchExists, frozenRowAdmission, loadFrozenRound, loadMergeAdmission, loadMergePlan, loadMergeResult, mergeBinding, mergeInstance,
  prebindPath, publishMergeAdmission, publishMergeResult, rowConsumed, type DeploymentSelection, type FrozenRound, type MergeRow, type MergeRowResult, type RoundContext,
} from "./docker-acceptance.mts";
import { type DockerCli, MERGE_INSTANCE_ENV, MERGE_RUN_ENV, reclaimOrphanContainers } from "./orphan-container.mts";
import { dockerCliProxyEnv, assertSwarmExecutionReady, type ReadyOptions, assertDockerCliProxyReachable, swarmAgent, swarmSandbox } from "./swarm-route.mts";
import { withFileLock, withFrozenAdmission, withLegacyAdmission } from "./drain-control.mts";
import { assertContainerIsolationContract, type IsolationObservation } from "./docker-isolation.mts";
import { repoInitLock, wrapBindMountCreate } from "./swarm-ticket.mts";

export { MERGE_CHECK_ID };

/** 旧入口准入：生产固定用 drain-control 的共享根；测试注入临时根。 */
export type LegacyAdmission = <T>(action: () => T | Promise<T>) => Promise<T>;
export interface AcceptedMergeOptions { repo?: string; admission?: LegacyAdmission; acceptance?: ReadyOptions["acceptance"]; docker?: DockerCli }

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

export type ConflictAgentRunner = (o: { cwd: string; sandbox: SandboxProvider; prompt: string }) => Promise<string>;
const runConflictWorker: ConflictAgentRunner = async ({ cwd, sandbox, prompt }) =>
  (await sandcastle.run({ cwd, branchStrategy: { type: "head" }, name: "merger-conflict", maxIterations: 1, agent: swarmAgent(), sandbox, prompt })).stdout;

interface MergeExecution {
  manifest: MergeManifest; cwd: string; evidenceDir: string; checkpoint: string; image: string;
  logWorker: (e: Record<string, unknown>) => void; docker?: DockerCli;
  instance: () => string;
  admitInput: (input: MergeInput) => void;
  conflictSandbox: (binding: string, identityEnv: Record<string, string>) => Promise<SandboxProvider>;
  runConflictAgent?: ConflictAgentRunner;
}
async function executeMerge(o: MergeExecution): Promise<MergeState> {
  const { manifest, cwd } = o;
  return runManagedMerge({
    cwd, manifest, checkpoint: o.checkpoint, checkId: MERGE_CHECK_ID,
    reclaimOrphans: () => { reclaimOrphanContainers({ cwd, image: o.image, runId: manifest.runId, instance: o.instance(), evidenceDir: o.evidenceDir, docker: o.docker }); },
    verify: (head, files) => verifyCombination(cwd, manifest, head, files),
    admit: o.admitInput,
    resolveConflict: async ({ head, incoming, files }) => {
      const instance = o.instance();
      const binding = mergeBinding({ runId: manifest.runId, base: manifest.base, instance, head, incoming });
      const sandbox = await o.conflictSandbox(binding, { [MERGE_RUN_ENV]: manifest.runId, [MERGE_INSTANCE_ENV]: instance });
      const final = prepareFinalReviewPrompt(
        `你只解决当前功能分支的这一次 merge 冲突。原 HEAD=${head}，输入=${incoming}。\n` +
        `只允许编辑以下冻结路径：${JSON.stringify(files)}。不要 git add/commit/merge/abort/reset，不 push、不改 main、不关票，不修改索引/受保护路径。\n` +
        `读取双方语义后解决冲突，不作范围外重构。本机不运行 cargo/rustc；宿主封包后验证精确组合 SHA。\n` +
        `只输出一个 <sandcastle-result>{"status":"ready","files":["实际编辑路径"],"verification":[]}</sandcastle-result>；` +
        `无法在路径边界内解决时 status=blocked。最后附 <promise>COMPLETE</promise>。`,
        manifest.base, { issue: manifest.inputs[0]!.issue, branch: manifest.branch },
        { logWorker: o.logWorker, notify: () => {} },
      );
      // head 策略仅挂已有工位；容器随调用回收，宿主工位/冲突现场不自动清理。
      return (o.runConflictAgent ?? runConflictWorker)({ cwd, sandbox, prompt: final.prompt });
    },
    onCheckpoint: s => o.logWorker({ phase: "merger", branch: manifest.branch, status: s.phase, head: s.snapshot.head, position: s.position, checkpoint: o.checkpoint }),
  });
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
  const start = () => withFileLock(repoInitLock(repo), () => {
    mkdirSync(evidenceDir, { recursive: true });
    if (!existsSync(cwd)) {
      execFileSync("git", ["worktree", "add", "-b", manifest.branch, cwd, manifest.base], { cwd: repo, stdio: "pipe" });
    }
  });
  if (manifest.inputs.some(i => i.source === undefined) && !admittedBefore(admissionPath, fact)) {
    await (options.admission ?? withLegacyAdmission)(async () => {
      mkdirSync(evidenceDir, { recursive: true });
      writeFileSync(`${admissionPath}.tmp`, JSON.stringify(fact, null, 2) + "\n", { mode: 0o600 });
      renameSync(`${admissionPath}.tmp`, admissionPath);
      await start();
    });
  } else await start();
  if (realpathSync(manifestPath).startsWith(realpathSync(cwd) + "/")) throw new Error("验收清单不得位于工蜂 worktree");
  const readiedCheckpoints = new Map<string, string>(); // binding → 准入核对过的镜像 Id
  let routeChecked = false;
  const result = await executeMerge({
    manifest, cwd, evidenceDir, checkpoint: join(evidenceDir, "checkpoint.json"), image, logWorker, docker: options.docker,
    instance: () => mergeInstance(evidenceDir, manifest.runId, manifest.base),
    admitInput: input => admitMergeInput(repo, input, facts),
    conflictSandbox: async (binding, identityEnv) => {
      if (!readiedCheckpoints.has(binding)) {
        const admittedId = assertSwarmExecutionReady(image, { rows: ["14", "15"], binding, acceptance: options.acceptance,
          // 占用 used-14/15 / 检查点之前预检；同一进程内成功一次即可
          modelProbe: routeChecked ? undefined : (id) => { assertDockerCliProxyReachable(id); routeChecked = true; } });
        readiedCheckpoints.set(binding, admittedId);
      }
      const launchImage = readiedCheckpoints.get(binding)!;
      return swarmSandbox(launchImage, { ...dockerCliProxyEnv(), ...identityEnv });
    },
  });
  console.log(`功能归并完成：${manifest.branch} @ ${result.snapshot.head}；仅待验收，不合 main、不关来源票。`);
  return result;
}

export interface DeploymentMergeDeps {
  ctx?: RoundContext;
  drainRoot?: string;
  logWorker: (e: Record<string, unknown>) => void;
  probe?: (image: string) => IsolationObservation;
  modelProbe?: (imageId: string) => void;
  provider?: (imageId: string, identityEnv: Record<string, string>) => SandboxProvider;
  runConflictAgent?: ConflictAgentRunner;
  docker?: DockerCli;
  barrier?: (point: "merge-run-before-admission" | "merge-before-consume" | "merge-after-consume") => Promise<void> | void;
}

const gitIn = (cwd: string, args: string[]) => execFileSync("git", ["-C", cwd, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
const optionalRefIn = (cwd: string, ref: string) => { try { return gitIn(cwd, ["rev-parse", "--verify", "-q", ref]); } catch { return ""; } };

export async function runDeploymentSwarmMerge(sel: DeploymentSelection & { row: MergeRow }, d: DeploymentMergeDeps): Promise<MergeRowResult> {
  const ctx = d.ctx ?? {};
  const round0 = loadFrozenRound(sel, ctx);
  const plan0 = loadMergePlan(round0);
  if (!plan0) throw new Error("本轮归并计划尚未封定（先由主控 bind-merges），拒绝");
  if (loadMergeResult(round0, sel.row)) throw new Error(`第 ${sel.row} 行已有完整完成结果，拒绝终态重放；查看用 deployment-acceptance status`);
  const run = plan0.runs[sel.row];
  const { repo } = run;
  for (const sha of [run.manifest.base, ...run.manifest.inputs.map(i => i.sha)]) {
    try { if (gitIn(repo, ["rev-parse", "--verify", "-q", `${sha}^{commit}`]) !== sha) throw new Error(); } catch { throw new Error(`归并输入 ${sha.slice(0, 12)} 在本仓库不可解析`); }
  }
  if (prebindPath(repo, [".sandcastle", "worktrees", run.manifest.branch.replaceAll("/", "-")]) !== run.cwd) throw new Error("归并工位预绑定路径漂移");
  const frozenLock = (round: FrozenRound) => ({ generation: round.purpose.generation, control: round.control, root: d.drainRoot });
  const recheck = (): FrozenRound => {
    const round = loadFrozenRound(sel, ctx);
    if (!isDeepStrictEqual(loadMergePlan(round), plan0)) throw new Error("本轮归并计划已变化，拒绝");
    return round;
  };
  return withFileLock(join(round0.roundDir, ROUND_FILES.runLock(sel.row)), async () => {
    const round = recheck();
    if (loadMergeResult(round, sel.row)) throw new Error(`第 ${sel.row} 行已有完整完成结果，拒绝终态重放`);
    const admitted = loadMergeAdmission(round, run);
    const consumed = rowConsumed(round, sel.row);
    const hasCheckpoint = existsSync(run.checkpoint);
    if (!admitted && (hasCheckpoint || consumed || existsSync(run.cwd) || branchExists(repo, run.manifest.branch))) throw new Error(`第 ${sel.row} 行没有 run 准入事实却已有工位/检查点/消费，不是本轮准入 run，现场保留`);
    if (consumed && !hasCheckpoint) throw new Error(`第 ${sel.row} 行已消费真实冲突但原检查点缺失：不能从基线重建冒充恢复`);
    if (hasCheckpoint && !existsSync(run.cwd)) throw new Error(`第 ${sel.row} 行检查点存在但工位缺失，交 owner 核认`);
    let probedImage: string | undefined;
    const probeAll = (r: FrozenRound) => {
      assertContainerIsolationContract(r.receipt.imageId, d.probe);
      (d.modelProbe ?? assertDockerCliProxyReachable)(r.receipt.imageId);
      probedImage = r.receipt.imageId;
    };
    if (!admitted || !existsSync(run.cwd)) {
      await withFrozenAdmission(async () => {
        const r = recheck();
        await d.barrier?.("merge-run-before-admission");
        if (!loadMergeAdmission(r, run)) { probeAll(r); recheck(); publishMergeAdmission(r, run); }
        await withFileLock(repoInitLock(repo), () => {
          if (branchExists(repo, run.manifest.branch) || existsSync(run.cwd)) throw new Error("归并工位在准入后被他人占用，拒绝");
          execFileSync("git", ["worktree", "add", "-b", run.manifest.branch, run.cwd, run.manifest.base], { cwd: repo, stdio: "pipe" });
          assertBoundPath(run.cwd);
        });
      }, frozenLock(round));
    }
    assertBoundPath(run.cwd);
    const state = await executeMerge({
      manifest: run.manifest, cwd: run.cwd, evidenceDir: run.evidenceDir, checkpoint: run.checkpoint, image: round.receipt.image, logWorker: d.logWorker, docker: d.docker,
      instance: () => run.instance,
      admitInput: () => undefined,
      runConflictAgent: d.runConflictAgent,
      conflictSandbox: async (binding, identityEnv) => {
        if (probedImage === undefined) await withFrozenAdmission(() => probeAll(recheck()), frozenLock(round));
        const inner = (d.provider ?? ((id, env) => swarmSandbox(id, { ...dockerCliProxyEnv(), ...env })))(round.receipt.imageId, identityEnv);
        return wrapBindMountCreate(inner, (create, { worktreePath }) => withFrozenAdmission(async () => {
          const r = recheck();
          if (!loadMergeAdmission(r, run)) throw new Error("run 准入事实缺失");
          if (realpathSync(worktreePath) !== run.cwd) throw new Error(`冲突工蜂挂载 ${worktreePath} 不是封定工位`);
          const cp = JSON.parse(readFileSync(run.checkpoint, "utf8")) as MergeState;
          const b = mergeBinding({ runId: run.manifest.runId, base: run.manifest.base, instance: run.instance, head: cp.before ?? "", incoming: cp.input ?? "" });
          if (cp.phase !== "conflict" || b !== binding || gitIn(run.cwd, ["rev-parse", "HEAD"]) !== cp.before || optionalRefIn(run.cwd, "MERGE_HEAD") !== cp.input) throw new Error("当前检查点/HEAD/MERGE_HEAD 不是本次冲突绑定");
          await d.barrier?.("merge-before-consume");
          const admission = frozenRowAdmission(r.receipt.image, ctx, sel.row, binding);
          admission.consume();
          await d.barrier?.("merge-after-consume");
          return create();
        }, frozenLock(round)));
      },
    });
    return publishMergeResult(recheck(), run, state);
  });
}
