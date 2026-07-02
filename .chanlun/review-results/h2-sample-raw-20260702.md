# H2 样本级验证原始数据：level1-4 第二类信号 N^δ 门拒绝阶段分解

- task: #8（codex H2 边界条件 L2 收口）
- **认识论等级**：L2（真实 BTC 全历史逐信号分解，可产否定性结果）
- **窗口**：bars=350000（2025-09-30→2026-05-31，全量=4613599），max_bars=350000
- **目标域**：level 1..=4 第二类(buy2/sell2)信号，进入 Γ 后逐信号分解

## 1. 信号计数（各级进入 Γ 的第二类信号数）
| level | 进入Γ信号数 |
|---|---|
| 1 | 97 |
| 2 | 23 |
| 3 | 5 |
| 4 | 2 |
| **合计** | **127** |

## 2. N^δ 门拒绝阶段分解（rung k=lvl+1 div_cand，互斥优先序）
| 阶段 | 信号数 | 占比 | 含义 |
|---|---|---|---|
| base_none | 113 | 88.98% | tower[lvl] 无 end==src 候选段（无定位） |
| no_upper | 0 | 0.00% | tower[lvl+1] 无含 src 段 ⟹ rungs 空退化 base-case |
| no_target | 0 | 0.00% | knode.sub_moves 无 end==src |
| cond1_dir | 0 | 0.00% | dir(m2)≠−δ（m2 非背驰段要求方向） |
| cond2_noprev | 0 | 0.00% | 无前序同向段 s_prev |
| **cond3_extreme** | **0** | **0.00%** | **Extreme 假（m2 未创新极值）= codex 互斥链** |
| cond4_weak | 0 | 0.00% | cond1∧2∧3 过但 div_cand=false ⟹ MACD 力度未衰减 |
| reject_elsewhere | 0 | 0.00% | rung lvl+1 cand=true 但 gate=false（更高 rung/is_sub） |
| gate_pass | 14 | 11.02% | 通过门 |

- confirm_side(δ) 假（δ 与 bits 侧不符，base 层拒，与互斥正交）：0

## 2b. 阶段0 诊断探针：区间套问题①（端点相等 vs 区间包含 base 级定位）
| 口径 | 命中数 | 占比 |
|---|---|---|
| 端点相等 find_move_by_end_index(tower[lvl],src) | 127 | 100.00% |
| 区间包含 start≤src≤end（同 exec_moves） | 127 | 100.00% |
| **false negative（包含命中∧端点未命中）** | **0** | **0.00%** |
| ↳ level 1 FN | 0 | |
| ↳ level 2 FN | 0 | |
| ↳ level 3 FN | 0 | |
| ↳ level 4 FN | 0 | |
- **判据**：base_false_neg=0 ⟹ 两口径无差异 ⟹ **NO-SHIP**（端点相等结论获区间包含加固，depth 归零非本口径伪影，无须改生产）。
- base_false_neg>0 ⟹ 端点相等系统性漏检坐实 ⟹ 按区间套.pdf §一 4 改区间包含定位（bottom-up + Sel_Θ 唯一），重跑三件套。

## 3. deliverable (a)：Extreme 必假占比（codex 等价 s_prev 使互斥成立）
- 到达 cond3 的信号数（cond1∧cond2 通过）：15
- 其中 **Extreme 必假**（互斥成立，s_prev 使 m2 无法创新极值）：0（0.00%）
- 其中 **Extreme 可满足**（m2 创新极值 vs 最近同向 s_prev=q）：15

**s_prev 与 s 间距（leg_gap=target_idx−j）分布**（=2 ⟹ 单条反向腿=codex「常见结构 s_prev==m1」；>2 ⟹ 多同向腿=可能 q≠m1）：
| leg_gap | 信号数 |
|---|---|
| 1 | 15 |

## 4. deliverable (b)：s_prev≠m1（Extreme 可满足 gap）样本的门通过/拒绝分布
- Extreme 可满足样本总数：15
  - 门**通过**（gate_pass）：0
  - 门**拒绝**（cond4_weak / reject_elsewhere）：0

**读解**：若 extreme_true=0 ⟹ 无 gap 样本，互斥链在 cond3 到达域内完全成立（Extreme 必假）。若 extreme_true>0 且门拒 ⟹ 这些 gap 样本被 cond4/更高 rung/is_sub 拒（非 cond3 互斥）——互斥链**不**是它们归零的原因。

## 5. deliverable (c)：level1-4 100%归零是否完全由 cond3 互斥链解释
- 被门拒信号数：113 / 127（gate_pass=14）
- 其中 cond3 互斥链（Extreme 必假）解释：0（占拒绝 0.00%）
- 其他机制（base_none/no_upper/no_target/cond1/cond2/cond4/elsewhere）解释：113

### 判定：**部分归零**：存在通过门的 level1-4 信号 ⟹ 与 acc「100%归零」不符（窗口差异，跨窗核对）。

## 6. 拒绝样本明细（前 60 条，spot-check）
| lvl | src | δ | bits | 阶段 | cond flags | leg_gap | s极值 s_prev极值 |
|---|---|---|---|---|---|---|---|
| 1 | 2365 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 3745 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 4878 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 7300 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 10450 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 15461 | +1 | 0x02 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=11182206000000 sp=11016223000000 |
| 1 | 15461 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 18167 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 18089 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 19984 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 21712 | +1 | 0x02 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=11050000000000 sp=11024400000000 |
| 1 | 21712 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 24597 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 3 | 26895 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 35650 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 42510 | +1 | 0x02 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=10989997000000 sp=10792506000000 |
| 1 | 44135 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 44156 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 47597 | -1 | 0x10 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=11074226000000 sp=11125000000000 |
| 1 | 48923 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 50102 | -1 | 0x10 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 66990 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 68133 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 77545 | +1 | 0x02 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 77545 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 81371 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 81371 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 81371 | -1 | 0x10 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 85481 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 86608 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 96022 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 108353 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 111089 | +1 | 0x02 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 111089 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 3 | 112026 | -1 | 0x10 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 112747 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 118630 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 121762 | -1 | 0x10 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 123279 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 4 | 123279 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 127336 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 127290 | +1 | 0x02 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=8746376000000 sp=8739016000000 |
| 1 | 133084 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 133084 | -1 | 0x10 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 137678 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 139444 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 141702 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 143818 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 147651 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 156610 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 161836 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 170835 | +1 | 0x02 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 170835 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 176933 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 176369 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 177739 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 180311 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 180311 | -1 | 0x10 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=7918681000000 sp=7928828000000 |
| 1 | 181943 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 181943 | -1 | 0x10 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |

