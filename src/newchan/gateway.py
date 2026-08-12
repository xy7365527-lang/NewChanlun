"""FastAPI 网关 — REST + WebSocket 回放 + 实时推送

端点：
- POST /api/replay/start  — 创建回放会话
- POST /api/replay/step   — 步进
- POST /api/replay/seek   — 跳转
- POST /api/replay/play   — 自动播放（后台 asyncio.Task）
- POST /api/replay/pause  — 暂停
- GET  /api/replay/status  — 查询状态
- WS   /ws/feed            — WebSocket 双向通信（回放模式）
- WS   /ws/live/{symbol}   — WebSocket 实时推送（live 模式）

启动方式：
    uvicorn newchan.gateway:app --port 8766
"""

from __future__ import annotations

import asyncio
import logging
import threading
import uuid
from dataclasses import asdict
from datetime import datetime, timezone
from typing import Any

import pandas as pd
from fastapi import FastAPI, HTTPException, Request, WebSocket, WebSocketDisconnect
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import JSONResponse

from newchan.a_stroke import Stroke
from newchan.backpressure import BackpressureQueue
from newchan.bi_engine import BiEngineSnapshot
from newchan.orchestrator.recursive import (
    RecursiveOrchestrator,
    RecursiveOrchestratorSnapshot,
)
from newchan.contracts.ws_messages import (
    ReplayControlRequest,
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


@app.exception_handler(Exception)
async def _unhandled_exception_handler(request: Request, exc: Exception):
    """全局兜底：未捕获异常 → 500 JSON（不泄露堆栈）。"""
    logger.exception("Unhandled exception on %s %s", request.method, request.url.path)
    return JSONResponse(
        status_code=500,
        content=WsError(message="服务器内部错误", code="internal_error").model_dump(),
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

# ── Live 模式全局状态 ──

logger = logging.getLogger(__name__)

# Live 模式：symbol -> RecursiveOrchestrator（每个标的一个引擎）
_live_engines: dict[str, RecursiveOrchestrator] = {}

# Live 模式：symbol -> 最新快照（每次 process_bar 后更新）
_live_snapshots: dict[str, RecursiveOrchestratorSnapshot] = {}

# Live 模式：symbol -> bar 计数（引擎已处理的 bar 数）
_live_bar_counts: dict[str, int] = {}

# Live 模式 WS 客户端：symbol -> set[WebSocket]
_live_clients: dict[str, set[WebSocket]] = {}

# Live 模式：每个 WS 客户端的背压队列和消费任务
_live_bp_queues: dict[WebSocket, BackpressureQueue] = {}
_live_bp_tasks: dict[WebSocket, asyncio.Task] = {}

# Live 模式：asyncio event loop 引用（用于从 feeder 线程调度到 async）
_live_loop: asyncio.AbstractEventLoop | None = None

# Live 预热竞态防护：预热期间缓冲 feeder bar，结束后按 ts 去重回放
_live_lock = threading.Lock()
_live_warming: set[str] = set()
_live_pending_bars: dict[str, list[Bar]] = {}


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


def _snap_events(snap: RecursiveOrchestratorSnapshot | BiEngineSnapshot) -> list[DomainEvent]:
    """统一获取快照中的事件列表。"""
    if isinstance(snap, RecursiveOrchestratorSnapshot):
        return snap.all_events
    return snap.events


def _snap_strokes(snap: RecursiveOrchestratorSnapshot | BiEngineSnapshot) -> list[Stroke]:
    """统一获取快照中的笔列表。"""
    if isinstance(snap, RecursiveOrchestratorSnapshot):
        return snap.bi_snapshot.strokes
    return snap.strokes


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


def _segment_to_dict(seg) -> dict:
    """Segment -> 可序列化 dict。"""
    return {
        "s0": seg.s0, "s1": seg.s1,
        "i0": seg.i0, "i1": seg.i1,
        "direction": seg.direction,
        "high": seg.high, "low": seg.low,
        "confirmed": seg.confirmed,
        "kind": seg.kind,
        "ep0_price": seg.ep0_price, "ep1_price": seg.ep1_price,
    }


def _zhongshu_to_dict(zs) -> dict:
    """Zhongshu -> 可序列化 dict。"""
    return {
        "zd": zs.zd, "zg": zs.zg,
        "seg_start": zs.seg_start, "seg_end": zs.seg_end,
        "seg_count": zs.seg_count, "settled": zs.settled,
        "break_seg": zs.break_seg, "break_direction": zs.break_direction,
        "gg": zs.gg, "dd": zs.dd,
    }


def _move_to_dict(m) -> dict:
    """Move -> 可序列化 dict。"""
    return {
        "kind": m.kind, "direction": m.direction,
        "seg_start": m.seg_start, "seg_end": m.seg_end,
        "zs_start": m.zs_start, "zs_end": m.zs_end,
        "zs_count": m.zs_count, "settled": m.settled,
        "high": m.high, "low": m.low,
    }


def _bsp_to_dict(bp) -> dict:
    """BuySellPoint -> 可序列化 dict。"""
    return {
        "kind": bp.kind, "side": bp.side,
        "level_id": bp.level_id, "seg_idx": bp.seg_idx,
        "price": bp.price, "confirmed": bp.confirmed,
        "settled": bp.settled,
        "overlaps_with": bp.overlaps_with,
    }


def _lstar_to_dict(ls) -> dict:
    """LStar -> 可序列化 dict。"""
    return {
        "level": ls.level,
        "center_idx": ls.center_idx,
        "regime": ls.regime.value if hasattr(ls.regime, "value") else str(ls.regime),
    }


def _recursive_level_to_dict(rl) -> dict:
    """RecursiveLevelSnapshot -> 可序列化 dict。"""
    return {
        "level_id": rl.level_id,
        "zhongshus": [
            {
                "zd": z.zd, "zg": z.zg,
                "comp_start": z.comp_start, "comp_end": z.comp_end,
                "comp_count": z.comp_count, "settled": z.settled,
                "break_comp": z.break_comp, "break_direction": z.break_direction,
                "gg": z.gg, "dd": z.dd, "level_id": z.level_id,
            }
            for z in rl.zhongshus
        ],
        "moves": [_move_to_dict(m) for m in rl.moves],
    }


def _snapshot_to_ws(snap: RecursiveOrchestratorSnapshot | BiEngineSnapshot) -> dict:
    """RecursiveOrchestratorSnapshot 或 BiEngineSnapshot -> WsSnapshot 消息 dict。"""
    if isinstance(snap, RecursiveOrchestratorSnapshot):
        strokes = snap.bi_snapshot.strokes
        event_count = len(snap.all_events)
        return WsSnapshot(
            bar_idx=snap.bar_idx,
            strokes=[_stroke_to_dict(s) for s in strokes],
            event_count=event_count,
            segments=[_segment_to_dict(s) for s in snap.seg_snapshot.segments],
            centers=[_zhongshu_to_dict(z) for z in snap.zs_snapshot.zhongshus],
            moves=[_move_to_dict(m) for m in snap.move_snapshot.moves],
            bsp=[_bsp_to_dict(b) for b in snap.bsp_snapshot.buysellpoints],
            lstar=_lstar_to_dict(snap.lstar) if snap.lstar else None,
            recursive_snapshots=[
                _recursive_level_to_dict(r) for r in snap.recursive_snapshots
            ] or None,
        ).model_dump()
    else:
        strokes = snap.strokes
        event_count = len(snap.events)
        return WsSnapshot(
            bar_idx=snap.bar_idx,
            strokes=[_stroke_to_dict(s) for s in strokes],
            event_count=event_count,
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


def _get_session_or_404(session_id: str) -> ReplaySession:
    """获取会话，不存在则抛出 HTTPException 404。"""
    sess = _sessions.get(session_id)
    if sess is None:
        raise HTTPException(status_code=404, detail=f"会话 {session_id} 不存在")
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
        raise HTTPException(status_code=404, detail=str(e))

    if not bars:
        raise HTTPException(status_code=404, detail="数据为空")

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
        engine = RecursiveOrchestrator(stroke_mode=req.stroke_mode, min_strict_sep=req.min_strict_sep)
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
            for ev in _snap_events(snap):
                events_ws.append(WsEvent(**_event_to_ws(ev, tf=tf, stream_id=sid)))

    for tf, snaps in tf_snapshots.items():
        tf_session = orch.sessions[tf]
        sid = orch._stream_ids.get(tf, "")
        for snap in snaps:
            bar_idx = snap.bar_idx
            if bar_idx < tf_session.total_bars:
                await _broadcast(req.session_id, _bar_to_ws(tf_session.bars[bar_idx], bar_idx, tf=tf, stream_id=sid))
            for ev in _snap_events(snap):
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
    events_ws = [WsEvent(**_event_to_ws(ev)) for snap in snapshots for ev in _snap_events(snap)]

    last_bar_idx = session.current_idx - 1
    bar = session.bars[last_bar_idx] if last_bar_idx < session.total_bars else None
    ws_bar = WsBar(**_bar_to_ws(bar, last_bar_idx)) if bar else None

    for snap in snapshots:
        bar_idx = snap.bar_idx
        if bar_idx < session.total_bars:
            await _broadcast(session.session_id, _bar_to_ws(session.bars[bar_idx], bar_idx))
        for ev in _snap_events(snap):
            await _broadcast(session.session_id, _event_to_ws(ev))
    await _broadcast(session.session_id, _status_to_ws(session))

    return ReplayStepResponse(bar_idx=last_snap.bar_idx, bar=ws_bar, events=events_ws)


@app.post("/api/replay/step", response_model=ReplayStepResponse)
async def replay_step(req: ReplayStepRequest):
    """步进指定数量的 bar。"""
    session = _get_session_or_404(req.session_id)

    orch = _orchestrators.get(req.session_id)
    if orch is not None:
        return await _step_multi_tf(req, session, orch)
    return await _step_single_tf(req, session)


@app.post("/api/replay/seek", response_model=ReplaySeekResponse)
async def replay_seek(req: ReplaySeekRequest):
    """跳转到指定位置。"""
    session = _get_session_or_404(req.session_id)

    _cancel_play_task(req.session_id)

    orch = _orchestrators.get(req.session_id)
    if orch is not None:
        # 多 TF seek
        tf_snaps = orch.seek(req.target_idx)
        base_snap = tf_snaps.get(orch.base_tf)
    else:
        base_snap = session.seek(req.target_idx)

    if base_snap is not None:
        snapshot_dict = _snapshot_to_ws(base_snap)
    else:
        snapshot_dict = WsSnapshot(bar_idx=0, strokes=[], event_count=0).model_dump()
    snapshot_ws = WsSnapshot(**snapshot_dict)

    await _broadcast(req.session_id, snapshot_dict)
    await _broadcast(req.session_id, _status_to_ws(session))

    return ReplaySeekResponse(
        bar_idx=base_snap.bar_idx if base_snap else 0,
        snapshot=snapshot_ws,
    )


@app.post("/api/replay/play")
async def replay_play(req: ReplayPlayRequest):
    """启动自动播放。"""
    session = _get_session_or_404(req.session_id)

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
    session = _get_session_or_404(req.session_id)

    _cancel_play_task(req.session_id)
    if session.mode == "playing":
        session.mode = "paused"

    await _broadcast(session.session_id, _status_to_ws(session))
    return _status_to_ws(session)


@app.get("/api/replay/status", response_model=ReplayStatusResponse)
async def replay_status(session_id: str):
    """查询回放状态。"""
    session = _get_session_or_404(session_id)

    return ReplayStatusResponse(**session.get_status())


@app.post("/api/replay/control")
async def replay_control(req: ReplayControlRequest):
    """统一回放控制端点 — play/pause/step/seek/stop。"""
    session = _get_session_or_404(req.session_id)

    if req.action == "play":
        _cancel_play_task(req.session_id)
        session.speed = req.speed
        session.mode = "playing"
        task = asyncio.create_task(_play_loop(req.session_id))
        _play_tasks[req.session_id] = task
        return _status_to_ws(session)

    elif req.action == "pause":
        _cancel_play_task(req.session_id)
        if session.mode == "playing":
            session.mode = "paused"
        await _broadcast(session.session_id, _status_to_ws(session))
        return _status_to_ws(session)

    elif req.action == "step":
        orch = _orchestrators.get(req.session_id)
        step_req = ReplayStepRequest(session_id=req.session_id, count=req.count)
        if orch is not None:
            result = await _step_multi_tf(step_req, session, orch)
        else:
            result = await _step_single_tf(step_req, session)
        return result.model_dump()

    elif req.action == "seek":
        _cancel_play_task(req.session_id)
        orch = _orchestrators.get(req.session_id)
        if orch is not None:
            tf_snaps = orch.seek(req.target_idx)
            base_snap = tf_snaps.get(orch.base_tf)
        else:
            base_snap = session.seek(req.target_idx)
        if base_snap is not None:
            snapshot_dict = _snapshot_to_ws(base_snap)
        else:
            snapshot_dict = WsSnapshot(bar_idx=0, strokes=[], event_count=0).model_dump()
        await _broadcast(req.session_id, snapshot_dict)
        await _broadcast(req.session_id, _status_to_ws(session))
        return _status_to_ws(session)

    elif req.action == "stop":
        _cancel_play_task(req.session_id)
        session.mode = "idle"
        # 清理会话
        _sessions.pop(req.session_id, None)
        _orchestrators.pop(req.session_id, None)
        _ws_clients.pop(req.session_id, None)
        return {"mode": "idle", "current_idx": 0, "total_bars": 0, "speed": 1}

    raise HTTPException(status_code=400, detail=f"未知 action: {req.action}")


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
            for ev in _snap_events(snap):
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
    for ev in _snap_events(snap):
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
    engine = RecursiveOrchestrator()
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
        for ev in _snap_events(snap):
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


# ════════════════════════════════════════════════
# Live 模式 — 实时 WebSocket 推送
# ════════════════════════════════════════════════


def _ensure_live_engine(symbol: str) -> RecursiveOrchestrator:
    """获取或创建指定标的的 live 引擎，用缓存中已有的 bar 预热。

    预热与 feeder 竞态：
    - 预热期间 ``_on_live_bar`` 不得因 engine 未注册而丢 bar
    - 也不得把实时 bar 插入预热序列造成乱序
    - 做法：标记 warming → 缓冲 pending → 预热 → 按 ts 去重 flush → 注册
    """
    with _live_lock:
        existing = _live_engines.get(symbol)
        if existing is not None:
            return existing
        _live_warming.add(symbol)
        _live_pending_bars.setdefault(symbol, [])

    engine = RecursiveOrchestrator(stream_id=f"live-{symbol}")
    snap = None
    warmed_bars: list[Bar] = []
    try:
        warmed_bars = _load_bars(symbol, "1min", "1m")
        for bar in warmed_bars:
            snap = engine.process_bar(bar)
        logger.info("Live engine %s 预热完成: %d bars", symbol, len(warmed_bars))
    except (ValueError, Exception) as e:
        logger.warning("Live engine %s 预热失败（无缓存数据）: %s", symbol, e)

    last_warmed_ts = warmed_bars[-1].ts if warmed_bars else None
    flushed = 0
    with _live_lock:
        pending = _live_pending_bars.pop(symbol, [])
        _live_warming.discard(symbol)
        for bar in pending:
            if last_warmed_ts is not None and bar.ts <= last_warmed_ts:
                continue  # 已包含在预热缓存中，避免重复
            snap = engine.process_bar(bar)
            last_warmed_ts = bar.ts
            flushed += 1
        _live_bar_counts[symbol] = engine._bi_engine.bar_count  # noqa: SLF001
        if snap is not None:
            _live_snapshots[symbol] = snap
        _live_engines[symbol] = engine
        if pending:
            logger.info(
                "Live engine %s flush pending: buffered=%d applied=%d count=%d",
                symbol, len(pending), flushed, _live_bar_counts[symbol],
            )
    return engine


def _live_current_snapshot(symbol: str) -> dict:
    """获取 live 引擎当前状态的 WsSnapshot dict。"""
    snap = _live_snapshots.get(symbol)
    if snap is not None:
        return _snapshot_to_ws(snap)
    return WsSnapshot(bar_idx=0, strokes=[], event_count=0).model_dump()


async def _live_broadcast(symbol: str, message: dict) -> None:
    """向指定标的的所有 live WS 客户端投递消息（经背压队列）。"""
    clients = _live_clients.get(symbol, set())
    for ws in clients:
        bpq = _live_bp_queues.get(ws)
        if bpq is not None:
            bpq.put_nowait(message)


async def _live_ws_consumer(ws: WebSocket, bpq: BackpressureQueue) -> None:
    """从背压队列取消息并发送到 WS 客户端。发送失败时静默退出。"""
    try:
        while True:
            message = await bpq.get()
            try:
                await ws.send_json(message)
            except Exception:
                break
    except asyncio.CancelledError:
        pass


def _on_live_bar(symbol: str, bar: Bar) -> None:
    """DatabentoLiveFeeder 的 on_bar 回调（在 feeder 线程中执行）。

    将 bar 送入对应引擎，然后通过 event loop 调度异步广播。
    若引擎正在预热，则缓冲到 ``_live_pending_bars``，由
    ``_ensure_live_engine`` 结束后按时间戳去重回放——禁止静默丢弃。
    """
    with _live_lock:
        if symbol in _live_warming:
            _live_pending_bars.setdefault(symbol, []).append(bar)
            return
        engine = _live_engines.get(symbol)
        if engine is None:
            return

    snap = engine.process_bar(bar)
    with _live_lock:
        bar_idx = _live_bar_counts.get(symbol, 0)
        _live_bar_counts[symbol] = bar_idx + 1
        _live_snapshots[symbol] = snap

    clients = _live_clients.get(symbol, set())
    if not clients:
        return

    bar_msg = _bar_to_ws(bar, bar_idx, stream_id=f"live-{symbol}")
    event_msgs = [_event_to_ws(ev, stream_id=f"live-{symbol}") for ev in snap.all_events]
    snapshot_msg = _snapshot_to_ws(snap)

    loop = _live_loop
    if loop is None or loop.is_closed():
        return

    async def _push():
        await _live_broadcast(symbol, bar_msg)
        for ev_msg in event_msgs:
            await _live_broadcast(symbol, ev_msg)
        await _live_broadcast(symbol, snapshot_msg)

    loop.call_soon_threadsafe(asyncio.ensure_future, _push())


@app.websocket("/ws/live/{symbol}")
async def ws_live(ws: WebSocket, symbol: str):
    """Live 模式 WebSocket — 实时推送缠论分析结果。

    连接后立即发送当前 snapshot，之后每收到新 bar 推送增量更新。
    每个客户端持有独立的 BackpressureQueue，队列满时丢弃非关键帧。
    """
    global _live_loop
    symbol = symbol.upper()

    await ws.accept()

    # 捕获 event loop 引用（供 feeder 线程回调使用）
    _live_loop = asyncio.get_running_loop()

    # 确保引擎已初始化
    _ensure_live_engine(symbol)

    # 创建背压队列和消费任务
    bpq = BackpressureQueue(maxsize=100)
    _live_bp_queues[ws] = bpq
    consumer_task = asyncio.create_task(_live_ws_consumer(ws, bpq))
    _live_bp_tasks[ws] = consumer_task

    # 注册客户端
    _live_clients.setdefault(symbol, set()).add(ws)

    try:
        # 发送当前快照
        snapshot = _live_current_snapshot(symbol)
        await ws.send_json(snapshot)

        # 保持连接，等待客户端消息或断连
        while True:
            try:
                data = await ws.receive_json()
                # 客户端可发送 ping 或其他控制消息
                if isinstance(data, dict) and data.get("action") == "ping":
                    await ws.send_json({"type": "pong"})
            except WebSocketDisconnect:
                break
    finally:
        _live_clients.get(symbol, set()).discard(ws)
        # 清理背压队列和消费任务
        consumer_task.cancel()
        try:
            await consumer_task
        except asyncio.CancelledError:
            pass
        _live_bp_queues.pop(ws, None)
        _live_bp_tasks.pop(ws, None)


# ════════════════════════════════════════════════
# Live 状态监控端点
# ════════════════════════════════════════════════


@app.get("/api/live/status")
async def api_live_status():
    """返回各标的连接状态、最后更新时间、重连次数、背压队列深度。"""
    import time as _time

    symbols_status: dict[str, dict] = {}
    for symbol, engine in _live_engines.items():
        snap = _live_snapshots.get(symbol)
        last_ts = engine.last_bar_ts if hasattr(engine, "last_bar_ts") else None
        # RecursiveOrchestrator 没有 last_bar_ts，从 _live_snapshots 推断
        last_update: float | None = None
        if snap is not None and hasattr(snap, "bar_idx"):
            last_update = _time.time()  # 近似：有 snapshot 说明曾收到数据

        # 统计该 symbol 所有客户端的背压队列
        clients = _live_clients.get(symbol, set())
        client_count = len(clients)
        total_queue_depth = 0
        total_dropped = 0
        for ws in clients:
            bpq = _live_bp_queues.get(ws)
            if bpq is not None:
                total_queue_depth += bpq.qsize()
                total_dropped += bpq.dropped_count

        symbols_status[symbol] = {
            "bar_count": _live_bar_counts.get(symbol, 0),
            "connected_clients": client_count,
            "queue_depth": total_queue_depth,
            "dropped_frames": total_dropped,
        }

    return {
        "active_symbols": len(_live_engines),
        "symbols": symbols_status,
    }
