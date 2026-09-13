// #1456：CLI 只接一次已指定任务，不包含队列、认领、push 或合入动作。
import { parseArgs } from "node:util";
import { writeSync } from "node:fs";
import { runCodexChild, SupervisionError, type ChildMode } from "./codex-child.ts";

const { values } = parseArgs({
  options: {
    ticket: { type: "string" },
    "parent-thread-id": { type: "string" },
    "manager-agent-id": { type: "string" },
    checkout: { type: "string" },
    "expected-head": { type: "string" },
    mode: { type: "string", default: "review" },
    "prompt-file": { type: "string" },
    "output-dir": { type: "string" },
    "idle-timeout-seconds": { type: "string", default: "600" },
    "timeout-seconds": { type: "string", default: "3600" },
    help: { type: "boolean", short: "h" },
  },
  strict: true,
  allowPositionals: false,
});

if (values.help) {
  writeSync(process.stdout.fd, "用法：node --import tsx .sandcastle/codex-child.mts --ticket <票号> --parent-thread-id <父任务ID> --manager-agent-id <原生子代理ID> --checkout <独立干净checkout绝对路径> --expected-head <40位SHA> --mode review|execute --prompt-file <绝对路径> --output-dir <不存在的绝对目录>\n");
} else {
  const controller = new AbortController();
  const cancel = () => controller.abort();
  const outputError = () => controller.abort(new SupervisionError("CLI stdout 输出渠道失败"));
  process.once("SIGINT", cancel);
  process.once("SIGTERM", cancel);
  process.stdout.on("error", outputError);
  try {
    const result = await runCodexChild({
      ticket: Number(values.ticket),
      parentThreadId: values["parent-thread-id"] ?? "",
      managerAgentId: values["manager-agent-id"] ?? "",
      checkout: values.checkout ?? "",
      expectedHead: values["expected-head"] ?? "",
      mode: values.mode as ChildMode,
      promptFile: values["prompt-file"] ?? "",
      outputDir: values["output-dir"] ?? "",
      idleTimeoutSeconds: Number(values["idle-timeout-seconds"]),
      timeoutSeconds: Number(values["timeout-seconds"]),
    }, {
      signal: controller.signal,
      // 少量状态行直接写 fd；EPIPE 同步回到监督器，不能变成未处理的 error 事件。
      onState: (state) => { writeSync(process.stdout.fd, JSON.stringify({
        run_id: state.run_id, ticket: state.ticket, parent_thread_id: state.parent_thread_id,
        manager_agent_id: state.manager_agent_id, worker_kind: state.worker_kind,
        status: state.status, process_pid: state.process_pid, external_session_id: state.external_session_id,
        external_session_log: state.external_session_log, result_path: `${state.output_dir}/result.json`,
      }) + "\n"); },
    });
    process.exitCode = result.status === "completed" ? 0 : result.status === "cancelled" ? 130 : 1;
  } catch (error) {
    try { writeSync(process.stderr.fd, (error instanceof Error ? error.message : "工蜂入口失败") + "\n"); } catch { /* 已完成清理；保留非零退出码。 */ }
    process.exitCode = 1;
  } finally {
    process.removeListener("SIGINT", cancel);
    process.removeListener("SIGTERM", cancel);
    process.stdout.removeListener("error", outputError);
  }
}
