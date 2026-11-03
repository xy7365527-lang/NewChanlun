import { createBindMountSandboxProvider, type SandboxProvider } from "@ai-hero/sandcastle";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { runDeploymentAcceptance } from "../../deployment-acceptance.mts";

const a = JSON.parse(process.argv[2]!) as { sel: { ticket: number; head: string; nonce: string; row: "15" }; dir: string; repo: string; head: string; imageId: string; drainRoot: string; marker: string; probe: string };
const provider = () => createBindMountSandboxProvider({ name: "fixture", create: async () => ({
  worktreePath: "/home/agent/workspace", exec: async () => ({ stdout: "", stderr: "", exitCode: 0 }),
  copyFileIn: async () => undefined, copyFileOut: async () => undefined, close: async () => undefined,
}) }) as unknown as SandboxProvider;
await runDeploymentAcceptance(a.sel, {
  ctx: { dir: a.dir, repo: a.repo, identity: { head: a.head, dirty: "" }, imageIdOf: () => a.imageId },
  drainRoot: a.drainRoot,
  probe: () => ({ apparmor: "docker-default (enforce)", seccomp: "2", capEff: "0000000000000000", capBnd: "00000000a80425fb", roWrite: "Read-only file system", rwWrite: "ok", roChildInRw: "Read-only file system", roFileAppend: "Read-only file system", renameMountpoint: "Device or resource busy", hardlinkAcross: "Invalid cross-device link", symlinkIntoRo: "Read-only file system" }),
  modelProbe: () => undefined,
  docker: (args) => (args[0] === "ps" ? "" : "[]"),
  mergeProvider: provider,
  runConflictAgent: async ({ cwd, sandbox }) => {
    await (sandbox as unknown as { create: (o: unknown) => Promise<unknown> }).create({ worktreePath: cwd, hostRepoPath: a.repo, mounts: [{ hostPath: cwd, sandboxPath: "/home/agent/workspace" }], env: {} });
    writeFileSync(join(cwd, a.probe), "孤儿工蜂写了一半\n");
    writeFileSync(a.marker, String(process.pid));
    await new Promise(() => undefined);
    return "";
  },
});
