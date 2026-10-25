// harvest.test.mts —— 把 harvest.sh 的 bash 离线回归（harvest.test.sh）接进 CI sandcastle-ts job（#1660）。
// 该 job 已运行 `npx tsx --test .sandcastle/*.test.mts`；不另改 workflow。
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));

test("harvest.sh PR 模式：推分支+开 PR+票面登记，不关票不合入不推 main 不跑 cargo", () => {
  const r = spawnSync("bash", [join(here, "harvest.test.sh")], { encoding: "utf8", timeout: 120_000 });
  const out = `${r.stdout ?? ""}${r.stderr ?? ""}`;
  assert.equal(r.status, 0, out);
  assert.match(out, /harvest\.test\.sh: pass=\d+ fail=0/, out);
});
