import assert from "node:assert/strict";
import { test } from "node:test";
import { mkdtempSync, mkdirSync, readFileSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { canonicalPath, parseCanonicalEdit, applyCanonicalEdit, readCanonicalTarget, writeCanonicalPlan, canonicalDiff } from "./canonical-edit.mts";

const fence = (value: unknown) => "```canonical_edit\n" + JSON.stringify(value) + "\n```\n";
const create = (path: string, content = "新\n") => ({ version: 1, operation: "create", path, content });

test("精确编辑边界拒绝穿越、绝对路径、反斜线和不可见控制符", () => {
  for (const p of ["/CONTEXT.md", "../CONTEXT.md", "docs/adr/../../a", "docs/adr//a", "docs/adr/./a", "docs/adr/a\\b", "docs/adr/a\0b", "docs/adr/a\nb", "CONTEXT.md/x", "rust/src/a"]) {
    assert.equal(canonicalPath(p), false, p);
    assert.throws(() => parseCanonicalEdit(fence(create(p)), p));
  }
  for (const p of ["CONTEXT.md", "docs/adr/a.md", ".chanlun/definitions/a.md"]) assert.equal(canonicalPath(p), true);
});

test("解析器拒绝多个对象、残缺第二个对象与多余字段", () => {
  const one = fence(create("CONTEXT.md"));
  assert.throws(() => parseCanonicalEdit(one + one, "CONTEXT.md"));
  assert.throws(() => parseCanonicalEdit(one + "```canonical_edit\n{}", "CONTEXT.md"));
  assert.throws(() => parseCanonicalEdit(fence({ ...create("CONTEXT.md"), fallback: true }), "CONTEXT.md"));
});

test("替换逐次唯一匹配，保留前后字节，允许删除和空文件新建", () => {
  const edit = parseCanonicalEdit(fence({ version: 1, operation: "edit", path: "CONTEXT.md", replacements: [{ before: "旧", after: "中" }, { before: "中", after: "" }] }), "CONTEXT.md");
  assert.equal(applyCanonicalEdit(edit, "前\r\n旧\r\n后"), "前\r\n\r\n后");
  assert.throws(() => applyCanonicalEdit(edit, "旧旧"));
  assert.throws(() => applyCanonicalEdit(edit, null));
  const empty = parseCanonicalEdit(fence(create("CONTEXT.md", "")), "CONTEXT.md");
  assert.equal(applyCanonicalEdit(empty, null), "");
  assert.throws(() => applyCanonicalEdit(empty, ""));
});

test("父目录与目标符号链接均拒绝，外部原件不被修改", () => {
  const root = mkdtempSync(join(tmpdir(), "f2-boundary-"));
  const outside = mkdtempSync(join(tmpdir(), "f2-outside-"));
  writeFileSync(join(outside, "a.md"), "外部原件\n");
  mkdirSync(join(root, "docs"));
  symlinkSync(outside, join(root, "docs/adr"));
  assert.throws(() => readCanonicalTarget(root, "docs/adr/a.md"), /符号链接/);
  symlinkSync(join(outside, "a.md"), join(root, "CONTEXT.md"));
  assert.throws(() => readCanonicalTarget(root, "CONTEXT.md"), /符号链接/);
  const edit = parseCanonicalEdit(fence(create("CONTEXT.md")), "CONTEXT.md");
  assert.throws(() => writeCanonicalPlan(root, { edit, sourceCommit: "a".repeat(40), before: null, after: "新\n", diff: canonicalDiff("CONTEXT.md", null, "新\n") }), /符号链接/);
  assert.equal(readFileSync(join(outside, "a.md"), "utf8"), "外部原件\n");
});

test("无效 UTF-8 不作有损转换，文件缺失与空文件有不同身份", () => {
  const root = mkdtempSync(join(tmpdir(), "f2-bytes-"));
  assert.equal(readCanonicalTarget(root, "CONTEXT.md"), null);
  writeFileSync(join(root, "CONTEXT.md"), "");
  assert.equal(readCanonicalTarget(root, "CONTEXT.md"), "");
  writeFileSync(join(root, "CONTEXT.md"), Buffer.from([0xff]));
  assert.throws(() => readCanonicalTarget(root, "CONTEXT.md"), /UTF-8/);
});

test("diff 只改文件头，不改正文中的临时文件名", () => {
  const diff = canonicalDiff("CONTEXT.md", "a/before\n", "a/after b/after\n");
  assert.ok(diff.includes("--- a/CONTEXT.md\n+++ b/CONTEXT.md\n"));
  assert.ok(diff.includes("-a/before\n+a/after b/after\n"));
});
