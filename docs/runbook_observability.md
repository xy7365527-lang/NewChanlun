# NewChanlun 可观测性运维手册

## 1. 概述

`newchan.obs` 子包提供两个核心模块：

- **`logger.py`** — 结构化日志，输出 JSON 格式，便于 `jq` 过滤和日志聚合平台消费。
- **`metrics.py`** — 引擎运行时指标，轻量级内存计数器，支持按需快照导出。

设计目标：在不引入外部依赖的前提下，为差分快照引擎（`bi_engine` / `bi_differ`）和回放系统（`replay`）提供开箱即用的诊断能力。

---

## 2. 结构化日志

### 2.1 StructuredLogger 使用方式

```python
from newchan.obs.logger import StructuredLogger

log = StructuredLogger("bi_engine")
log.info("stroke_confirmed", bar_idx=1024, tf="1m", event_count=3)
log.warning("gap_detected", bar_idx=1025, tf="1m", gap_size=2)
```

每条日志输出为单行 JSON，可直接写入文件或 stdout。

### 2.2 日志字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `ts` | str | ISO-8601 时间戳（UTC） |
| `level` | str | 日志级别：DEBUG / INFO / WARNING / ERROR |
| `logger` | str | 日志器名称，标识来源模块 |
| `msg` | str | 事件消息 |
| `bar_idx` | int | 当前 bar 索引，定位时间位置 |
| `tf` | str | 时间框架标识（如 `"1m"`、`"5m"`） |
| `event_count` | int | 本次处理产生的事件数量 |

所有字段均为可选 kwargs，按需传入。

### 2.3 日志过滤和查看

```bash
# 查看所有 WARNING 及以上
cat engine.log | jq 'select(.level == "WARNING" or .level == "ERROR")'

# 按 bar_idx 范围过滤
cat engine.log | jq 'select(.bar_idx >= 1000 and .bar_idx <= 1100)'

# 按模块过滤
cat engine.log | jq 'select(.logger == "bi_differ")'
```

---

## 3. 引擎指标

### 3.1 EngineMetrics 字段说明

| 字段 | 类型 | 说明 |
|------|------|------|
| `events_total` | int | 引擎累计产生的事件总数 |
| `violations_total` | int | 不变量违规累计次数 |
| `bars_processed` | int | 已处理的 bar 总数 |
| `last_bar_ts` | str | 最后处理的 bar 时间戳 |
| `last_process_duration_ms` | float | 最近一次 `process_bar` 耗时（毫秒） |

### 3.2 获取指标快照

```python
from newchan.obs.metrics import EngineMetrics

metrics = EngineMetrics()
# 引擎处理循环中自动更新 ...
snapshot = metrics.snapshot()  # 返回 dict，可序列化为 JSON
```

快照通过 `/api/metrics` 端点或 WebSocket `metrics_request` 消息获取。

---

## 4. 排障指南

### 4.1 事件丢失排查

1. 检查 `events_total` 是否持续递增；若停滞，确认 `process_bar` 是否被调用。
2. 对比 `bars_processed` 与数据源 bar 数量，确认无跳过。
3. 开启 DEBUG 日志，检查 `bi_differ` 是否因 diff 为空而未产出事件。
4. 确认 WebSocket 客户端已订阅正确的 `tf` 频道。

### 4.2 不变量违规排查

当出现 `InvariantViolation` 事件时，关注三个关键字段：

- **`code`** — 违规类型编码（如 `STROKE_OVERLAP`、`INCLUSION_BREAK`）
- **`reason`** — 人类可读的违规描述
- **`snapshot_hash`** — 违规时刻引擎状态的哈希，用于复现

排查步骤：

1. 根据 `code` 在 `ab_bridge_newchan.py` 的 `NEWCHAN_ASSERT` 中定位断言。
2. 使用 `snapshot_hash` 通过回放系统 seek 到违规 bar，逐步调试。
3. 检查 `violations_total` 增长趋势，判断是偶发还是系统性问题。

### 4.3 多 TF 串扰排查

当多个时间框架同时运行时，事件可能被错误路由。

1. 检查每条日志的 `tf` 字段，确认事件归属正确。
2. 确认 `EventBus` 实例按 `tf` 隔离，不同 TF 不共享同一 bus。
3. 过滤日志：`jq 'select(.tf == "1m")'`，验证单一 TF 的事件序列完整性。

### 4.4 回放不确定性排查

回放系统要求严格确定性（相同输入 → 相同输出）。

1. 运行 `test_replay_determinism.py`，确认基线通过。
2. 对比两次回放的事件序列（按 `bar_idx` 排序后 diff）。
3. 检查 `engine.reset()` 是否完整清除状态——特别注意缓存和全局变量。
4. 确认数据源在 seek 后返回的 bar 数据完全一致（浮点精度）。

---

## 5. 未来扩展方向

### 5.1 Prometheus Exporter

将 `EngineMetrics` 暴露为 Prometheus 格式端点（`/metrics`），支持 `Counter`、`Gauge`、`Histogram` 类型映射。计划使用 `prometheus_client` 库，零侵入集成。

### 5.2 Grafana Dashboard

预置 Dashboard JSON，包含以下面板：

- 事件产出速率（events/sec by tf）
- 违规次数时间线
- 单 bar 处理耗时分布（P50 / P95 / P99）
- 回放 seek 延迟

### 5.3 AlertManager 集成

基于 Prometheus 告警规则，覆盖以下场景：

- `violations_total` 在 5 分钟内增长超过阈值
- `last_process_duration_ms` P99 超过 SLA
- `bars_processed` 长时间未更新（引擎停滞）
