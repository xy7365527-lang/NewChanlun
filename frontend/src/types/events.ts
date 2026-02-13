/**
 * 域事件类型定义 — 笔事件 + 线段事件 + 中枢事件 + 走势类型事件
 *
 * 与后端 src/newchan/events.py 和 contracts/ws_messages.py 保持一致。
 * 所有时间戳使用 epoch 秒 (number)，与 overlay 坐标系对齐。
 */

// ── 域事件 ──

export interface DomainEventBase {
  event_type: string;
  bar_idx: number;
  bar_ts: number; // epoch 秒
  seq: number;
  event_id: string;
  schema_version: number;
  tf?: string; // 多 TF 标识（空串 = base TF）
  stream_id?: string; // MVP-B0: 流标识
}

export interface StrokeCandidateEvent extends DomainEventBase {
  event_type: "stroke_candidate";
  stroke_id: number;
  direction: "up" | "down";
  i0: number;
  i1: number;
  p0: number;
  p1: number;
}

export interface StrokeSettledEvent extends DomainEventBase {
  event_type: "stroke_settled";
  stroke_id: number;
  direction: "up" | "down";
  i0: number;
  i1: number;
  p0: number;
  p1: number;
}

export interface StrokeExtendedEvent extends DomainEventBase {
  event_type: "stroke_extended";
  stroke_id: number;
  direction: "up" | "down";
  old_i1: number;
  new_i1: number;
  old_p1: number;
  new_p1: number;
}

export interface StrokeInvalidatedEvent extends DomainEventBase {
  event_type: "stroke_invalidated";
  stroke_id: number;
  direction: "up" | "down";
  i0: number;
  i1: number;
  p0: number;
  p1: number;
}

// ── 线段事件（MVP-B1）──

export interface SegmentBreakPendingEvent extends DomainEventBase {
  event_type: "segment_break_pending";
  segment_id: number;
  direction: "up" | "down";
  break_at_stroke: number;
  gap_class: "none" | "gap";
  fractal_type: "top" | "bottom";
  s0: number;
  s1: number;
}

export interface SegmentSettleEvent extends DomainEventBase {
  event_type: "segment_settle";
  segment_id: number;
  direction: "up" | "down";
  s0: number;
  s1: number;
  ep0_price: number;
  ep1_price: number;
  gap_class: "none" | "gap";
  new_segment_s0: number;
  new_segment_direction: "up" | "down";
}

export interface SegmentInvalidateEvent extends DomainEventBase {
  event_type: "segment_invalidate";
  segment_id: number;
  direction: "up" | "down";
  s0: number;
  s1: number;
}

// -- 中枢事件（MVP-C0）--

export interface ZhongshuCandidateEvent extends DomainEventBase {
  event_type: "zhongshu_candidate";
  zhongshu_id: number;
  zd: number;
  zg: number;
  seg_start: number;
  seg_end: number;
  seg_count: number;
}

export interface ZhongshuSettleEvent extends DomainEventBase {
  event_type: "zhongshu_settle";
  zhongshu_id: number;
  zd: number;
  zg: number;
  seg_start: number;
  seg_end: number;
  seg_count: number;
  break_seg_id: number;
  break_direction: "up" | "down";
}

export interface ZhongshuInvalidateEvent extends DomainEventBase {
  event_type: "zhongshu_invalidate";
  zhongshu_id: number;
  zd: number;
  zg: number;
  seg_start: number;
  seg_end: number;
}

// -- 走势类型事件（MVP-D0）--

export interface MoveCandidateEvent extends DomainEventBase {
  event_type: "move_candidate";
  move_id: number;
  kind: "consolidation" | "trend";
  direction: "up" | "down";
  seg_start: number;
  seg_end: number;
  zs_start: number;
  zs_end: number;
  zs_count: number;
}

export interface MoveSettleEvent extends DomainEventBase {
  event_type: "move_settle";
  move_id: number;
  kind: "consolidation" | "trend";
  direction: "up" | "down";
  seg_start: number;
  seg_end: number;
  zs_start: number;
  zs_end: number;
  zs_count: number;
}

export interface MoveInvalidateEvent extends DomainEventBase {
  event_type: "move_invalidate";
  move_id: number;
  kind: "consolidation" | "trend";
  direction: "up" | "down";
  seg_start: number;
  seg_end: number;
}

export type ChanEvent =
  | StrokeCandidateEvent
  | StrokeSettledEvent
  | StrokeExtendedEvent
  | StrokeInvalidatedEvent
  | SegmentBreakPendingEvent
  | SegmentSettleEvent
  | SegmentInvalidateEvent
  | ZhongshuCandidateEvent
  | ZhongshuSettleEvent
  | ZhongshuInvalidateEvent
  | MoveCandidateEvent
  | MoveSettleEvent
  | MoveInvalidateEvent;

// ── WebSocket 消息（服务端 → 客户端）──

export interface WsBarMessage {
  type: "bar";
  idx: number;
  ts: number; // epoch 秒
  o: number;
  h: number;
  l: number;
  c: number;
  v: number | null;
  tf?: string; // 多 TF 标识
  stream_id?: string; // MVP-B0: 流标识
}

export interface WsEventMessage {
  type: "event";
  event_type: string;
  bar_idx: number;
  bar_ts: number;
  seq: number;
  payload: Record<string, unknown>;
  event_id: string;
  schema_version: number;
  tf?: string; // 多 TF 标识
  stream_id?: string; // MVP-B0: 流标识
}

export interface WsSnapshotMessage {
  type: "snapshot";
  bar_idx: number;
  strokes: Array<Record<string, unknown>>;
  event_count: number;
}

export interface WsReplayStatusMessage {
  type: "replay_status";
  mode: "idle" | "playing" | "paused" | "done";
  current_idx: number;
  total_bars: number;
  speed: number;
}

export interface WsErrorMessage {
  type: "error";
  message: string;
  code: string;
}

export type WsServerMessage =
  | WsBarMessage
  | WsEventMessage
  | WsSnapshotMessage
  | WsReplayStatusMessage
  | WsErrorMessage;

// ── WebSocket 命令（客户端 → 服务端）──

export interface WsCommand {
  action:
    | "subscribe"
    | "unsubscribe"
    | "replay_start"
    | "replay_step"
    | "replay_seek"
    | "replay_play"
    | "replay_pause";
  symbol?: string;
  tf?: string;
  step_count?: number;
  seek_idx?: number;
  speed?: number;
}

// ── 回放状态 ──

export interface ReplayStatus {
  mode: "idle" | "playing" | "paused" | "done";
  currentIdx: number;
  totalBars: number;
  speed: number;
}
