"""FastAPI 网关 — REST + WebSocket 回放系统

端点：
- POST /api/replay/start  — 创建回放会话
- POST /api/replay/step   — 步进
- POST /api/replay/seek   — 跳转
- POST /api/replay/play   — 自动播放（后台 asyncio.Task）
- POST /api/replay/pause  — 暂停
- GET  /api/replay/status  — 查询状态
- WS   /ws/feed            — WebSocket 双向通信

启动方式：
    uvicorn newchan.gateway:app --port 8766
"""

from __future__ import annotations

import asyncio
import uuid
from dataclasses import asdict
from datetime import datetime, timezone
from typing import Any

import pandas as pd
from fastapi import FastAPI, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware

from newchan.a_stroke import Stroke
from newchan.bi_engine import BiEngine, BiEngineSnapshot
from newchan.contracts.ws_messages import (
    ReplayPauseRequest,
    ReplayPlayRequest,
    ReplaySeekRequest,
    ReplaySeekResponse,
    ReplayStartRequest,
    ReplayStartResponse,
    ReplayStatusResponse,
    ReplayStepRequest,
    ReplayStepResponse,
    WsBar,
    WsCommand,
    WsError,
    WsEvent,
    WsReplayStatus,
    WsSnapshot,
)
from newchan.events import DomainEvent
from newchan.orchestrator.timeframes import TFOrchestrator
from newchan.replay import ReplaySession
from newchan.types import Bar

# ── FastAPI 应用 ──

app = FastAPI(title="NewChan Gateway", version="0.1.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173", "http://127.0.0.1:5173"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

# ── 全局状态 ──

# 活跃回放会话：session_id -> ReplaySession
_sessions: dict[str, ReplaySession] = {}

# 多 TF 调度器：session_id -> TFOrchestrator（仅多 TF 会话有）
_orchestrators: dict[str, TFOrchestrator] = {}

# 自动播放后台任务：session_id -> asyncio.Task
_play_tasks: dict[str, asyncio.Task] = {}

# WebSocket 连接：session_id -> set[WebSocket]
_ws_clients: dict[str, set[WebSocket]] = {}


# ════════════════════════════════════════════════
# 工具函数
# ════════════════════════════════════════════════


def _load_bars(symbol: str, interval: str, tf: str) -> list[Bar]:
    """从缓存加载数据，按需 resample，转为 Bar 列表。"""
    from newchan.cache import load_df

    cache_name = f"{symbol}_{interval}_raw"
    df_raw = load_df(cache_name)
    if df_raw is None:
        raise ValueError(f"缓存 {cache_name} 不存在，请先拉取数据")

    # 如果目标周期与原始周期不同，做 resample
    if tf != interval and tf != _interval_to_tf(interval):
        try:
            from newchan.b_timeframe import resample_ohlc
            df_raw = resample_ohlc(df_raw, tf)
        except (ImportError, ValueError):
            # resample 不可用或周期相同，直接使用原始数据
            pass

    return _df_to_bars(df_raw)


def _interval_to_tf(interval: str) -> str:
    """将缓存 interval 格式转为 tf 格式（如 '1min' -> '1m'）。"""
    mapping = {"1min": "1m", "5min": "5m", "15min": "15m", "30min": "30m"}
    return mapping.get(interval, interval)


def _df_to_bars(df: pd.DataFrame) -> list[Bar]:
    """将 OHLCV DataFrame 转为 Bar 列表。"""
    bars: list[Bar] = []
    for ts, row in df.iterrows():
        dt = ts.to_pydatetime() if hasattr(ts, "to_pydatetime") else ts
        if not isinstance(dt, datetime):
            dt = pd.Timestamp(dt).to_pydatetime()
        # 确保有时区信息
        if dt.tzinfo is None:
            dt = dt.replace(tzinfo=timezone.utc)
        vol = float(row["volume"]) if "volume" in row.index and pd.notna(row["volume"]) else None
        bars.append(Bar(
            ts=dt,
            open=float(row["open"]),
            high=float(row["high"]),
            low=float(row["low"]),
            close=float(row["close"]),
            volume=vol,
        ))
    return bars


def _stroke_to_dict(s: Stroke) -> dict:
    """Stroke -> 可序列化 dict。"""
    return {
        "i0": s.i0, "i1": s.i1,
        "direction": s.direction,
        "high": s.high, "low": s.low,
        "p0": s.p0, "p1": s.p1,
        "confirmed": s.confirmed,
    }


def _event_to_ws(ev: DomainEvent, tf: str = "", stream_id: str = "") -> dict:
    """将域事件转为 WsEvent 消息 dict。"""
    _exclude = {"event_type", "bar_idx", "bar_ts", "seq", "event_id", "schema_version"}
    payload = {k: v for k, v in asdict(ev).items() if k not in _exclude}
    return WsEvent(
        event_type=ev.event_type,
        bar_idx=ev.bar_idx,
        bar_ts=ev.bar_ts,
        seq=ev.seq,
        payload=payload,
        event_id=ev.event_id,
        schema_version=ev.schema_version,
        tf=tf,
        stream_id=stream_id,
    ).model_dump()


def _snapshot_to_ws(snap: BiEngineSnapshot) -> dict:
    """BiEngineSnapshot -> WsSnapshot 消息 dict。"""
    return WsSnapshot(
        bar_idx=snap.bar_idx,
        strokes=[_stroke_to_dict(s) for s in snap.strokes],
        event_count=len(snap.events),
    ).model_dump()


def _bar_to_ws(bar: Bar, idx: int, tf: str = "", stream_id: str = "") -> dict:
    """Bar -> WsBar 消息 dict。"""
    ts_epoch = bar.ts.timestamp() if bar.ts.tzinfo else bar.ts.replace(tzinfo=timezone.utc).timestamp()
    return WsBar(
        idx=idx,
        ts=ts_epoch,
        o=bar.open, h=bar.high, l=bar.low, c=bar.close,
        v=bar.volume,
        tf=tf,
        stream_id=stream_id,
    ).model_dump()


def _status_to_ws(session: ReplaySession) -> dict:
    """ReplaySession -> WsReplayStatus 消息 dict。"""
    return WsReplayStatus(
        mode=session.mode,
        current_idx=session.current_idx,
        total_bars=session.total_bars,
        speed=session.speed,
    ).model_dump()


def _get_session(session_id: str) -> ReplaySession:
    """获取会话，不存在则抛出 ValueError。"""
    sess = _sessions.get(session_id)
    if sess is None:
        raise ValueError(f"会话 {session_id} 不存在")
    return sess


async def _broadcast(session_id: str, message: dict) -> None:
    """向指定会话的所有 WS 客户端广播消息。"""
    clients = _ws_clients.get(session_id, set())
    dead: list[WebSocket] = []
    for ws in clients:
        try:
            await ws.send_json(message)
        except Exception:
            dead.append(ws)
    for ws in dead:
        clients.discard(ws)


# ════════════════════════════════════════════════
# REST 端点
# ════════════════════════════════════════════════


@app.post("/api/replay/start", response_model=ReplayStartResponse)
async def replay_start(req: ReplayStartRequest):
    """创建回放会话（支持单 TF 或多 TF）。"""
    try:
        bars = _load_bars(req.symbol, req.interval, req.tf)
    except ValueError as e:
        return WsError(message=str(e), code="data_error").model_dump()

    if not bars:
        return WsError(message="数据为空", code="data_error").model_dump()

    session_id = str(uuid.uuid4())

    # 确定实际 TF 列表
    timeframes = req.timeframes if req.timeframes else [req.tf]

    if len(timeframes) > 1:
        # 多 TF：创建 TFOrchestrator
        orch = TFOrchestrator(
            session_id=session_id,
            base_bars=bars,
            timeframes=timeframes,
            stroke_mode=req.stroke_mode,
            min_strict_sep=req.min_strict_sep,
            symbol=req.symbol,
        )
        _orchestrators[session_id] = orch
        # 也注册 base session 以兼容 _get_session
        _sessions[session_id] = orch.base_session
    else:
        # 单 TF：走原有路径
        engine = BiEngine(stroke_mode=req.stroke_mode, min_strict_sep=req.min_strict_sep)
        session = ReplaySession(
            session_id=session_id,
            bars=bars,
            engine=engine,
        )
        _sessions[session_id] = session

    return ReplayStartResponse(
        session_id=session_id,
        total_bars=len(bars),
        timeframes=timeframes,
    )


async def _step_multi_tf(req, session, orch):
    """多 TF 步进，返回 ReplayStepResponse。"""
    tf_snapshots = orch.step(req.count)
    base_snaps = tf_snapshots.get(orch.base_tf, [])
    if not base_snaps:
        return ReplayStepResponse(bar_idx=session.current_idx - 1)

    events_ws = []
    for tf, snaps in tf_snapshots.items():
        sid = orch._stream_ids.get(tf, "")
        for snap in snaps:
            for ev in snap.events:
                events_ws.append(WsEvent(**_event_to_ws(ev, tf=tf, stream_id=sid)))

    for tf, snaps in tf_snapshots.items():
        tf_session = orch.sessions[tf]
        sid = orch._stream_ids.get(tf, "")
        for snap in snaps:
            bar_idx = snap.bar_idx
            if bar_idx < tf_session.total_bars:
                await _broadcast(req.session_id, _bar_to_ws(tf_session.bars[bar_idx], bar_idx, tf=tf, stream_id=sid))
            for ev in snap.events:
                await _broadcast(req.session_id, _event_to_ws(ev, tf=tf, stream_id=sid))
    await _broadcast(req.session_id, _status_to_ws(session))

    last_snap = base_snaps[-1]
    last_bar_idx = session.current_idx - 1
    bar = session.bars[last_bar_idx] if last_bar_idx < session.total_bars else None
    ws_bar = WsBar(**_bar_to_ws(bar, last_bar_idx)) if bar else None
    return ReplayStepResponse(bar_idx=last_snap.bar_idx, bar=ws_bar, events=events_ws)


async def _step_single_tf(req, session):
    """单 TF 步进，返回 ReplayStepResponse。"""
    snapshots = session.step(req.count)
    if not snapshots:
        return ReplayStepResponse(bar_idx=session.current_idx - 1)

    last_snap = snapshots[-1]
    events_ws = [WsEvent(**_event_to_ws(ev)) for snap in snapshots for ev in snap.events]

    last_bar_idx = session.current_idx - 1
    bar = session.bars[last_bar_idx] if last_bar_idx < session.total_bars else None
    ws_bar = WsBar(**_bar_to_ws(bar, last_bar_idx)) if bar else None

    for snap in snapshots:
        bar_idx = snap.bar_idx
        if bar_idx < session.total_bars:
            await _broadcast(session.session_id, _bar_to_ws(session.bars[bar_idx], bar_idx))
        for ev in snap.events:
            await _broadcast(session.session_id, _event_to_ws(ev))
    await _broadcast(session.session_id, _status_to_ws(session))

    return ReplayStepResponse(bar_idx=last_snap.bar_idx, bar=ws_bar, events=events_ws)


@app.post("/api/replay/step", response_model=ReplayStepResponse)
async def replay_step(req: ReplayStepRequest):
    """步进指定数量的 bar。"""
    try:
        session = _get_session(req.session_id)
    except ValueError as e:
        return WsError(message=str(e), code="session_not_found").model_dump()

    orch = _orchestrators.get(req.session_id)
    if orch is not None:
        return await _step_multi_tf(req, session, orch)
    return await _step_single_tf(req, session)


@app.post("/api/replay/seek", response_model=ReplaySeekResponse)
async def replay_seek(req: ReplaySeekRequest):
    """跳转到指定位置。"""
    try:
        session = _get_session(req.session_id)
    except ValueError as e:
        return WsError(message=str(e), code="session_not_found").model_dump()

    _cancel_play_task(req.session_id)

    orch = _orchestrators.get(req.session_id)
    if orch is not None:
        # 多 TF seek
        tf_snaps = orch.seek(req.target_idx)
        base_snap = tf_snaps.get(orch.base_tf)
    else:
        base_snap = session.seek(req.target_idx)

    snapshot_ws = WsSnapshot(
        bar_idx=base_snap.bar_idx if base_snap else 0,
        strokes=[_stroke_to_dict(s) for s in (base_snap.strokes if base_snap else [])],
        event_count=len(base_snap.events) if base_snap else 0,
    )

    await _broadcast(req.session_id, snapshot_ws.model_dump())
    await _broadcast(req.session_id, _status_to_ws(session))

    return ReplaySeekResponse(
        bar_idx=base_snap.bar_idx if base_snap else 0,
        snapshot=snapshot_ws,
    )


@app.post("/api/replay/play")
async def replay_play(req: ReplayPlayRequest):
    """启动自动播放。"""
    try:
        session = _get_session(req.session_id)
    except ValueError as e:
        return WsError(message=str(e), code="session_not_found").model_dump()

    # 取消已有播放任务
    _cancel_play_task(req.session_id)

    session.speed = req.speed
    session.mode = "playing"

    # 启动后台播放
    task = asyncio.create_task(_play_loop(req.session_id))
    _play_tasks[req.session_id] = task

    return _status_to_ws(session)


@app.post("/api/replay/pause")
async def replay_pause(req: ReplayPauseRequest):
    """暂停自动播放。"""
    try:
        session = _get_session(req.session_id)
    except ValueError as e:
        return WsError(message=str(e), code="session_not_found").model_dump()

    _cancel_play_task(req.session_id)
    if session.mode == "playing":
        session.mode = "paused"

    await _broadcast(session.session_id, _status_to_ws(session))
    return _status_to_ws(session)


@app.get("/api/replay/status", response_model=ReplayStatusResponse)
async def replay_status(session_id: str):
    """查询回放状态。"""
    try:
        session = _get_session(session_id)
    except ValueError as e:
        return WsError(message=str(e), code="session_not_found").model_dump()

    return ReplayStatusResponse(**session.get_status())


# ════════════════════════════════════════════════
# 自动播放
# ════════════════════════════════════════════════


def _cancel_play_task(session_id: str) -> None:
    """取消指定会话的自动播放任务。"""
    task = _play_tasks.pop(session_id, None)
    if task and not task.done():
        task.cancel()


async def _play_multi_tf(session_id: str, session, orch) -> bool:
    """多 TF 播放一步。返回 False 表示应停止。"""
    tf_snapshots = orch.step(1)
    if not tf_snapshots.get(orch.base_tf):
        return False
    for tf, snaps in tf_snapshots.items():
        tf_session = orch.sessions[tf]
        sid = orch._stream_ids.get(tf, "")
        for snap in snaps:
            bi = snap.bar_idx
            if bi < tf_session.total_bars:
                await _broadcast(session_id, _bar_to_ws(tf_session.bars[bi], bi, tf=tf, stream_id=sid))
            for ev in snap.events:
                await _broadcast(session_id, _event_to_ws(ev, tf=tf, stream_id=sid))
    return True


async def _play_single_tf(session_id: str, session) -> bool:
    """单 TF 播放一步。返回 False 表示应停止。"""
    bar_idx = session.current_idx
    snapshots = session.step(1)
    if not snapshots:
        return False
    snap = snapshots[0]
    bar = session.bars[bar_idx] if bar_idx < session.total_bars else None
    if bar is not None:
        await _broadcast(session_id, _bar_to_ws(bar, bar_idx))
    for ev in snap.events:
        await _broadcast(session_id, _event_to_ws(ev))
    return True


async def _play_loop(session_id: str) -> None:
    """自动播放后台循环，按 speed 控制推送间隔。"""
    try:
        session = _sessions.get(session_id)
        if session is None:
            return

        orch = _orchestrators.get(session_id)

        while session.mode == "playing" and session.current_idx < session.total_bars:
            interval = 1.0 / max(session.speed, 0.1)
            await asyncio.sleep(interval)

            if session.mode != "playing":
                break

            if orch is not None:
                ok = await _play_multi_tf(session_id, session, orch)
            else:
                ok = await _play_single_tf(session_id, session)

            if not ok:
                break

            await _broadcast(session_id, _status_to_ws(session))

        if session.mode == "playing":
            session.mode = "done"
            await _broadcast(session_id, _status_to_ws(session))

    except asyncio.CancelledError:
        pass
    finally:
        _play_tasks.pop(session_id, None)


# ════════════════════════════════════════════════
# WebSocket 端点
# ════════════════════════════════════════════════


@app.websocket("/ws/feed")
async def ws_feed(ws: WebSocket):
    """WebSocket 双向通信。

    服务端推送：bar, event, snapshot, replay_status, error
    客户端发送：WsCommand（subscribe, replay_start, replay_step, etc.）
    """
    await ws.accept()
    bound_session_id: str | None = None

    try:
        while True:
            data = await ws.receive_json()
            try:
                cmd = WsCommand(**data)
            except Exception as e:
                await ws.send_json(WsError(message=f"无效命令: {e}", code="invalid_command").model_dump())
                continue

            try:
                await _handle_ws_command(ws, cmd, bound_session_id)
                # 绑定会话（首次 replay_start 后）
                if cmd.action == "replay_start" and bound_session_id is None:
                    # 查找最近创建的会话
                    for sid, sess in reversed(list(_sessions.items())):
                        if ws in _ws_clients.get(sid, set()):
                            bound_session_id = sid
                            break
            except Exception as e:
                await ws.send_json(WsError(message=str(e), code="handler_error").model_dump())

    except WebSocketDisconnect:
        pass
    finally:
        # 清理 WS 连接
        if bound_session_id and bound_session_id in _ws_clients:
            _ws_clients[bound_session_id].discard(ws)


async def _ws_require_session(ws: WebSocket, bound_session_id: str | None) -> str | None:
    """检查 WS 是否绑定了会话，未绑定则发送错误。返回 session_id 或 None。"""
    if bound_session_id is None:
        await ws.send_json(WsError(message="未绑定会话", code="no_session").model_dump())
        return None
    return bound_session_id


async def _handle_ws_replay_start(ws: WebSocket, cmd: WsCommand) -> str | None:
    """处理 replay_start 命令，返回新 session_id。"""
    try:
        bars = _load_bars(cmd.symbol.upper(), "1min", cmd.tf)
    except ValueError as e:
        await ws.send_json(WsError(message=str(e), code="data_error").model_dump())
        return None

    if not bars:
        await ws.send_json(WsError(message="数据为空", code="data_error").model_dump())
        return None

    session_id = str(uuid.uuid4())
    engine = BiEngine()
    session = ReplaySession(session_id=session_id, bars=bars, engine=engine)
    _sessions[session_id] = session

    _ws_clients.setdefault(session_id, set()).add(ws)

    await ws.send_json({
        "type": "replay_started",
        "session_id": session_id,
        "total_bars": session.total_bars,
    })
    await ws.send_json(_status_to_ws(session))
    return session_id


async def _handle_ws_replay_step(ws: WebSocket, bound_session_id: str | None) -> None:
    """处理 replay_step 命令：步进并广播 bar/event/status。"""
    sid = await _ws_require_session(ws, bound_session_id)
    if sid is None:
        return
    session = _get_session(sid)
    snapshots = session.step(1)
    for snap in snapshots:
        bar_idx = snap.bar_idx
        if bar_idx < session.total_bars:
            await _broadcast(sid, _bar_to_ws(session.bars[bar_idx], bar_idx))
        for ev in snap.events:
            await _broadcast(sid, _event_to_ws(ev))
    await _broadcast(sid, _status_to_ws(session))


async def _handle_ws_replay_seek(ws: WebSocket, cmd: WsCommand, bound_session_id: str | None) -> None:
    """处理 replay_seek 命令：跳转并广播 snapshot/status。"""
    sid = await _ws_require_session(ws, bound_session_id)
    if sid is None:
        return
    session = _get_session(sid)
    _cancel_play_task(sid)
    snap = session.seek(cmd.seek_idx)
    if snap:
        await _broadcast(sid, _snapshot_to_ws(snap))
    await _broadcast(sid, _status_to_ws(session))


async def _handle_ws_replay_play(ws: WebSocket, cmd: WsCommand, bound_session_id: str | None) -> None:
    """处理 replay_play 命令：启动自动播放。"""
    sid = await _ws_require_session(ws, bound_session_id)
    if sid is None:
        return
    session = _get_session(sid)
    _cancel_play_task(sid)
    session.speed = cmd.speed
    session.mode = "playing"
    task = asyncio.create_task(_play_loop(sid))
    _play_tasks[sid] = task
    await _broadcast(sid, _status_to_ws(session))


async def _handle_ws_replay_pause(ws: WebSocket, bound_session_id: str | None) -> None:
    """处理 replay_pause 命令：暂停播放。"""
    sid = await _ws_require_session(ws, bound_session_id)
    if sid is None:
        return
    session = _get_session(sid)
    _cancel_play_task(sid)
    if session.mode == "playing":
        session.mode = "paused"
    await _broadcast(sid, _status_to_ws(session))


async def _handle_ws_command(ws: WebSocket, cmd: WsCommand, bound_session_id: str | None) -> None:
    """处理 WS 客户端命令（分派到具体 handler）。"""

    if cmd.action == "replay_start":
        await _handle_ws_replay_start(ws, cmd)
    elif cmd.action == "subscribe":
        if bound_session_id:
            _ws_clients.setdefault(bound_session_id, set()).add(ws)
    elif cmd.action == "replay_step":
        await _handle_ws_replay_step(ws, bound_session_id)
    elif cmd.action == "replay_seek":
        await _handle_ws_replay_seek(ws, cmd, bound_session_id)
    elif cmd.action == "replay_play":
        await _handle_ws_replay_play(ws, cmd, bound_session_id)
    elif cmd.action == "replay_pause":
        await _handle_ws_replay_pause(ws, bound_session_id)
    elif cmd.action == "unsubscribe":
        if bound_session_id and bound_session_id in _ws_clients:
            _ws_clients[bound_session_id].discard(ws)
