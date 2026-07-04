# H2 样本级验证原始数据：level1-4 第二类信号 N^δ 门拒绝阶段分解

- task: #8（codex H2 边界条件 L2 收口）
- **认识论等级**：L2（真实 BTC 全历史逐信号分解，可产否定性结果）
- **窗口**：bars=4613599（2017-08-17→2026-05-31，全量=4613599），max_bars=100000000
- **目标域**：level 1..=4 第二类(buy2/sell2)信号，进入 Γ 后逐信号分解

## 1. 信号计数（各级进入 Γ 的第二类信号数）
| level | 进入Γ信号数 |
|---|---|
| 1 | 4073 |
| 2 | 2033 |
| 3 | 872 |
| 4 | 263 |
| **合计** | **7241** |

## 2. N^δ 门拒绝阶段分解（rung k=lvl+1 div_cand，互斥优先序）
| 阶段 | 信号数 | 占比 | 含义 |
|---|---|---|---|
| base_none | 6574 | 90.79% | tower[lvl] 无 end==src 候选段（无定位） |
| no_upper | 0 | 0.00% | tower[lvl+1] 无含 src 段 ⟹ rungs 空退化 base-case |
| no_target | 0 | 0.00% | knode.sub_moves 无 end==src |
| cond1_dir | 0 | 0.00% | dir(m2)≠−δ（m2 非背驰段要求方向） |
| cond2_noprev | 0 | 0.00% | 无前序同向段 s_prev |
| **cond3_extreme** | **0** | **0.00%** | **Extreme 假（m2 未创新极值）= codex 互斥链** |
| cond4_weak | 0 | 0.00% | cond1∧2∧3 过但 div_cand=false ⟹ MACD 力度未衰减 |
| reject_elsewhere | 0 | 0.00% | rung lvl+1 cand=true 但 gate=false（更高 rung/is_sub） |
| gate_pass | 667 | 9.21% | 通过门 |

- confirm_side(δ) 假（δ 与 bits 侧不符，base 层拒，与互斥正交）：0

## 2b. 阶段0 诊断探针：区间套问题①（端点相等 vs 区间包含 base 级定位）
| 口径 | 命中数 | 占比 |
|---|---|---|
| 端点相等 find_move_by_end_index(tower[lvl],src) | 7241 | 100.00% |
| 区间包含 start≤src≤end（同 exec_moves） | 7241 | 100.00% |
| **false negative（包含命中∧端点未命中）** | **0** | **0.00%** |
| ↳ level 1 FN | 0 | |
| ↳ level 2 FN | 0 | |
| ↳ level 3 FN | 0 | |
| ↳ level 4 FN | 0 | |
- **判据**：base_false_neg=0 ⟹ 两口径无差异 ⟹ **NO-SHIP**（端点相等结论获区间包含加固，depth 归零非本口径伪影，无须改生产）。
- base_false_neg>0 ⟹ 端点相等系统性漏检坐实 ⟹ 按区间套.pdf §一 4 改区间包含定位（bottom-up + Sel_Θ 唯一），重跑三件套。

## 3. deliverable (a)：Extreme 必假占比（codex 等价 s_prev 使互斥成立）
- 到达 cond3 的信号数（cond1∧cond2 通过）：2468
- 其中 **Extreme 必假**（互斥成立，s_prev 使 m2 无法创新极值）：0（0.00%）
- 其中 **Extreme 可满足**（m2 创新极值 vs 最近同向 s_prev=q）：2468

**s_prev 与 s 间距（leg_gap=target_idx−j）分布**（=2 ⟹ 单条反向腿=codex「常见结构 s_prev==m1」；>2 ⟹ 多同向腿=可能 q≠m1）：
| leg_gap | 信号数 |
|---|---|
| 1 | 1108 |
| 2 | 977 |
| 3 | 303 |
| 4 | 59 |
| 5 | 16 |
| 6 | 5 |

## 4. deliverable (b)：s_prev≠m1（Extreme 可满足 gap）样本的门通过/拒绝分布
- Extreme 可满足样本总数：2468
  - 门**通过**（gate_pass）：31
  - 门**拒绝**（cond4_weak / reject_elsewhere）：0

**读解**：若 extreme_true=0 ⟹ 无 gap 样本，互斥链在 cond3 到达域内完全成立（Extreme 必假）。若 extreme_true>0 且门拒 ⟹ 这些 gap 样本被 cond4/更高 rung/is_sub 拒（非 cond3 互斥）——互斥链**不**是它们归零的原因。

## 5. deliverable (c)：level1-4 100%归零是否完全由 cond3 互斥链解释
- 被门拒信号数：6574 / 7241（gate_pass=667）
- 其中 cond3 互斥链（Extreme 必假）解释：0（占拒绝 0.00%）
- 其他机制（base_none/no_upper/no_target/cond1/cond2/cond4/elsewhere）解释：6574

### 判定：**部分归零**：存在通过门的 level1-4 信号 ⟹ 与 acc「100%归零」不符（窗口差异，跨窗核对）。

## 6. 拒绝样本明细（前 60 条，spot-check）
| lvl | src | δ | bits | 阶段 | cond flags | leg_gap | s极值 s_prev极值 |
|---|---|---|---|---|---|---|---|
| 1 | 1852 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 3336 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 3477 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 15402 | -1 | 0x10 | base_none | c1=1 c2=1 ext=1 dc=1 | gap=2 | s=438649000000 sp=436700000000 |
| 1 | 15180 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 15441 | -1 | 0x10 | base_none | c1=1 c2=1 ext=1 dc=1 | gap=2 | s=438649000000 sp=436700000000 |
| 1 | 15738 | -1 | 0x10 | base_none | c1=1 c2=1 ext=1 dc=0 | gap=2 | s=438649000000 sp=436700000000 |
| 2 | 15180 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 19825 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 19956 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 19978 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 20222 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 20356 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 20452 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 20613 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 22757 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 22823 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 22889 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 23511 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 23553 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 23575 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 23695 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 23631 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 23825 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 25510 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 25890 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 25759 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 25949 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 25412 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 30165 | -1 | 0x10 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=2 | s=458614000000 sp=459490000000 |
| 1 | 33799 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 33925 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 34088 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 36254 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 37609 | +1 | 0x02 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 37803 | +1 | 0x02 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 38171 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 38525 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 41835 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 42372 | +1 | 0x02 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=313401000000 sp=281700000000 |
| 1 | 42315 | +1 | 0x02 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=313401000000 sp=281700000000 |
| 1 | 42551 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 42573 | +1 | 0x02 | base_none | c1=1 c2=1 ext=0 dc=0 | gap=1 | s=313401000000 sp=281700000000 |
| 1 | 42759 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 43732 | +1 | 0x02 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 43732 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 51194 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 51194 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 51352 | -1 | 0x10 | base_none | c1=1 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 51352 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 51451 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 52121 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 52281 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 52465 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 52587 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 52690 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 2 | 52815 | -1 | 0x10 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 52957 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 53427 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |
| 1 | 53640 | +1 | 0x02 | base_none | c1=0 c2=0 ext=0 dc=0 | gap=0 | s=0 sp=0 |

