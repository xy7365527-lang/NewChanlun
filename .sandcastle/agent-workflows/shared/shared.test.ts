import { test } from "node:test";
import assert from "node:assert/strict";
import { parseDiffLines } from "./diff-lines.ts";
import {
  filterInlineComments,
  filterReplies,
  reviewOutputSchema,
  type InlineComment,
} from "./review-output.ts";
import { standardSchema, asRecord, asString } from "./common.ts";

test("parseDiffLines maps added/context new-file line numbers", () => {
  const diff = [
    "diff --git a/foo.ts b/foo.ts",
    "index 0000000..1111111 100644",
    "--- a/foo.ts",
    "+++ b/foo.ts",
    "@@ -1,3 +1,4 @@",
    " line1",
    "+added",
    " line2",
    "-removed",
    " line3",
  ].join("\n");
  const lines = parseDiffLines(diff);
  assert.deepEqual([...lines.get("foo.ts") ?? []].sort((a, b) => a - b), [
    1, 2, 3, 4,
  ]);
});

test("parseDiffLines skips removed lines and handles multiple files", () => {
  const diff = [
    "+++ b/a.ts",
    "@@ -1 +1 @@",
    "+only-added",
    "+++ b/b.ts",
    "@@ -5 +5 @@",
    "-only-removed",
    " kept",
  ].join("\n");
  const lines = parseDiffLines(diff);
  assert.deepEqual([...(lines.get("a.ts") ?? [])], [1]);
  assert.deepEqual([...(lines.get("b.ts") ?? [])], [5]);
});

test("filterInlineComments drops comments outside the diff", () => {
  const diffLines = new Map<string, Set<number>>([
    ["a.ts", new Set([10, 11])],
  ]);
  const comments: InlineComment[] = [
    { path: "a.ts", line: 10, body: "ok" },
    { path: "missing.ts", line: 10, body: "file not in diff" },
    { path: "a.ts", line: 99, body: "line not in diff" },
  ];
  const kept = filterInlineComments(comments, diffLines);
  assert.deepEqual(
    kept.map((c) => c.body),
    ["ok"],
  );
});

test("filterReplies drops replies to unknown comment ids", () => {
  const valid = new Set(["id-1"]);
  const replies = [
    { commentId: "id-1", body: "kept" },
    { commentId: "id-2", body: "dropped" },
  ];
  assert.deepEqual(filterReplies(replies, valid), [
    { commentId: "id-1", body: "kept" },
  ]);
});

test("reviewOutputSchema validates shape and coerces inline comments", () => {
  const parsed = reviewOutputSchema["~standard"].validate({
    summary: "a summary",
    inlineComments: [{ path: "a.ts", line: 3, body: "b" }],
    replies: [],
  });
  assert.ok("value" in parsed);
  assert.equal(parsed.value.summary, "a summary");
  assert.equal(parsed.value.inlineComments[0].path, "a.ts");
});

test("reviewOutputSchema rejects a missing summary", () => {
  const parsed = reviewOutputSchema["~standard"].validate({
    inlineComments: [],
    replies: [],
  });
  assert.ok("issues" in parsed);
});

test("standardSchema helper returns issues on validation failure", () => {
  const schema = standardSchema((v) => asString(asRecord(v, "r").name, "name"));
  const ok = schema["~standard"].validate({ name: "x" });
  const bad = schema["~standard"].validate({});
  assert.ok("value" in ok);
  assert.ok("issues" in bad);
});
