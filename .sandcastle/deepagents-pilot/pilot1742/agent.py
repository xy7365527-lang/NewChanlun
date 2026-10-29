from __future__ import annotations

import json
import os
import shutil
import subprocess
from datetime import datetime, timezone
from pathlib import Path
from typing import Any, Literal

from deepagents import (
    FilesystemMiddleware,
    FsToolName,
    GeneralPurposeSubagentProfile,
    HarnessProfile,
    create_deep_agent,
    register_harness_profile,
)
from deepagents.backends import CompositeBackend, FilesystemBackend, StateBackend
from langchain.agents.middleware import AgentMiddleware, TodoListMiddleware
from langchain_core.callbacks import BaseCallbackHandler
from langchain_core.language_models import BaseChatModel
from langchain_core.tools import StructuredTool, tool
from langchain_openai import ChatOpenAI
from langgraph.types import interrupt
from pydantic import BaseModel, ConfigDict, SecretStr, ValidationError, field_validator

from .domain import Awaiting, CommitReceipt, WorkerReceipt
from .run import PSTACK_MODELS, SKILLS_ROOT, Halted, PilotRun, ToolRefusal, gate_json

from .boat_execution import ExecutionUnknown
from .cloud_package import PSTACK_TARGET, PROJECT_TARGET, RULES_TARGET

TOOL_ERRORS: tuple[type[Exception], ...] = (ToolRefusal, Halted, ExecutionUnknown)


def _tool_errors(run: PilotRun) -> tuple[type[Exception], ...]:
    if run.spec.v != 2:
        return TOOL_ERRORS
    from .executor import EXECUTOR_ERRORS

    return (*TOOL_ERRORS, *EXECUTOR_ERRORS)

POTETO_MODE = SKILLS_ROOT + "poteto-mode/SKILL.md"
CLIPROXY_BASE_URL = "http://127.0.0.1:8318/v1"
KEY_ENV = "CLIPROXY_LOCAL_KEY"
BRAIN_EFFORT = "high"
BRAIN_MAX_TOKENS = 64000
READ_ONLY_FS_TOOLS: list[FsToolName] = ["ls", "read_file", "glob", "grep"]
TRACING_ENV = ("LANGSMITH_TRACING", "LANGCHAIN_TRACING_V2", "LANGSMITH_API_KEY", "LANGCHAIN_API_KEY",
               "LANGSMITH_ENDPOINT", "LANGCHAIN_ENDPOINT")


class MissingManagedCredential(RuntimeError):
    pass


def disable_external_tracing() -> None:
    for name in TRACING_ENV:
        os.environ.pop(name, None)
    os.environ["LANGSMITH_TRACING"] = "false"


def managed_cliproxy_key() -> SecretStr:
    """沿用 codex-child 已批准的机制：进程环境优先，否则 launchctl getenv 取一次；值只留在内存。"""
    value = os.environ.get(KEY_ENV)
    if not value:
        value = subprocess.run(["launchctl", "getenv", KEY_ENV], capture_output=True, text=True).stdout.strip()
    if not value:
        raise MissingManagedCredential(f"缺少 {KEY_ENV}：进程环境与 launchctl 均未提供")
    return SecretStr(value)


def build_model(slug: str) -> ChatOpenAI:
    return ChatOpenAI(model=slug, base_url=CLIPROXY_BASE_URL, api_key=managed_cliproxy_key(),
                      reasoning_effort=BRAIN_EFFORT, max_completion_tokens=BRAIN_MAX_TOKENS, timeout=1200,
                      max_retries=2)


LEGACY_CAVEAT = "旧工蜂协议未单独采集 confidence 与 caveats；low 是适配器保守标记，不是模型自报。"


class SubagentResponse(BaseModel):
    model_config = ConfigDict(strict=True, extra="forbid")

    conclusion: str
    evidence: list[str]
    actions_taken: list[str]
    confidence: Literal["high", "medium", "low"]
    caveats: list[str]


class WorkerToolInput(BaseModel):
    model_config = ConfigDict(strict=True, extra="forbid")

    role: Literal["design", "judge", "author", "review"]
    brief: str

    @field_validator("brief")
    @classmethod
    def nonblank_brief(cls, value: str) -> str:
        if not value.strip():
            raise ValueError("brief 不能是空白")
        return value


class LegacyWorkerOutput(BaseModel):
    model_config = ConfigDict(strict=True, extra="forbid")

    status: Literal["completed", "blocked", "failed"]
    summary: str
    findings: list[str]
    validation: list[str]


class ReceiptContent(BaseModel):
    model_config = ConfigDict(strict=True, extra="forbid")

    summary: str
    findings: tuple[str, ...]
    validation: tuple[str, ...]
    changed_paths: tuple[str, ...]


class EmptyWorkerResponse(ValueError):
    pass


def _receipt_json(receipt: WorkerReceipt) -> dict:
    result: dict[str, Any] = {"role": receipt.key.role, "target_head": receipt.key.target_head, "status": receipt.status,
              "model": receipt.model, "external_session_id": receipt.external_session_id,
              "external_session_log": receipt.external_session_log, "output_dir": receipt.output_dir,
              "summary": receipt.summary, "findings": receipt.findings, "validation": receipt.validation,
              "changed_paths": receipt.changed_paths_after, "failure_reason": receipt.failure_reason}
    if receipt.summary is None or isinstance(receipt.summary, str) and not receipt.summary.strip():
        raise EmptyWorkerResponse("工蜂回执为空，没有可交回主控的结论")
    caveats = [LEGACY_CAVEAT]
    if receipt.status != "completed":
        caveats.append(f"工蜂终态为 {receipt.status}，不构成成功结果。")
    if receipt.failure_reason:
        caveats.append(receipt.failure_reason)
    raw_output = None
    try:
        source = Path(receipt.output_dir) / "result.json"
        if source.exists():
            raw_output = json.loads(source.read_text()).get("result")
            LegacyWorkerOutput.model_validate(raw_output)
        content = ReceiptContent(summary=receipt.summary, findings=receipt.findings,
                                 validation=receipt.validation, changed_paths=receipt.changed_paths_after)
        labels = {"author": "作者发现", "review": "独审发现", "judge": "裁判嫁接或拒绝建议",
                  "design-a": "候选拒绝的替代形状", "design-b": "候选拒绝的替代形状"}
        response = SubagentResponse(
            conclusion=content.summary,
            evidence=[*content.validation, *(f"{labels[receipt.key.role]}：{f}" for f in content.findings)],
            actions_taken=[f"宿主观察到修改路径：{p}" for p in content.changed_paths],
            confidence="low", caveats=caveats)
        result["response_validated"] = True
    except ValidationError as error:
        result["unvalidated_output"] = raw_output
        response = SubagentResponse(
            conclusion=str(receipt.summary), evidence=[f"原始回执内容：{result!r}"],
            actions_taken=[], confidence="low",
            caveats=[*caveats, f"回执内容未通过校验，不可作为已核验答案：{error}"])
        result["response_validated"] = False
    result["response"] = response.model_dump()
    return result


def _commit_json(run: PilotRun, receipt: CommitReceipt) -> dict:
    checkout = run.spec.author_checkout
    show = subprocess.run(["git", "-C", checkout, "show", "--stat", "--patch", "--format=%H%n%B", receipt.commit],
                          capture_output=True, text=True).stdout
    return {"commit": receipt.commit, "parent": receipt.intent.parent, "paths": list(receipt.intent.paths),
            "recovered": receipt.recovered, "show": show[:20000]}


def build_tools(run: PilotRun) -> list:
    errors: tuple[type[Exception], ...] = (*_tool_errors(run), EmptyWorkerResponse)

    @tool(args_schema=WorkerToolInput)
    def run_worker(role: Literal["design", "judge", "author", "review"], brief: str) -> str:
        """派 Sandcastle 工蜂并等到终态。head、模型、checkout 与输出目录都由宿主从回执推导，你只给 brief。

        design：在基线上并行派两个只读设计候选（architect runners 行的两个不同家族模型，各用独立只读 checkout），
        两者收到同一个 brief，返回两份设计包。judge：两份设计包都完成后，派一个与主控不同家族的只读交叉裁判，
        brief 是你的 rubric，裁判只按“候选 1/候选 2”标签看到两份设计包。author：在当前 head 上实现（可写 checkout）。
        review：只读独立审阅当前候选提交。同一角色同一 head 重复调用会收养已有工蜂，不会重派。
        本 run 要求设计 arena 时，judge 完成前不能派 author；只有当前候选被独审明确 BLOCK 后，才能再派 author 返修。"""
        try:
            if role == "design":
                receipts = [_receipt_json(r) for r in run.run_design(brief)]
                return json.dumps({"ok": all(r["response_validated"] and r["status"] == "completed" for r in receipts),
                                   "receipts": receipts}, ensure_ascii=False)
            receipt = _receipt_json(run.run_worker(role, brief))
            return json.dumps({"ok": receipt["response_validated"] and receipt["status"] == "completed",
                               "receipt": receipt}, ensure_ascii=False)
        except errors as error:
            return json.dumps({"ok": False, type(error).__name__: str(error)}, ensure_ascii=False)

    @tool
    def commit_candidate() -> str:
        """把当前 head 上已完成的作者改动做成宿主受管本地提交（只提交写集内文件，不 push），返回候选提交与 diff。"""
        try:
            return json.dumps({"ok": True, "candidate": _commit_json(run, run.commit_candidate())}, ensure_ascii=False)
        except errors as error:
            return json.dumps({"ok": False, type(error).__name__: str(error)}, ensure_ascii=False)

    @tool
    def check_gate() -> str:
        """检查当前候选的门：需要同一 head 的异家族独审 PASS 和已登记的人类批准。没有批准时本 run 在此暂停为 awaiting_approval。"""
        decision = run.gate()
        while isinstance(decision, Awaiting):
            interrupt({"ticket": run.spec.ticket, "scope": run.spec.scope, "awaiting_approval": gate_json(decision)})
            decision = run.gate()
        return json.dumps(gate_json(decision), ensure_ascii=False)

    @tool
    def publish_candidate() -> str:
        """当前候选有同 head 异家族独审 PASS 时，由宿主用 App 身份把这个冻结 SHA 精确发布到本票专属分支，开 draft PR，
        写 deepagents/review 状态与票面交付标记，停在 awaiting_merge。远端出现任何其他 head 时返回 stale，不覆盖。
        重复调用会先读回已有结果，不会重复推送、开 PR 或发评论。合入 main 由人决定。"""
        try:
            result = run.publish_candidate()
            while result.get("decision") == "awaiting_approval":
                interrupt({"ticket": run.spec.ticket, "awaiting_approval": result["approval"]})
                result = run.publish_candidate()
            return json.dumps(result, ensure_ascii=False)
        except errors as error:
            return json.dumps({"ok": False, type(error).__name__: str(error)}, ensure_ascii=False)

    @tool
    def run_status() -> str:
        """只读：当前阶段、current head、各工蜂回执、候选提交与独审结论。"""
        return json.dumps(run.status(), ensure_ascii=False)

    if run.spec.v == 2:
        return [operator_pausable(run, t) for t in (run_worker, commit_candidate, publish_candidate, run_status)]
    return [run_worker, commit_candidate, check_gate, run_status]


def operator_pausable(run: PilotRun, wrapped: Any) -> Any:
    """操作员中断：每次工具调用前若有中断请求，图在此 checkpoint 处停下。已派出的工蜂不被杀，恢复时按回执收养。
    恢复载荷的内容一律不用；中断请求由恢复入口清除后才放行。"""

    def call(*args: Any, **kwargs: Any) -> str:
        while run.interrupt_requested() is not None:
            interrupt({"ticket": run.spec.ticket, "operator_interrupt": run.interrupt_requested()})
        from .approval import decision_json
        from .executor import ApprovalRequired

        while True:
            try:
                with run.tool_call():
                    return wrapped.func(*args, **kwargs)
            except ApprovalRequired as required:
                interrupt({"ticket": run.spec.ticket, "awaiting_approval": decision_json(required.decision)})

    return StructuredTool.from_function(call, name=wrapped.name, description=wrapped.description,
                                        args_schema=wrapped.args_schema)


def design_mapping(run: PilotRun) -> str:
    spec = run.spec
    if spec.design_models is None:
        return "- 本 run 创建时没有设计通道，architect 与 arena 的并行候选无法派发。\n"
    required = "本 run 要求设计 arena，judge 完成前 author 会被拒绝。" if spec.task.require_design else ""
    return (f'- architect Phase B 与 arena 的并行候选 = run_worker(role="design", brief)：同一个设计任务同时派给两个固定的只读 '
            f"runner（{spec.design_models[0]} 与 {spec.design_models[1]}），各自独立 checkout，返回两份设计包。"
            f'arena Phase C 的只读交叉裁判 = run_worker(role="judge", brief=你的 rubric)，固定模型 {spec.judge_model}。'
            "Phase D 与 E 由你自己完成：逐份读完两个设计包，按 rubric 逐条打分并与裁判对照，选 base、嫁接。"
            "随后的 author brief 以“合成说明”开头：单独一行写 `BASE: 候选 1` 或 `BASE: 候选 2`，再写裁判推荐及你是否采纳和理由、"
            f"每项嫁接及其来源候选、拒绝了什么及理由，然后才是合成后的设计。缺这一行时 author 会被拒绝。{required}\n")


V1_ENDING = """- 最后调用 check_gate。门需要人类批准，本试点不自批，awaiting_approval 是合法终点。
- 不能 push、开 PR、合 main、发评论或改写集外文件；playbook 里的这些步骤写 skip。
"""


def _v2_ending(run: PilotRun) -> str:
    unit = run.spec.unit
    return (f"- 同 head 异家族独审 PASS 后调用 publish_candidate：宿主用 GitHub App 身份把该冻结 SHA 发布到 `{unit.branch}`、"
            "开 draft PR、写状态与票面交付标记，awaiting_merge 是合法终点。返回 stale 或 halted 时停下汇报，不要绕过。\n"
            "- 你没有其他 push、PR、评论或合 main 的途径；playbook 的 Opening a PR 步骤由 publish_candidate 完成，"
            "合入 main 写 skip（由人决定）。不能改写集外文件。\n"
            f"- 本 run 属于 #{unit.ticket}，登记的 playbook 为 {unit.playbook}。\n")


def project_skills_root(run: PilotRun) -> str:
    if run.execution is not None:
        return PROJECT_TARGET + "/.agents/skills/"
    return str(Path(run.spec.author_checkout) / ".agents/skills") + "/"


def system_prompt(run: PilotRun) -> str:
    if run.execution is not None:
        run.execution.controller_context()
        models = (run.dir / "boat/skill-bundle/rules/pstack-models.md").read_text()
        skills_root = PSTACK_TARGET + "/installed/skills/"
        poteto_mode = skills_root + "poteto-mode/SKILL.md"
    else:
        models = PSTACK_MODELS.read_text() if PSTACK_MODELS.exists() else "(pstack-models.md 缺失)"
        skills_root, poteto_mode = SKILLS_ROOT, POTETO_MODE
    task = run.spec.task
    ending = V1_ENDING if run.spec.v == 1 else _v2_ending(run)
    role = "#1742 Deep Agents 试点" if run.spec.v == 1 else f"#{run.spec.ticket} Deep Agents 新执行器 run"
    return f"""你是 {role}的主控，本线程收到了 $pstack-upstream-poteto-mode。

先用 read_file 读取 /project/AGENTS.md、/project/CONTEXT.md 和任务相关纪律，再完整读取 {poteto_mode}（read_file 每次最多 100 行，用 offset 分页读到文件末尾），按其 Playbooks 选择适用 playbook，读取该 playbook 文件和你实际应用的原则叶技能 SKILL.md，然后用 write_todos 原样抄入 playbook 步骤；不做的步骤保留并写 skip 原因。技能根目录是 {skills_root}，项目技能根目录是 {project_skills_root(run)}。其中相对引用以各技能目录为基准。

本 harness 的映射：
{design_mapping(run)}- pstack 的 Task/spawn_agent 代码委派 = run_worker(role="author", brief)，由 Sandcastle 外部工蜂执行；独立审阅 = run_worker(role="review", brief)。没有其他子代理、shell 或写文件工具。
- 工蜂不 commit；commit_candidate 做宿主受管本地提交并返回 diff，你要自己读 diff 再派独审。
- 逐份消费 receipt.response 或 receipts[i].response 的 conclusion、evidence、actions_taken、confidence、caveats。用 run_status 与 commit_candidate 返回的事实便宜复核，向用户转述结论时保留全部 caveats；ok=false 时只报告诊断，不作为成功答案。
- 独审结论只认 VERDICT: PASS/BLOCK。BLOCK 时把完整 findings、validation、response.caveats 写进新的 author brief 返修，再 commit_candidate、再 run_worker(review)。
{ending}- 一条消息只调用一个业务工具，等它返回再决定下一步。

模型角色表（~/.agents/pstack-models.md，本 run 已冻结：作者 {run.spec.author_model}，独审 {run.spec.reviewer_model}，主控 {run.spec.brain_model}）：
{models}

任务：{task.title}
写集：{", ".join(task.allowed_prefixes)}
需求原文：
{task.requirement}
"""


def initial_message(run: PilotRun) -> str:
    what = "试点" if run.spec.v == 1 else "交付"
    poteto = PSTACK_TARGET + "/installed/skills/poteto-mode/SKILL.md" if run.execution is not None else POTETO_MODE
    return (f"$pstack-upstream-poteto-mode\n\n执行 #{run.spec.ticket} {what} run {run.spec.run_id}：按系统提示完成“{run.spec.task.title}”，"
            f"从读取 {poteto} 全文开始。")


def build_agent(run: PilotRun, checkpointer: Any, model: BaseChatModel | None = None):
    brain = model or build_model(run.spec.brain_model)
    register_harness_profile(f"openai:{run.spec.brain_model}", HarnessProfile(
        general_purpose_subagent=GeneralPurposeSubagentProfile(enabled=False)))
    project_root = project_skills_root(run)
    if run.execution is not None:
        context = run.execution.controller_context()
        project_docs = context / "repo"
        project_skills = context / "project-skills"
        bundle = run.dir / "boat/skill-bundle"
        skills_root = PSTACK_TARGET + "/installed/skills/"
        skills_dir = bundle / "pstack/installed/skills"
        rules_root, rules_dir = RULES_TARGET + "/", bundle / "rules"
    else:
        rules_dir = run.dir / "read-only-rules"
        rules_dir.mkdir(exist_ok=True)
        if PSTACK_MODELS.is_file():
            (rules_dir / "pstack-models.md").write_bytes(PSTACK_MODELS.read_bytes())
        rules_root = str(PSTACK_MODELS.parent) + "/"
        project_docs = run.dir / "read-only-project"
        if project_docs.exists():
            shutil.rmtree(project_docs)
        project_docs.mkdir()
        for relative in ("AGENTS.md", "CONTEXT.md"):
            source = Path(run.spec.author_checkout) / relative
            if source.is_file():
                (project_docs / relative).write_bytes(source.read_bytes())
        for relative in ("docs/agents", ".sandcastle/docs"):
            source = Path(run.spec.author_checkout) / relative
            if source.is_dir():
                shutil.copytree(source, project_docs / relative)
        project_skills = Path(project_root)
        skills_root, skills_dir = SKILLS_ROOT, Path(SKILLS_ROOT)
    routes = {
        "/project/": FilesystemBackend(root_dir=str(project_docs), virtual_mode=True),
        skills_root: FilesystemBackend(root_dir=str(skills_dir), virtual_mode=True),
        project_root: FilesystemBackend(root_dir=str(project_skills), virtual_mode=True),
        rules_root: FilesystemBackend(root_dir=str(rules_dir), virtual_mode=True),
    }
    if run.execution is not None:
        routes[PSTACK_TARGET + "/"] = FilesystemBackend(root_dir=str(bundle / "pstack"), virtual_mode=True)
    backend = CompositeBackend(default=StateBackend(), routes=routes)
    middleware: list[AgentMiddleware[Any, Any, Any]] = [
        FilesystemMiddleware(backend=backend, tools=READ_ONLY_FS_TOOLS), TodoListMiddleware()]
    return create_deep_agent(
        model=brain,
        tools=build_tools(run),
        system_prompt=system_prompt(run),
        middleware=middleware,
        skills=[skills_root, project_root],
        backend=backend,
        checkpointer=checkpointer,
        name="pilot1742",
    )


def _now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


class TraceRecorder(BaseCallbackHandler):
    """把每次模型请求的工具面、模型输出与工具调用写进 agent-trace.jsonl，供验收核对真实轨迹。"""

    def __init__(self, path: Path):
        self.path = path

    def _write(self, **fields: Any) -> None:
        with open(self.path, "a") as fh:
            fh.write(json.dumps({"at": _now(), **fields}, ensure_ascii=False, default=str) + "\n")

    def on_chat_model_start(self, serialized: dict, messages: list, **kwargs: Any) -> None:
        params = kwargs.get("invocation_params") or {}
        tools = [t.get("function", t).get("name") for t in params.get("tools") or []]
        self._write(event="model_request", model=params.get("model") or params.get("model_name"),
                    tools=sorted(tools), message_count=sum(len(m) for m in messages))

    def on_llm_end(self, response: Any, **kwargs: Any) -> None:
        for generations in response.generations:
            for generation in generations:
                message = getattr(generation, "message", None)
                if message is None:
                    continue
                self._write(event="model_output", text=str(message.content)[:4000],
                            tool_calls=[{"name": c["name"], "args": c["args"]} for c in message.tool_calls])

    def on_tool_start(self, serialized: dict, input_str: str, **kwargs: Any) -> None:
        self._write(event="tool_start", tool=serialized.get("name"), input=str(kwargs.get("inputs") or input_str)[:4000])

    def on_tool_end(self, output: Any, **kwargs: Any) -> None:
        self._write(event="tool_end", tool=kwargs.get("name"), output=str(getattr(output, "content", output))[:4000])
