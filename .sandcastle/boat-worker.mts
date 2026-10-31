import { codex, type BindMountSandboxHandle } from "@ai-hero/sandcastle";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { execFileSync } from "node:child_process";
import { existsSync, lstatSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { CodexTranscript, OUTPUT_SCHEMA, PROVIDER_DEFINITION, snapshotCheckout } from "./codex-child.ts";
import { assertContainerIsolationContract, hardenProvider } from "./docker-isolation.mts";

const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";
const requestPath = process.argv[2]!;
const request = JSON.parse(readFileSync(requestPath, "utf8"));
const root: string = request.root;
const checkout = join(root, "repo");
const claim = join(root, "claim");
const save = (path: string, data: unknown) => {
  writeFileSync(path + ".tmp", JSON.stringify(data) + "\n", { mode: 0o600 });
  renameSync(path + ".tmp", path);
};
let handle: BindMountSandboxHandle | undefined;
const record: Record<string, any> = {
  schema_version: 1, run_id: randomUUID(), ticket: request.ticket,
  parent_thread_id: request.identity.run_id, manager_agent_id: "/deepagents/boat/" + request.identity.run_id,
  manager_kind: "codex_native_subagent", worker_kind: "sandcastle_external_codex", native_subagent: false,
  status: "failed", mode: request.mode, model: request.model, effort: "high", model_provider: "cliproxy-cloud",
  sandbox_backend: "docker", codex_sandbox: "danger-full-access", approval_policy: "never",
  automatic_claim: false, automatic_push: false, automatic_merge: false, checkout_preserved: true,
  expected_head: request.identity.target_head, checkout_before: null, checkout_after: null,
  started_at: new Date().toISOString(), ended_at: null, process_exit: null,
  external_session_id: null, external_session_log: null, result: null, failure_reason: null,
};
const files: Array<{path: string; mode: string; content?: string}> = [];
let exitCode: number | null = null;
process.once("SIGTERM", () => {
  void (async () => {
    try { await handle?.close(); } finally { process.exit(143); }
  })();
});
try {
  if (!/^sha256:[a-f0-9]{64}$/.test(request.image)) throw new Error("Docker image identity invalid");
  const before = snapshotCheckout(checkout);
  if (before.head !== request.identity.target_head || before.status) throw new Error("Frozen source is not clean");
  record.checkout_before = before;
  save(join(claim, "isolation.json"), assertContainerIsolationContract(request.image));
  const keyFile = join(root, "../private/model-key");
  const key = readFileSync(keyFile, "utf8").trim();
  rmSync(keyFile, { force: true });
  if (!key) throw new Error("Managed model capability missing");
  const base = hardenProvider(docker({ imageName: request.image, containerUid: process.getuid!(), containerGid: process.getgid!(),
    env: { CLIPROXY_BASE_URL: request.endpoint, CLIPROXY_LOCAL_KEY: key } }));
  const provider = base as typeof base & { create(options: any): Promise<BindMountSandboxHandle> };
  handle = await provider.create({ worktreePath: checkout, hostRepoPath: checkout, env: {}, mounts: [
    { hostPath: checkout, sandboxPath: "/workspace/repo", readonly: request.mode === "review" },
    { hostPath: join(root, "pstack"), sandboxPath: "/opt/newchanlun/pstack", readonly: true },
    { hostPath: join(root, "rules"), sandboxPath: "/opt/newchanlun/rules", readonly: true },
    { hostPath: join(root, "project-skills"), sandboxPath: "/workspace/repo/.agents/skills", readonly: true },
  ] });
  const schema = join(claim, "output.schema.json");
  writeFileSync(schema, JSON.stringify(OUTPUT_SCHEMA));
  await handle.copyFileIn(schema, "/tmp/output.schema.json");
  const home = "/tmp/codex-" + record.run_id;
  const agent = codex(request.model, { effort: "high", sessionStorage: { sandboxSessionsDir: home + "/sessions" } });
  const prompt = "先完整读取 /opt/newchanlun/pstack/installed/skills/poteto-mode/SKILL.md、适用playbook和原则叶技能，" +
    "读取 /workspace/repo/AGENTS.md 与任务相关 .agents/skills。只修改授权写集，不提交、不push、不读任何凭据。" +
    "最终仅输出规定JSON。\n" + request.prompt;
  const built = agent.buildPrintCommand({ prompt, dangerouslySkipPermissions: false });
  const bypass = " --dangerously-bypass-approvals-and-sandbox";
  if (!built.command.startsWith("codex exec --json ") || built.command.split(bypass).length !== 2 || built.stdin !== prompt)
    throw new Error("Sandcastle Codex command contract changed");
  const overrides = { ...PROVIDER_DEFINITION, base_url: request.endpoint };
  const command = built.command.replace("codex exec", "codex -a never exec --ignore-user-config --sandbox danger-full-access")
    .replace(bypass, "") + " --output-schema /tmp/output.schema.json --disable multi_agent" +
    " -c 'features.apps=false' -c 'features.plugins=false' -c 'web_search=\"disabled\"'" +
    " -c 'model_provider=\"cliproxy-local\"'" + Object.entries(overrides).map(([key, value]) =>
      " -c " + quote("model_providers.cliproxy-local." + key + "=" + JSON.stringify(value))).join("") + " -";
  const transcript = new CodexTranscript(agent);
  const result = await handle.exec("mkdir -p " + quote(home) + "; env CODEX_HOME=" + quote(home) + " " + command +
    "; code=$?; printf '%s' \"$code\" > /tmp/newchanlun-exit; exit \"$code\"", {
    stdin: built.stdin, onLine(line) { writeFileSync(join(claim, "events.jsonl"), line + "\n", { flag: "a" }); transcript.accept(line); },
  });
  const marker = join(claim, "exit-code");
  await handle.copyFileOut("/tmp/newchanlun-exit", marker);
  const codeText = readFileSync(marker, "utf8");
  if (!/^[0-9]{1,3}$/.test(codeText)) throw new Error("Missing explicit worker exit code");
  exitCode = Number(codeText);
  record.process_exit = { code: exitCode, signal: null };
  if (exitCode !== 0 || result.exitCode !== 0) throw new Error("Worker exited nonzero");
  record.result = transcript.output();
  record.external_session_id = transcript.sessionId;
  const sessionList = await handle.exec("find " + quote(home + "/sessions") + " -type f -name '*.jsonl'");
  const session = sessionList.stdout.split("\n").find((line) => line.includes(transcript.sessionId!));
  if (!session) throw new Error("Actual Codex session log missing");
  const sessionTarget = join(claim, "session.jsonl");
  await handle.copyFileOut(session, sessionTarget);
  record.external_session_log = sessionTarget;
  const contexts = readFileSync(sessionTarget, "utf8").split("\n").filter(Boolean).map((line) => JSON.parse(line))
    .filter((event) => event.type === "turn_context" && typeof event.payload?.model === "string");
  const actualModels = new Set(contexts.map((event) => event.payload.model));
  if (actualModels.size !== 1 || !actualModels.has(request.model)) throw new Error("Actual session model differs from binding");
  record.actual_model = [...actualModels][0];
  const after = snapshotCheckout(checkout);
  record.checkout_after = after;
  if (after.head !== before.head || after.branch !== before.branch) throw new Error("Worker changed source identity");
  if (request.mode === "review" && after.status) throw new Error("Read-only worker changed files");
  const status = execFileSync("git", ["-C", checkout, "status", "--porcelain=v1", "-z", "--untracked-files=all"], { encoding: "utf8" }).split("\0");
  const paths = new Set<string>();
  for (let i = 0; i < status.length; i++) {
    const entry = status[i]; if (!entry) continue;
    paths.add(entry.slice(3));
    if (entry[0] === "R" || entry[0] === "C") paths.add(status[++i]!);
  }
  for (const path of paths) {
    if (path.startsWith("/") || path.split("/").some((p) => ["..", ".git", ".agents", ".codex"].includes(p)) ||
      !request.allowed_prefixes.some((prefix: string) => path.startsWith(prefix))) throw new Error("Worker path outside write set");
    const file = join(checkout, path);
    if (!existsSync(file)) { files.push({ path, mode: "deleted" }); continue; }
    const stat = lstatSync(file);
    if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 32 * 1024 * 1024) throw new Error("Worker artifact mode invalid");
    files.push({ path, mode: stat.mode & 0o111 ? "100755" : "100644", content: readFileSync(file).toString("base64") });
  }
  record.status = record.result.status === "completed" ? "completed" : "failed";
  if (record.status !== "completed") record.failure_reason = "Worker reported " + record.result.status;
} catch (error) {
  record.failure_reason = error instanceof Error ? error.message : "Worker supervision failed";
} finally {
  try { await handle?.close(); } catch { record.status = "failed"; record.failure_reason = "Container cleanup failed"; }
  record.ended_at = new Date().toISOString();
  save(join(claim, "receipt.json"), { identity: request.identity, package_id: request.package_id,
    state: exitCode === null ? "unknown" : "terminal", exit_code: exitCode, record, files });
}
