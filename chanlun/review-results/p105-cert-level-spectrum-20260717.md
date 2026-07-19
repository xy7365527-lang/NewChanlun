# p105 nest 证书级别谱统计（新框架事实底座）

日期：2026-07-17　数据：btc_1m_full.json（4,613,599 bar）　状态：**定稿**
探针：`rust/src/bin/p105_cert_level_spectrum.rs`（新写，只读；`rust/Cargo.toml:76-81` 追加 [[bin]] 注册块，本 swarm 唯一授权改动点）
输入 dump：`/tmp/p92_ckpt_dump.txt`（只读，下称 dump，行号引其；CERT 明细 = dump:753-818，共 66 行）
探针输出：`/tmp/p105_full.txt`（全量 P105_* 行，附录 §8 摘录关键行）

复跑命令：

```text
cd rust && cargo run --release --features backtest_bin --bin p105_cert_level_spectrum -- /tmp/p92_ckpt_dump.txt
```

## 0. 结论（三问直答）

1. **证书级别身份**：66 张证书的 exec 级分布 = exec1 60 / exec2 4 / exec3 2；top 级分布 = top1 42 / top2 16 / top3 7 / top4 1（A、B 明细见 §2）。链深 = top−exec+1：深度 1 共 48 张、深度 2 共 12 张、深度 3 共 5 张、深度 4 共 1 张。**全样本未观察到 exec≥4 或 top≥5 的证书**（塔顶 max_lvl=4）。
2. **落点**：按『证书级 = top』的构成语义，证书标出的高级别买卖点落在 L1–L4 全部四个有 BSP 的级别上，但 63.6%（42/66）落在 L1；按『同级同 source BSP 身份』语义，只有 25 张（A）/ 24 张（B）证书的 top 身份在 BSP 账本上存在——**17 张（A）/ 1 张（B）多级链的高级 top 事件在 BSP 账本上根本没有对应买卖点，证书是该级别买卖点存在的唯一记录**。
3. **占比**：证书标记的去重 BSP（A∪B 合并，身份键 (lvl, source)）= L1 21 个（占 L1 BSP 4,476 的 **0.469%**）、L2 2 个（占 2,407 的 **0.083%**）、L3 2 个（占 1,210 的 **0.165%**）、L0 与 L4 **0 个**。合计 25/28,417 = 0.088%。L0 的 19,776 个 BSP **结构上不可能**被证书标记（nest provider 只监听 level≥1，见 §3），L4 的 548 个 BSP 无一张证书落在其上。

## 1. 复现核验（先固定『探针 = 生产路径』）

- **BSP 侧逐字复现 p100**：`P105_BSP events_total=28417 buys=14762 sells=13655 max_lvl=4`；分层 L0 10,298/9,478、L1 2,318/2,158、L2 1,199/1,208、L3 559/651、L4 388/160——与 `p100-cert-bsp-recon-20260717.md:74-79` 逐字一致。提取循环 = 生产 `IncrementalClassifier::classify_at` + runner 同语义 seen-set diff（与 p100 探针 `p100_cert_bsp_recon.rs:146-168` 同一段代码模式；644 meta-rule：无影子分叉）。
- **CERT 侧**：66 行解析（A=41 / B=25）与 #105 口径一致（`p100-cert-bsp-recon-20260717.md:14`）；探针解析时强制结构自校验（`p105_cert_level_spectrum.rs:137-146`）：每张证书链首元 level == top、末元 level == exec，66/66 全过——dump 的 ids 身份向量确为『高→低、含基例』（发射处 `p92_nest_replay_postruling.rs:805-834`）。
- **基例身份 100% 命中**：`P105_JOIN base_hit=41/41（A）、25/25（B）`。基例 (exec, turn_source) 必有同级同向 BSP——这正是生产装配门 `terminal_bits_new` 的判据（`p92_nest_replay_postruling.rs:931-944`：以 `event.level` 直接索引 `classification.levels`，找 `source_index == turn_source` 且 `bits.confirm_side(side)`，`types.rs:216-221`）。探针独立重放命中 66/66 = 两大生产 artifact（dump 与因果塔）互洽的硬证据。

## 2. 证书级别身份（纯 dump 侧，`P105_CERT_*` 行）

### 2.1 exec / top / (exec,top) / 链深分布

| caliber | n | exec=1 | exec=2 | exec=3 |
|---|---:|---:|---:|---:|
| A | 41 | 38 | 2 | 1 |
| B | 25 | 22 | 2 | 1 |

| caliber | top=1 | top=2 | top=3 | top=4 |
|---|---:|---:|---:|---:|
| A | 21 | 13 | 6 | 1 |
| B | 21 | 3 | 1 | 0 |

(exec,top) 联合：A = (1,1)×21、(1,2)×11、(1,3)×5、(1,4)×1、(2,2)×2、(3,3)×1；B = (1,1)×21、(1,2)×1、(2,2)×2、(3,3)×1。

| caliber | depth=1 | depth=2 | depth=3 | depth=4 |
|---|---:|---:|---:|---:|
| A | 24 | 11 | 5 | 1 |
| B | 24 | 1 | 0 | 0 |

链深 = ids 段数 = top−exec+1。exec=2/3 的 6 张（A 3 + B 3）全为深度 1 单级证书。

### 2.2 方向 × 级别交叉表

side × top（`P105_CERT_SIDE_TOP`）：

| caliber | side | top=1 | top=2 | top=3 | top=4 | 合计 |
|---|---|---:|---:|---:|---:|---:|
| A | Long | 11 | 4 | 2 | 0 | 17 |
| A | Short | 10 | 9 | 4 | 1 | 24 |
| B | Long | 11 | 0 | 1 | 0 | 12 |
| B | Short | 10 | 3 | 0 | 0 | 13 |

side × exec（`P105_CERT_SIDE_EXEC`）：A Long exec1×16/exec3×1，A Short exec1×22/exec2×2；B Long exec1×11/exec3×1，B Short exec1×11/exec2×2。

交叉核对：
- **B 口径 exec=1 且 top≥2 的 Long = 0**（上表 B-Long 行 top=2 为 0；唯一 B Long top=3 是 exec=3 单级 trend 证书）——与 #102 可复现现象逐字一致（`p102-b-long-zero-attribution-20260717.md:24`）。
- A 侧高级别（top≥2）Short 偏多：Short 14 张 vs Long 6 张；B 侧高级别 Short 3 张 vs Long 1 张。高级别买卖点构成的方向不对称在本数据集持续存在（#102 已定非口径缺陷，`p102-b-long-zero-attribution-20260717.md:99-102`）。

## 3. 层级索引对齐：为什么『nest 级 k == classifier lvl k』不是新假设

p101 条款 5 的『lvl 不硬映射』针对的是**时间就近绑定**（judge_max → 最近 BSP，`p100-cert-bsp-recon-20260717.md:50-53`）。本报告不做时间绑定，只做**身份连接**，而身份连接是生产装配自己使用的查法：

- nest provider 按塔级构造候选：`by_level[level]` 消费 `tower[level]`，且 `level == 0` 被跳过（`p92_nest_replay_postruling.rs:618-619`、:536-537）——**nest 体系不监听 L0，exec ≥ 1 恒成立**（checkpoint 装配循环 `for exec in 1..events.len()`，:850）。
- 基例门 `terminal_bits_new` 用 `event.level` 直接索引 `classification.levels`（`p92_nest_replay_postruling.rs:936-938`）。
- 增量分类器 `levels` 按 `level_idx in 0..=l_max` 逐级 push，与 tower 快照同索引（`classifier/mod.rs:1553,1595,2015`）。

⟹ 证书 ids 里的级号与 BSP 事件的 lvl 是**同一坐标系**，身份键 `(level, turn_source) ↔ (lvl, source_index)` 的连接是复用生产查法，不引入任何新映射。L0 BSP 不可能被证书标记是结构性事实，不是统计结果。

## 4. 身份连接结果（`P105_JOIN` / `P105_CERT_DETAIL`）

| caliber | n | base_hit | top_hit | top_side_hit | rung_hit / rung_total |
|---|---:|---:|---:|---:|---:|
| A | 41 | **41（100%）** | 25（61.0%） | 25 | 42 / 65 |
| B | 25 | **25（100%）** | 24（96.0%） | 24 | 25 / 26 |

- **base_hit 100%**：生产门要求的基例 BSP 全部在账本（§1 已述）。
- **top_hit**：命中的 25（A）/ 24（B）张里，21 张是深度 1（top=exec=1，top 即基例，必中）；其余为 A 的 2:132770、2:1327264（exec=2 单级）、3:2534466（exec=3 单级）、**3:689893（exec=1 top=3 Long 链之顶，唯一命中的高级 rung）**；B 为前三张同名身份。命中的 top BSP 全部方向确认（top_side_hit = top_hit）。
- **top_miss**：A 16 张 = 全部 exec=1 top≥2 链除 3:689893 外（11 张 top=2：704358/1574631/1832757/3750585/74621/150305/1723619/3060088/3105191/3629138/4130201；4 张 top=3：32117/156189/1766034/3625589；1 张 top=4：1905266）；B 1 张 = top=2 的 2:147126。这些高级 rung 只是背驰事件（D1 裁定 rung 不要求 divergence_confirmed，更不要求 BSP，`nest.rs:550-556`），其 turn_source 上同级从未出现过任何 bit 的 BSP。
- **中间 rung 零命中**：A 的 65 个链位置中 42 命中 = 41 基例 + 3:689893；23 个非基例高级 rung 除 3:689893 外全部未命中。⟹ **区间套的高级环节是『背驰事件嵌套』，不是『BSP 嵌套』**——多级证书在 L2+ 构成的买卖点身份，BSP 账本上基本不存在。
- **基例集合封闭性**：全部 17 张（A）/ 1 张（B）exec=1 多级链的基例身份（1:706241、1:79510、1:147126、1:1743555 等）都已是深度 1 证书的基例——L1 被标记身份集 = 21 个，不多不少；多级证书不在 L1 新增任何买卖点身份。

## 5. 级别谱占比（核心表）

### 5.1 读法一：构成语义（证书级 = top，『按级别用证书』的直接口径）

每级被证书构成的买卖点个数以证书的 top 身份计（不论该级 BSP 账本是否有同身份点），分母为该级 BSP 事件数：

| 级别 | BSP 总数（buy/sell） | A 构成 | A 占比 | B 构成 | B 占比 |
|---|---|---:|---:|---:|---:|
| L1 | 4,476（2,318/2,158） | 21（L11/S10） | 0.469% | 21（L11/S10） | 0.469% |
| L2 | 2,407（1,199/1,208） | 13（L4/S9） | 0.540% | 3（L0/S3） | 0.125% |
| L3 | 1,210（559/651） | 6（L2/S4） | 0.496% | 1（L1/S0） | 0.083% |
| L4 | 548（388/160） | 1（L0/S1） | 0.182% | 0 | 0 |
| L0 | 19,776（10,298/9,478） | 0（结构性，§3） | 0 | 0 | 0 |

方向细分（占比分母 = 该级该向 BSP 数）：A Long L1 11/2,318=0.475%、L2 4/1,199=0.334%、L3 2/559=0.358%；A Short L1 10/2,158=0.463%、L2 9/1,208=0.745%、L3 4/651=0.614%、L4 1/160=0.625%；B Long L1 0.475%、L3 1/559=0.179%；B Short L1 0.463%、L2 3/1,208=0.248%。**Short 侧高级别占比系统高于 Long**（L2：A 0.745% vs 0.334%），与 §2.2 方向不对称互为表里。

### 5.2 读法二：BSP 账本身份语义（top 身份确有同级同 source BSP）

| 级别 | BSP 总数 | A 标记 | B 标记 | A∪B 去重标记 | 合并占比 |
|---|---|---:|---:|---:|---:|
| L1 | 4,476 | 21（L11/S10） | 21（L11/S10） | **21** | 0.469% |
| L2 | 2,407 | 2（S2：132770、1327264） | 2（同左） | **2** | 0.083% |
| L3 | 1,210 | 2（L2：689893、2534466） | 1（L1：2534466） | **2** | 0.165% |
| L0 | 19,776 | 0 | 0 | **0** | 0 |
| L4 | 548 | 0 | 0 | **0** | 0 |
| 合计 | 28,417 | 25 | 24 | **25** | 0.088% |

两读法的关系：深度 1 证书两读法重合（42 张 → L1 21 个身份）；多级证书在构成语义下分布于 L2–L4（A 20 张 / B 4 张），但其高级 top 只有 1 例（3:689893）落进 BSP 账本。**新框架『按级别用证书』若要求级别买卖点身份可回查 BSP 账本，则多级证书的高级身份当前只有证书本身作记录**——下游设计（级别信号源、级别仓位映射）必须消费证书 ids 而非反查 BSP。

### 5.3 与 p100 时间就近口径的对照（非矛盾，是两种语义）

p100 报告证书时间最近 BSP 87.9% 落 L0（`p100-cert-bsp-recon-20260717.md:45-48`）；本报告身份连接落点 L0 = 0、L1 = 主体。两口径回答不同问题：时钟就近 ≈ 交易执行粒度（L0 密度最高），身份连接 = 结构构成级别（证书链所在级）。反向覆盖率亦不同义：p100 的 w1440 时间覆盖 345/28,417=1.214%（:148-149）是时钟邻域命中，本报告 25/28,417=0.088% 是身份命中，后者是前者的真子集语义，不可混用。

## 6. 附带事实（BSP 身份去重）

BSP 事件按 (lvl, source, bits) 展开计数（28,417，与 p100 同口径）；按 (lvl, source) 去重后的身份数（`P105_BSP_IDS`）：L0 17,843、L1 4,130、L2 2,209、L3 1,188、L4 531（合计 25,901，整体冗余 8.9%）。同一 source 可在不同时钟携带不同 bit 组合重复入场；冗余度 L0 9.8%、L1 7.7%、L2 8.2%、L3 1.8%、L4 3.1%。§5 占比分母沿用事件数（与 p100 基线对齐）；若改用身份数，L1 合并占比 21/4,130=0.508%，量级结论不变。

## 7. 对下游的接口提示（事实陈列，不越权裁定）

1. **exec>1 的 6 张证书**（每口径 3 张：2:132770 trend、2:1327264 pan、3:2534466 trend）未递归到最小级别 L1（深度 1 且 exec=top=2/3）。新框架『向下递归到最小级别做区间套确认』的严格读法下，这 6 张的级别买卖点构成地位需要一条明确裁定；本报告只固定其存在与身份（`P105_CERT_DETAIL` 各行）。
2. **L4 空白**：塔顶 L4 有 548 个 BSP 但无任何证书 top/rung 落点；唯一 top=4 链（A，Short，4:1905266）的高级身份不在账本。L4 级别买卖点的『证书构成』在本数据集为零样本。
3. **B 口径 L2–L3 构成极端稀疏**（L2 三张全 Short、L3 一张 exec=3 trend）：B 作为生产口径，其高级别买卖点几乎全部集中在 L1（21/25 = 84%）。
4. 25 个被标记 BSP 身份清单可直接从 `P105_CERT_DETAIL` 的 top_bsp=true 行提取（L1 21 个 + L2 2 个 + L3 2 个），供下游『按级别用证书』做联表主键。

## 8. 附录：P105_* 关键行原文

```text
P105_INPUT bars=4613599 certs_total=66 A=41 B=25
P105_CERT_SIDE caliber=A side=Long n=17
P105_CERT_SIDE caliber=A side=Short n=24
P105_CERT_EXEC caliber=A exec=1 n=38
P105_CERT_EXEC caliber=A exec=2 n=2
P105_CERT_EXEC caliber=A exec=3 n=1
P105_CERT_TOP caliber=A top=1 n=21
P105_CERT_TOP caliber=A top=2 n=13
P105_CERT_TOP caliber=A top=3 n=6
P105_CERT_TOP caliber=A top=4 n=1
P105_CERT_DEPTH caliber=A depth=1 n=24
P105_CERT_DEPTH caliber=A depth=2 n=11
P105_CERT_DEPTH caliber=A depth=3 n=5
P105_CERT_DEPTH caliber=A depth=4 n=1
P105_CERT_SIDE_TOP caliber=A side=Long top=1 n=11
P105_CERT_SIDE_TOP caliber=A side=Long top=2 n=4
P105_CERT_SIDE_TOP caliber=A side=Long top=3 n=2
P105_CERT_SIDE_TOP caliber=A side=Short top=1 n=10
P105_CERT_SIDE_TOP caliber=A side=Short top=2 n=9
P105_CERT_SIDE_TOP caliber=A side=Short top=3 n=4
P105_CERT_SIDE_TOP caliber=A side=Short top=4 n=1
P105_CERT_EXEC caliber=B exec=1 n=22
P105_CERT_EXEC caliber=B exec=2 n=2
P105_CERT_EXEC caliber=B exec=3 n=1
P105_CERT_TOP caliber=B top=1 n=21
P105_CERT_TOP caliber=B top=2 n=3
P105_CERT_TOP caliber=B top=3 n=1
P105_CERT_DEPTH caliber=B depth=1 n=24
P105_CERT_DEPTH caliber=B depth=2 n=1
P105_CERT_SIDE_TOP caliber=B side=Long top=1 n=11
P105_CERT_SIDE_TOP caliber=B side=Long top=3 n=1
P105_CERT_SIDE_TOP caliber=B side=Short top=1 n=10
P105_CERT_SIDE_TOP caliber=B side=Short top=2 n=3
P105_BSP events_total=28417 buys=14762 sells=13655 max_lvl=4
P105_BSP_LVL lvl=0 buys=10298 sells=9478 total=19776
P105_BSP_LVL lvl=1 buys=2318 sells=2158 total=4476
P105_BSP_LVL lvl=2 buys=1199 sells=1208 total=2407
P105_BSP_LVL lvl=3 buys=559 sells=651 total=1210
P105_BSP_LVL lvl=4 buys=388 sells=160 total=548
P105_BSP_IDS lvl=0 distinct_sources=17843
P105_BSP_IDS lvl=1 distinct_sources=4130
P105_BSP_IDS lvl=2 distinct_sources=2209
P105_BSP_IDS lvl=3 distinct_sources=1188
P105_BSP_IDS lvl=4 distinct_sources=531
P105_JOIN caliber=A n=41 base_hit=41 top_hit=25 top_side_hit=25 rung_hit=42 rung_total=65
P105_MARK_LVL caliber=A lvl=1 bsp_total=4476 top_marked=21 chain_marked=21 top_rate=0.004692 chain_rate=0.004692
P105_MARK_LVL caliber=A lvl=2 bsp_total=2407 top_marked=2 chain_marked=2 top_rate=0.000831 chain_rate=0.000831
P105_MARK_LVL caliber=A lvl=3 bsp_total=1210 top_marked=2 chain_marked=2 top_rate=0.001653 chain_rate=0.001653
P105_JOIN caliber=B n=25 base_hit=25 top_hit=24 top_side_hit=24 rung_hit=25 rung_total=26
P105_MARK_LVL caliber=B lvl=1 bsp_total=4476 top_marked=21 chain_marked=21 top_rate=0.004692 chain_rate=0.004692
P105_MARK_LVL caliber=B lvl=2 bsp_total=2407 top_marked=2 chain_marked=2 top_rate=0.000831 chain_rate=0.000831
P105_MARK_LVL caliber=B lvl=3 bsp_total=1210 top_marked=1 chain_marked=1 top_rate=0.000826 chain_rate=0.000826
P105_DONE
```

（66 行 `P105_CERT_DETAIL` 与 lvl=0/4 的零值 `P105_MARK_LVL*` 行见 `/tmp/p105_full.txt` 全文，共 163 行。）

## 9. 纪律声明

- 生产源码零改动（divergence/bsp/signal/level_view/nest.rs 等只读）；新增文件仅 `rust/src/bin/p105_cert_level_spectrum.rs` 与本报告；`rust/Cargo.toml` 仅追加授权的一个 [[bin]] 注册块（:76-81）。
- 未做任何 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入（数据经 worktree 内 symlink 只读消费）。
- 探针只读：不动判据、不写主路径状态；BSP 提取与 p100 同循环（644 meta-rule）。
- 声明与能力一致：本报告只做级别谱统计与身份连接，不声明任何择时 alpha；exec>1 证书的地位、L4 零样本的处置均列为待裁定项（§7），未擅自解释。
