# H2 样本级验证原始数据：level1-4 第二类信号 N^δ 门拒绝阶段分解

- task: #8（codex H2 边界条件 L2 收口）
- **认识论等级**：L2（真实 BTC 全历史逐信号分解，可产否定性结果）
- **窗口**：bars=4613599（2017-08-17→2026-05-31，全量=4613599），max_bars=5000000
- **目标域**：level 1..=4 第二类(buy2/sell2)信号，进入 Γ 后逐信号分解

## 1. 信号计数（各级进入 Γ 的第二类信号数）
| level | 进入Γ信号数 |
|---|---|
| 1 | 1059 |
| 2 | 324 |
| 3 | 69 |
| 4 | 21 |
| **合计** | **1473** |

## 2. N^δ 门拒绝阶段分解（rung k=lvl+1 div_cand，互斥优先序）
| 阶段 | 信号数 | 占比 | 含义 |
|---|---|---|---|
| base_none | 0 | 0.00% | tower[lvl] 无 end==src 候选段（无定位） |
| no_upper | 0 | 0.00% | tower[lvl+1] 无含 src 段 ⟹ rungs 空退化 base-case |
| no_target | 0 | 0.00% | knode.sub_moves 无 end==src |
| cond1_dir | 865 | 58.72% | dir(m2)≠−δ（m2 非背驰段要求方向） |
| cond2_noprev | 351 | 23.83% | 无前序同向段 s_prev |
| **cond3_extreme** | **257** | **17.45%** | **Extreme 假（m2 未创新极值）= codex 互斥链** |
| cond4_weak | 0 | 0.00% | cond1∧2∧3 过但 div_cand=false ⟹ MACD 力度未衰减 |
| reject_elsewhere | 0 | 0.00% | rung lvl+1 cand=true 但 gate=false（更高 rung/is_sub） |
| gate_pass | 0 | 0.00% | 通过门 |

- confirm_side(δ) 假（δ 与 bits 侧不符，base 层拒，与互斥正交）：0

## 3. deliverable (a)：Extreme 必假占比（codex 等价 s_prev 使互斥成立）
- 到达 cond3 的信号数（cond1∧cond2 通过）：257
- 其中 **Extreme 必假**（互斥成立，s_prev 使 m2 无法创新极值）：257（100.00%）
- 其中 **Extreme 可满足**（m2 创新极值 vs 最近同向 s_prev=q）：0

**s_prev 与 s 间距（leg_gap=target_idx−j）分布**（=2 ⟹ 单条反向腿=codex「常见结构 s_prev==m1」；>2 ⟹ 多同向腿=可能 q≠m1）：
| leg_gap | 信号数 |
|---|---|
| 1 | 257 |

## 4. deliverable (b)：s_prev≠m1（Extreme 可满足 gap）样本的门通过/拒绝分布
- Extreme 可满足样本总数：0
  - 门**通过**（gate_pass）：0
  - 门**拒绝**（cond4_weak / reject_elsewhere）：0

**读解**：若 extreme_true=0 ⟹ 无 gap 样本，互斥链在 cond3 到达域内完全成立（Extreme 必假）。若 extreme_true>0 且门拒 ⟹ 这些 gap 样本被 cond4/更高 rung/is_sub 拒（非 cond3 互斥）——互斥链**不**是它们归零的原因。

## 5. deliverable (c)：level1-4 100%归零是否完全由 cond3 互斥链解释
- 被门拒信号数：1473 / 1473（gate_pass=0）
- 其中 cond3 互斥链（Extreme 必假）解释：257（占拒绝 17.45%）
- 其他机制（base_none/no_upper/no_target/cond1/cond2/cond4/elsewhere）解释：1216

### 判定：**混合（互斥链非唯一机制）**：cond3 互斥链解释部分归零，其余由 cond1 方向/cond2 无前段/base/cond4/更高 rung 解释——codex H2 互斥链在 level1-4 **部分成立**，边界缺口（其他机制）实证存在，裁决需标注「互斥链是子集机制，非全部」。

## 6. 拒绝样本明细（前 60 条，spot-check）
| lvl | src | δ | bits | 阶段 | cond flags | leg_gap | s极值 s_prev极值 |
|---|---|---|---|---|---|---|---|
| 1 | 7656 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 7539 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 20613 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 20613 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 24072 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 27300 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 35657 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 35657 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 38983 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 47014 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 50812 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 54957 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 57205 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 64374 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 65348 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 65348 | -1 | 0x10 | cond3_extreme | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=433000000000 sp=433812000000 |
| 1 | 68235 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 68235 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 71423 | +1 | 0x02 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 3 | 75281 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 84011 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 88522 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 88522 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 88522 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 93205 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 97484 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 100678 | -1 | 0x10 | cond3_extreme | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=571499000000 sp=573502000000 |
| 1 | 102464 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 108097 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 112271 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 124596 | +1 | 0x02 | cond3_extreme | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=610500000000 sp=610000000000 |
| 2 | 125756 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 127379 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 133462 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 134182 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 134182 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 135071 | +1 | 0x02 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 142829 | +1 | 0x02 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 142829 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 147050 | +1 | 0x02 | cond3_extreme | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=916600000000 sp=901781000000 |
| 1 | 148376 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 157046 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 163044 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 163044 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 163926 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 171941 | +1 | 0x02 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 177832 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 177832 | -1 | 0x10 | cond2_noprev | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 178595 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 178433 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 182331 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 184756 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 187994 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 191856 | +1 | 0x02 | cond3_extreme | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=1351200000000 sp=1315000000000 |
| 1 | 191856 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 193041 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 193041 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 194051 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 200959 | -1 | 0x10 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 202439 | +1 | 0x02 | cond1_dir | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |

