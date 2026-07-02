# Task #7 acc-classification：level 塔中间级空洞 H1/H2/H3 判别

**认识论等级**：L2（真实 BTC 数据逐信号分级别计数，可产否定性结果）。
**窗口**：bars=4613599（2017-08-17→2026-05-31，全量=4613599），max_bars=99000000。
**判别**：三路 instrument N^δ 门前后分级别计数（bit-exact 复制 decompose_capturable_spread 收集路径）。

## 分级别计数表（cls.levels.len() 最大=7）
| level | tower段数 | bsp_pre(门前) | 第一类 | 第二类 | 第三类 | Γ非Flat | sig_post(门后) | 门滤除 |
|---|---|---|---|---|---|---|---|---|
| 0 | 40028 | 11153 | 0 | 0 | 11153 | 11153 | 7744 | 3409 |
| 1 | 12077 | 1059 | 0 | 1059 | 0 | 1059 | 0 | 1059 |
| 2 | 3565 | 324 | 0 | 324 | 0 | 324 | 0 | 324 |
| 3 | 992 | 69 | 0 | 69 | 0 | 69 | 0 | 69 |
| 4 | 244 | 21 | 0 | 21 | 0 | 21 | 0 | 21 |
| 5 | 61 | 4 | 0 | 4 | 0 | 4 | 1 | 3 |
| 6 | 6 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

## H1/H2/H3 判定（中间级 = level 1..levels_seen_max-1）
- level1: bsp_pre=1059 (一/二/三=0/1059/0) Γ非Flat=1059 sig_post=0 → 门滤空(H1候选)
- level2: bsp_pre=324 (一/二/三=0/324/0) Γ非Flat=324 sig_post=0 → 门滤空(H1候选)
- level3: bsp_pre=69 (一/二/三=0/69/0) Γ非Flat=69 sig_post=0 → 门滤空(H1候选)
- level4: bsp_pre=21 (一/二/三=0/21/0) Γ非Flat=21 sig_post=0 → 门滤空(H1候选)
- level5: bsp_pre=4 (一/二/三=0/4/0) Γ非Flat=4 sig_post=1 → 有信号

### 判定：**H1（N^δ 门滤空）**：中间级 bsp_pre>0 但门后 sig_post=0 ⟹ [J_{ℓ-1}⊆J_ℓ] 嵌套链严格滤掉中间级。需 codex 异质确认（约束4）。

## 结构 sanity：level≥1 通过门信号的 rung 链（team-lead ③）
| lvl | source_index | δ | bits(u8) | rung层数(tower[lvl+1..]含src) |
|---|---|---|---|---|
| 5 | 782184 | -1 | 0x10 | 0 |

**读解**：level_L 信号的 rung 层数 = 它上方 tower 各级含该 source_index 的段数。这些是**上级语境**（N^δ 递归链），不是「该信号也算作 level_(L-1) 信号」——每个 bsp 只在提取它的那个 classifier level 计一次（mod.rs 分级提取），rung 链是 N^δ 门的准入语境，非计数重复。

## P1 FullNest 验收：通过门信号有效跨级深度分布（task #22，含 level0）
**有效跨级深度** = n_delta 递归实际穿越的连续 cand=true 跨级层数（build_nest_certificate 与生产门共用构造，bit-exact）。
- depth=0 ⟹ 纯 base-case confirm_side（**退化**：等价单 bit 检查，**无区间套跨级触达**，与旧单级门弱化）。
- depth≥1 ⟹ 真跨级 [J_{ℓ-1}⊆J_ℓ] 触达（N^δ ≥2 层证书 e→ℓ，Q5 验收要求）。
| 有效深度 | 通过门信号数 | 占比 |
|---|---|---|
| 0 | 7474 | 96.50% |
| 1 | 269 | 3.47% |
| 2 | 2 | 0.03% |

**depth=0（退化 base-case）：7474/7745；depth≥1（真跨级触达）：271/7745**

### 逐执行级 × 深度矩阵
| exec_level | depth0 | depth1 | depth2 | depth3 | depth≥4 |
|---|---|---|---|---|---|
| 0 | 7473 | 269 | 2 | 0 | 0 |
| 5 | 1 | 0 | 0 | 0 | 0 |

### P1 判定：**FullNest 部分触达（混合）**：部分信号跨级（depth≥1），部分退化（depth=0）。区间套在有效域内**真触达但非全覆盖**——如实标注：depth=0 那部分等价 base-case，depth≥1 那部分是真 N^δ。

## 配对后 decomps level 分布（sig_post=门后配对前 vs decomps=配对后）
| level | sig_post(门后) | decomps(配对后) | 配对丢失 |
|---|---|---|---|
| 0 | 7744 | 7741 | 3 |
| 5 | 1 | 1 | 0 |

- n_unpaired（无配对出场反转信号，右删失剔除）=3
- **配对机制**：next_opp 表跨级别混合（所有 level 信号按 entry_bar 排序）——中间级信号找「下一个反向信号」时不分级别，几乎总配 level0（信号 7741/7742 是 level0）。decomp.level=入场信号 level（配对出场 level 不影响）。

