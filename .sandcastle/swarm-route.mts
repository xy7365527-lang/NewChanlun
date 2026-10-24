/** main-swarm 的 Docker 隔离与主控同型路由（#1617）。 */
import type { AgentProvider, SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { claudeWorker } from "./claude-route.mts";
import { CLIPROXY_CONFIG, cliProxyEnv } from "./cliproxy-env.mts";
import { MAIN_MODEL } from "./main-model.mts";

export const DOCKER_CLIPROXY_BASE_URL = "http://host.docker.internal:8318";

/** 容器不能访问宿主回环；密钥只经运行时 env 进入，不挂配置或 OAuth 文件。 */
export function dockerCliProxyEnv(configPath: string = CLIPROXY_CONFIG): Record<string, string> {
  return cliProxyEnv(configPath, DOCKER_CLIPROXY_BASE_URL);
}

export function swarmSandbox(
  imageName: string,
  env: Record<string, string> = dockerCliProxyEnv(),
): SandboxProvider {
  return docker({ imageName, env });
}

/** planner/implementer/reviewer/merger 各自调用，保留独立 session。 */
export function swarmAgent(): AgentProvider {
  return claudeWorker(MAIN_MODEL);
}

const DOCKER_PROXY_PROBE = String.raw`
const net = require("node:net");
const socket = net.connect(8318, "host.docker.internal");
const timer = setTimeout(() => { console.error("timeout"); process.exit(2); }, 8000);
socket.on("connect", () => { clearTimeout(timer); socket.end(); });
socket.on("error", (error) => { clearTimeout(timer); console.error(error.code || error.message); process.exit(1); });
`;

export type DockerProbeRunner = (imageName: string, probe: string) => void;

const runDockerProbe: DockerProbeRunner = (imageName, probe) => {
  execFileSync("docker", ["run", "--rm", "--entrypoint", "node", imageName, "-e", probe], {
    stdio: ["ignore", "pipe", "pipe"],
    timeout: 15_000,
  });
};

/** 领票前的硬门；失败时不启动 planner、不认领、不回退其它模型。 */
export function assertDockerCliProxyReachable(
  imageName: string,
  run: DockerProbeRunner = runDockerProbe,
): void {
  try {
    run(imageName, DOCKER_PROXY_PROBE);
  } catch (error) {
    throw new Error(
      `[FAIL-loud #1617] Docker 内无法连接 ${DOCKER_CLIPROXY_BASE_URL}。` +
      "CLIProxy 只绑宿主 127.0.0.1 时 main-swarm 不可用；请配置受控的容器可达端点，" +
      "或使用当前已验证的 main.mts 本机路径。不会回退 k3/Claude，也不会将领票继续执行。" +
      ` 原始错误：${String(error).slice(0, 240)}`,
    );
  }
}
