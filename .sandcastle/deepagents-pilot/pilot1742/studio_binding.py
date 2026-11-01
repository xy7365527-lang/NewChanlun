"""官方 Studio 与已登记 v2 run 的绑定。

Agent Server 的线程是 UUID，run 自己的 SQLite checkpoint 用 run_id 作 thread_id。每个绑定 run 对应一个确定的
线程 UUID；官方 `checkpointer.path` 接口挂上 RunBoundCheckpointer，把该线程的读写原样转到 run 目录的
graph.sqlite，CLI 与 Studio 因此续的是同一条 checkpoint。未绑定线程读为空、写一律拒绝；绑定线程只接受持有该 run
锁的那次官方执行（Lease）写入。
"""

from __future__ import annotations

import os
import threading
import uuid
from collections.abc import AsyncIterator, Sequence
from contextlib import asynccontextmanager
from contextvars import ContextVar
from pathlib import Path
from typing import Any

from langchain_core.runnables import RunnableConfig
from langgraph.checkpoint.base import (
    BaseCheckpointSaver,
    ChannelVersions,
    Checkpoint,
    CheckpointMetadata,
    CheckpointTuple,
)
from langgraph.checkpoint.sqlite.aio import AsyncSqliteSaver

from .domain import RunId, parse_run_id

NAMESPACE = uuid.UUID("5f0c1742-7d1e-4c3a-9a7b-6e1742d0a1b2")
NAMESPACE_GRAPH = uuid.UUID("6ba7b821-9dad-11d1-80b4-00c04fd430c8")
GRAPH_ID = "pilot1742"
ASSISTANT_IDS = frozenset({GRAPH_ID, str(uuid.uuid5(NAMESPACE_GRAPH, GRAPH_ID))})


def thread_for(run_id: str) -> str:
    return str(uuid.uuid5(NAMESPACE, f"pilot1742:{run_id}"))


def state_root() -> Path:
    return Path(os.environ.get("PILOT1742_STATE_ROOT", "/Volumes/AgentStorage/deepagents-exec"))


def bound_runs() -> dict[str, RunId]:
    """线程 UUID → run_id。只有服务启动环境里显式列出的 run 才绑定。"""
    raw = os.environ.get("PILOT1742_STUDIO_RUNS", "")
    runs = [parse_run_id(name.strip()) for name in raw.split(",") if name.strip()]
    return {thread_for(run): run for run in runs}


def run_for_thread(thread_id: object) -> RunId | None:
    return bound_runs().get(str(thread_id)) if thread_id else None


class Lease:
    """一次官方执行在 run 锁内签出的 checkpoint 写权。

    签出写进当前执行上下文（ContextVar），只有该执行及其派生的子任务看得见；同进程里的其他请求是另一条上下文，
    拿不到它。锁释放前作废，作废后即使持有旧上下文副本也写不进。"""

    __slots__ = ("run_id", "_active")

    def __init__(self, run_id: RunId) -> None:
        self.run_id = run_id
        self._active = threading.Event()
        self._active.set()

    @property
    def active(self) -> bool:
        return self._active.is_set()

    def revoke(self) -> None:
        self._active.clear()


_LEASE: ContextVar[Lease | None] = ContextVar("pilot1742_checkpoint_lease", default=None)


def grant(run_id: RunId) -> Lease:
    """只在已持有该 run 锁的执行上下文里调用，且不能放进 to_thread：那里的设置不会回到调用方上下文。"""
    lease = Lease(run_id)
    _LEASE.set(lease)
    return lease


class UnboundThreadWrite(PermissionError):
    pass


def _swap(config: RunnableConfig | None, thread_id: str) -> RunnableConfig | None:
    if config is None:
        return None
    configurable = {**config.get("configurable", {}), "thread_id": thread_id}
    return {**config, "configurable": configurable}


def _back(item: CheckpointTuple, thread_id: str) -> CheckpointTuple:
    return CheckpointTuple(_swap(item.config, thread_id) or item.config, item.checkpoint, item.metadata,
                           _swap(item.parent_config, thread_id), item.pending_writes)


class RunBoundCheckpointer(BaseCheckpointSaver[str]):
    """官方自定义 checkpointer：绑定线程转到 run 的 graph.sqlite，其余线程不存储任何东西。"""

    def __init__(self, root: Path | None = None) -> None:
        super().__init__()
        self.root = root

    def _target(self, config: RunnableConfig) -> tuple[RunId, Path] | None:
        run_id = run_for_thread(config.get("configurable", {}).get("thread_id"))
        if run_id is None:
            return None
        return run_id, (self.root or state_root()) / "runs" / run_id / "graph.sqlite"

    @asynccontextmanager
    async def _saver(self, path: Path) -> AsyncIterator[AsyncSqliteSaver]:
        async with AsyncSqliteSaver.from_conn_string(str(path)) as saver:
            yield saver

    def _writable(self, config: RunnableConfig) -> tuple[RunId, Path]:
        target = self._target(config)
        if target is None:
            raise UnboundThreadWrite("未绑定已登记 v2 run 的线程不可写")
        lease = _LEASE.get()
        if lease is None or not lease.active or lease.run_id != target[0]:
            raise UnboundThreadWrite(f"当前请求不是持有 run {target[0]} 锁的那次执行，拒绝写 checkpoint")
        return target

    async def aget_tuple(self, config: RunnableConfig) -> CheckpointTuple | None:
        target = self._target(config)
        if target is None or not target[1].exists():
            return None
        thread_id = str(config["configurable"]["thread_id"])
        async with self._saver(target[1]) as saver:
            item = await saver.aget_tuple(_swap(config, target[0]) or config)
        return _back(item, thread_id) if item else None

    async def alist(self, config: RunnableConfig | None, *, filter: dict[str, Any] | None = None,
                    before: RunnableConfig | None = None, limit: int | None = None) -> AsyncIterator[CheckpointTuple]:
        target = self._target(config) if config else None
        if target is None or not target[1].exists():
            return
        assert config is not None
        thread_id = str(config["configurable"]["thread_id"])
        async with self._saver(target[1]) as saver:
            async for item in saver.alist(_swap(config, target[0]), filter=filter,
                                          before=_swap(before, target[0]), limit=limit):
                yield _back(item, thread_id)

    async def aput(self, config: RunnableConfig, checkpoint: Checkpoint, metadata: CheckpointMetadata,
                   new_versions: ChannelVersions) -> RunnableConfig:
        run_id, path = self._writable(config)
        async with self._saver(path) as saver:
            stored = await saver.aput(_swap(config, run_id) or config, checkpoint, metadata, new_versions)
        return _swap(stored, str(config["configurable"]["thread_id"])) or stored

    async def aput_writes(self, config: RunnableConfig, writes: Sequence[tuple[str, Any]], task_id: str,
                          task_path: str = "") -> None:
        run_id, path = self._writable(config)
        async with self._saver(path) as saver:
            await saver.aput_writes(_swap(config, run_id) or config, writes, task_id, task_path)

    def get_next_version(self, current: str | None, channel: None) -> str:
        return AsyncSqliteSaver.get_next_version(self, current, channel)  # type: ignore[arg-type]


def make_checkpointer() -> RunBoundCheckpointer:
    return RunBoundCheckpointer()
