/** #1617：工蜂保持 workspace-write；宿主仅封包本票显式产物。 */
import { execFileSync } from "node:child_process";
import { isAbsolute, normalize } from "node:path";
import { realpathSync } from "node:fs";
export interface ManagedStage { cwd: string; branch: string; head: string; issue: number; phase: "implementer" | "reviewer" }
export interface StageReport { status: "ready" | "blocked"; files: string[]; verification: string[] }
const git = (cwd: string, args: string[]) => execFileSync("git", args, { cwd, encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
const paths = (raw: string) => raw.split("\0").filter(Boolean).sort();
const changed = (cwd: string) => [...new Set([...paths(git(cwd, ["diff", "HEAD", "--name-only", "--no-renames", "-z"])), ...paths(git(cwd, ["ls-files", "--others", "--exclude-standard", "-z"]))])].sort();
export function beginManagedStage(cwd: string, branch: string, issue: number, phase: ManagedStage["phase"]): ManagedStage {
  if (!Number.isSafeInteger(issue) || issue <= 0 || branch !== `sandcastle/issue-${issue}`) throw new Error("受管提交只接受本票分支");
  if (git(cwd, ["symbolic-ref", "--short", "HEAD"]).trim() !== branch) throw new Error("工位分支不匹配");
  if (realpathSync(git(cwd, ["rev-parse", "--show-toplevel"]).trim()) !== realpathSync(cwd)) throw new Error("工位必须为仓库根");
  if (changed(cwd).length || git(cwd, ["diff", "--cached", "--name-only"]).trim()) throw new Error("阶段开始前工位不干净，保留现有改动");
  return { cwd: realpathSync(cwd), branch, head: git(cwd, ["rev-parse", "HEAD"]).trim(), issue, phase };
}
export function parseStageReport(stdout: string): StageReport {
  const matches = [...stdout.matchAll(/<sandcastle-result>([\s\S]*?)<\/sandcastle-result>/g)];
  if (matches.length !== 1) throw new Error("缺少唯一 sandcastle-result 阶段报告");
  const r = JSON.parse(matches[0]![1]!);
  if (!r || !["ready", "blocked"].includes(r.status) || !Array.isArray(r.files) || !Array.isArray(r.verification) || !r.verification.every((x: unknown) => typeof x === "string")) throw new Error("阶段报告结构无效");
  for (const file of r.files) if (typeof file !== "string" || !file || isAbsolute(file) || normalize(file) !== file || file.split("/").some((p: string) => ["..", ".git", ".agents", ".codex", ".aws"].includes(p))) throw new Error("阶段报告包含无效或受保护路径");
  if (new Set(r.files).size !== r.files.length) throw new Error("阶段报告文件重复");
  return r;
}
export function finishManagedStage(stage: ManagedStage, stdout: string): { head: string; committed: boolean; report: StageReport } {
  const report = parseStageReport(stdout);
  if (report.status !== "ready") throw new Error("工蜂报告 blocked；未提交，保留工位");
  const { cwd, branch, issue, phase } = stage;
  if (git(cwd, ["symbolic-ref", "--short", "HEAD"]).trim() !== branch) throw new Error("提交前分支改变");
  const head = git(cwd, ["rev-parse", "HEAD"]).trim();
  const marker = `Sandcastle-Stage: ${issue}/${phase}/${stage.head}`;
  // 已完成同一封包后重入：只认直接父提交、阶段标记、完整文件集合与干净工位。
  if (head !== stage.head) {
    const parents = git(cwd, ["show", "-s", "--format=%P", head]).trim();
    const message = git(cwd, ["show", "-s", "--format=%B", head]);
    const files = paths(git(cwd, ["diff-tree", "--no-commit-id", "--name-only", "--no-renames", "-r", "-z", head]));
    if (parents === stage.head && message.split("\n").includes(marker) && JSON.stringify(files) === JSON.stringify([...report.files].sort()) && !changed(cwd).length && !git(cwd, ["diff", "--cached", "--name-only"]).trim()) return { head, committed: true, report };
    throw new Error("阶段 HEAD 改变，拒绝提交或猜测恢复");
  }
  const actual = changed(cwd);
  if (JSON.stringify(actual) !== JSON.stringify([...report.files].sort())) throw new Error("实际变更与工蜂文件清单不一致，保留全部改动");
  if (!actual.length) return { head, committed: false, report };
  const staged = paths(git(cwd, ["diff", "--cached", "--name-only", "--no-renames", "-z"]));
  if (staged.some(file => !actual.includes(file))) throw new Error("索引含其他改动");
  git(cwd, ["--literal-pathspecs", "add", "--", ...actual]);
  git(cwd, ["diff", "--cached", "--check"]);
  if (git(cwd, ["rev-parse", "HEAD"]).trim() !== stage.head) throw new Error("提交前基线改变");
  git(cwd, ["commit", "-m", `${phase === "reviewer" ? "review" : "feat"}(sandcastle): #${issue}——${phase} 受管封包`, "-m", `编号声明：本提交中的 #${issue} 仅指 GitHub Issue。\n\n${marker}`]);
  const committedHead = git(cwd, ["rev-parse", "HEAD"]).trim();
  if (changed(cwd).length || git(cwd, ["diff", "--cached", "--name-only"]).trim()) throw new Error("提交后仍有变更，保留工位待核");
  return { head: committedHead, committed: true, report };
}

/** 只验已封包的精确提交；不能执行工蜂返回的命令。 */
export function verifyManagedRust(stage: ManagedStage, sealed: ReturnType<typeof finishManagedStage>, run = (ref: string) => {
  const verificationRef = `refs/sandcastle-verification/issue-${stage.issue}/${ref}`;
  git(stage.cwd, ["update-ref", verificationRef, ref]);
  try { execFileSync(`${process.env.HOME}/.local/bin/nc-remote`, [verificationRef, "rust", "cargo", "check", "--locked"], {
    cwd: stage.cwd, env: { ...process.env, NC_REPO: stage.cwd, NC_TARGET: `sandcastle-issue-${stage.issue}` },
    timeout: 600_000, maxBuffer: 16 * 1024 * 1024, stdio: ["ignore", "pipe", "pipe"],
  }); } finally { git(stage.cwd, ["update-ref", "-d", verificationRef, ref]); }
}): string {
  if (!sealed.report.files.some(file => file.startsWith("rust/") || /(^|\/)Cargo\.(toml|lock)$/.test(file))) return "Rust 未改动，无新增远端检查";
  try { run(sealed.head); return `nc-remote cargo check --locked 通过：${sealed.head}`; }
  catch { return `Rust 未验证或远端检查失败：${sealed.head}；需 nc-remote/CI 成功证据，不能验收合入`; }
}
