# Codex diagnose — 2026-02-26 02:47:51 UTC

## 元数据

- **mode**: diagnose
- **subject**: 从形式化到可执行代码的转化阻塞：BSP断链、回放引擎降级、嵌套背驰孤岛
- **model**: gpt-5.2-codex
- **timestamp**: 2026-02-26 02:47:51 UTC
- **context-file**: tmp/codex-stagnation-diagnosis-ctx.md

## Prompt

## 诊断目标

从形式化到可执行代码的转化阻塞：BSP断链、回放引擎降级、嵌套背驰孤岛

## 上下文

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


## 诊断要求

请执行严格根因诊断：
1. 失败现象：观测到什么？
2. 直接原因：哪行代码导致了失败？
3. 根本原因：为什么会写出那行代码？是设计错误还是实现疏忽？
4. 分类判定：实现错误 or 定义冲突？
5. 修复方案（仅限实现错误）：精确的修复步骤

如果是定义冲突，输出：
- **冲突方**：A 说什么 vs B 说什么
- **不可弥合理由**：为什么不能同时满足两边


## Response

以下为严格代码层诊断报告。每个问题均按 **失败现象 → 直接原因 → 根本原因 → 分类判定 → 修复方案/风险** 的链式结构输出；仅在实现错误场景给出修复方案。

---

# 问题 1：BSP 断链（overlay 无买卖点）

## 1) 失败现象  
- `build_overlay_newchan()` 输出 JSON **没有 `bsp` 字段**。  
- 前端图表无法显示 type1/type2/type3 买卖点。  
- 现象已在 `ab_bridge_newchan.py` 的返回 dict 代码层明确存在。

## 2) 直接原因  
`ab_bridge_newchan.py:build_overlay_newchan()` 使用的是 **静态批量管线** `_run_a_pipeline()`（`a_recursive_engine.build_recursive_levels()`），**没有调用 `BuySellPointEngine`**，并且返回字典中**缺少 `bsp` 字段**。  
> 直接可定位代码：  
> - `build_overlay_newchan()` 返回 dict 中未写入 `bsp`  
> - 未使用 `RecursiveOrchestrator`（该类才产出 BSP）

## 3) 根本原因  
B 系统 overlay 的接口是 **在 BSP 引擎完善之前完成** 的，未随 A 系统新增能力同步更新（设计滞后/实现疏漏）。  

## 4) 分类判定  
**实现错误**（不改变定义即可修复）。

## 5) 最小修复路径  
**最小改动**要求保留现有 overlay 结构，仅增加 BSP 输出。可行路径如下：

### 方案 A（最小入侵，双管线并存）
- 在 `ab_bridge_newchan.py:build_overlay_newchan()` 中：
  1. 保留 `_run_a_pipeline()` 生成 strokes/segments/centers。
  2. 额外实例化 `RecursiveOrchestrator`，对同一 `df_raw` 批量 `process_bar`，拿最终 `snap.bsp_*`。
  3. 将 BSP 结果映射到 overlay 结构，添加 `bsp` 字段。

**涉及文件**：  
- `ab_bridge_newchan.py`（新增 orchestrator 调用 & JSON 字段）

**风险**：  
- **性能风险**：同一请求执行两套管线（静态+递归），耗时翻倍。  
- **一致性风险**：BSP 的时间索引来源于递归引擎，需与静态 `merged_to_raw` 对齐，否则坐标错位。  
- **Schema 风险**：新增字段可能破坏前端/测试期望（需同步更新前端与 `tests/test_ab_bridge_overlay.py`）。

### 方案 B（替换为递归引擎）
- 在 `build_overlay_newchan()` 中用 `RecursiveOrchestrator` 取代 `_run_a_pipeline()`，用 `snap` 统一输出 strokes/segments/centers/bsp。

**风险**更高：输出结构与当前 overlay 格式可能不兼容，影响更大。

## 6) 优先级  
**高优先级**（用户可见功能缺失，且 A 系统能力已存在）。

---

# 问题 2：回放引擎降级（Replay 只出笔，不出线段/中枢/买卖点）

## 1) 失败现象  
WebSocket 回放系统只产生 `stroke_candidate` / `stroke_settled` 事件；  
没有 `segment_* / zhongshu_* / move_* / bsp_*` 事件。  

## 2) 直接原因  
`gateway.py` 的 `ReplaySession` **只接受 `BiEngine`**：  
```python
engine = BiEngine(...)
session = ReplaySession(..., engine=engine)
```
`ReplaySession.step()` 仅处理 `BiEngineSnapshot`。  

## 3) 根本原因  
回放系统是在笔级引擎阶段搭建，未抽象出可替换的引擎接口，导致 A 系统新增的递归链路无法接入（架构接口未演进）。

## 4) 分类判定  
**实现错误**（接口可扩展，无需改定义）。

## 5) 最小修复路径  
### 改动点  
1. **gateway.py**  
   - 将 `engine = BiEngine(...)` 替换为 `RecursiveOrchestrator(...)`（或增加参数选择）。  
2. **ReplaySession**（位于 `gateway.py` 或其依赖文件）  
   - `step()` 接受 `RecursiveOrchestratorSnapshot`，并序列化其 `segment/zhongshu/move/bsp` 事件。  
   - 如果不破坏旧行为，可将 `ReplaySession` 抽象为 `EngineProtocol` + 多态事件输出。  

### 风险  
- **接口兼容性风险**：前端/客户端当前只消费 stroke 事件，新事件需协议更新。  
- **性能风险**：递归引擎比笔引擎重，实时推送可能导致吞吐下降。  
- **正确性风险**：递归引擎要求更严格的 bar 序列一致性，否则事件顺序可能与旧前端假设不一致。

## 6) 优先级  
**中高优先级**（回放是主要交互入口，但升级会引入协议升级成本）。

---

# 问题 3：嵌套背驰孤岛（nested divergence 无接口）

## 1) 失败现象  
`run_nested_search()` 已实现，但：  
- `server.py` 无端点调用  
- `build_overlay_newchan()` 无输出字段  
- 用户无法通过任何 API 获取嵌套背驰结果  

## 2) 直接原因  
嵌套背驰管线 `nested_pipeline.py` **未被任何 API 或 overlay 调用**。  

## 3) 根本原因  
该模块作为算法验证或离线管线存在，未纳入 B 系统输出接口（未打通 A→B）。

## 4) 分类判定  
**实现错误**（接口缺失可补）。

## 5) 最小修复路径  
### 方案 A（overlay 增强）
- 在 `build_overlay_newchan()` 中增加调用 `run_nested_search()`，并在返回 dict 中加入 `nested_divergences` 字段。

**涉及文件**：  
- `ab_bridge_newchan.py`

**风险**：  
- **性能风险**：`run_nested_search()` 内部会创建 `RecursiveOrchestrator` 并全量扫 bars，若 overlay 也执行静态管线，则再次重复计算。  
- **资源风险**：每次 HTTP 请求全量回放可能超时，尤其在大数据窗口下。

### 方案 B（新增独立 API）
- 在 `server.py` 新增 `/api/nested_divergence` 端点调用 `run_nested_search()`。  
- 避免 overlay 每次请求都执行重计算。

## 6) 优先级  
**中优先级**（功能缺失但非主交易可视链路，且性能代价高）。

---

# 总结优先级建议

1. **BSP 断链（高）**：直接影响买卖点核心功能展示，且最小改动可实现。  
2. **回放引擎降级（中高）**：修复后可形成完整事件链，但需协议升级。  
3. **嵌套背驰孤岛（中）**：功能价值高但计算代价大，适合作为独立 API 暴露。

---

# 结论分类表

| 问题 | 是否真实存在 | 分类 | 根因类型 |
|---|---|---|---|
| BSP 断链 | 确认存在 | 实现错误 | B 系统未同步 A 系统能力 |
| 回放引擎降级 | 确认存在 | 实现错误 | Replay 接口未抽象升级 |
| 嵌套背驰孤岛 | 确认存在 | 实现错误 | 功能未暴露接口 |

如需，我可以进一步提供每个修复点的具体代码修改 diff 级建议。
