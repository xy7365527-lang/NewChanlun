"""Session Handoff Schema — CC session 间的状态传递协议。

409号谱系：Daemon = DAG 中的 liveness 节点。
Handoff Schema 是 Daemon 与 CC session 之间唯一的通信契约。

架构原则：
- Daemon 无状态：不持有认知权威、不持有概念判断
- CC session 短暂性是特性：每个 session 是一次性工作脉冲
- Handoff Schema 是跨 session 的记忆桥梁

持久化路径：.chanlun/handoff/current.json
纯 Python dataclass + JSON 序列化，零外部依赖。
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from datetime import datetime, timezone
from pathlib import Path


# ---------------------------------------------------------------------------
# 持久化路径
# ---------------------------------------------------------------------------

_REPO_ROOT = Path(__file__).resolve().parent.parent
HANDOFF_DIR = _REPO_ROOT / ".chanlun" / "handoff"
HANDOFF_PATH = HANDOFF_DIR / "current.json"
HANDOFF_HISTORY_DIR = HANDOFF_DIR / "history"


# ---------------------------------------------------------------------------
# CC 输入：daemon → CC session
# ---------------------------------------------------------------------------

@dataclass
class SessionInput:
    """Daemon 构造给 CC session 的上下文包。

    CC session 启动时读取此结构，获得：
    1. DAG 当前状态快照（block topology 摘要）
    2. 上一个 session 的结论
    3. 待处理的 agent 返回结果
    4. 外部事件（channel 消息、cron 等）
    5. 纲目位置（当前在知识纲目中的位置）
    """
    # DAG 状态快照（ceremony_scan 输出的子集 + 逢亮穿越状态）
    dag_snapshot: dict = field(default_factory=dict)

    # 上一个 session 的结论摘要
    last_session_conclusion: str = ""

    # 上一个 session 的 ID（用于溯源）
    last_session_id: str = ""

    # 待处理的 agent 返回结果
    pending_agent_results: list[dict] = field(default_factory=list)

    # 外部事件队列（channel 消息、cron 触发等）
    pending_events: list[dict] = field(default_factory=list)

    # 纲目位置：当前在 gangmu.yaml 中的位置标记
    gangmu_position: str = ""

    # ceremony_scan 的最新输出（完整 JSON）
    ceremony_scan_output: dict = field(default_factory=dict)

    # 逢亮穿越状态摘要（从 daemon.status() 获取）
    traversal_status: dict = field(default_factory=dict)


# ---------------------------------------------------------------------------
# CC 输出：CC session → daemon
# ---------------------------------------------------------------------------

@dataclass
class SessionOutput:
    """CC session 产出的结果包。

    CC session 结束时写入此结构，包含：
    1. DAG 变更指令（谱系写入、定义更新等）
    2. 需要派发的任务
    3. tuche 检测结果（structural forcing）
    4. 升级标记（是否需要编排者介入）
    5. session 总结
    """
    # DAG 变更指令列表
    dag_changes: list[dict] = field(default_factory=list)

    # 需要派发的任务（工位描述）
    tasks_to_dispatch: list[dict] = field(default_factory=list)

    # tuche 检测：是否触发了 structural forcing
    tuche_detected: bool = False

    # tuche 描述（如果检测到）
    tuche_description: str = ""

    # 升级标记：是否需要编排者介入
    needs_escalation: bool = False

    # 升级原因
    escalation_reason: str = ""

    # session 总结（本次工作脉冲的产出摘要）
    session_summary: str = ""

    # 新结算的谱系编号列表
    settled_genealogy_ids: list[str] = field(default_factory=list)

    # commit hash（如果本次 session 产生了 commit）
    commit_hash: str = ""


# ---------------------------------------------------------------------------
# 完整 Handoff 记录
# ---------------------------------------------------------------------------

@dataclass
class HandoffRecord:
    """一次完整的 handoff 交接记录。

    包含输入（daemon→CC）和输出（CC→daemon）两侧，
    加上元数据（时间戳、epoch 等）。
    """
    # epoch：单调递增的 session 轮次编号
    epoch: int = 0

    # session 标识符（时间戳格式：YYYY-MM-DD-HHMM）
    session_id: str = ""

    # 输入侧（daemon 构造）
    input: SessionInput = field(default_factory=SessionInput)

    # 输出侧（CC session 写入）
    output: SessionOutput = field(default_factory=SessionOutput)

    # 时间戳
    created_at: str = ""
    completed_at: str = ""

    # 状态：pending（等待 CC 处理）/ completed（CC 已完成）
    status: str = "pending"

    def to_dict(self) -> dict:
        """序列化为 JSON 兼容的 dict。"""
        return {
            "epoch": self.epoch,
            "session_id": self.session_id,
            "status": self.status,
            "created_at": self.created_at,
            "completed_at": self.completed_at,
            "input": {
                "dag_snapshot": self.input.dag_snapshot,
                "last_session_conclusion": self.input.last_session_conclusion,
                "last_session_id": self.input.last_session_id,
                "pending_agent_results": self.input.pending_agent_results,
                "pending_events": self.input.pending_events,
                "gangmu_position": self.input.gangmu_position,
                "ceremony_scan_output": self.input.ceremony_scan_output,
                "traversal_status": self.input.traversal_status,
            },
            "output": {
                "dag_changes": self.output.dag_changes,
                "tasks_to_dispatch": self.output.tasks_to_dispatch,
                "tuche_detected": self.output.tuche_detected,
                "tuche_description": self.output.tuche_description,
                "needs_escalation": self.output.needs_escalation,
                "escalation_reason": self.output.escalation_reason,
                "session_summary": self.output.session_summary,
                "settled_genealogy_ids": self.output.settled_genealogy_ids,
                "commit_hash": self.output.commit_hash,
            },
        }

    @classmethod
    def from_dict(cls, data: dict) -> HandoffRecord:
        """从 dict 反序列化。"""
        inp_data = data.get("input", {})
        out_data = data.get("output", {})

        inp = SessionInput(
            dag_snapshot=inp_data.get("dag_snapshot", {}),
            last_session_conclusion=inp_data.get("last_session_conclusion", ""),
            last_session_id=inp_data.get("last_session_id", ""),
            pending_agent_results=inp_data.get("pending_agent_results", []),
            pending_events=inp_data.get("pending_events", []),
            gangmu_position=inp_data.get("gangmu_position", ""),
            ceremony_scan_output=inp_data.get("ceremony_scan_output", {}),
            traversal_status=inp_data.get("traversal_status", {}),
        )

        out = SessionOutput(
            dag_changes=out_data.get("dag_changes", []),
            tasks_to_dispatch=out_data.get("tasks_to_dispatch", []),
            tuche_detected=out_data.get("tuche_detected", False),
            tuche_description=out_data.get("tuche_description", ""),
            needs_escalation=out_data.get("needs_escalation", False),
            escalation_reason=out_data.get("escalation_reason", ""),
            session_summary=out_data.get("session_summary", ""),
            settled_genealogy_ids=out_data.get("settled_genealogy_ids", []),
            commit_hash=out_data.get("commit_hash", ""),
        )

        return cls(
            epoch=data.get("epoch", 0),
            session_id=data.get("session_id", ""),
            input=inp,
            output=out,
            created_at=data.get("created_at", ""),
            completed_at=data.get("completed_at", ""),
            status=data.get("status", "pending"),
        )


# ---------------------------------------------------------------------------
# 持久化操作
# ---------------------------------------------------------------------------

def _atomic_write_json(path: Path, payload: dict) -> None:
    """原子写入 JSON：tmp→os.replace()。"""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp_path = path.with_suffix(".json.tmp")
    tmp_path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    os.replace(str(tmp_path), str(path))


def write_handoff(record: HandoffRecord) -> None:
    """写入当前 handoff 记录到 current.json，并归档到 history/。"""
    data = record.to_dict()
    _atomic_write_json(HANDOFF_PATH, data)

    # 归档到 history（按 epoch 命名）
    if record.session_id:
        history_path = HANDOFF_HISTORY_DIR / f"{record.session_id}.json"
        _atomic_write_json(history_path, data)


def read_handoff() -> HandoffRecord | None:
    """读取当前 handoff 记录。文件不存在或损坏返回 None。"""
    if not HANDOFF_PATH.exists():
        return None
    try:
        data = json.loads(HANDOFF_PATH.read_text(encoding="utf-8"))
        return HandoffRecord.from_dict(data)
    except (json.JSONDecodeError, OSError, KeyError):
        return None


def create_session_input(
    epoch: int,
    last_handoff: HandoffRecord | None,
    ceremony_scan_output: dict | None = None,
    traversal_status: dict | None = None,
    pending_events: list[dict] | None = None,
) -> HandoffRecord:
    """构造新的 handoff 记录（daemon→CC 方向）。

    从上一轮 handoff 提取 last_session_conclusion，
    合并 ceremony_scan 和逢亮穿越状态。
    """
    now = datetime.now(timezone.utc).isoformat()
    session_id = datetime.now().strftime("%Y-%m-%d-%H%M")

    # 从上一轮 handoff 提取上下文
    last_conclusion = ""
    last_session_id = ""
    gangmu_pos = ""
    if last_handoff is not None:
        last_conclusion = last_handoff.output.session_summary
        last_session_id = last_handoff.session_id
        gangmu_pos = last_handoff.input.gangmu_position

    # DAG 快照从 ceremony_scan 输出提取
    dag_snapshot = {}
    if ceremony_scan_output:
        dag_snapshot = {
            "workstations_count": len(ceremony_scan_output.get("workstations", [])),
            "pending_genealogy": ceremony_scan_output.get("pending", 0),
            "settled_genealogy": ceremony_scan_output.get("settled", 0),
            "definitions_count": ceremony_scan_output.get("definitions", 0),
            "clean_terminate": ceremony_scan_output.get("clean_terminate", False),
            "head": ceremony_scan_output.get("head", ""),
        }

    inp = SessionInput(
        dag_snapshot=dag_snapshot,
        last_session_conclusion=last_conclusion,
        last_session_id=last_session_id,
        pending_agent_results=[],
        pending_events=pending_events or [],
        gangmu_position=gangmu_pos,
        ceremony_scan_output=ceremony_scan_output or {},
        traversal_status=traversal_status or {},
    )

    return HandoffRecord(
        epoch=epoch,
        session_id=session_id,
        input=inp,
        output=SessionOutput(),
        created_at=now,
        status="pending",
    )


def mark_session_complete(
    record: HandoffRecord,
    output: SessionOutput,
) -> HandoffRecord:
    """标记 session 完成，填入输出侧。返回新的 HandoffRecord（不可变）。"""
    return HandoffRecord(
        epoch=record.epoch,
        session_id=record.session_id,
        input=record.input,
        output=output,
        created_at=record.created_at,
        completed_at=datetime.now(timezone.utc).isoformat(),
        status="completed",
    )
