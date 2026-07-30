# #736 skip 断内两刀（#686 备料①，票面裁定 (i) 落地）

工位 `/private/tmp/wt-736`（分支 `research/skip-two-cuts`，main 尖端，已含 #711 落地的
`p127_skip_impact.rs`）。纪律：只写不判、生产文件零改动（`cand_event`/`chain_cert`/
`recursive_tower` 全程未碰，#668 在飞不受影响）；本票在既有 `p127_skip_impact.rs` 里追加两个
读数函数（`cut1_inside_outside_separation` / `cut2_skip_edge_position`），零新增 bin，零改动既有
`census`/`hit_rate` 逻辑。

## 零、执行环境

- 数据源：`/private/tmp/analysis/data_cache/btc_1m_full.json`（软链，指向主仓
  `analysis/data_cache/btc_1m_full.json`，与 #711 同一份数据）。
- 三窗：20,000 / 100,000 / 300,000 根（票面点名，`ParseLayerIncr` 严格只喂窗口内前 W 根）。
- 链簿推进节拍沿用 #711 订正后的口径：`CHAIN_ADVANCE_EVERY=1000`（周期推进，末根必推）——单次
  终态推进会让断边类观测数学上不可达，#711 已有完整推导，本票直接复用不重推。
- 基线 `cargo test --lib`：**2584 passed / 0 failed / 138 ignored**（改动前后逐字节相同，见附录）。

## 一、刀一：inside/outside 分离度（假设 1）

### 1.1 口径

`ChainEdge.skipped_levels: Vec<SkippedLevel>` 对每条跳边（`ChainEdgeKind::Skip`）逐级记
`alive_at_level` / `inside_parent`。本刀只取「断」的两档（`alive_at_level>0` 的两支），「缺」
（`alive_at_level==0`）不是票面比较对象，不计入：

- **断-在父外**（`inside_parent==0`）：该级有存活候选，但没有一个的区间落在父端点区间内。
- **断-在父内接不上**（`inside_parent>0`）：该级有候选落在父区间内，只是没有一个同时套住子端点。

对每次出现（不是每条链、是每条边的每一级 `SkippedLevel` 记录），关联三维：该记录所属链证书的
**结局**（`ChainStatus`：Open/Closed/Invalidated）、**方向**（链头 `nodes[0].key.side`：
Long/Short）、**窗口**（20k/100k/300k）。探针只输出分桶计数（`P736_CUT1` / `P736_CUT1_TOTAL`），
判定在本节人工做。

### 1.2 三窗读数（`P736_CUT1` 原始行，见附录 A 全文）

| 窗口 | status | side | broken_outside | broken_inside |
|---|---|---|---:|---:|
| 20k | Closed | Long | 0 | 5 |
| 20k | Closed | Short | 0 | 4 |
| 100k | Open | Short | 0 | 14 |
| 100k | Closed | Long | 6 | 6 |
| 100k | Closed | Short | 0 | 15 |
| 300k | Open | Short | 0 | 19 |
| 300k | Closed | Long | 16 | 34 |
| 300k | Closed | Short | 34 | 41 |
| 300k | Invalidated | Short | 3 | 0 |

合计：`outside=59`（0+6+53）、`inside=138`（9+35+94）。三窗均无 `Open×Long`、`Invalidated×Long`
组合出现（该数据集本身 Long 侧样本更少，见 §1.4）。

### 1.3 分离判定口径（自定，审定理由）

阈值：两档在某维度上的**占比差 ≥15 个百分点**，且方向在**全部可比较窗口上不反号**，才判「系统
差异」；否则判「不足以判系统差异」（不等于「确认无差异」，是「本次样本不支持下判」）。15pp 的
选取理由：三窗里最小合计样本（20k，`outside+inside=9`）下，1 个计数即约等于 11pp 的占比波动，
15pp 高于这个最小可分辨粒度的量级，避免把小样本的整数噪声读成系统效应；同时该阈值仍能捕到下面
实际观测到的 24pp 量级差异，不是空判据。

### 1.4 判定结果

**结局维度（status）——判定：系统差异，成立。**

| | Closed | Open | Invalidated |
|---|---:|---:|---:|
| 断-在父外（合计 59） | 94.9%（56） | **0.0%（0）** | 5.1%（3） |
| 断-在父内接不上（合计 138） | 76.1%（105） | **23.9%（33）** | 0.0%（0） |

Open 占比差 23.9pp，超过 15pp 阈值；方向在两个可比窗口（100k：outside 0/6 vs inside 14/35 = 0% vs
40%；300k：outside 0/53 vs inside 19/94 = 0% vs 20.2%）上完全一致——**断-在父外这一档，在本次三窗
样本里从未出现在仍 `Open`（未封口）的链上，断-在父内接不上则有约 24% 落在 `Open` 链上**。读法：
「父外」（候选整体不落父区间）大概率意味着这条支路已经跟父端点脱钩，对应的链更容易已经走到
终态（`Closed`/`Invalidated`）；「父内接不上」（候选还在父区间里，只是没套住子端点）更像是「还
没长到位」，链仍处于可延展的 `Open` 态——这与两档命名本身的几何含义（在外/在内）方向一致，不是
巧合读数。

`Invalidated` 占比（5.1% vs 0%）方向上也偏向「父外」，但只在 300k 一个窗口出现（N=3），不满足
「≥2 窗口方向一致」的复核条件，**按口径不计入「系统差异」结论，只作观察记录**，不足以下判。

**方向维度（side）——判定：不成立系统差异。**

| | Long | Short |
|---|---:|---:|
| 断-在父外（合计 59） | 37.3%（22） | 62.7%（37） |
| 断-在父内接不上（合计 138） | 32.6%（45） | 67.4%（93） |

占比差 4.7pp，远低于 15pp 阈值。且这个 Short 偏多的倾向与数据集本身的整体方向分布
（Long 67 / Short 130，合计占比 34.0%/66.0%）几乎重合——两档都只是跟随了样本整体的 Short 偏多，
不构成两档之间的差异化信号。

**窗口维度**：`outside_share`（outside/(outside+inside)）随窗口单调上升——20k=0%、100k=14.6%、
300k=36.1%。这是一个协变量趋势而非「两档互比」，本刀口径不对它单独下系统性判定，只如实记录：
可能是窗口越大、历史积累的「候选整体脱钩父区间」情形越多的真实结构效应，也可能是 20k 样本太小
（合计仅 9）导致的下限伪影——现有 3 个点不足以区分这两种解释，需要更多窗口点（如 50k/150k/500k）
才能判斜率是否稳健，本票不为此单独展开（属于窗口密度问题，不是本票两刀之一）。

## 二、刀二：skip 边位置分布（假设 5）

### 2.1 口径

对每条链证书的 `edges`（root→leaf 排序），把其中 `ChainEdgeKind::Skip` 的边按在 `edges` 中的下标
分三类：`idx==0` 为**首边**（紧邻链头/大级别端）、`idx==len-1`（`len>1`）为**末边**（紧邻链尾/
小级别端）、其余为**中间边**；`len==1`（链只有一条边，首末重合）单列为**单边链**，不占首末计数
（避免把"只有一种可能"的退化情形混进"首/末"的比较）。

### 2.2 三窗读数（`P736_CUT2`，见附录 A）

| 窗口 | skip 边总数 | 首边 | 中间边 | 末边 | 单边链 |
|---|---:|---:|---:|---:|---:|
| 20k | 9 | 0 | 0 | 0 | 9（100%） |
| 100k | 41 | 0 | 0 | 0 | 41（100%） |
| 300k | 128 | 9（7.0%） | 0（0%） | 2（1.6%） | 117（91.4%） |

### 2.3 判定口径与结果

判定门槛（自定，审定理由）：位置分布只在非单边样本（首+中+末）上有意义（单边链的"首末"是几何
退化，谈不上"位置"）；要求非单边样本量 **≥30** 才谈"系统性靠 leaf/root"或"均匀"，因为三分类
（首/中/末）在样本量个位数时任何单一格子都可能因二项噪声偶然偏离均匀（1/3）20pp 以上。

三窗里 20k、100k 的非单边样本量恒为 **0**——本数据集在这两个窗口下，产生跳边的链清一色是长度 2
的路径（root 直接到 leaf，中间没有别的存活端点入链），跳边位置根本没有"首/中/末"可言。300k
唯一给出非退化样本，但 **非单边合计仅 11**（9 首 + 0 中 + 2 末），远低于 30 的门槛。

**判定：INCONCLUSIVE，本票样本不支持对假设 5（系统性靠 leaf ≈ 小转大方向性 / 均匀或靠 root ≈
几何噪声）下任何一侧的结论。** 如实记录 300k 那 11 个非退化样本里首边（9）多于末边（2）这一方向
——若字面外推，这个方向其实更接近"靠 root"，与 44 课"小转大"预期的"靠 leaf"相反——但 N=11 下
这个方向本身也可能整个是噪声,不构成任何证据强度,只如实登记不外推。

## 三、与 #641「粗读数」（73/96）的差异说明

票面引用 #641：「300k = 73 在外 / 96 在内」，本票 300k 实测 **53 在外 / 94 在内**。20k（0/9）、
100k（6/35）两窗与 #641 表 5.3 逐位相同，唯独 300k 不同。核对 #641 报告（`issue641-n3-chain-impl-
20260729.md` §5.1），其链簿推进节拍是**按窗口大小换算**（20k 用 `chain_every=2000`、100k 用
`5000`、300k 用 `25000`，即 300k 只推进 12 次），而本票（沿用 #711 订正）三窗统一用固定
`CHAIN_ADVANCE_EVERY=1000`（300k 推进 300 次）。推进越密，越能在窗口内捕捉到「候选曾经存活、
后来状态变化」的中间态（这也正是 #711 发现「单次终态推进下断边数学上不可达」的同一机理，只是
这里作用在 `SkippedLevel` 的 inside/outside 分档上，不是 `crossed_nodes`）。20k/100k 两窗恰好在
两种节拍下收敛到同一读数（可能是这两个窗口的候选结构变化不够密集，粗细节拍捕捉到的是同一组
稳态事实），300k 窗口候选变化更密集，粗节拍（12 次推进）漏掉了细节拍（300 次推进）能看到的一部
分「断-在父外/在父内」记录，故不一致。**本票 53/94 是节拍订正后的读数，替代 #641 的 73/96 作为
本刀的权威基线**（沿用 #711 已确立的"周期推进"纪律，不是另立新口径）。

## 四、假设 2/3 插桩必要性评估

**假设 2**（"曾经 Confirmed 后证伪"，票面机制落在 `crossed_nodes` 维度）：#711 已实测三窗
`crossed_nodes` 恒 0（`CHAIN_ADVANCE_EVERY=1000`），并用 `every=50` 做过粒度稳健性复核仍恒 0。
本票复跑（同一 `every=1000`，本票 §一/§二两刀跑数所在的同一次执行）再次确认：三窗
`P127_BREAK_POS window=* none=true`、`broken_edges=0`——与 #711 完全一致。加上 #641 用更粗节拍
（12/20/10 次推进）跑出的读数里 `crossed_nodes` 同样是 0（见 #641 报告 §5.2「被边跨过的节点」
一栏三窗全 0）。也就是说，**从粗到细三种推进粒度（12~300 次），`crossed_nodes` 在本数据集上
无一例外恒为 0**——这不是节拍伪影（节拍越密只会让该数更容易非零，实测反而更稳固地钉在 0），
是这套分类规则+这份 BTC 数据的真实结构事实。**建议：不为假设 2 的 `crossed_nodes` 机制insert
新插桩**——没有实例可供插桩去解释；`nodes_falsified` 本身非零（300k=49，本票 §一跑数同批
`P127_DIAG` 已印出），但那些证伪节点无一例外是**链头证伪**（导致整条链 `Invalidated`），不是
路径**中间**节点被跨过——这是两件不同的事，若要继续追假设 2，需要先把"曾经 Confirmed 后证伪"
从 `crossed_nodes`（路径中间节点跨接）重新定义到"链头证伪"这条已有且非零的分支上，而不是加新
探针去找一个目前看来结构性不出现的中间态。

**假设 3**（几何余量）：本票没有直接跑数，只能类比评估。两点观察对假设 3 的插桩决策有参考价值：
(1) 刀一/刀二都是**零新插桩**完成的——`SkippedLevel.inside_parent`（刀一）与 `cert.edges` 的
既有排序（刀二）都是生产代码已经暴露的字段，本票只是新写了两个纯读数聚合函数，没有改动或新增
任何生产判据的计算逻辑；(2) 两刀的信号强度都严重依赖样本量，300k 窗口是唯一给出可读信号的窗口，
20k/100k 在刀二上甚至完全退化（非单边样本=0）。**建议：假设 3 插桩前，先用与本票同等成本（零新
插桩、只读现有字段）跑一版探索性读数，看"几何余量"能否从现有 `PredicateBreach`（`parent_interval`
/`child_interval`/`reason`）或 `SkippedLevel` 字段直接派生出来**——若可以，同样不需要新插桩；
若确实需要新字段，鉴于本票两刀在 300k 之外样本量普遍过薄的经验，插桩前应先把假设 3 的判定口径
（效应量/样本量门槛，仿本票 §1.3/§2.3）写清楚，避免插桩后又在小样本上打不出可判的结论。

## 五、可复跑命令

```bash
cd rust
cargo build --release --bin p127_skip_impact
./target/release/p127_skip_impact /private/tmp/analysis/data_cache/btc_1m_full.json
# 三窗全量输出（P127_* 沿用 #711 既有读数 + 本票新增 P736_CUT1/P736_CUT1_TOTAL/P736_CUT2）
# 可选：P127_DIAG=1 前缀，额外打印 book.summarize() 的簿级摘要（nodes_alive/falsified/crossed_nodes 等）
```

## 附录 A：本次三窗完整输出（`P127_*` 沿用 #711，`P736_CUT*` 为本票新增）

```
P127_INPUT loaded_bars=303000 load_limit=303000 windows=[20000, 100000, 300000]
P127_CHAIN_ADVANCES window=20000 every=1000 advances=20
P127_WINDOW window=20000 bars=20000
P127_CENSUS window=20000 chains=18 closed=18 skip_chains=9 skip_chain_ratio=0.5000 broken_chains=0 broken_chain_ratio=0.0000
P127_TIER window=20000 loose_a_closed=18 strict_b_closed_zero_skip=9 layered_c_closed_zero_break=18
P127_EDGES window=20000 total_edges=23 skip_edges=9 broken_edges=0 skip_edge_ratio=0.3913 broken_edge_ratio=0.0000
P127_SKIP_POS window=20000 L2->L0 count=9
P127_BREAK_POS window=20000 none=true
P127_HITRATE window=20000 n_bar=500 group=broken total=0 evaluated=0 hits=0 misses=0 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.0000 hit_rate_incl_flat=0.0000
P127_HITRATE window=20000 n_bar=500 group=control_no_break total=18 evaluated=18 hits=10 misses=8 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.5556 hit_rate_incl_flat=0.5556
P127_HITRATE window=20000 n_bar=1000 group=broken total=0 evaluated=0 hits=0 misses=0 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.0000 hit_rate_incl_flat=0.0000
P127_HITRATE window=20000 n_bar=1000 group=control_no_break total=18 evaluated=18 hits=10 misses=8 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.5556 hit_rate_incl_flat=0.5556
P736_CUT1 window=20000 status=Closed side=Long broken_outside=0 broken_inside=5
P736_CUT1 window=20000 status=Closed side=Short broken_outside=0 broken_inside=4
P736_CUT1_TOTAL window=20000 broken_outside=0 broken_inside=9 outside_share=0.0000
P736_CUT2 window=20000 total_skip_edges=9 first=0 middle=0 last=0 single=9 first_share=0.0000 middle_share=0.0000 last_share=0.0000 single_share=1.0000
P127_CHAIN_ADVANCES window=100000 every=1000 advances=100
P127_WINDOW window=100000 bars=100000
P127_CENSUS window=100000 chains=93 closed=78 skip_chains=41 skip_chain_ratio=0.4409 broken_chains=0 broken_chain_ratio=0.0000
P127_TIER window=100000 loose_a_closed=78 strict_b_closed_zero_skip=51 layered_c_closed_zero_break=78
P127_EDGES window=100000 total_edges=105 skip_edges=41 broken_edges=0 skip_edge_ratio=0.3905 broken_edge_ratio=0.0000
P127_SKIP_POS window=100000 L2->L0 count=41
P127_BREAK_POS window=100000 none=true
P127_HITRATE window=100000 n_bar=500 group=broken total=0 evaluated=0 hits=0 misses=0 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.0000 hit_rate_incl_flat=0.0000
P127_HITRATE window=100000 n_bar=500 group=control_no_break total=78 evaluated=77 hits=31 misses=46 flat=1 out_of_window=0 no_confirmed_at=0 hit_rate=0.4026 hit_rate_incl_flat=0.3974
P127_HITRATE window=100000 n_bar=1000 group=broken total=0 evaluated=0 hits=0 misses=0 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.0000 hit_rate_incl_flat=0.0000
P127_HITRATE window=100000 n_bar=1000 group=control_no_break total=78 evaluated=60 hits=25 misses=35 flat=18 out_of_window=0 no_confirmed_at=0 hit_rate=0.4167 hit_rate_incl_flat=0.3205
P736_CUT1 window=100000 status=Open side=Short broken_outside=0 broken_inside=14
P736_CUT1 window=100000 status=Closed side=Long broken_outside=6 broken_inside=6
P736_CUT1 window=100000 status=Closed side=Short broken_outside=0 broken_inside=15
P736_CUT1_TOTAL window=100000 broken_outside=6 broken_inside=35 outside_share=0.1463
P736_CUT2 window=100000 total_skip_edges=41 first=0 middle=0 last=0 single=41 first_share=0.0000 middle_share=0.0000 last_share=0.0000 single_share=1.0000
P127_CHAIN_ADVANCES window=300000 every=1000 advances=300
P127_WINDOW window=300000 bars=300000
P127_CENSUS window=300000 chains=341 closed=256 skip_chains=128 skip_chain_ratio=0.3754 broken_chains=0 broken_chain_ratio=0.0000
P127_TIER window=300000 loose_a_closed=256 strict_b_closed_zero_skip=150 layered_c_closed_zero_break=256
P127_EDGES window=300000 total_edges=337 skip_edges=128 broken_edges=0 skip_edge_ratio=0.3798 broken_edge_ratio=0.0000
P127_SKIP_POS window=300000 L2->L0 count=98
P127_SKIP_POS window=300000 L3->L0 count=19
P127_SKIP_POS window=300000 L3->L1 count=11
P127_BREAK_POS window=300000 none=true
P127_HITRATE window=300000 n_bar=500 group=broken total=0 evaluated=0 hits=0 misses=0 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.0000 hit_rate_incl_flat=0.0000
P127_HITRATE window=300000 n_bar=500 group=control_no_break total=256 evaluated=246 hits=108 misses=138 flat=10 out_of_window=0 no_confirmed_at=0 hit_rate=0.4390 hit_rate_incl_flat=0.4219
P127_HITRATE window=300000 n_bar=1000 group=broken total=0 evaluated=0 hits=0 misses=0 flat=0 out_of_window=0 no_confirmed_at=0 hit_rate=0.0000 hit_rate_incl_flat=0.0000
P127_HITRATE window=300000 n_bar=1000 group=control_no_break total=256 evaluated=235 hits=117 misses=118 flat=21 out_of_window=0 no_confirmed_at=0 hit_rate=0.4979 hit_rate_incl_flat=0.4570
P736_CUT1 window=300000 status=Open side=Short broken_outside=0 broken_inside=19
P736_CUT1 window=300000 status=Closed side=Long broken_outside=16 broken_inside=34
P736_CUT1 window=300000 status=Closed side=Short broken_outside=34 broken_inside=41
P736_CUT1 window=300000 status=Invalidated side=Short broken_outside=3 broken_inside=0
P736_CUT1_TOTAL window=300000 broken_outside=53 broken_inside=94 outside_share=0.3605
P736_CUT2 window=300000 total_skip_edges=128 first=9 middle=0 last=2 single=117 first_share=0.0703 middle_share=0.0000 last_share=0.0156 single_share=0.9141
```

## 附录 B：测试计数

- 改动前 `cargo test --lib`：2584 passed / 0 failed / 138 ignored
- 改动后 `cargo test --lib`：2584 passed / 0 failed / 138 ignored（无回归，本票只加两个纯读数
  聚合函数，不改任何生产判据/既有测试）
