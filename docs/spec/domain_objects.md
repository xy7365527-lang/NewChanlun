# 域对象规格 — MVP-B0

## InstrumentId

标的身份值对象。

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `symbol` | str | 非空，全大写 | 品种代码，如 "BZ", "AAPL" |
| `inst_type` | str | FUT / STK / ETF / SPREAD | 标的类型 |
| `exchange` | str | 非空 | 交易所，如 "CME", "NASDAQ" |

- `canonical` 属性：`"{exchange}:{symbol}"`，如 `CME:BZ`
- 元数据来源：`data_databento.SYMBOL_CATALOG`

## ScaleSpec

粒度规格值对象。

| 字段 | 类型 | 默认 | 约束 | 说明 |
|------|------|------|------|------|
| `base_interval` | str | — | 非空 | 数据源采集周期（"1min"） |
| `display_tf` | str | — | 非空 | 显示/重采样周期（"5m"） |
| `level_id` | int | 0 | >= 0 | 递归级别（CLAUDE.md：级别=递归层级） |

- `canonical` 属性：`"{base_interval}@{display_tf}:L{level_id}"`
- level_id=0 为笔级（MVP-B0 唯一使用值），后续线段/中枢递增

## StreamId

流标识值对象。

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `instrument` | InstrumentId | — | 标的身份 |
| `scale` | ScaleSpec | — | 粒度规格 |
| `source` | str | "replay" | 数据来源：replay / live / backtest |

- `value` 属性：`"{instrument.canonical}/{scale.canonical}/{source}"`
  - 示例：`CME:BZ/1min@5m:L0/replay`
- `short_hash` 属性：SHA256(value)[:12]，用于日志标签
- `__hash__`/`__eq__` 基于 `value`，可做 dict key

## BarV1（冻结规格）

标准化 K 线 V1。

| 字段 | 类型 | 默认 | 约束 | 说明 |
|------|------|------|------|------|
| `bar_time` | float | — | > 0 | **bar 开始时间**（epoch 秒） |
| `open` | float | — | — | 开盘价 |
| `high` | float | — | >= low (warn) | 最高价 |
| `low` | float | — | — | 最低价 |
| `close` | float | — | — | 收盘价 |
| `volume` | float | 0.0 | — | 成交量 |
| `is_closed` | bool | True | — | True=已完成，False=实时更新中 |
| `stream_id` | str | "" | — | 所属流 StreamId.value |

### 冻结的不变量

1. **bar_time = start_time**：bar_time 始终为 bar 的开始时间（OHLC 行业标准）
2. **high >= low**：异常数据发出 warning，不 raise（真实市场可能有闪崩）
3. **is_closed 语义**：回放场景始终 True；live 场景 partial bar 为 False
4. **幂等规则**：同一 (stream_id, bar_time) 最终只有一个 is_closed=True 的 bar

## EventEnvelopeV1

事件信封（传输包装层）。

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `schema_version` | int | 1 | 信封 schema 版本 |
| `event_id` | str | "" | 来自 DomainEvent.event_id（透传） |
| `stream_id` | str | "" | 事件所属流 |
| `bar_time` | float | 0.0 | 触发 bar 时间 |
| `seq` | int | 0 | 流内事件序号 |
| `subject_id` | str | "" | 事件主题（"stroke:3", "segment:1"） |
| `parents` | tuple[str, ...] | () | 父事件 event_id 列表 |
| `provenance` | str | "" | 来源描述（"bi_differ:v1"） |
| `event` | Any | None | DomainEvent 引用（不参与序列化） |

### 关键设计决策

- **parents 不参与 event_id 计算**：防止哈希链级联，保护确定性红线
- **DomainEvent 零修改**：stream_id/parents 只在 TaggedEvent 和 EventEnvelopeV1 中
- **envelope_id**（PR-B0.2）：`sha256(event_id + stream_id + sorted(parents))[:16]`
