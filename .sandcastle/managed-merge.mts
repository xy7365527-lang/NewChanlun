/** #1617：已验收固定 SHA → 宿主双父归并；不操作 main/来源票。 */
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync, renameSync, mkdirSync, openSync, closeSync, unlinkSync, lstatSync, readlinkSync, realpathSync } from "node:fs";
import { dirname, basename, join, resolve, relative, isAbsolute } from "node:path";
import { parseStageReport } from "./managed-commit.mts";
import { assertHostPackagingSafe } from "./docker-isolation.mts";
import { validateMergeInputSource, type MergeInput } from "./merge-source.mts";

export type { MergeInput } from "./merge-source.mts";
export interface MergeManifest { runId: string; branch: string; base: string; inputs: MergeInput[] }
export interface MergeCheck { head: string; checkId: string; status: "passed" | "failed"; evidence: unknown }
interface Snapshot { head: string; mergeHead: string; index: string; diff: string; status: string; files: Record<string, string> }
interface Entry { sha: string; head: string; kind: "merged" | "already_integrated"; check: MergeCheck }
export interface MergeState {
  manifest: MergeManifest; cwd: string; checkId: string; position: number;
  phase: "ready" | "merging" | "conflict" | "prepared" | "committed" | "failed" | "done";
  snapshot: Snapshot; before?: string; input?: string; allowed?: string[]; tree?: string;
  entries: Entry[]; checks: MergeCheck[];
}
export interface MergeOptions {
  cwd: string; checkpoint: string; manifest: MergeManifest; checkId: string;
  resolveConflict: (input: { head: string; incoming: string; files: string[] }) => Promise<string>;
  verify: (head: string, files: string[]) => Promise<MergeCheck>;
  /** 来源复核：每个输入归并前、提交前、成功记账前调用；抛错即停，现场与检查点保持可恢复。 */
  admit?: (input: MergeInput) => void;
  /** 只在已持久化检查点后发事件；可用于日志及中断恢复测试。 */
  onCheckpoint?: (state: MergeState) => void;
}
const git = (cwd: string, args: string[]) => execFileSync("git", args, { cwd, encoding: "utf8", maxBuffer: 64 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"] });
const list = (s: string) => [...new Set(s.split("\0").filter(Boolean))].sort();
const equal = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);
function optionalRef(cwd: string, ref: string): string { try { return git(cwd, ["rev-parse", "--verify", ref]).trim(); } catch { return ""; } }
function fileImage(cwd: string, file: string): string {
  const parts = file.split("/");
  for (let n = 1; n < parts.length; n++) {
    try { if (lstatSync(join(cwd, ...parts.slice(0, n))).isSymbolicLink()) throw new Error("路径父目录为 symlink，拒绝越界读取"); }
    catch (e) { if ((e as NodeJS.ErrnoException).code === "ENOENT") return "absent"; throw e; }
  }
  const path = join(cwd, file);
  try { const st = lstatSync(path); return st.isSymbolicLink() ? `link:${readlinkSync(path)}` : `${st.mode}:${readFileSync(path).toString("base64")}`; }
  catch (e) { if ((e as NodeJS.ErrnoException).code === "ENOENT") return "absent"; throw e; }
}
function snapshot(cwd: string, extra: string[] = []): Snapshot {
  const files = list(git(cwd, ["ls-files", "--others", "--exclude-standard", "-z"]));
  return {
    head: git(cwd, ["rev-parse", "HEAD"]).trim(), mergeHead: optionalRef(cwd, "MERGE_HEAD"),
    index: git(cwd, ["ls-files", "--stage", "-z"]), diff: git(cwd, ["diff", "--binary", "HEAD"]),
    status: git(cwd, ["status", "--porcelain=v1", "-z", "--untracked-files=all"]),
    files: Object.fromEntries([...new Set([...files, ...extra])].sort().map(f => [f, fileImage(cwd, f)])),
  };
}
function assertBranch(cwd: string, branch: string) {
  if (git(cwd, ["symbolic-ref", "--short", "HEAD"]).trim() !== branch) throw new Error("归并工位分支不匹配");
  if (realpathSync(git(cwd, ["rev-parse", "--show-toplevel"]).trim()) !== realpathSync(cwd)) throw new Error("归并工位不是仓根");
}
export function validateMergeManifest(value: MergeManifest): void {
  if (!value || !/^[a-zA-Z0-9][a-zA-Z0-9_-]{0,79}$/.test(value.runId) || value.branch !== `sandcastle/merge-${value.runId}`) throw new Error("只接受本运行专用功能归并分支，拒绝 main/其他分支");
  if (!/^[a-f0-9]{40}$/.test(value.base) || !Array.isArray(value.inputs) || !value.inputs.length) throw new Error("归并基线或输入清单无效");
  for (const i of value.inputs) validateMergeInputSource(i);
}
function canonicalPath(path: string): string {
  let current = resolve(path); const tail: string[] = [];
  while (!existsSync(current)) { tail.unshift(basename(current)); current = dirname(current); }
  return join(realpathSync(current), ...tail);
}
export async function runManagedMerge(o: MergeOptions): Promise<MergeState> {
  o = { ...o, checkpoint: canonicalPath(o.checkpoint) };
  validateMergeManifest(o.manifest);
  assertBranch(o.cwd, o.manifest.branch);
  const inside = relative(realpathSync(o.cwd), resolve(o.checkpoint));
  if (inside !== ".." && !inside.startsWith("../") && !isAbsolute(inside)) throw new Error("宿主检查点不得放进工蜂 worktree");
  if (!o.checkId) throw new Error("缺少组合验证配置标识");
  // 固定清单保留所有票的追溯；重复 SHA 在执行序列去重。
  const inputs = o.manifest.inputs.filter((x, n, all) => all.findIndex(y => y.sha === x.sha) === n);
  for (const sha of [o.manifest.base, ...inputs.map(i => i.sha)]) if (git(o.cwd, ["rev-parse", `${sha}^{commit}`]).trim() !== sha) throw new Error("输入不是精确 commit");
  mkdirSync(dirname(o.checkpoint), { recursive: true });
  const lock = `${o.checkpoint}.lock`;
  const fd = openSync(lock, "wx", 0o600);
  writeFileSync(fd, JSON.stringify({ pid: process.pid, cwd: o.cwd })); closeSync(fd);
  try {
    let s: MergeState;
    let recoveredCommit = false;
    if (existsSync(o.checkpoint)) {
      s = JSON.parse(readFileSync(o.checkpoint, "utf8"));
      if (!equal(s.manifest, o.manifest) || s.cwd !== realpathSync(o.cwd) || s.checkId !== o.checkId) throw new Error("恢复合同/工位/检查配置改变");
      const current = snapshot(o.cwd, s.allowed);
      if (!equal(s.snapshot, current)) {
        // git commit 成功、写 checkpoint 前进程退出：只认已保存树、准确父集和本运行标记。
        const marker = `Sandcastle-Merge: ${o.manifest.runId}/${s.input}`;
        if (s.phase === "prepared" && s.tree && !current.status && !current.mergeHead
          && git(o.cwd, ["show", "-s", "--format=%P", "HEAD"]).trim() === `${s.before} ${s.input}`
          && git(o.cwd, ["rev-parse", "HEAD^{tree}"]).trim() === s.tree
          && git(o.cwd, ["show", "-s", "--format=%B", "HEAD"]).split("\n").includes(marker)) {
          s.phase = "committed"; s.snapshot = current; recoveredCommit = true;
        } else throw new Error("现场与检查点不一致，未知 WIP 保留给 owner");
      }
      if (s.phase === "merging") throw new Error("merge 状态转换中断，交 owner 核认，不猜恢复");
    } else {
      const current = snapshot(o.cwd);
      if (current.head !== o.manifest.base || current.status || current.mergeHead) throw new Error("归并起点不干净或不等于固定基线");
      s = { manifest: o.manifest, cwd: realpathSync(o.cwd), checkId: o.checkId, position: 0, phase: "ready", snapshot: current, entries: [], checks: [] };
    }
    const save = (phase: MergeState["phase"]) => {
      s.phase = phase; s.snapshot = snapshot(o.cwd, s.allowed);
      writeFileSync(`${o.checkpoint}.tmp`, JSON.stringify(s, null, 2), { mode: 0o600 });
      renameSync(`${o.checkpoint}.tmp`, o.checkpoint);
      o.onCheckpoint?.(structuredClone(s));
    };
    if (recoveredCommit) save("committed");
    const check = async (head: string) => {
      let result = s.checks.find(c => c.head === head && c.checkId === o.checkId && c.status === "passed");
      if (!result) {
        try { result = await o.verify(head, list(git(o.cwd, ["diff", "--name-only", "--no-renames", "-z", `${o.manifest.base}...${head}`]))); }
        catch (e) { result = { head, checkId: o.checkId, status: "failed", evidence: String(e) }; }
        assertBranch(o.cwd, o.manifest.branch);
        if (!equal(s.snapshot, snapshot(o.cwd, s.allowed))) throw new Error("验证改变工位，保留现场待核");
        if (result.head !== head || result.checkId !== o.checkId) throw new Error("组合验证结果 SHA/配置不匹配");
        s.checks.push(result);
      }
      if (result.status !== "passed") { save("failed"); throw new Error(`组合验证失败：${head}，停止后续输入`); }
      return result;
    };
    if (s.phase === "done") return s;
    while (s.position < inputs.length) {
      assertBranch(o.cwd, o.manifest.branch);
      const input = inputs[s.position]!;
      let kind: Entry["kind"] = "merged";
      if (s.phase === "ready") {
        o.admit?.(input);
        s.before = s.snapshot.head; s.input = input.sha; s.allowed = []; delete s.tree;
        let ancestor = false;
        try { git(o.cwd, ["merge-base", "--is-ancestor", input.sha, s.before]); ancestor = true; } catch { /* 非祖先进入真实归并 */ }
        if (ancestor) { kind = "already_integrated"; save("committed"); }
        else {
          save("merging");
          try { git(o.cwd, ["-c", "merge.autoStash=false", "merge", "--no-ff", "--no-commit", input.sha]); }
          catch (e) { if (optionalRef(o.cwd, "MERGE_HEAD") !== input.sha) throw e; }
          if (optionalRef(o.cwd, "MERGE_HEAD") !== input.sha || git(o.cwd, ["rev-parse", "HEAD"]).trim() !== s.before) throw new Error("merge 父集异常");
          s.allowed = list(git(o.cwd, ["diff", "--name-only", "--diff-filter=U", "-z"]));
          if (s.allowed.length) save("conflict"); else save("prepared");
        }
      } else if (s.snapshot.head === input.sha || (s.snapshot.head === s.before && !s.snapshot.mergeHead)) kind = "already_integrated";
      if (s.phase === "conflict") {
        // 同单票路径规则；保护目录即使冲突也不交工蜂越权编辑。
        parseStageReport(`<sandcastle-result>${JSON.stringify({ status: "ready", files: s.allowed, verification: [] })}</sandcastle-result>`);
        const before = s.snapshot;
        const outside = () => git(o.cwd, ["diff", "--binary", "--", ".", ...s.allowed!.map(p => `:(top,literal,exclude)${p}`)]);
        const outsideBefore = outside();
        const report = parseStageReport(await o.resolveConflict({ head: s.before!, incoming: input.sha, files: [...s.allowed!] }));
        if (report.status !== "ready") throw new Error("冲突工蜂 blocked，保留现场");
        const now = snapshot(o.cwd, s.allowed);
        const edited = s.allowed!.filter(p => before.files[p] !== now.files[p]).sort();
        const unknown = Object.keys(now.files).filter(p => !s.allowed!.includes(p));
        if (now.head !== before.head || now.mergeHead !== before.mergeHead || now.index !== before.index || outside() !== outsideBefore || !equal(unknown.map(p => [p, now.files[p]]), Object.keys(before.files).filter(p => !s.allowed!.includes(p)).map(p => [p, before.files[p]]))) throw new Error("冲突工蜂越过冻结边界，保留现场");
        if (!equal([...report.files].sort(), edited)) throw new Error("冲突实际编辑与报告不一致");
        git(o.cwd, ["--literal-pathspecs", "add", "--", ...s.allowed!]);
        if (git(o.cwd, ["ls-files", "--unmerged"]).trim()) throw new Error("仍有未解决索引项");
        save("prepared");
      }
      if (s.phase === "prepared") {
        // 冲突工蜂可能等待很久；提交前重新复核来源，失效则停在已保存的 prepared 检查点，MERGE_HEAD 保留。
        o.admit?.(input);
        assertHostPackagingSafe(o.cwd); // #1742：容器可写的仓内属性不得触发宿主命令型 filter/diff/merge 驱动
        if (s.snapshot.head !== s.before || s.snapshot.mergeHead !== input.sha) throw new Error("提交前归并父集不匹配");
        git(o.cwd, ["diff", "--cached", "--check"]);
        s.tree = git(o.cwd, ["write-tree"]).trim(); save("prepared");
        const provenance = input.source === "deepagents" ? [`\nMerge-Source: deepagents ticket=${input.issue} run=${input.run.id} gen=${input.run.generation} branch=${input.branch}`] : [];
        git(o.cwd, ["commit", "-m", `merge(sandcastle): #${input.issue}——受管功能归并`, "-m", `编号声明：#${input.issue} 指 GitHub Issue。\nSandcastle-Merge: ${o.manifest.runId}/${input.sha}${provenance.join("")}`]);
        if (git(o.cwd, ["show", "-s", "--format=%P", "HEAD"]).trim() !== `${s.before} ${input.sha}` || git(o.cwd, ["rev-parse", "HEAD^{tree}"]).trim() !== s.tree) throw new Error("归并提交父集/树不匹配");
        if (git(o.cwd, ["status", "--porcelain"]).trim()) throw new Error("提交后出现未知 WIP");
        save("committed");
      }
      const result = await check(s.snapshot.head);
      // 成功记账前复核到目前为止的全部输入，验证期间来源失效则停在 committed，不记 done。
      for (const prior of inputs.slice(0, s.position + 1)) o.admit?.(prior);
      s.entries.push({ sha: input.sha, head: s.snapshot.head, kind, check: result });
      s.position++; s.allowed = []; delete s.before; delete s.input; delete s.tree;
      save(s.position === inputs.length ? "done" : "ready");
    }
    return s;
  } finally { unlinkSync(lock); }
}
