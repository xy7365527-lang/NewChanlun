# Codex diagnose — 2026-04-05 18:37:24 UTC

## 元数据

- **mode**: diagnose
- **subject**: 走势引擎为何将连续递升中枢产出为独立盘整而非趋势
- **model**: gpt-5.3-codex
- **timestamp**: 2026-04-05 18:37:24 UTC
- **context-file**: /tmp/codex-diagnose-ctx.md

## Prompt

## 诊断目标

走势引擎为何将连续递升中枢产出为独立盘整而非趋势

## 上下文

# Diagnose: 走势引擎为何不将连续同向中枢合并为趋势

## 现象描述

金油比日线数据（5282根K线）跑完管线，产出 6 个走势，全部是 `consolidation`（盘整）。
前 4 个中枢依次递升：
- zs[0]: dd=8.25, gg=8.77
- zs[1]: dd=12.55, gg=15.79
- zs[2]: dd=18.02, gg=27.28
- zs[3]: dd=40.93, gg=46.97

这4个中枢满足趋势条件（c2.dd > c1.gg，严格递升不重叠），但管线把每个中枢都封装成独立的盘整走势，没有合并为趋势。

## 核心代码：`src/newchan/a_move_v1.py`（完整）

```python
"""A 系统 — 走势类型实例 v1

从已闭合中枢列表构造走势类型实例（Move）。

核心规则（冻结 v1 spec）：
- 盘整 = 包含恰好 1 个走势中枢
- 趋势 = 包含 2+ 个依次同向走势中枢
  - 上涨：C2.DD > C1.GG（后枢波动下界 严格高于 前枢波动上界，中心定理二）
  - 下跌：C2.GG < C1.DD（后枢波动上界 严格低于 前枢波动下界，中心定理二）
- 区间重叠的相邻中枢 → 截断为不同 move
"""

from __future__ import annotations

from dataclasses import dataclass, replace
from typing import Literal

from newchan.a_zhongshu_v1 import Zhongshu


@dataclass(frozen=True, slots=True)
class Move:
    kind: Literal["consolidation", "trend"]
    direction: Literal["up", "down"]
    seg_start: int
    seg_end: int
    zs_start: int
    zs_end: int
    zs_count: int
    settled: bool
    high: float = 0.0
    low: float = 0.0
    first_seg_s0: int = 0
    last_seg_s1: int = 0


def _is_ascending(c1: Zhongshu, c2: Zhongshu) -> bool:
    """后枢 DD 严格高于 前枢 GG → 上涨延续（中心定理二）。"""
    return c2.dd > c1.gg


def _is_descending(c1: Zhongshu, c2: Zhongshu) -> bool:
    """后枢 GG 严格低于 前枢 DD → 下跌延续（中心定理二）。"""
    return c2.gg < c1.dd


def _filter_settled(zhongshus: list[Zhongshu]) -> tuple[list[int], list[Zhongshu]]:
    """过滤 settled 中枢，返回 (原始索引列表, settled中枢列表)。"""
    indices: list[int] = []
    settled: list[Zhongshu] = []
    for i, zs in enumerate(zhongshus):
        if zs.settled:
            indices.append(i)
            settled.append(zs)
    return indices, settled


def _greedy_group(settled_zs: list[Zhongshu]) -> list[tuple[list[int], str]]:
    """贪心分组：同向中枢归入同一 group。返回 [(offsets, direction)]。"""
    groups: list[tuple[list[int], str]] = []
    current_offsets: list[int] = [0]
    current_dir: str = ""

    for k in range(1, len(settled_zs)):
        prev_zs = settled_zs[k - 1]
        curr_zs = settled_zs[k]

        if _is_ascending(prev_zs, curr_zs) and current_dir in ("", "up"):
            current_offsets.append(k)
            current_dir = "up"
        elif _is_descending(prev_zs, curr_zs) and current_dir in ("", "down"):
            current_offsets.append(k)
            current_dir = "down"
        else:
            groups.append((current_offsets, current_dir))
            current_offsets = [k]
            current_dir = ""

    groups.append((current_offsets, current_dir))
    return groups


def _group_to_move(
    offsets: list[int], direction: str,
    settled_zs: list[Zhongshu], settled_indices: list[int],
    next_seg_start: int | None, num_segments: int | None,
) -> Move:
    """将一个 group 转换为 Move。"""
    first_zs = settled_zs[offsets[0]]
    last_zs = settled_zs[offsets[-1]]
    zs_count = len(offsets)

    base_seg_end = last_zs.seg_end
    if next_seg_start is not None:
        base_seg_end = next_seg_start - 1
    elif num_segments is not None and num_segments > 0:
        base_seg_end = num_segments - 1

    if zs_count >= 2:
        kind: Literal["consolidation", "trend"] = "trend"
        move_dir: Literal["up", "down"] = direction  # type: ignore[assignment]
    else:
        kind = "consolidation"
        move_dir = first_zs.break_direction  # type: ignore[assignment]

    group_centers = [settled_zs[o] for o in offsets]
    return Move(
        kind=kind, direction=move_dir,
        seg_start=first_zs.seg_start, seg_end=base_seg_end,
        zs_start=settled_indices[offsets[0]], zs_end=settled_indices[offsets[-1]],
        zs_count=zs_count, settled=True,
        high=max(zs.gg for zs in group_centers),
        low=min(zs.dd for zs in group_centers),
        first_seg_s0=first_zs.first_seg_s0, last_seg_s1=last_zs.last_seg_s1,
    )


def moves_from_zhongshus(
    zhongshus: list[Zhongshu],
    num_segments: int | None = None,
) -> list[Move]:
    """从中枢列表构造 Move（贪心分组，只处理 settled 中枢）。"""
    settled_indices, settled_zs = _filter_settled(zhongshus)
    if not settled_zs:
        return []

    groups = _greedy_group(settled_zs)

    result: list[Move] = []
    for g_idx, (offsets, direction) in enumerate(groups):
        if g_idx < len(groups) - 1:
            next_first_offset = groups[g_idx + 1][0][0]
            next_seg_start: int | None = settled_zs[next_first_offset].seg_start
        else:
            next_seg_start = None

        result.append(_group_to_move(
            offsets, direction, settled_zs, settled_indices,
            next_seg_start, num_segments,
        ))

    if result:
        result[-1] = replace(result[-1], settled=False)

    return result
```

## 中枢定义：`src/newchan/a_zhongshu_v1.py` 核心部分

```python
@dataclass(frozen=True, slots=True)
class Zhongshu:
    zd: float           # 中枢下沿
    zg: float           # 中枢上沿
    seg_start: int      # 第一段索引
    seg_end: int        # 最后包含段索引
    seg_count: int      # 构成段数 >= 3
    settled: bool       # True = 已被突破段闭合
    break_seg: int      # 突破段索引（-1=未闭合）
    break_direction: str # "up" / "down" / ""
    first_seg_s0: int
    last_seg_s1: int
    gg: float           # 波动区间上界 GG = max(所有构成段的 high)
    dd: float           # 波动区间下界 DD = min(所有构成段的 low)

# 中枢的 settled 判定：j < n（是否已有突破段）
settled = j < n
```

## 实验验证：贪心分组逻辑本身正确

用4个模拟递升中枢测试，`_greedy_group` 正确返回 1 个 group（全部4枚），会产生 1 个 trend Move。

**关键问题**：实际数据中，这4个中枢的 `settled` 状态是什么？如果管线最终只返回了全部是 `consolidation` 的 moves，且 `moves_from_zhongshus` 只消费 `settled=True` 的中枢——

**怀疑根因**：缠论中，一个中枢的 `settled`（被后续段突破）在代码层依赖于"从当前时间点看已经完成的中枢"。如果在数据的**最后时间点**，大多数中枢的 `settled=True`，那理论上能合并。但如果中枢层和走势层之间存在**时序问题**，导致在某个历史时刻调用 `moves_from_zhongshus` 时只有1个settled中枢可见（其他中枢此时还没被突破），则会产生6个独立的consolidation。

## 需要诊断的问题

1. `moves_from_zhongshus` 在**什么时机**被调用？是全量历史数据最终一次调用，还是每根K线增量调用？
2. `MoveEngine.process_zhongshu_snapshot` 每次收到 `ZhongshuSnapshot` 时，其中 `zhongshus` 列表包含多少 settled 中枢？
3. `diff_moves` 如何比较前后两次的 moves？如果一个盘整 move 在上一时刻"已结算"，后续时刻它不会被重新合并进更大的趋势 move 吗？
4. **核心假设**：管线是否存在这样的问题——每次新中枢出现后，前一个中枢刚好变成 settled，`_greedy_group` 此时只能看到 1 个 settled 中枢（当前刚结算的那个），而上一次已结算的中枢产生的 move 已被 diff 标记为"结算"，不会被重新评估？

## 核心技术问题

`diff_moves` 是否会"锁定"已结算的 move，使其不能被后续的合并逻辑覆盖？这是否导致每个中枢被结算时都独立生成一个 consolidation move，而不是等待足够多的同向中枢再合并成 trend？


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

以下是严格诊断结论（按你要求的链路）：

## 1) 失败现象

观测到：  
- 存在连续递升中枢（至少前4个满足 `c2.dd > c1.gg`）。  
- 最终却产出 6 个 `consolidation`，没有 `trend`。

这与 `a_move_v1` 的冻结规则直接冲突（2+同向中枢应为趋势）。

---

## 2) 直接原因（代码级）

在 `a_move_v1.py` 里，`Move.kind` 的判定只看 `zs_count`：

- `_group_to_move(...)` 中：
  - `zs_count >= 2` → `trend`
  - 否则 → `consolidation`

所以“全是 consolidation”**等价于**：传入 `_group_to_move` 的每个 group 都是单元素（`zs_count==1`）。

而 `_greedy_group` 对递升序列本身是能合并的（你已用模拟验证）。  
因此直接原因不在 `_greedy_group` 算法本体，而在**输入/发布链路**：它没有让多个可合并中枢在同一次有效评估中进入同一 group，或评估结果被后续 diff 机制冻结成单中枢盘整。

---

## 3) 根本原因（设计/实现层）

根本原因是**走势层的增量发布策略与 `moves_from_zhongshus` 的全量重算语义不一致**：

- `moves_from_zhongshus` 设计语义：  
  “给我当前可见的 settled 中枢全集，我返回当前 moves 全貌（最后一个标 unsettled 作为可延展尾部）”。
- 你的管线现象表明：  
  实际发布链路（`process_zhongshu_snapshot` + `diff_moves`）把历史 move 过早“结算/锁定”，没有允许尾部从“1中枢盘整”被后续同向中枢扩展为“趋势”。

这就是你提的核心怀疑：`diff_moves` 锁定已结算 move，阻断后续合并 —— 与现象完全一致。

---

## 4) 分类判定

**实现错误**（不是定义冲突）。

理由：  
你给出的 v1 定义已明确“趋势=2+同向中枢”；当前行为是发布/状态管理实现违背该定义，而不是定义本身自相矛盾。

---

## 5) 修复方案（仅实现错误）

### 修复目标
保证每个 snapshot 下，move 结果以**全量重算结果为准**，diff 只做“变化通知”，不能做“语义裁剪/冻结”。

### 精确步骤

1. **`process_zhongshu_snapshot` 输入改为累计全集**  
   传入 `moves_from_zhongshus` 的必须是“截至当前时刻的全部 zhongshus”，不是“本次新增”。

2. **每次 snapshot 全量重算并覆盖内存态**  
   `new_moves = moves_from_zhongshus(all_zhongshus, num_segments)`  
   `self._moves = new_moves`  
   （不要只 append delta）

3. **`diff_moves` 改为“允许尾部替换”**  
   - 仅把“共同稳定前缀”视为不变；
   - 其后部分允许 remove/add（尤其允许 `consolidation(zs_count=1)` 被替换为 `trend(zs_count>=2)`）。

4. **加回归测试（必须）**
   - 用 4 个递升中枢做 snapshot 序列测试（逐步输入）：
     - 期望演进：`consolidation(1)` → `trend(2)` → `trend(3)` → `trend(4)`（最后一个可 unsettled）
   - 用你的金油比样本做端到端断言：
     - 至少应出现 1 个 `trend/up` 覆盖前述递升中枢。

---

如果你把 `MoveEngine.process_zhongshu_snapshot` 和 `diff_moves` 代码贴出来，我可以给你逐行定位“锁定发生在哪个条件分支”，并给出可直接粘贴的修补 patch。
