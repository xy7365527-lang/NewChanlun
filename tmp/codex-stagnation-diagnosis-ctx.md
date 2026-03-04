# Codex 代码层诊断：从形式化到可执行代码的转化阻塞

## 任务

诊断：系统有 14 个已结算缠论定义、完整的五层递归引擎、买卖点模块——但这些东西加在一起能做什么？用户拿到这个系统能干什么？什么阻碍了从形式化到可执行代码的转化？

## 代码结构概览

### 已实现的核心算法层（A 系统）

```
src/newchan/
├── bi_engine.py              # 笔引擎（BiEngine）
├── core/recursion/
│   ├── segment_engine.py     # 线段引擎
│   ├── zhongshu_engine.py    # 中枢引擎
│   ├── move_engine.py        # 走势引擎
│   ├── buysellpoint_engine.py # 买卖点引擎（BuySellPointEngine）
│   ├── recursive_level_engine.py # 递归级别引擎
│   └── recursive_stack.py    # 递归栈
├── orchestrator/
│   └── recursive.py          # RecursiveOrchestrator（五层+递归栈完整链）
├── a_buysellpoint_v1.py      # 买卖点识别（type1/type2/type3）
├── a_nested_divergence.py    # 嵌套背驰搜索
├── nested_pipeline.py        # run_nested_search() 便利包装
└── a_level_fsm_adapter.py    # L* 选择适配器
```

### 用户接口层（B 系统）

```
src/newchan/
├── server.py                 # HTTP API 服务（bottle）
│   └── api_newchan_overlay() # GET /api/newchan_overlay → 调用 build_overlay_newchan()
├── gateway.py                # FastAPI + WebSocket 回放系统
│   └── ReplaySession         # 逐 bar 回放
├── ab_bridge_newchan.py      # A→B 桥接：build_overlay_newchan()
├── b_chart.py                # TradingView 风格前端 HTML
└── cli.py                    # CLI 入口（fetch/plot/chart/synthetic/fetch-db）
```

## 关键代码路径

### 路径 1：图表 overlay（server.py → ab_bridge_newchan.py）

```python
# server.py:408
def api_newchan_overlay():
    result = build_overlay_newchan(resampled, ...)
    return _json_resp(result)

# ab_bridge_newchan.py:139
def build_overlay_newchan(df_raw, ...) -> dict:
    # 使用 a_recursive_engine.build_recursive_levels()（批量静态版本）
    # 不使用 RecursiveOrchestrator（事件驱动版本）
    df_merged, merged_to_raw, fractals, strokes, segments, rec_levels = _run_a_pipeline(...)
    centers, trends = _resolve_level1(rec_levels, segments, center_sustain_m)
    df_macd = compute_macd(...)
    lstar_obj = select_lstar_newchan(level_views, last_price)

    return {
        "schema_version": "newchan_overlay_v2",
        "lstar": ...,
        "strokes": ...,
        "segments": ...,
        "centers": ...,
        "trends": ...,
        "levels": ...,
        "macd": ...,
        # ← 没有 "bsp" 字段
        # ← 没有 "nested_divergences" 字段
    }
```

### 路径 2：WebSocket 回放（gateway.py）

```python
# gateway.py:250
engine = BiEngine(stroke_mode=req.stroke_mode, min_strict_sep=req.min_strict_sep)
session = ReplaySession(session_id=session_id, bars=bars, engine=engine)
# ← 只用 BiEngine，不用 RecursiveOrchestrator
# ← 回放只产生笔事件，不产生线段/中枢/走势/买卖点事件

# replay.py:step()
snap = self.engine.process_bar(self.bars[self.current_idx])
# snap 是 BiEngineSnapshot，只含 strokes + stroke events
```

### 路径 3：RecursiveOrchestrator（已实现但未接入接口）

```python
# orchestrator/recursive.py
class RecursiveOrchestrator:
    def process_bar(self, bar: Bar) -> RecursiveOrchestratorSnapshot:
        bi_snap = self._bi_engine.process_bar(bar)
        seg_snap = self._seg_engine.process_snapshot(bi_snap)
        zs_snap = self._zs_engine.process_segment_snapshot(seg_snap)
        move_snap = self._move_engine.process_zhongshu_snapshot(zs_snap, ...)
        bsp_snap = self._bsp_engine.process_snapshots(move_snap, zs_snap, seg_snap)
        recursive_snaps = self._recursive_stack.process_level1_move_snapshot(move_snap)
        snap.lstar = select_lstar_from_recursive_snapshot(snap, bar.close)
        return snap
# ← 完整链，包含 BSP + L* + 递归层
# ← 但 gateway.py 和 ab_bridge_newchan.py 都没有使用它
```

### 路径 4：嵌套背驰（已实现但未接入接口）

```python
# nested_pipeline.py
def run_nested_search(bars, ...) -> tuple[list[NestedDivergence], RecursiveOrchestratorSnapshot | None]:
    orch = RecursiveOrchestrator(...)
    for bar in bars:
        snap = orch.process_bar(bar)
    return nested_divergence_search(snap, ...)
# ← 完整实现
# ← 但 server.py 没有任何端点调用它
# ← build_overlay_newchan() 也没有调用它
```

## 诊断问题

请从代码层诊断以下三个具体问题：

### 问题 1：BSP 断链

`BuySellPointEngine` 已实现并接入 `RecursiveOrchestrator`，但 `build_overlay_newchan()` 使用的是 `a_recursive_engine.build_recursive_levels()`（批量静态版本），不是 `RecursiveOrchestrator`。

- overlay 输出 JSON 没有 `bsp` 字段
- 前端图表无法显示买卖点
- 用户看不到 type1/type2/type3 买卖点

**问题**：将 BSP 接入 overlay 输出，最小改动路径是什么？是否有代码层风险？

### 问题 2：回放引擎降级

`gateway.py` 的 WebSocket 回放系统使用 `BiEngine`（只有笔），而不是 `RecursiveOrchestrator`（完整五层+递归）。

- 回放只产生 `stroke_candidate`/`stroke_settled` 事件
- 不产生 `segment_*`/`zhongshu_*`/`move_*`/`bsp_*` 事件
- `ReplaySession` 绑定 `BiEngine`，接口不支持 `RecursiveOrchestrator`

**问题**：将回放引擎升级为 `RecursiveOrchestrator`，需要改动哪些文件？`ReplaySession` 的接口需要如何调整？

### 问题 3：嵌套背驰孤岛

`run_nested_search()` 完整实现了区间套跨级别背驰搜索，但：
- `server.py` 没有任何端点暴露它
- `build_overlay_newchan()` 没有调用它
- 用户无法通过任何接口获取嵌套背驰结果

**问题**：将嵌套背驰结果加入 overlay 输出，最小改动路径是什么？性能影响如何（每次 HTTP 请求都跑一遍 RecursiveOrchestrator）？

## 评估标准

对每个问题，请给出：
1. 代码层问题是否真实存在（确认或否定）
2. 最小改动路径（涉及哪些文件、哪些函数）
3. 代码层风险（接口兼容性、性能、正确性）
4. 优先级建议（哪个先做）

## 附：已有测试覆盖

- `tests/test_bsp_*.py` — BSP 单元测试（type1/type2/type3/overlap/invariants）
- `tests/test_real_data_e2e.py` — TFOrchestrator 真实数据 E2E
- `tests/test_real_data_recursive_e2e.py` — RecursiveOrchestrator 真实数据 E2E
- `tests/test_nested_pipeline.py` — 嵌套背驰管线测试
- `tests/test_ab_bridge_overlay.py` — overlay 桥接测试
