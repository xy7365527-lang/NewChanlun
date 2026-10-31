import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import type { BindMountSandboxHandle } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { assertContainerIsolationContract, hardenProvider } from "./docker-isolation.mts";

const [source, image] = process.argv.slice(2);
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
