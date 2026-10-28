/** main-swarm 的 Docker 隔离与主控同型路由（#1617）。 */
import type { AgentProvider, SandboxProvider } from "@ai-hero/sandcastle";
import { execFileSync } from "node:child_process";
import { docker } from "@ai-hero/sandcastle/sandboxes/docker";
import { codexWorker, codexRouteEnv } from "./codex-route.mts";
import type { KeyLookup } from "./codex-child.ts";
import { assertContainerIsolationContract, hardenProvider, type IsolationObservation } from "./docker-isolation.mts";
import { IMAGE_ID, checkAdmission, type Admission, type AcceptanceRow, type AdmissionContext } from "./docker-acceptance.mts";

import { MAIN_MODEL } from "./main-model.mts";

export const DOCKER_CLIPROXY_BASE_URL = "http://host.docker.internal:8318";

/** 容器不能访问宿主回环；密钥只经运行时 env 进入，不挂配置或 OAuth 文件。 */
export function dockerCliProxyEnv(lookup?: KeyLookup): Record<string, string> {
  return { ...codexRouteEnv(lookup), CLIPROXY_BASE_URL: DOCKER_CLIPROXY_BASE_URL };
}

export function swarmSandbox(
  imageId: string,
  env?: Record<string, string>,
  /** 仅测试：注入 launchctl 查找（CI 上没有 launchctl 里的 key）。 */
  lookup?: KeyLookup,
): SandboxProvider {
  // 容器必须以准入核对过的 sha256 Id 启动（assertSwarmExecutionReady 的返回值），不接受 tag：准入与启动之间 tag 可被重指。
  if (!IMAGE_ID.test(imageId)) throw new Error(`[FAIL-loud #1742] Docker 容器必须以准入核对过的 sha256 镜像 Id 启动，收到 ${JSON.stringify(imageId).slice(0, 80)}`);
  // #1742 方案B：git 挂载与受保护子路径只读；其余启动参数仍是 Sandcastle 的 docker()，不加 security-opt/cap。
  // env 只在 Id 校验通过后才求值：拿 tag 调用时，无论环境里有没有 CLIPROXY_LOCAL_KEY 都先报 Id 错误。
  return hardenProvider(docker({ imageName: imageId, env: env ?? dockerCliProxyEnv(lookup) }));
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

export interface ReadyOptions {
  probe?: (image: string) => IsolationObservation;
  admit?: (image: string, ctx: AdmissionContext) => Admission;
  /** 受控验收的运行绑定：13=验收票，14/15=归并 run+检查点。已登记验收后的普通放行不需要。 */
  binding?: string;
  /** 此入口允许的验收行（普通 swarm 只接受 13，归并冲突只接受 14/15）。 */
  rows?: readonly AcceptanceRow[];
  /** 准入通过后、探针与占用之前的入口自检（失败即拒绝，不烧验收行）。 */
  verify?: (admission: Admission) => void;
  /** 仅供进程内调用方（单元测试）注入验收目录/代码身份/验收行；生产入口不传，走宿主固定目录。 */
  acceptance?: Pick<AdmissionContext, "dir" | "env" | "identity" | "now" | "roots" | "imageIdOf">;
}

/** #1617/#1742：领票前先验精确 acceptance 事实（缺失/漂移/重复/越权即 FAIL-loud），再逐项观测容器隔离合同；
 * 观测成立只是必要条件，不代替验收。受控入口在观测通过后才占用（或恢复）对应验收行。 */
export function assertSwarmExecutionReady(imageName: string, options: ReadyOptions = {}): string {
  const admit = options.admit ?? ((image: string, ctx: AdmissionContext) => checkAdmission(image, ctx));
  const admission = admit(imageName, { ...options.acceptance, binding: options.binding, rows: options.rows });
  options.verify?.(admission);
  // 观测与后续启动都用准入核对过的 Id，三条入口（受控行、已登记放行、冲突工蜂）一致。
  assertContainerIsolationContract(admission.imageId, options.probe);
  admission.consume();
  return admission.imageId;
}
