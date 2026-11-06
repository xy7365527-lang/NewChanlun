import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { codex, type BindMountSandboxHandle } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { assertContainerIsolationContract, hardenProvider } from "./docker-isolation.mts";
import { CodexTranscript, DEEPAGENTS_OUTPUT_PROTOCOL, outputProtocol, workerOutputSchema } from "./codex-child.ts";

const [source, image, requestFile] = process.argv.slice(2);
if (!source || !image) throw new Error("需要技能包目录和冻结image ID");
const root = resolve(source);
const repo = mkdtempSync(join(tmpdir(), "boat-package-smoke-"));
mkdirSync(join(repo, ".agents/skills"), { recursive: true });
writeFileSync(join(repo, "README"), "这是路径验收夹具，不是业务任务。\n");
for (const args of [["init", "-q", "--template=", "-b", "codex/skill-smoke"],
  ["add", "."], ["-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", "commit", "-qm", "fixture"]]) {
  execFileSync("git", ["-C", repo, ...args]);
}
const isolation = assertContainerIsolationContract(image);
const provider = hardenProvider(docker({ imageName: image, containerUid: process.getuid!(), containerGid: process.getgid!(), env: {} })) as unknown as {
  create(options: unknown): Promise<BindMountSandboxHandle>;
};
const handle = await provider.create({ worktreePath: repo, hostRepoPath: repo, env: {}, mounts: [
  { hostPath: repo, sandboxPath: "/workspace/repo" },
  { hostPath: join(root, "pstack"), sandboxPath: "/opt/newchanlun/pstack", readonly: true },
  { hostPath: join(root, "rules"), sandboxPath: "/opt/newchanlun/rules", readonly: true },
  { hostPath: join(root, "project-skills"), sandboxPath: "/workspace/repo/.agents/skills", readonly: true },
] });
try {
  if (requestFile) {
    const request = JSON.parse(readFileSync(requestFile, "utf8"));
    const protocol = outputProtocol(request.output_protocol);
    if (protocol !== DEEPAGENTS_OUTPUT_PROTOCOL || !request.prompt.includes("confidence")
      || !request.prompt.includes("caveats")) throw new Error("Deep Agents 提示与协议不一致");
    const output = { status: "completed", summary: "零模型回执", findings: [], validation: [],
      confidence: "medium", caveats: ["夹具只验证协议"] };
    const schemaFile = join(repo, "output-smoke.json");
    writeFileSync(schemaFile, JSON.stringify({ schema: workerOutputSchema(protocol), output }));
    await handle.copyFileIn(schemaFile, "/tmp/output-smoke.json");
    const probe = await handle.exec("bun -e 'const {schema,output}=await Bun.file(\"/tmp/output-smoke.json\").json();"
      + "if(schema.additionalProperties!==false || schema.required.sort().join()!==Object.keys(output).sort().join())process.exit(1);"
      + "console.log(JSON.stringify(output));'");
    if (probe.exitCode !== 0) throw new Error(`云端六键 schema smoke 失败 (${probe.exitCode})：${probe.stderr}`);
    const transcript = new CodexTranscript(codex("gpt-6.1-sol"), protocol);
    for (const event of [
      { type: "thread.started", thread_id: "smoke-session" },
      { type: "item.completed", item: { type: "agent_message", text: probe.stdout.trim() } },
      { type: "turn.completed" },
    ]) transcript.accept(JSON.stringify(event));
    if (JSON.stringify(transcript.output()) !== JSON.stringify(output)) throw new Error("自报字段往返不一致");
    console.log(JSON.stringify({ outputProtocol: protocol, fixture: true, realModel: false, output: transcript.output() }));
  }
  const result = await handle.exec(`set -eu
pstack=$(find /opt/newchanlun/pstack/installed/skills -name SKILL.md | wc -l)
project=$(find /workspace/repo/.agents/skills -name SKILL.md | wc -l)
test "$pstack" = 58
test "$project" = 63
find /opt/newchanlun/pstack/installed/skills -type f -exec test -r {} \\;
find /workspace/repo/.agents/skills -type f -exec test -r {} \\;
test -r /opt/newchanlun/rules/pstack-models.md
bun --version
if touch /workspace/repo/.git/boat-write-probe; then exit 20; fi
if touch /workspace/repo/.agents/skills/boat-write-probe; then exit 21; fi
printf 'fixture' > /workspace/repo/probe-result
git -C /workspace/repo rev-parse HEAD
printf 'pstack=%s project=%s\\n' "$pstack" "$project"`);
  if (result.exitCode !== 0 || !result.stdout.includes("pstack=58 project=63")) throw new Error("技能路径验收失败");
  console.log(JSON.stringify({ fixture: true, realModel: false, image, isolation, ...result }));
} finally {
  await handle.close();
}
