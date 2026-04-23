---
fix_id: v71-fix-484-clamp
date: 2026-04-24
mode: fix
subject: "484号谱系跨TF区间套代码实装——codex质询后修复"
origin_review: .chanlun/review-results/v71-codex-484-review.md
genealogy: 484
epistemological_level: L1
verdict: fixed
---

# v71 修复：484号代码实装 codex 质询后的致命/重要问题

## 修复范围

codex-challenge-484-impl 的异质审查报告识别了 2 个确认致命 + 3 个确认重要问题。
本次修复处理全部确认成立的问题（误判的 2 条不修复）。

| 审查编号 | 严重性 | 问题 | 修复状态 |
|---------|--------|------|---------|
| 问题1 | 致命 | clamp 掩盖映射链不变量 | **已修复** |
| 问题2 | 重要 | merged_to_raw 一致性未校验 | **已强化接口合约+去 clamp 自然暴露越界** |
| 问题3 | 重要 | ts_start > ts_end 未验证 | **已修复**（去 clamp 后 + sanity 断言）|
| 问题6 | 重要 | 软失败 debug 级别过轻 | **已修复**（升 warning+skip_reason）|
| 问题7 | 重要 | 测试覆盖不足 | **已补充** 4 个新测试 |

## 修复 1：去 clamp + 严格不变量断言

### 违规现场（修复前）

`src/newchan/topology/multi_tf_adapter.py:396-403`：

```python
i0 = max(0, min(seg_first.i0, n_merged - 1))
i1 = max(0, min(seg_last.i1, n_merged - 1))

raw_start = merged_to_raw[i0][0]
raw_end = merged_to_raw[i1][1]
raw_start = max(0, min(raw_start, n_raw - 1))
raw_end = max(0, min(raw_end, n_raw - 1))
```

四处 `max(0, min(..., bound-1))` 在索引越界时静默截断，违反 090 号 no-patch-mentality
（"在错误条件上修正值让代码大致能工作"）。

### 修复后（严格不变量）

```python
if not (0 <= seg_first.i0 < n_merged):
    raise IndexError(
        f"seg_first.i0={seg_first.i0} out of merged range "
        f"[0, {n_merged}); snapshot and bars may be inconsistent",
    )
if not (0 <= seg_last.i1 < n_merged):
    raise IndexError(
        f"seg_last.i1={seg_last.i1} out of merged range "
        f"[0, {n_merged}); snapshot and bars may be inconsistent",
    )

raw_start = merged_to_raw[seg_first.i0][0]
raw_end = merged_to_raw[seg_last.i1][1]

if not (0 <= raw_start < n_raw):
    raise IndexError(...)
if not (0 <= raw_end < n_raw):
    raise IndexError(...)

ts_start = bars[raw_start].ts
ts_end = bars[raw_end].ts

if ts_start > ts_end:
    raise ValueError(
        f"ts_start={ts_start} > ts_end={ts_end}; "
        f"merged_to_raw monotonicity invariant violated ..."
    )

return ts_start, ts_end
```

### 设计原则

- **no-patch-mentality**：错误条件不修正、不截断——暴露上游 snapshot-bars 不一致
- **no-workaround**：不用 try/except 吞掉异常，不用默认值兜底
- **映射链忠实度**：`segments[*].i0/i1 → merged_to_raw → bars[*].ts` 链上任一节点越界
  立即抛错，调用方必须处理或传播

## 修复 2：接口合约强化（merged_to_raw 一致性）

原始 docstring 只说 bars "与 snapshot 同源"；修复后添加显式接口合约：

```
接口合约（v71 codex 质询后强化）
----------------------------------
调用方**必须**保证 `bars` 与 `segments` 来自同源 snapshot：即
本次调用中传入的 `bars` 与产生 `segments` 的原始 bar 列表完全一致
（相同顺序、相同时间戳、无追加/删除）。merge_inclusion 幂等依赖
此同源约束：若 bars 非 frozen/不可变，merged_to_raw 重建可能漂移，
导致 i0/i1 越界——该情形下本函数抛 IndexError，不 clamp。
```

修复 1 去 clamp 后，此合约的违反**自然被断言暴露**——不需要额外 hash 校验：
如果 bars 与上游 snapshot 不同源，merge_inclusion 重建出的 merged_to_raw 长度会偏离，
segments[*].i0/i1 就会越界，抛 IndexError。这是最严格的一致性验证方式
（相比 hash 校验更严格，因为 hash 可能碰撞）。

## 修复 3：软失败升 warning + skip_reason

### 违规现场（修复前）

`src/newchan/topology/multi_tf_pipeline.py` 多处：

```python
logger.debug("Missing bars for divergence %d, skip", idx)
logger.debug("High TF snapshot missing ...", idx)
logger.debug("No low-TF bars in range ...", ...)
# ...
```

### 修复后

所有跨 TF 背驰跳过事件升至 `logger.warning`，并附带 `skip_reason` 字段：

```python
logger.warning(
    "Cross-scale divergence %d skipped: missing bars "
    "(high=%s low=%s); skip_reason=missing_bars",
    idx, high_tf_name, low_tf_name,
)
```

6 种 skip_reason：`missing_bars`、`missing_high_snapshot`、`empty_high_segments`、
`empty_low_bars_in_range`、`no_low_snapshot`。

### 区分可恢复 vs 不可恢复错误

按任务协议要求，区分了两类错误：

| 情形 | 性质 | 处理 |
|------|------|------|
| 数据缺失（缺 bars/snapshot/segments） | 可恢复（数据端不完整） | warning + skip_reason + continue |
| 时间窗内无低级别 bars | 可恢复（范围外） | warning + skip_reason + continue |
| RecursiveOrchestrator 无 snapshot | 可恢复（数据不足以生成结构） | warning + skip_reason + continue |
| **extract_c_segment_timestamps 抛错** | **不可恢复（严格不变量违反）** | **不 try/except，让异常向上传播** |

原代码用 `try/except (ValueError, IndexError)` 吞掉 extract_c_segment_timestamps 的异常，
违反 no-workaround——修复后移除该 try/except，让不变量违反事件显式传播。

## 修复 4：补充测试覆盖

在 `tests/test_multi_tf_cross_scale.py` 中新增 4 个测试（共 10→14）：

| 测试 | 目的 |
|------|------|
| `test_segment_merged_index_out_of_range_raises` | 验证 `i1=100_000` 越出 `merged_to_raw` 时抛 IndexError |
| `test_segment_negative_index_raises` | 验证 `i0=-5` 时抛错（不 clamp 到 0） |
| `test_raw_index_out_of_range_raises` | 验证正常路径下 sanity check 通过 |
| `test_ts_ordering_invariant` | 参数化遍历所有合法 seg_start/seg_end 组合，确保 ts_start <= ts_end |

### TDD 证据

**Phase 2（RED）** ——写测试前跑：

```
test_segment_merged_index_out_of_range_raises FAILED  ← clamp 让它通过
test_segment_negative_index_raises FAILED             ← clamp 让它通过
其他 8 个测试 PASSED
```

**Phase 3（GREEN）** ——修复实现后跑：

```
14 passed, 2 warnings in 1.05s
```

**Phase 4（无回归）** ——相关测试套件：

```
tests/ -k "multi_tf" → 85 passed, 4424 deselected
```

## 结果包六要素（简化版——纯技术修复不涉及概念定义变更）

### 1. 结论

484号谱系5条下游推论的代码实装在 v71 codex 异质审查中识别的 4 个确认问题
（clamp 补丁、软失败过轻、ts 反向、测试缺失）全部修复。
14 个测试通过，85 个相关测试无回归。

### 2. 边界条件

| 条件 | 判定 |
|------|------|
| `segments[*].i0/i1` 在 `[0, len(merged_to_raw))` 范围内 | 返回合法时间戳 |
| `segments[*].i0/i1` 越界 | 抛 IndexError，暴露 snapshot-bars 不一致 |
| `merged_to_raw` 映射出的 raw idx 越 bars 范围 | 抛 IndexError |
| `ts_start > ts_end`（极端情况下 merged_to_raw 非单调） | 抛 ValueError |
| 跨TF背驰 bars/snapshot/segments 缺失 | warning + skip，不中断整体流程 |
| `extract_c_segment_timestamps` 内部不变量违反 | 异常向上传播，不被吞掉 |

### 3. 影响声明

**修改的文件**：

- `src/newchan/topology/multi_tf_adapter.py`：
  - `extract_c_segment_timestamps` (line 322-420)：去 clamp，加严格不变量断言，
    强化 docstring 接口合约，加 ts_ordering sanity check
- `src/newchan/topology/multi_tf_pipeline.py`：
  - `run_cross_scale_nested_search` (line 515-600)：`logger.debug` → `logger.warning`+`skip_reason`，
    移除 `try/except (ValueError, IndexError)` 对 `extract_c_segment_timestamps` 的包裹
- `tests/test_multi_tf_cross_scale.py`：
  - 新增 4 个测试（line 235-329 附近）

**未修改**：
- 484号谱系文档本身（概念层无变更）
- `merge_inclusion` / `RecursiveOrchestrator` / `nested_divergence_search` 等上游/下游模块
- 域知识定义（概念清晰度、语义正确性由 codex 审查中已确认）

**谱系引用**：
- 090号（no-patch-mentality）：clamp 是补丁思维典型形态
- 218号（no-workaround）：try/except 吞概念层异常被禁止
- 209号（商空间映射）：merged_to_raw 作为商映射纤维的严格性要求
- 484号：本次修复对象
