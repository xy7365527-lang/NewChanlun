/** main-swarm 的 Docker 隔离与主控同型路由（#1617）。 */
import type { AgentProvider, SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { codexWorker, codexRouteEnv } from "./codex-route.mts";
import { assertContainerIsolationContract, hardenProvider, type IsolationObservation } from "./docker-isolation.mts";
import { checkAdmission, type Admission } from "./docker-acceptance.mts";

import { MAIN_MODEL } from "./main-model.mts";

export const DOCKER_CLIPROXY_BASE_URL = "http://host.docker.internal:8318";

/** 容器不能访问宿主回环；密钥只经运行时 env 进入，不挂配置或 OAuth 文件。 */
export function dockerCliProxyEnv(): Record<string, string> {
  return { ...codexRouteEnv(), CLIPROXY_BASE_URL: DOCKER_CLIPROXY_BASE_URL };
}

export function swarmSandbox(
  imageName: string,
  env: Record<string, string> = dockerCliProxyEnv(),
): SandboxProvider {
  // #1742 方案B：git 挂载与受保护子路径只读；其余启动参数仍是 Sandcastle 的 docker()，不加 security-opt/cap。
  return hardenProvider(docker({ imageName, env }));
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
    signal: AbortSignal.timeout(60000),
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
    timeout: 75_000,
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

/** #1617/#1742：领票前先验精确 acceptance receipt（缺失/漂移/重复即 FAIL-loud），再逐项观测容器隔离合同；
 * 观测成立只是必要条件，不代替验收。受控入口在观测通过后才占用对应验收行。 */
export function assertSwarmExecutionReady(
  imageName: string,
  probe?: (image: string) => IsolationObservation,
  admit: (image: string) => Admission = checkAdmission,
): void {
  const admission = admit(imageName);
  assertContainerIsolationContract(imageName, probe);
  admission.consume();
}
