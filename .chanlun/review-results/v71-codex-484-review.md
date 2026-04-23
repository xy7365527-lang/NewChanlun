---
review_id: v71-codex-484-review
date: 2026-04-24
mode: review
subject: "484号谱系跨TF区间套链路代码实装（extract_c_segment_timestamps + run_cross_scale_nested_search）"
verdict: partial_fail
codex_verdict: fail
agent_verdict: 2个问题确认致命，2个误判，3个问题确认重要
genealogy: 484
epistemological_level: L1
---

# v71 Codex 异质审查：484号谱系代码实装

## 完整交互记录

### Prompt（上下文）

参见 `/tmp/codex-484-review-ctx.md`（含 5 条推论、实装代码、缠论定义、审查要求）。

### Codex Response（原文）

```
结论：**未通过（fail）**。核心链路已接上，但存在会导致错误结果或直接崩溃的代码层问题。

### 1) clamp 掩盖上游不一致，破坏定义忠实度
位置：multi_tf_adapter.py: extract_c_segment_timestamps()（i0/i1/raw_start/raw_end clamp）
严重性：**致命**

### 2) 本地重建 merged_to_raw 缺少一致性校验
位置：multi_tf_adapter.py: extract_c_segment_timestamps()（merge_inclusion 调用段）
严重性：**重要**

### 3) 可能返回反向时间窗（ts_start > ts_end）未处理
位置：multi_tf_adapter.py: extract_c_segment_timestamps()（返回前）
严重性：**重要**

### 4) run_cross_scale_nested_search 在空过滤结果时会用未定义变量 low_snap
位置：multi_tf_pipeline.py: run_cross_scale_nested_search()
严重性：**致命**

### 5) max_levels 参数疑似未生效，接口与行为不一致
位置：multi_tf_pipeline.py: run_cross_scale_nested_search()
严重性：**建议**

### 6) 软失败策略（continue + debug）过轻，影响可观测性
位置：multi_tf_pipeline.py: run_cross_scale_nested_search()
严重性：**重要**

### 7) 测试覆盖对关键边界不足
位置：tests/test_multi_tf_cross_scale.py
严重性：**重要**
```

---

## Agent 质询判定

### 问题1：clamp 掩盖不变量（致命）→ **确认成立**

`i0/i1/raw_start/raw_end` 的四处 `max/min` clamp 在索引越界时不报错而截断，会给出错误时间窗。
这是 no-patch-mentality 的直接违反：在错误条件上"修正"值等同于补丁思维。

**修复要求**：去掉 clamp，改为严格不变量断言。

### 问题2：merged_to_raw 重建一致性（重要）→ **确认成立（部分）**

每次 `merge_inclusion(df_raw)` 是幂等重建，但没有显式验证与上游 snapshot 生成时使用的合并结果一致。
如果 bars 有时区差异、排序变化等微小偏差，映射会漂移。

**边界条件**：若调用方保证 bars 是同一不可变列表（frozen），风险较低；但接口声明没有此约束。

### 问题3：ts_start > ts_end 未验证（重要）→ **确认成立**

clamp 可能使 `raw_start > raw_end`（当段索引映射异常时），返回倒置时间窗后，
`align_bars_by_timestamp` 会返回空列表，但错误来源被静默掩盖。

### 问题4：low_snap 未定义（致命）→ **误判**

实际代码（multi_tf_pipeline.py:575-583）：
```python
low_snap = None
for bar in filtered_low_bars:
    low_snap = low_orch.process_bar(bar)
if low_snap is None:
    logger.debug(...)
    continue
```

`low_snap = None` 初始化存在，`if low_snap is None: continue` 保护存在。
Codex 依据精简版 prompt，误读了流程。**此条否定不成立**。

### 问题5：max_levels 未传入（建议）→ **误判**

实际代码（multi_tf_pipeline.py:570-573）：
```python
low_orch = RecursiveOrchestrator(
    stream_id=f"cross_scale_{low_tf_name}_div{idx}",
    max_levels=max_levels,
    stroke_mode=stroke_mode,
)
```
max_levels 已传入。**此条否定不成立**。

### 问题6：软失败可观测性（重要）→ **确认成立（上下文相关）**

前置条件缺失（缺 bars、缺 snapshot、缺 segments）全部是 `logger.debug + continue`。
对于跨TF搜索失败这一非平凡事件，debug 级别过轻，会导致生产环境下跳过背驰被静默忽略。

**修复建议**：升至 `logger.warning`，加 skip_reason 字段。

### 问题7：测试覆盖边界缺失（重要）→ **确认成立（部分）**

确认缺失的高风险测试：
- `seg i0/i1` 越界时应抛 `IndexError/ValueError`（而非 clamp 后给错误结果）
- `ts_start > ts_end` 返回应抛 `ValueError`（clamp 修复后的正确行为验证）
- merged_to_raw 不一致时（如排序异常 bars）应报错

测试中已有：`test_seg_index_out_of_range_raises`——但这测的是 `seg_end >= len(segments)`，
不测 `i0/i1/raw_start/raw_end` 的 clamp 路径（那段逻辑测试缺失）。

---

## 结果包六要素

### 1. 结论

484号谱系5条推论的代码实装**链路已接通**，但存在 2 个确认重要问题（clamp 补丁、ts 反向）和 2 个确认成立的观测性问题（merged_to_raw 一致性、软失败）。Codex 的 2 个"致命"判定中，1 个确认成立（clamp），1 个是误判（low_snap 已有保护）。5 条推论的功能覆盖完整，无膨胀，无欠缺。

**最终判定：代码层存在问题，需修复后方可视为 L1 验证通过。**

### 2. 定义依据

- 484号谱系推论1-2 声明的映射链：Move.seg_start/seg_end → segments[*].i0/i1 → merged_to_raw → bars[*].ts
  代码忠实实现了该链路，但 clamp 破坏了映射链的严格性。
- 484号谱系推论3-5 声明的三步链路（过滤→重构→搜索）：代码完整实现。
- no-patch-mentality：clamp 是典型补丁思维（在错误条件上修正值，让代码"大致能工作"）。

### 3. 边界条件

| 条件 | 当前判定 | 翻转阈值 |
|------|---------|---------|
| clamp 问题严重性 | 致命 | 若上游保证 i0/i1 始终在 merged 范围内（如 segment 构造合约），则 clamp 变为防御性编程 |
| merged_to_raw 一致性 | 重要 | 若接口合约明确 bars 参数不可变且唯一来源，则风险降为低 |
| ts 反向 | 重要 | 去掉 clamp 后若断言保护正确，该问题消失 |
| 软失败可观测性 | 重要 | 若调用方有独立监控，debug 级别可接受 |

### 4. 下游推论

- 若 clamp 被修复为严格断言，则测试中的 `test_seg_index_out_of_range_raises` 需同步验证 clamp 路径不再存在
- merged_to_raw 一致性问题若要彻底修复，需要从 RecursiveOrchestratorSnapshot 暴露 merged_to_raw（接口改动）或增加同源校验
- 软失败升级为 warning 后，监控需要关注 warning 级别日志

### 5. 谱系引用

- **484号**：本次审查对象——multi_tf架构审计+跨TF区间套缺口
- **090号**：no-patch-mentality——clamp 是补丁思维的典型形态
- **209号**：商空间映射（merged_to_raw 的来源谱系）

### 6. 影响声明

- **不修改任何代码**（本工位职责是审查，不是修复）
- `multi_tf_adapter.py: extract_c_segment_timestamps()` 存在 clamp 补丁问题（L1 层代码缺陷）
- `multi_tf_pipeline.py: run_cross_scale_nested_search()` 的 low_snap/max_levels 路径经验证正确，软失败日志级别偏低
- `tests/test_multi_tf_cross_scale.py` 对 clamp 越界、ts 反向场景缺乏测试
- 修复工位需处理：clamp→断言、ts 反向校验、warning 升级、测试补充

---

## Codex 误判记录

| Codex 问题 | 判定 | 原因 |
|-----------|------|------|
| low_snap 未定义（致命） | 误判 | 代码有 `low_snap = None` + `if low_snap is None: continue` 保护，Codex 看精简版 prompt 遗漏 |
| max_levels 未传入（建议） | 误判 | 代码第572行有 `max_levels=max_levels` 传入，Codex 精简版 prompt 遗漏 |

误判原因：context 文件的精简代码片段未包含这两处实现细节，Codex 依据不完整上下文做出否定。
这是 agent 层构建 context 时的精度不足，不是 Codex 的系统性偏差。
