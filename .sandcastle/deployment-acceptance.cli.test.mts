/** #1867 正式 CLI 组合（固定生产路径）：只在专用测试机上、以独立 UID 拥有固定 drain 根并放置 fixture 标记时运行。
 * 用真实的 drain-control.py、真实 receipt 目录（passwd 主目录）、真实模块检出身份；docker/gh 是记录调用的替身。 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { chmodSync, existsSync, lstatSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { DRAIN_ROOT } from "./drain-control.mts";
import { acceptanceRoot } from "./acceptance-dir.mts";
import { moduleRepo } from "./docker-acceptance.mts";

const MARKER = join(DRAIN_ROOT, ".test-fixture-1867");
const skip = process.env.NC1867_FIXED_ROOT_FIXTURE !== "1" ? "仅在专用测试机设 NC1867_FIXED_ROOT_FIXTURE=1 时运行（会改写固定 drain 根）"
  : !existsSync(MARKER) || lstatSync(DRAIN_ROOT).uid !== process.getuid?.() ? "固定 drain 根不是本测试 UID 的 fixture" : false;
const here = dirname(fileURLToPath(import.meta.url));
const TICKET = 18671;
const IMG_ID = "sha256:" + "1".repeat(64);

test("正式 CLI 组合：签发只认 draining；专用 run 参数白名单、身份负控、预检失败零消费；普通 CLI 冻结 75、open 时不能消费冻结 receipt", { skip, timeout: 240_000 }, (t) => {
  const repo = moduleRepo();
  const git = (...args: string[]) => execFileSync("git", ["-C", repo, ...args], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  assert.equal(git("status", "--porcelain", "--", ".sandcastle"), "", "模块检出必须干净");
  const head = git("rev-parse", "HEAD");
  const root = mkdtempSync(join(tmpdir(), "nc-1867-cli-"));
  const bin = join(root, "bin"); execFileSync("mkdir", [bin]);
  const calls = join(root, "calls");
  writeFileSync(join(bin, "docker"), `#!/bin/sh\nprintf 'docker %s\\n' "$*" >> ${calls}\ncase "$1 $2" in "image inspect") printf '%s\\t[]\\n' '${IMG_ID}'; exit 0 ;; esac\nexit 42\n`);
  writeFileSync(join(bin, "gh"), `#!/bin/sh\nprintf 'gh %s\\n' "$*" >> ${calls}\nexit 42\n`);
  chmodSync(join(bin, "docker"), 0o755); chmodSync(join(bin, "gh"), 0o755);
  const env = { PATH: `${bin}:${dirname(process.execPath)}:/usr/bin:/bin`, HOME: process.env.HOME ?? "", LANG: "C.UTF-8", TZ: "UTC" };
  const tsx = (file: string, args: string[], extra: Record<string, string> = {}) => spawnSync(process.execPath, ["--import", "tsx", join(here, file), ...args], { cwd: repo, env: { ...env, ...extra }, encoding: "utf8", timeout: 120_000 });
  const drain = (...args: string[]) => execFileSync("python3", [join(here, "drain-control.py"), ...args], { encoding: "utf8" });
  const called = () => (existsSync(calls) ? readFileSync(calls, "utf8").trim().split("\n").filter(Boolean) : []);
  const acc = acceptanceRoot();
  const claims = () => (existsSync(acc) ? readdirSync(acc).filter((f) => /^(used|resume|cp)-/.test(f)) : []);
  // fixture 伙伴：在更早的提交上只新增探针文档的独立分支（父不等于部署 head）。
  const partnerTree = join(root, "partner");
  git("worktree", "add", "-q", "-b", "sandcastle/issue-1840", partnerTree, "HEAD~1");
  execFileSync("mkdir", ["-p", join(partnerTree, "docs/agents")]);
  writeFileSync(join(partnerTree, "docs/agents/sandcastle-docker-acceptance-1742.md"), "# 探针\n伙伴\n写入方：宿主冲突伙伴\n");
  execFileSync("git", ["-C", partnerTree, "add", "."]); execFileSync("git", ["-C", partnerTree, "-c", "user.name=t", "-c", "user.email=t@example.invalid", "commit", "-qm", "partner"]);
  const partnerSha = execFileSync("git", ["-C", partnerTree, "rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  git("worktree", "remove", partnerTree);
  t.after(() => { tsx("docker-acceptance.mts", ["retire"]); try { git("branch", "-D", "sandcastle/issue-1840"); } catch { /* 已删除 */ } rmSync(root, { recursive: true, force: true }); writeFileSync(join(DRAIN_ROOT, "state.json"), JSON.stringify({ v: 1, mode: "open", generation: 0 }) + "\n"); });
  const review = join(root, "partner-review.md"); writeFileSync(review, `VERDICT: PASS\n${partnerSha}\n`);
  const partnerInput = join(root, "partner.json");
  writeFileSync(partnerInput, JSON.stringify({ issue: 1840, branch: "sandcastle/issue-1840", sha: partnerSha, parent: git("rev-parse", "HEAD~2"), review, approval: "fixture-approval" }));

  writeFileSync(join(DRAIN_ROOT, "state.json"), JSON.stringify({ v: 1, mode: "open", generation: 0 }) + "\n");
  tsx("docker-acceptance.mts", ["retire"]);
  const issueArgs = ["issue-frozen", "--image", "sandcastle:newchanlun", "--swarm-ticket", String(TICKET), "--generation", "1", "--approval", "fixture-1867", "--partner-input", partnerInput];
  const wrongParent = tsx("docker-acceptance.mts", issueArgs);
  assert.equal(wrongParent.status, 1, wrongParent.stderr); assert.match(wrongParent.stderr, /唯一父与批准输入不符/, "伙伴父取自批准输入，填错即拒绝");
  writeFileSync(partnerInput, JSON.stringify({ issue: 1840, branch: "sandcastle/issue-1840", sha: partnerSha, parent: git("rev-parse", "HEAD~1"), review, approval: "fixture-approval" }));
  const openIssue = tsx("docker-acceptance.mts", issueArgs);
  assert.equal(openIssue.status, 75, openIssue.stderr); assert.equal(existsSync(join(acc, "receipt.json")), false);
  drain("freeze");
  const issued = tsx("docker-acceptance.mts", issueArgs);
  assert.equal(issued.status, 0, issued.stderr);
  const nonce = JSON.parse(issued.stdout).nonce as string;
  const sel = ["--ticket", String(TICKET), "--head", head, "--nonce", nonce];

  const extraArg = tsx("deployment-acceptance.mts", ["run", ...sel, "--row", "13", "--root", "/tmp/x"]);
  assert.equal(extraArg.status, 64, extraArg.stderr); assert.match(extraArg.stderr, /不接受的参数 --root/);
  const wrongNonce = tsx("deployment-acceptance.mts", ["run", "--ticket", String(TICKET), "--head", head, "--nonce", "00000000-0000-4000-8000-000000000000", "--row", "13"]);
  assert.equal(wrongNonce.status, 1); assert.match(wrongNonce.stderr, /不是当前冻结验收轮/);
  assert.deepEqual(claims(), []);

  const issueCalls = called().length; // 签发本身解析镜像 Id（docker image inspect）
  const legacy = tsx("main-swarm.mts", [], { SANDCASTLE_DOCKER_ACCEPTANCE_ROW: "13", SANDCASTLE_DOCKER_ACCEPTANCE_TICKET: String(TICKET), CLIPROXY_LOCAL_KEY: "fixture-no-network" });
  assert.equal(legacy.status, 75, legacy.stderr + legacy.stdout);
  assert.deepEqual(claims(), [], "普通旧调用冻结时零消费");
  assert.deepEqual(called().slice(issueCalls), [], "普通旧调用冻结时不碰 docker/gh");

  const probeFail = tsx("deployment-acceptance.mts", ["run", ...sel, "--row", "13"]);
  assert.equal(probeFail.status, 1, probeFail.stderr); assert.match(probeFail.stderr, /隔离|Docker|observ|探针|合同/i);
  assert.deepEqual(claims(), [], "隔离探针失败不消费 used-13");
  assert.throws(() => git("rev-parse", "--verify", "-q", `refs/heads/sandcastle/issue-${TICKET}`), "没有专票分支");
  assert.ok(called().some((c) => c.startsWith("docker run")), "探针确实在锁内运行了真实入口的 docker 调用");
  assert.ok(!called().some((c) => c.startsWith("gh ")), "专用入口不调用 gh");

  const status = tsx("deployment-acceptance.mts", ["status", ...sel]);
  assert.equal(status.status, 0, status.stderr);
  assert.deepEqual(JSON.parse(status.stdout).claims, { "13": null, "14": null, "15": null });
  const bind = tsx("docker-acceptance.mts", ["bind-merges", ...sel, "--candidate-review", review, "--controller-admission", review]);
  assert.equal(bind.status, 1); assert.match(bind.stderr, /尚无已发布候选/);

  drain("resume", "--generation", "1");
  const opened = tsx("deployment-acceptance.mts", ["run", ...sel, "--row", "13"]);
  assert.equal(opened.status, 75, opened.stderr);
  const before = called().length;
  const legacyOpen = tsx("main-swarm.mts", [], { SANDCASTLE_DOCKER_ACCEPTANCE_ROW: "13", SANDCASTLE_DOCKER_ACCEPTANCE_TICKET: String(TICKET), CLIPROXY_LOCAL_KEY: "fixture-no-network" });
  assert.equal(legacyOpen.status, 1); assert.match(legacyOpen.stderr, /冻结部署验收用途，只能经 deployment-acceptance/);
  assert.equal(called().length, before, "open 时普通入口在镜像解析前即拒绝冻结 receipt");
  assert.deepEqual(claims(), []);
  assert.match(drain("status"), /"mode": "open", "generation": 1/, "验收命令从不改写 mode/generation");
});
