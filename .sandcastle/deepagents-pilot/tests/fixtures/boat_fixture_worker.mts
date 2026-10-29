// No-model fixture for live Boat supervision tests. It replaces boat-worker.mts inside a test package only:
// same request, same receipt protocol, fixed file edits instead of a model, and fixture=true in the record.
import { execFileSync } from "node:child_process";
import { appendFileSync, chmodSync, existsSync, lstatSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { setTimeout as sleep } from "node:timers/promises";
import { snapshotCheckout } from "./codex-child.ts";

const requestPath = process.argv[2]!;
const request = JSON.parse(readFileSync(requestPath, "utf8"));
const root: string = request.root;
const checkout = join(root, "repo");
const claim = join(root, "claim");
const control = JSON.parse(readFileSync(join(root, "runtime/fixture-mode.json"), "utf8"));
const save = (path: string, data: unknown) => {
  writeFileSync(path + ".tmp", JSON.stringify(data) + "\n", { mode: 0o600 });
  renameSync(path + ".tmp", path);
};
appendFileSync(join(root, "../fixture-launches.log"), `${request.identity.role} ${process.pid} ${new Date().toISOString()}\n`);
const keyFile = join(root, "../private/model-key");
const keyPresent = readFileSync(keyFile, "utf8").length > 0;
rmSync(keyFile, { force: true });
const before = snapshotCheckout(checkout);
await sleep(control.sleepSeconds * 1000);
if (request.mode === "execute") {
  writeFileSync(join(checkout, "demo/中文 定义.md"), "夹具修改：非 ASCII 路径\n");
  writeFileSync(join(checkout, "demo/新建 文件.md"), "夹具新建\n");
  rmSync(join(checkout, "demo/删除我.md"));
  chmodSync(join(checkout, "demo/keep.sh"), 0o755);
}
const after = snapshotCheckout(checkout);
const status = execFileSync("git", ["-C", checkout, "status", "--porcelain=v1", "-z", "--untracked-files=all"], { encoding: "utf8" }).split("\0");
const files: Array<{ path: string; mode: string; content?: string }> = [];
for (let i = 0; i < status.length; i++) {
  const entry = status[i]; if (!entry) continue;
  const path = entry.slice(3);
  if (entry[0] === "R" || entry[0] === "C") i++;
  const file = join(checkout, path);
  if (!existsSync(file)) { files.push({ path, mode: "deleted" }); continue; }
  files.push({ path, mode: lstatSync(file).mode & 0o111 ? "100755" : "100644", content: readFileSync(file).toString("base64") });
}
const session = join(claim, "session.jsonl");
writeFileSync(session, JSON.stringify({ type: "turn_context", payload: { model: request.model }, fixture: true }) + "\n");
save(join(claim, "receipt.json"), {
  identity: request.identity, package_id: request.package_id, state: "terminal", exit_code: 0, files,
  record: {
    schema_version: 1, run_id: randomUUID(), ticket: request.ticket, status: "completed", mode: request.mode,
    model: request.model, actual_model: request.model, fixture: true, realModel: false, key_present: keyPresent,
    expected_head: request.identity.target_head, checkout_before: before, checkout_after: after,
    external_session_id: "fixture-" + request.identity.role, external_session_log: session, failure_reason: null,
    result: { status: "completed", summary: request.mode === "review" ? "VERDICT: PASS\n夹具独审，无模型" : "夹具作者完成",
              findings: [], validation: [] },
  },
});
