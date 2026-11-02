import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";

const repoRoot = join(__dirname, "..");
const entrypoint = join(
  repoRoot,
  ".sandcastle/agent-workflows/update-branch/update-branch.ts",
);
const tsx = join(repoRoot, "node_modules", ".bin", "tsx");

type GitShape = "up-to-date" | "clean" | "conflict";

function runEntrypoint(shape: GitShape, legacy = false) {
  const dir = mkdtempSync(join(tmpdir(), `update-branch-${shape}-`));
  try {
    const bin = join(dir, "bin");
    const output = join(dir, "output");
    const gitCalls = join(dir, "git-calls.log");
    mkdirSync(bin, { recursive: true });

    const fakeGit = `#!/bin/sh
printf '%s\\n' "$*" >> "$GIT_CALL_LOG"
case "$1" in
  fetch) exit 0 ;;
  rev-parse)
    if [ "$2" = "HEAD" ]; then printf '%s\\n' head-sha; else printf '%s\\n' base-sha; fi
    exit 0 ;;
  merge-base)
    ${shape === "up-to-date" ? "printf '%s\\n' base-sha" : "printf '%s\\n' merge-base-sha"}
    exit 0 ;;
  merge)
    ${shape === "conflict" ? "exit 1" : "exit 0"}
    ;;
  diff)
    ${shape === "conflict" ? "printf '%s\\n' conflict.txt" : ":"}
    exit 0 ;;
  push) exit 0 ;;
  *) exit 99 ;;
esac
`;
    const git = join(bin, "git");
    writeFileSync(git, fakeGit);
    chmodSync(git, 0o755);

    const result = spawnSync(tsx, [entrypoint], {
      cwd: dir,
      encoding: "utf8",
      timeout: 30_000,
      env: {
        ...process.env,
        PATH: `${bin}:${process.env.PATH ?? ""}`,
        GIT_CALL_LOG: gitCalls,
        EVENT_PAYLOAD: legacy ? '["agent:model:deepseek-v4-pro"]' : "[]",
        ...(legacy ? { DEEPSEEK_API_KEY: "fixture-secret" } : {}),
        PR_NUMBER: "1701",
        BRANCH: "codex/issue-1617-kimi-main-model",
        BASE_REF: "main",
        OUTPUT_DIR: output,
      },
    });

    assert.equal(result.error, undefined, `${shape}: process must finish`);
    if (legacy) {
      assert.equal(result.status, 0, `${shape}: explicit legacy route must run\n${result.stdout}\n${result.stderr}`);
      assert.equal(readFileSync(join(output, "should_push.txt"), "utf8"), "false");
      assert.match(readFileSync(join(output, "comment.md"), "utf8"), /already up to date/);
      assert.match(readFileSync(gitCalls, "utf8"), /fetch origin main/);
      return;
    }
    assert.equal(result.status, 1, `${shape}: retired entry must fail\n${result.stdout}\n${result.stderr}`);
    const failure = readFileSync(join(output, "failure_reason.txt"), "utf8");
    assert.match(failure, /Hosted agent default is retired \(#1617\)/, shape);
    assert.equal(existsSync(join(output, "should_push.txt")), false, `${shape}: no push handoff`);
    assert.equal(existsSync(join(output, "comment.md")), false, `${shape}: no success comment`);
    assert.equal(existsSync(gitCalls), false, `${shape}: no fetch/merge/push or other git call`);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

for (const shape of ["up-to-date", "clean", "conflict"] as const) {
  test(`#1617 update-branch 入口在 ${shape} 路径先退役，再接触 Git`, () => {
    runEntrypoint(shape);
  });
}

test("update-branch 显式 DeepSeek legacy 路线不被退役默认吞掉", () => {
  runEntrypoint("up-to-date", true);
});
