# Move Rules v1 — 走势类型实例规范

> MVP-D0 引入，四层同构管线的第四层。

## 1. 缠论定义

| 概念 | 定义 |
|------|------|
| **盘整** | 包含恰好 1 个走势中枢 |
| **趋势** | 包含 2+ 个依次同向走势中枢 |
| **上涨趋势** | C2.DD > C1.GG（后枢波动下界 严格高于 前枢波动上界，中心定理二） |
| **下跌趋势** | C2.GG < C1.DD（后枢波动上界 严格低于 前枢波动下界，中心定理二） |
| **区间重叠** | 不满足上涨/下跌条件（GG/DD 有重叠） → 截断为不同 move |

## 2. 数据类

```python
@dataclass(frozen=True, slots=True)
class Move:
    kind: Literal["consolidation", "trend"]
    direction: Literal["up", "down"]
    seg_start: int      # identity key
    seg_end: int
    zs_start: int       # 在 zhongshu list 中的索引
    zs_end: int
    zs_count: int       # >= 1
    settled: bool       # 最后一个 = False
    high: float         # max(center.gg) — 波动极值（031号谱系结算）
    low: float          # min(center.dd) — 波动极值（031号谱系结算）
    first_seg_s0: int   # 前端定位
    last_seg_s1: int    # 前端定位
```

## 3. 身份/状态分离

| 字段 | 归属 | 说明 |
|------|------|------|
| `seg_start` | **身份** | 第一个中枢的首段索引，稳定不变 |
| `kind` | 状态 | 可从 consolidation → trend |
| `direction` | 状态 | 盘整可能随升级变化 |
| `zs_end` | 状态 | 趋势延伸时增长 |
| `settled` | 状态 | 后续 move 出现时升级 |

**身份键**：`(seg_start,)` — 单字段，跨 consolidation→trend 升级稳定。

## 4. 纯函数算法 — `moves_from_zhongshus()`

1. 过滤 `settled=True` 的中枢 → `settled_zs`
2. 若空 → 返回 `[]`
3. 贪心向右扫描：
   - `_is_ascending(c1, c2)`: `c2.dd > c1.gg` → 上涨延续（中心定理二：波动区间）
   - `_is_descending(c1, c2)`: `c2.gg < c1.dd` → 下跌延续（中心定理二：波动区间）
   - 同向且方向一致 → 归入当前 group
   - 否则 → 截断，开始新 group
4. 每个 group → 一个 Move：
   - `zs_count >= 2` → `kind="trend"`
   - `zs_count == 1` → `kind="consolidation"`
   - 盘整方向 = 中枢的 `break_direction`
5. 最后一个 move → `settled=False`

## 5. Diff 算法 — `diff_moves()`

与 `diff_zhongshu` 同构：

```
1. 找公共前缀（_move_equal 严格比较 kind+direction+seg_start+zs_end+settled）
2. prev 后缀：同身份(seg_start) → 跳过 invalidate；否则 → MoveInvalidateV1
3. curr 后缀：
   - 全新 + settled=True → Candidate + Settle（保证 I19）
   - 全新 + settled=False → Candidate
   - 同身份 + settled 升级(F→T) → Settle
   - 同身份 + zs_end 或 kind 变化 → Candidate（延伸/升级）
```

## 6. 事件类型

| 事件 | event_type | 触发条件 |
|------|-----------|---------|
| MoveCandidateV1 | `move_candidate` | 新 move 出现 / 延伸 / 升级 |
| MoveSettleV1 | `move_settle` | settled=False → True |
| MoveInvalidateV1 | `move_invalidate` | move 消失（身份消失） |

## 7. 同身份状态变化事件矩阵

| prev 状态 | curr 状态 | 事件 |
|-----------|-----------|------|
| consolidation, unsettled | consolidation, unsettled | 无（公共前缀） |
| consolidation, unsettled | consolidation, **settled** | MoveSettle |
| consolidation, unsettled | **trend**, unsettled | **MoveCandidate**（kind+zs_end 变） |
| trend, zs_count=2, unsettled | trend, zs_count=**3**, unsettled | MoveCandidate（延伸） |
| trend, zs_count=2, unsettled | trend, zs_count=2, **settled** | MoveSettle |
| 任意 | （身份消失） | MoveInvalidate |

## 8. 不变量

| Code | 名称 | 检查方式 |
|------|------|---------|
| I18 | MOVE_MIN_CENTER | zs_count >= 1（checker 实时） |
| I19 | MOVE_CANDIDATE_BEFORE_SETTLE | Settle 前必有同 move_id 的 Candidate（checker 实时） |
| I20 | MOVE_PARENTS_TRACEABLE | zs_end >= zs_start, zs_count >= 1（checker 实时） |
| I21 | MOVE_INVALIDATE_TERMINAL | invalidate 后同身份不复活（checker 实时） |
| I22 | MOVE_REPLAY_DETERMINISM | 同输入 → 同 event_id（测试覆盖） |

## 9. 测试覆盖

- `test_move_golden.py` — 15 个 golden 测试（纯函数 + diff）
- `test_move_determinism.py` — 3 个确定性测试（I22）
- `test_move_invariants.py` — 9 个不变量正/反例测试（I18-I21）

## 10. 前端 markers

| 事件 | 形状 | 颜色 | 文本 |
|------|------|------|------|
| move_candidate (consolidation) | square | #2196f3 (蓝) | M? |
| move_candidate (trend up) | square | #4caf50 (绿) | M↑ |
| move_candidate (trend down) | square | #f44336 (红) | M↓ |
| move_settle (up) | circle | #388e3c (深绿) | M✓ |
| move_settle (down) | circle | #c62828 (深红) | M✓ |
| move_invalidate | — | — | 不显示 |
