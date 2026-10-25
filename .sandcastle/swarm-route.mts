import { useScheduler } from "./scheduler-config.mts";
/** main-swarm 的 Docker 隔离与主控同型路由（#1617）。 */
import type { AgentProvider, SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { codexWorker, codexRouteEnv } from "./codex-route.mts";

import { MAIN_MODEL } from "./main-model.mts";

const scheduler = useScheduler("swarm-route", ["dockerModelHttpTimeoutMs", "dockerModelProcessTimeoutMs", "dockerNamespaceTimeoutMs"]);

export const DOCKER_CLIPROXY_BASE_URL = "http://host.docker.internal:8318";

/** 容器不能访问宿主回环；密钥只经运行时 env 进入，不挂配置或 OAuth 文件。 */
export function dockerCliProxyEnv(): Record<string, string> {
  return { ...codexRouteEnv(), CLIPROXY_BASE_URL: DOCKER_CLIPROXY_BASE_URL };
}

export function swarmSandbox(
  imageName: string,
  env: Record<string, string> = dockerCliProxyEnv(),
): SandboxProvider {
  return docker({ imageName, env });
}

/** planner/implementer/reviewer/merger 各自调用，保留独立 session。 */
export function swarmAgent(): AgentProvider {
  return codexWorker({ docker: true });
}

export const DOCKER_MODEL_PROBE = String.raw`
(async () => {
  const baseUrl = process.env.CLIPROXY_BASE_URL;
  const token = process.env.CLIPROXY_LOCAL_KEY;
  if (!baseUrl || !token) throw new Error("missing Codex route environment");
  const response = await fetch(baseUrl + "/v1/responses", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      "authorization": "Bearer " + token,
    },
    body: JSON.stringify({
      model: "${MAIN_MODEL}",
      input: "Reply OK.",
    }),
    signal: AbortSignal.timeout(${scheduler.dockerModelHttpTimeoutMs}),
  });
  const payload = await response.json();
  if (!response.ok) throw new Error("HTTP " + response.status + ": " + (payload?.error?.type || "model request failed"));
  if (payload?.object !== "response" || payload?.status !== "completed" || payload?.model !== "${MAIN_MODEL}") {
    throw new Error("response is not a ${MAIN_MODEL} message");
  }
  console.log("MAIN_MODEL_HTTP_OK model=" + payload.model);
})().catch((error) => { console.error(error instanceof Error ? error.message : String(error)); process.exit(1); });
`;

export type DockerProbeRunner = (imageName: string, probe: string) => void;

const runDockerProbe: DockerProbeRunner = (imageName, probe) => {
  const routeEnv = dockerCliProxyEnv();
  execFileSync("docker", [
    "run", "--rm",
    "--env", "CLIPROXY_BASE_URL",
    "--env", "CLIPROXY_LOCAL_KEY",
    "--entrypoint", "node", imageName, "-e", probe,
  ], {
    stdio: ["ignore", "pipe", "pipe"],
    timeout: scheduler.dockerModelProcessTimeoutMs,
    env: { ...process.env, ...routeEnv },
  });
};

/** 领票前实际调用主控模型；TCP 可达或 /v1/models 成功均不代替此行为证据。 */
export function assertDockerCliProxyReachable(
  imageName: string,
  run: DockerProbeRunner = runDockerProbe,
): void {
  try {
    run(imageName, DOCKER_MODEL_PROBE);
  } catch (error) {
    process.exitCode = 1;
    throw new Error(
      `[FAIL-loud #1617] Docker 内主控模型 ${MAIN_MODEL} 预检失败（${DOCKER_CLIPROXY_BASE_URL}/v1/responses），main-swarm 不可用。` +
      "请配置受控的容器可达端点并确认主控模型真实响应，" +
      "或使用当前已验证的 main.mts 本机路径。不会回退 k3/Claude，也不会将领票继续执行。" +
      ` 原始错误：${String(error).slice(0, 240)}`,
    );
  }
}

/** #1617：先证实默认容器允许工具所需 namespace，再碰 planner/认领。 */
export function assertSwarmExecutionReady(imageName: string, probe = (image: string) => {
  execFileSync("docker", ["run", "--rm", "--network", "none", "--entrypoint", "unshare", image, "-Ur", "true"], { stdio: "pipe", timeout: scheduler.dockerNamespaceTimeoutMs });
}): void {
  try { probe(imageName); }
  catch { throw new Error("[FAIL-loud #1617] 默认 Docker namespace 不可用；不认领、不扩大容器权限。Docker 提交合同另待验收。"); }
}
