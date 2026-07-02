# Task #7 acc-classification：level 塔中间级空洞 H1/H2/H3 判别

**认识论等级**：L2（真实 BTC 数据逐信号分级别计数，可产否定性结果）。
**窗口**：bars=300000（2025-11-04→2026-05-31，全量=4613599），max_bars=300000。
**判别**：三路 instrument N^δ 门前后分级别计数（bit-exact 复制 decompose_capturable_spread 收集路径）。

## 分级别计数表（cls.levels.len() 最大=6）
| level | tower段数 | bsp_pre(门前) | 第一类 | 第二类 | 第三类 | Γ非Flat | sig_post(门后) | 门滤除 |
|---|---|---|---|---|---|---|---|---|
| 0 | 2502 | 714 | 0 | 0 | 714 | 714 | 714 | 0 |
| 1 | 756 | 77 | 0 | 77 | 0 | 77 | 11 | 66 |
| 2 | 220 | 22 | 0 | 22 | 0 | 22 | 22 | 0 |
| 3 | 62 | 5 | 0 | 5 | 0 | 5 | 5 | 0 |
| 4 | 15 | 2 | 0 | 2 | 0 | 2 | 2 | 0 |
| 5 | 4 | 1 | 0 | 1 | 0 | 1 | 1 | 0 |

## H1/H2/H3 判定（中间级 = level 1..levels_seen_max-1）
- level1: bsp_pre=77 (一/二/三=0/77/0) Γ非Flat=77 sig_post=11 → 有信号
- level2: bsp_pre=22 (一/二/三=0/22/0) Γ非Flat=22 sig_post=22 → 有信号
- level3: bsp_pre=5 (一/二/三=0/5/0) Γ非Flat=5 sig_post=5 → 有信号
- level4: bsp_pre=2 (一/二/三=0/2/0) Γ非Flat=2 sig_post=2 → 有信号

### 判定：**中间级有信号**：与反演不符，可能 300K 窗与全历史窗差异（跨窗对比 H2 判别）。

## 结构 sanity：level≥1 通过门信号的 rung 链（team-lead ③）
| lvl | source_index | δ | bits(u8) | rung层数(tower[lvl+1..]含src) |
|---|---|---|---|---|
| 2 | 3810 | -1 | 0x10 | 0 |
| 1 | 8430 | 1 | 0x02 | 1 |
| 1 | 8407 | 1 | 0x02 | 1 |
| 3 | 12183 | 1 | 0x02 | 0 |
| 1 | 15721 | -1 | 0x10 | 1 |
| 2 | 16990 | 1 | 0x02 | 0 |
| 2 | 16990 | -1 | 0x10 | 1 |
| 2 | 27545 | 1 | 0x02 | 0 |
| 2 | 31371 | 1 | 0x02 | 0 |
| 2 | 31371 | -1 | 0x10 | 0 |
| 3 | 62026 | -1 | 0x10 | 0 |
| 1 | 70450 | -1 | 0x10 | 1 |
| 4 | 73279 | -1 | 0x10 | 0 |
| 2 | 77336 | 1 | 0x02 | 0 |
| 2 | 77290 | 1 | 0x02 | 0 |
| 1 | 91702 | -1 | 0x10 | 2 |
| 1 | 97651 | -1 | 0x10 | 1 |
| 1 | 104585 | -1 | 0x10 | 2 |
| 2 | 149261 | -1 | 0x10 | 0 |
| 2 | 158347 | -1 | 0x10 | 0 |
| 2 | 162568 | 1 | 0x02 | 0 |
| 2 | 167732 | 1 | 0x02 | 0 |
| 2 | 167724 | 1 | 0x02 | 0 |
| 1 | 169160 | -1 | 0x10 | 1 |
| 2 | 176549 | -1 | 0x10 | 0 |
| 3 | 188382 | 1 | 0x02 | 0 |
| 2 | 193639 | -1 | 0x10 | 0 |
| 3 | 203019 | 1 | 0x02 | 0 |
| 3 | 202979 | 1 | 0x02 | 0 |
| 2 | 209192 | 1 | 0x02 | 0 |
| 2 | 209192 | -1 | 0x10 | 0 |
| 4 | 216655 | -1 | 0x10 | 0 |
| 1 | 217747 | 1 | 0x02 | 1 |
| 1 | 238035 | -1 | 0x10 | 1 |
| 1 | 238011 | -1 | 0x10 | 1 |
| 2 | 239371 | -1 | 0x10 | 0 |
| 2 | 246359 | 1 | 0x02 | 0 |
| 2 | 267086 | 1 | 0x02 | 0 |
| 2 | 266851 | 1 | 0x02 | 0 |
| 5 | 270577 | 1 | 0x02 | 0 |
| 2 | 280941 | -1 | 0x10 | 0 |

**读解**：level_L 信号的 rung 层数 = 它上方 tower 各级含该 source_index 的段数。这些是**上级语境**（N^δ 递归链），不是「该信号也算作 level_(L-1) 信号」——每个 bsp 只在提取它的那个 classifier level 计一次（mod.rs 分级提取），rung 链是 N^δ 门的准入语境，非计数重复。

## P1 FullNest 验收：通过门信号有效跨级深度分布（task #22，含 level0）
**有效跨级深度** = n_delta 递归实际穿越的连续 cand=true 跨级层数（build_nest_certificate 与生产门共用构造，bit-exact）。
- depth=0 ⟹ 纯 base-case confirm_side（**退化**：等价单 bit 检查，**无区间套跨级触达**，与旧单级门弱化）。
- depth≥1 ⟹ 真跨级 [J_{ℓ-1}⊆J_ℓ] 触达（N^δ ≥2 层证书 e→ℓ，Q5 验收要求）。
| 有效深度 | 通过门信号数 | 占比 |
|---|---|---|
| 0 | 469 | 62.12% |
| 1 | 196 | 25.96% |
| 2 | 46 | 6.09% |
| 3 | 11 | 1.46% |
| 4 | 4 | 0.53% |

**depth=0（退化 base-case）：469/755；depth≥1（真跨级触达）：257/755**

### 逐执行级 × 深度矩阵
| exec_level | depth0 | depth1 | depth2 | depth3 | depth≥4 |
|---|---|---|---|---|---|
| 0 | 469 | 186 | 44 | 11 | 4 |
| 1 | 0 | 9 | 2 | 0 | 0 |
| 2 | 0 | 1 | 0 | 0 | 0 |

### P1 判定：**FullNest 部分触达（混合）**：部分信号跨级（depth≥1），部分退化（depth=0）。区间套在有效域内**真触达但非全覆盖**——如实标注：depth=0 那部分等价 base-case，depth≥1 那部分是真 N^δ。

## 小转大通道命中（**C2+C3(breakout) xzd**——codex #44 终局裁定(c)：level==1 硬门=C2∧C3新中枢突破，level>=2 维持 C2-only）
**小转大通过**：29 条（占通过门总数 755 的 3.84%）。level==1 的通过数已隐含 C3(新中枢+突破) 硬门；level>=2 仍为 C2-only（旧 C3 same_side_same_center 字段降为诊断，不参门）。
- 通道分离：区间套（descend=Some，depth 直方图 726 条）与小转大（descend=None，29 条）输入域 Some/None 互斥（codex §6-4 构造同义反复，非经验重叠）。
- **小转大域触达/C2/旧C3 分项（诊断，旧 C3=same_side_same_center 不参门）**：路由到 Xzd=95，其中 C2 成立=95，旧 C3 成立=0，门通过=29。
- **StructBreak 收紧测量（task #62，codex 终局裁决A）**：零 bit（bsp_class==0）候选中通过门=0 条（新代码下恒 0——`bsp_cand_type` 六 bit 全零恒 StructBreak ⟹ `build_gate_certificate` 恒 None，不再经旧 else=>Type3 分派误入 Nest/Xzd 通道）。

## C3 死门诊断探针（task #41，裁定 §5.4 精确规格）
| level | Xzd routed | C2 成立 | C3 成立(same_side_same_center) | C3 命中率 |
|---|---|---|---|---|
| 1 | 66 | 66 | 0 | 0.00% |
| 2 | 21 | 21 | 0 | 0.00% |
| 3 | 5 | 5 | 0 | 0.00% |
| 4 | 2 | 2 | 0 | 0.00% |
| 5 | 1 | 1 | 0 | 0.00% |

### lvl==1 子集分裂断点（裁定 §5.4：last_zs_exists / same_side_l0_type3_any / same_center_any / same_side_causal_ok）
- lvl==1 routed=66
- last_zs_exists=66 (100.00%)：s 跨度内存在次级中枢
- same_side_l0_type3_any=66 (100.00%)：存在同向 L0 Type3（不问 center）
- same_center_any=0 (0.00%)：存在任意侧 Type3 其 center==last_zs
- same_side_causal_ok=66 (100.00%)：同向 Type3 候选中存在 source_index<=confirm_index 者（否则=时间确认问题）
- **level==1 也是死门（本窗实测）**：same_side_same_center=0/lvl==1 routed ⟹ C2-only 全域退化处置在本窗成立，无需分级处置。

### C3 新判据（新中枢+突破）level==1 命中率（task #47，codex #44 终局裁定(c)）
- c3_new_center_exists=0 (0.00%)：source_index~confirm_index 间存在新确认次级中枢
- c3_new_center_breakout_ok=0 (0.00%)：新中枢被其后次级走势反向突破（level==1 硬门参门项）
- **判据健康度（终局不变量，codex #55/#56）**：level==1 C3 命中恒 0 为市场几何事实；breakout_ok>0 ⟹ #56 候选(2) 被推翻 / center-tower 漂移 ⟹ Xzd 口径失效，硬失败回 codex 复审。

### lvl>=2 死门真封（sub_bsp_type3_count 预期恒为 0）
- lvl>=2 routed=29，sub_bsp Type3 点总数=0
- **真封通过**：lvl>=2 结构性死门坐实（sub_bsp_type3_count=0），与 codex §5.1 静态代码分析一致。

## 配对后 decomps level 分布（sig_post=门后配对前 vs decomps=配对后）
| level | sig_post(门后) | decomps(配对后) | 配对丢失 |
|---|---|---|---|
| 0 | 714 | 710 | 4 |
| 1 | 11 | 11 | 0 |
| 2 | 22 | 22 | 0 |
| 3 | 5 | 5 | 0 |
| 4 | 2 | 2 | 0 |
| 5 | 1 | 1 | 0 |

- n_unpaired（无配对出场反转信号，右删失剔除）=4
- **配对机制**：next_opp 表跨级别混合（所有 level 信号按 entry_bar 排序）——中间级信号找「下一个反向信号」时不分级别，几乎总配 level0（信号 710/751 是 level0）。decomp.level=入场信号 level（配对出场 level 不影响）。

