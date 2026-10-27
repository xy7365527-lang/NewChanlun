/** #1617：受管执行面；每阶段独立 Codex home，同阶段可续跑，不切换 provider。 */
import { codex, type AgentProvider } from "@ai-hero/sandcastle";
import { mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { installCargoGuard } from "./cargo-guard.mts";
import { MAIN_MODEL } from "./main-model.mts";
import { providerOverrides, providerEnvironment } from "./codex-child.ts";

const quote = (value: string) => "'" + value.replaceAll("'", "'\\''") + "'";
export function codexRouteEnv(): Record<string, string> {
  const env = providerEnvironment(process.env.CLIPROXY_LOCAL_KEY ? { CLIPROXY_LOCAL_KEY: process.env.CLIPROXY_LOCAL_KEY } : {});
  return { CLIPROXY_LOCAL_KEY: env.CLIPROXY_LOCAL_KEY };
}

export function codexWorker(options: { docker?: boolean; effort?: "low" | "medium" | "high" | "xhigh" } = {}): AgentProvider {
  const docker = options.docker;
  const home = mkdtempSync(join(tmpdir(), "sandcastle-codex-"));
  const workerHome = docker ? `/tmp/${home.split("/").at(-1)}` : home;
  const sessions = join(home, "sessions");
  if (!docker) {
    const guard = installCargoGuard();
    // zsh -lc 会在 /etc/zprofile 重排 PATH；.zlogin 在其后重新前置垫片。
    for (const file of [".zshenv", ".zlogin"]) writeFileSync(join(home, file), `source ${quote(guard.envFile)}\n`);
  }
  const provider = codex(MAIN_MODEL, {
    effort: options.effort ?? "medium",
    sessionStorage: { hostSessionsDir: sessions, sandboxSessionsDir: `${workerHome}/sessions` },
  });
  const overrides = providerOverrides().map((entry) => docker
    ? entry.replace("http://127.0.0.1:8318/v1", "http://host.docker.internal:8318/v1") : entry);
  return {
    ...provider,
    buildPrintCommand(options) {
      const built = provider.buildPrintCommand(options);
      const bypass = " --dangerously-bypass-approvals-and-sandbox";
      if (!built.command.startsWith("codex exec") || built.command.split(bypass).length !== 2 || built.stdin !== options.prompt) {
        throw new Error("Sandcastle Codex 命令契约已变，拒绝猜测权限替换");
      }
      const command = built.command.replace(bypass, "")
        .replace("codex exec", "codex -a never -s workspace-write exec --ignore-user-config")
        + overrides.map((value) => ` -c ${quote(value)}`).join("")
        + (docker ? "" : ` -c ${quote(`shell_environment_policy.set.ZDOTDIR=${JSON.stringify(home)}`)}`)
        + ` -c 'sandbox_workspace_write.network_access=true' --disable multi_agent`;
      // CODEX_HOME 只含隔离路径；密钥由 sandbox env 注入，不进命令和会话记录。
      return { command: `mkdir -p ${quote(workerHome)} && env CODEX_HOME=${quote(workerHome)} ${command}`, stdin: built.stdin };
    },
  };
}
