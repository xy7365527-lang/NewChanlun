import { execFileSync } from "node:child_process";
import { lstatSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";

export type CanonicalEdit =
  | { version: 1; operation: "create"; path: string; content: string }
  | { version: 1; operation: "edit"; path: string; replacements: Array<{ before: string; after: string }> };
export interface CanonicalPlan {
  edit: CanonicalEdit;
  sourceCommit: string;
  before: string | null;
  after: string;
  diff: string;
}

export function canonicalPath(path: string): boolean {
  if (!path || path.includes("\\") || /[\x00-\x1f\x7f]/.test(path) || path.split("/").some((s) => !s || s === "." || s === "..")) return false;
  return path === "CONTEXT.md" || /^\.chanlun\/definitions\/.+/.test(path) || /^docs\/adr\/.+/.test(path);
}

export function parseCanonicalEdit(draft: string, target: string): CanonicalEdit {
  const fences = [...draft.matchAll(/^```canonical_edit\s*\r?\n([\s\S]*?)^```\s*$/gm)];
  const openings = [...draft.matchAll(/^```canonical_edit[^\r\n]*$/gm)];
  if (openings.length !== 1 || fences.length !== 1) throw new Error("必须有且只有一个 canonical_edit 代码块");
  const value: unknown = JSON.parse(fences[0][1]);
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("canonical_edit 必须是对象");
  const j = value as Record<string, unknown>;
  if (j.version !== 1 || typeof j.path !== "string" || j.path !== target || !canonicalPath(j.path)) throw new Error("canonical_edit 版本或正本落点不一致");
  const keys = Object.keys(j).sort().join(",");
  if (j.operation === "create" && keys === "content,operation,path,version" && typeof j.content === "string") {
    return { version: 1, operation: "create", path: j.path, content: j.content };
  }
  if (j.operation === "edit" && keys === "operation,path,replacements,version" && Array.isArray(j.replacements) && j.replacements.length > 0) {
    const replacements = j.replacements.map((v: unknown) => {
      if (!v || typeof v !== "object" || Array.isArray(v)) throw new Error("replacement 必须是 before/after 对象");
      const r = v as Record<string, unknown>;
      if (Object.keys(r).sort().join(",") !== "after,before" || typeof r.before !== "string" || !r.before || typeof r.after !== "string" || r.before === r.after) throw new Error("replacement 必须有非空 before 与不同的 after");
      return { before: r.before, after: r.after };
    });
    return { version: 1, operation: "edit", path: j.path, replacements };
  }
  throw new Error("canonical_edit 动作或字段非法");
}

export function applyCanonicalEdit(edit: CanonicalEdit, before: string | null): string {
  if (edit.operation === "create") {
    if (before !== null) throw new Error("create 目标已存在，必须改为 edit 并重新独审");
    return edit.content;
  }
  if (before === null) throw new Error("edit 目标不存在");
  let after = before;
  for (const r of edit.replacements) {
    const at = after.indexOf(r.before);
    if (at < 0 || after.indexOf(r.before, at + 1) >= 0) throw new Error("before 必须精确且唯一命中一次");
    after = after.slice(0, at) + r.after + after.slice(at + r.before.length);
  }
  if (after === before) throw new Error("canonical_edit 没有产生改动");
  return after;
}

export function readCanonicalTarget(root: string, target: string): string | null {
  if (!canonicalPath(target)) throw new Error("正本路径越界");
  let path = resolve(root);
  if (lstatSync(path).isSymbolicLink()) throw new Error("正本 worktree 不得是符号链接");
  const parts = target.split("/");
  for (let i = 0; i < parts.length; i++) {
    path = join(path, parts[i]);
    let stat;
    try { stat = lstatSync(path); } catch (e) {
      if ((e as NodeJS.ErrnoException).code === "ENOENT") return null;
      throw e;
    }
    if (stat.isSymbolicLink()) throw new Error("正本路径含符号链接");
    if (i < parts.length - 1 && !stat.isDirectory()) throw new Error("正本父路径不是目录");
    if (i === parts.length - 1 && !stat.isFile()) throw new Error("正本目标不是普通文件");
  }
  const bytes = readFileSync(path);
  const text = bytes.toString("utf8");
  if (!Buffer.from(text, "utf8").equals(bytes)) throw new Error("正本不是有效 UTF-8");
  return text;
}

export function canonicalDiff(target: string, before: string | null, after: string): string {
  const dir = mkdtempSync(join(tmpdir(), "canonical-diff-"));
  const old = join(dir, "before");
  const next = join(dir, "after");
  try {
    writeFileSync(old, before ?? "");
    writeFileSync(next, after);
    let diff: string;
    try { diff = execFileSync("git", ["diff", "--no-index", "--no-ext-diff", "--no-color", "--", before === null ? "/dev/null" : "before", "after"], { cwd: dir, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }); }
    catch (e) {
      const failure = e as { status?: number; stdout?: string };
      if (failure.status !== 1 || typeof failure.stdout !== "string") throw e;
      diff = failure.stdout;
    }
    return diff.split("\n").map((line) => {
      if (line === "diff --git a/before b/after" || line === "diff --git a/after b/after") return `diff --git a/${target} b/${target}`;
      if (line === "--- a/before") return `--- a/${target}`;
      if (line === "+++ b/after") return `+++ b/${target}`;
      return line;
    }).join("\n");
  } finally { rmSync(dir, { recursive: true, force: true }); }
}

export function validateCanonicalPlan(plan: CanonicalPlan, edit: CanonicalEdit): void {
  if (JSON.stringify(plan.edit) !== JSON.stringify(edit) || !/^[a-f0-9]{40,64}$/.test(plan.sourceCommit) ||
      (plan.before !== null && typeof plan.before !== "string") || typeof plan.after !== "string" ||
      applyCanonicalEdit(edit, plan.before) !== plan.after || canonicalDiff(edit.path, plan.before, plan.after) !== plan.diff) {
    throw new Error("获审 canonical_edit 与断点不一致，必须重新起草和独审");
  }
}

export function writeCanonicalPlan(root: string, plan: CanonicalPlan): void {
  const target = plan.edit.path;
  if (readCanonicalTarget(root, target) !== plan.before) throw new Error("正本目标原件已漂移，必须重新起草和独审");
  const path = join(root, target);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, plan.after, { flag: plan.edit.operation === "create" ? "wx" : "w" });
}
