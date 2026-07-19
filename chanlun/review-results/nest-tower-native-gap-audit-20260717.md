# 塔内原生化缺口审计：nest 证书管线 vs 递归塔（区间套判定对象映射）

日期：2026-07-17　性质：只读审计（未跑 cargo、未改任何 Rust 源码、主仓零写入；行号锚 = worktree
`kimi-nest-mainline-20260717` HEAD）。

## 0. 裁定语境与审计对象

新框架（刚裁定，替代旧 sidecar/加权框架）：**买卖点必须由背驰出现的级别向下递归到最小级别做
区间套确认；nest 证书 = 递归区间套背驰的成立记录，是买卖点『级别形成』的构成条件**——下游按
级别用证书，不按权重用。最终形态要求区间套判定在**递归塔内部原生实现**；nest 管线
（`classifier/nest.rs`）是独立对照实现。本审计回答三个问题：

1. nest 侧每个判定门对应的塔侧对象是否存在（存在 / 缺 / 语义偏差）；
2. 原生化的缺口清单（按依赖序）；
3. 两实现 bit-exact 互验的方案草案。

审计对象两侧：

- **nest 侧**：`rust/src/theta_v0/classifier/nest.rs`（`NestCertificate`/`Cand^δ`/`is_sub`/`Sel_Θ`，
  两条装配路径——#92 typed 路径 `assemble_typed_certificate`（nest.rs:465）为 #105 现行生产口径，
  旧 `CandDeltaEvent` 路径 `assemble_certificate_snapshot/terminal`（nest.rs:699/715）为对照）。
- **塔侧**：`rust/src/theta_v0/classifier/{recursive_tower,level_view,level_view_store,mod}.rs`
  及判据单一来源 `signal.rs`/`divergence.rs`/`bsp.rs`/`descend.rs`。
- 口径事实参照：`p100-cert-bsp-recon-20260717.md`（#105 口径 A=41/B=25 全量对账）、
  `p102-b-long-zero-attribution-20260717.md`（B Long 多级=0 为数据事实）、
  `p101-ruling-gap-analysis-20260717.md`（接入裁定缺口）；dump `/tmp/p92_ckpt_dump.txt`（只读）。

## 1. 两侧对象模型的根本差异（一切映射的前提）

**塔是「自底向上 compose」模型**：L0 线段 → 中枢窗 → 上级走势（`RMove::Compose` 携 `subs`，
recursive_tower.rs:102-116；窗口化 compose 契约锚 `composeStep`，mod.rs:383-386）。级间唯一的
原生联系是 **结构构成关系**（`LeveledMove.sub_moves`，recursive_tower.rs:112；`descend`，
descend.rs:115-120——整体-部分，part-whole）。BSP 在每级**独立**产出（`classify_impl`，
mod.rs:404-421：L0 走 `signal::extract_signals_with_hist`（signal.rs:831），级别-N 走
`extract_first_third_for_level`（mod.rs:250-267），B2 走 `extract_second_for_level`
（mod.rs:2139））。塔内不存在任何「跨级确认」对象。

**nest 管线是「自执行级向上装配、语义上自背驰级别向下确认」的区间套模型**：事件按级别分桶
（`events_by_level: &[Vec<_>]`，下标 = 塔级别），从执行级 e 基例向 top_level ℓ 逐級爬链
（`extend_typed_upward`，nest.rs:525；`extend_upward`，nest.rs:780），级间联系是**坐标区间
包含** `is_sub`（nest.rs:65-67）。`descend`（结构组成）与 `Sub`（坐标包含）同 port 自
`Origin.SubLevelDescent` 的不同侧面——塔实装了前者，nest 实装了后者，**没有第三处实装**。

时钟坐标同域（同为 source_index/bar 序），但钟的登记位置不同，见 §3。

## 2. nest 判定门 ↔ 塔侧对象映射

状态图例：【存在】塔侧有同语义对象且在主路径；【存在-旁路】对象在塔侧文件内但属
sidecar/seam，主路径不消费；【偏差】对象存在但语义不同；【缺】无对应对象。

### G1 基例终端确认 `Conf^δ_e`（typed 门 nest.rs:479-481；旧门 nest.rs:746）

- nest：`terminal_of(base) → BspBits`，`terminal.confirm_side(base.side)`（types.rs:216-221；
  `conf_plus`/`conf_minus` types.rs:202-212）。
- 塔：`LevelState.bsp: Rc<Vec<BspPoint>>`（mod.rs:183），per-level 生产（mod.rs:404-421）。
- **状态：存在（per-level 逐 bit 同源）**。偏差①：事件→BSP 的绑定桥（
  `point.source_index == event.turn_source && confirm_side`）在 p92 bin
  `terminal_bits_new`（p92_nest_replay_postruling.rs:931-944）与 runner 旧路径
  （runner.rs:225-229），**不在塔对象内**——塔无「背驰事件 ↔ BSP」稳定身份边。

### G2 基例力度门 `divergence_confirmed`（typed 门 nest.rs:476）

- nest typed：provider 独立合取 `segments_diverge`（level_view.rs:574-580、633-639），
  `divergence_confirmed` 为事件独立字段（level_view.rs:482）。
- 塔：同一 MACD 引擎在 `signal::judge_first_cached`（signal.rs:276）内折叠进 `bits.buy1/sell1`；
  P1 已证 ℓ=0 时 `cand_delta ≡ buy1∨sell1` 逐 bit（recursive_tower.rs:2133 派生 +
  recursive_tower.rs:2215-2267 对拍测试）。
- **状态：存在（单一引擎，两个消费点）**。力度真值在塔主路径已内化为 B1/S1 bit，typed 路径
  把它外化为字段——同源不同形，无判据分叉。

### G3 递归级候选谓词 `Cand^δ_ℓ`（typed 初筛 nest.rs:550-559；旧必要门 nest.rs:810-813）

- nest 侧两种语义并存（**审计必须记录的内部不一致**）：
  - 旧路径：`CandDeltaEvent.cand_delta` = D 背驰确认（MACD 严格 C<A ≡ buy1∨sell1，
    recursive_tower.rs:1146-1147、1187-1188、2133），装配时逐事件硬门（nest.rs:811）。
  - typed 路径：Cand = 纯结构宽候选 `dir ∧ Comparable ∧ Extreme`（provider 固定即真，
    level_view.rs:468-473 注释；Extreme 门 level_view.rs:567-573），力度留在 ②基例门
    （#97 D1 裁定，nest.rs:550-551 注释）；梯级 `cand` 恒 true 推入（nest.rs:559）。
- 塔侧对象：`CandDeltaEvent` + `level_cand_delta`（recursive_tower.rs:1149、1992）由
  `cand_delta_tower` 驱动（mod.rs:498）——**存在-旁路**（P1 谓词层，「纯增量只读层：不改
  classify 任何行为」，mod.rs:496）；typed `NestCandidateEvent` + `provide_nest_candidate_events`
  （level_view.rs:475、532）属 C2 seam，**默认关闭**（`C2LevelViewConfig.enabled=false`，
  level_view.rs:23-27）。`classify_impl` 主路径（mod.rs:327-449）不读任一者。
- **状态：存在-旁路 + 语义偏差（两套 Cand 定义未统一）**。P1 对拍只覆盖 ℓ=0
  （recursive_tower.rs:2215），ℓ≥1 的 Cand 等价无对拍。

### G4 相邻级区间套 `Sub` 包含（typed 门 nest.rs:555-557；旧门 nest.rs:818-821）

- nest：`is_sub(inner, outer)`（nest.rs:65-67，闭口径 `start≥ ∧ end≤`）。两条路径的边语义
  不同（内部不一致②）：typed = `C_child ⊆ C_parent`（双边同口径 `interval_b`，
  nest.rs:448-458）；旧 = `I(A_child) ⊆ D_parent`（`d_child_interval` nest.rs:658-664 对
  `d_parent_interval_snapshot/terminal` nest.rs:631-649）。
- 塔：**无任何跨级区间包含谓词**。全仓 grep `is_sub(` 只命中 nest.rs、bins
  （strict_nest_check.rs:406、p83/p102）与 backtest/econ_positive.rs:967——塔侧四个文件
  （recursive_tower/level_view/mod/descend）零命中。塔的原生跨级关系只有结构组成：
  `sub_moves`（recursive_tower.rs:112）、`descend`（descend.rs:115）、B2 的
  `second_type_via_sublevel_type1`（descend.rs:183-192，价格几何 `sub_broke_below/above`，
  非坐标包含）。
- **状态：缺（核心缺口）**。塔的区间字段齐备（`LeveledMove.start_index/end_index`
  recursive_tower.rs:106-109；`CpStructureIdentity.source_start/source_end`
  recursive_tower.rs:1037-1038），缺的是**跨级包含判定**这个谓词本身。

### G5 `Sel_Θ` 选择器（`sel_order`/`select_best`，nest.rs:56-90）

- nest：`selKey=(end_time,start_time,idx)` 字典序（nest.rs:47-60）；`select_best` 无生产消费方
  （仅 nest.rs 测试与 econ_positive.rs:1032）；`Chi`/`LevelNode`（nest.rs:93-141）同样只有
  测试消费。实际装配不用 `select_best`，用 DFS 排序+回溯取字典序最早可行链——且两条路径
  排序键互不一致（typed：`(sel_key, turn_source, index)` 升序，nest.rs:543-547；旧：
  `(D_parent, I(A), 原索引)` 升序，nest.rs:798-807），二者也都不等于 `sel_order` 的偏好方向
  （end_time 大优先）。∃ 语义下不影响存在性，只影响选中链身份。
- 塔：确定性身份 = `ElementId{level, ordinal}`（recursive_tower.rs:75-81，compose 产出序号），
  是**产出序**不是**候选间偏好序**；塔没有「多个包含父中选一」的概念。
- **状态：缺 / 语义偏差**。新框架若要求唯一定位见证（`selected_key_unique`，
  nest.rs:993-1001），选择器语义须裁定（或明示接受 ∃ 语义 + 身份归因标签，nest.rs:388-396
  `NestEventIdentity` 先例）。

### G6 方向 δ 链一致（typed 门 nest.rs:552；旧门 nest.rs:811）

- 塔：per-event `side`（recursive_tower.rs:1152；level_view.rs:477）+ per-level `BspBits` 侧
  bit（types.rs:173-179）。
- **状态：存在（per-level）；链级方向载体缺**（塔无链对象，见 G8）。

### G7 级别链数据 `events_by_level`（nest.rs:540-542）

- 塔：`Classification.levels`（mod.rs:194）与 `tower_snapshots` 同构（mod.rs:323-326、377），
  nest 事件桶下标 = 塔级别（p92 `collect_snapshot_candidates` by_level[level]，
  p92_nest_replay_postruling.rs:618-686；exec 从 1 起，p92:736-737）。
- **状态：存在（级别数据同源同下标）**。

### G8 链式证书对象与复验（`NestCertificate`/`TypedNestCertificate`/`n_delta`）

- nest：`NestCertificate`（nest.rs:232）+ `n_delta` 自高向低复验（nest.rs:318-339）+
  构造即校验 debug_assert（nest.rs:514、773）；typed sidecar `kinds/judge_at/identities`
  （nest.rs:407-445）。
- 塔：**无任何跨级链式对象**。`Classification.levels` 是 per-level `LevelState` 并列 Vec
  （mod.rs:190-195）；塔内证书族全是**单级**对象（`CpScanOwnership` recursive_tower.rs:434、
  `CpCertificateView` :1126、`FullTrendCQualified` :1114、`ThirdClassInCp` :1043）。
  唯一原生跨级 BSP 构成规则是 B2 的次级别一类构成（descend.rs:183；mod.rs:2160-2192
  `second_for_parent`）——一步结构下降、只产二类、走价格几何，不是多级背驰段区间套。
- **状态：缺（新框架『级别形成构成条件』的正主）**。

## 3. 时钟对照

| 钟 | nest 侧 | 塔侧 | 差异 |
|---|---|---|---|
| 结构端点 | `turn_source` = seg_c.1 / source_index（level_view.rs:589、648） | `BspPoint.source_index`；`LeveledMove.end_index`（recursive_tower.rs:108） | 同坐标域，对象粒度不同 |
| 算法确认时点 | 旧 `divergence_confirm_src`（recursive_tower.rs:1154-1158） | `cp_certificate_confirm_src`（recursive_tower.rs:1177） | 塔有同类字段但只在 c_p 证书族 |
| 首次可证钟 | typed `judge_at` = prefix 首见（level_view.rs:484 注释；**由 p92 bin 的 book `or_insert` 登记**，p92:479-483、725-733） | `FrozenCompletedMove.judge_at`（level_view_store.rs:27）属 CompletedFreeze 事件，不管背驰事件 | **首证钟登记在 bin，不在塔对象** |
| 快照钟 | `LevelViewQuery.as_of`（level_view.rs:150-155） | 同（C2 seam 内）；塔主路径逐 bar 因果（`classify_with_tower_incremental` ≡ 全量，mod.rs:1391-1425；p92:206-208 实测对拍） | 同机制 |
| 几何/钟拼合 | 区间几何取**终态快照**（p92:170-177 `as_of=max_bars-1`），钟取 prefix 首见——两者拼合 | 塔原生对象无此拼合形态 | 原生化须裁定因果形态：全程因果 vs 终态几何+首证钟 |
| D3 递降钟（父 judge ≤ 子 judge） | 只计数不作门（`d3_descent_stats`，nest.rs:441-445） | 无 | 未裁定是否成门 |
| `created_at` | 禁用（#91 裁定④；`CompletedFreezeEvent.created_at` level_view_store.rs:55 不读） | 同 | 一致 |

## 4. 区间构造对照

| 区间 | nest 侧构造 | 塔侧最近对象 | 差异 |
|---|---|---|---|
| typed B 口径（生产） | `interval_b = seg_c`：Trend pair.seg_c（level_view.rs:586）；Pan structure.seg_c（level_view.rs:645） | 无背驰段区间对象；`LeveledMove` move 跨度（recursive_tower.rs:106-109） | **背驰段区间只活在 C2 seam / sidecar** |
| typed A 口径（诊断） | `interval_a = structural_pair_span` leave→retest（level_view.rs:513-526；B 的 26–98 倍宽，p102 报告§3） | 无 | 已裁定仅并行诊断（nest.rs:379-386） |
| 旧 D_parent | `c_interval_full`（recursive_tower.rs:1169；snapshot nest.rs:631-637 / terminal nest.rs:640-649） | `CpStructureIdentity.source_start/source_end`（recursive_tower.rs:1037-1038）+ `cp_terminal_certificate`（:1354） | 对象在塔侧，消费在 nest |
| 旧 I(A) | `a_interval`（recursive_tower.rs:1163；`d_child_interval` nest.rs:658-664） | 无（A 段配对在 signal/divergence 内部，不沉淀为塔对象字段） | 同上 |
| 坐标域 | source_index（K 序） | source_index | **同域，可 bit-exact 比较**——好消息 |

## 5. 原生化缺口清单（按依赖序）

**N0（裁定前置，非代码）——`Cand^δ` 语义二分未统一**。typed 结构宽候选
（level_view.rs:468-473，rung cand 恒 true nest.rs:559）vs 旧 `cand_delta` 力度确认
（recursive_tower.rs:1146-1147、2133，nest.rs:811 硬门）。P1 只证 ℓ=0 等价
（recursive_tower.rs:2215-2267），ℓ≥1 无对拍。原生 Cand 取哪个定义须先裁定；同时裁定
Sub 边语义（C⊆C 还是 I(A)⊆D_parent，§2.G4 内部不一致②）。

**N1——背驰段区间对象缺塔内原生载体**（依赖 N0）。`seg_c`/`interval_b` 只在默认关闭的
C2 seam（level_view.rs:532；开关 level_view.rs:23-27）与 sidecar 驱动的 `CandDeltaEvent`
（recursive_tower.rs:1149；驱动器 mod.rs:498）。`classify_impl` 主路径（mod.rs:327-449）
不产生、不保存背驰段区间；BSP 端点只有 `source_index` 点坐标。

**N2——跨级区间包含谓词缺**（依赖 N1）。`is_sub` 唯一实装在 nest.rs:65-67；塔侧四文件零
实装（§2.G4 grep 证据）。塔只有结构组成关系（`sub_moves` recursive_tower.rs:112；`descend`
descend.rs:115）。原生化 = 在塔对象上定义背驰段区间的跨级包含谓词。

**N3——跨级链/证书对象缺**（依赖 N2）。塔无链式类型（§2.G8）；B2 构成（descend.rs:183）
是唯一原生跨级规则但不同构。需原生「级别链证书」对象（或将 `NestCertificate` 提升为塔
对象并消解其 crate 内构造边界，nest.rs:165-202、342-377）。

**N4——事件 ↔ BSP 稳定身份边缺**（依赖 N1）。绑定靠 `source_index` 等值匹配（p92:931-944；
runner.rs:225-229）；`CandDeltaCpEdge`（recursive_tower.rs:1137-1141）只链事件→c_p，不链
BSP。塔内 `BspPoint`（bsp.rs）与 Cand 事件互无引用。

**N5——首证钟字段缺**（依赖 N4）。`judge_at` 登记在 p92 bin book（p92:479-483、725-733），
塔对象无 first-provable 字段；`level_view_store.rs:27` 的 `judge_at` 属 CompletedFreeze。
并须裁定因果形态（§3「几何/钟拼合」行）。

**N6——`Sel_Θ` 选择器缺**（依赖 N3）。`sel_order`/`select_best`（nest.rs:56-90）无塔对应；
塔只有产出序 `ElementId.ordinal`（recursive_tower.rs:75-81）。nest 内部两 DFS 序互不一致
（nest.rs:543-547 vs 798-807），须裁定唯一链语义或接受 ∃ 语义 + 身份归因。

**N7——生产消费点缺（最深缺口）**（依赖 N3–N6）。nest 装配无任何生产消费：
`classify_impl` BSP 置位不读证书（mod.rs:404-421）；runner 唯一装配调用是 env-gated
sidecar（runner.rs:220；开关 runner.rs:241-245）；typed 装配只在 bins（p92:785、853；
p102_b_long_zero_probe.rs:94-101）。现状 = BSP 先行、证书事后标注（p100/p101 报告的打标
模式，28,417 BSP vs 66 证书）。新框架要求证书成为买卖点级别形成的**构成条件**——即 BSP
级别形成须消费 N3 链证书，这与现状主路径方向相反，是原生化的最终落点。

## 6. 对照验证方案草案（两实现 bit-exact 互验）

目标：原生实现 T′ 与 nest 管线 N 在同一输入上产出**同一证书集合**（身份、钟、口径逐字段）。
约束：T′ 不得复读 nest.rs 函数（禁「等价自证」）；只允许共享更上游单一来源
（`divergence.rs` 引擎、`types.rs::BspBits`、塔对象与坐标系）。

可复用先例：全量/增量塔 bit-exact（mod.rs:1391-1425；p92:206-208 对拍 diff=0）；P1 谓词
对拍形态（recursive_tower.rs:2215-2267）；终态独立重建对拍（p102 报告§8：
candidates=3,683 ≡ P92_YIELD、CERT 66/66 双向 diff=0）；golden = `/tmp/p92_ckpt_dump.txt`
（66 行 CERT + 18 个 CKPT 快照，只读）。

- **L1 门级对拍**：固定 tower snapshot 输入，逐门等价——T′ 的 Cand/方向/Sub/终端判定 vs
  nest 对应门逐事件同真值。Cand 对拍必须覆盖 ℓ≥1（补 P1 只覆盖 ℓ=0 的空白，
  recursive_tower.rs:2215 现状）；Sub 对拍固定 B 口径（nest.rs:448-458 typed_interval）。
- **L2 事件级对拍**：T′ 事件集 vs `provide_nest_candidate_events`（level_view.rs:532）/
  `level_cand_delta`（recursive_tower.rs:1992）输出的 EventKey 多重集相等
  （`(level, side, turn_source, interval_b)`，p92 `EventKey` 语义）。
- **L3 链级对拍**：同一 (exec, top) 枚举下证书主键集相等——主键 = 条款 8
  `(caliber, exec, base turn_source, judge_max, side)`（p101 报告:120），golden = dump 66 行
  （A=41/B=25）。方向与 bucket 分层复核（A Long=17/Short=24；B Long=12/Short=13，
  p101 报告:18）。
- **L4 钟级对拍**：`judge_at`/`confirm_src` 逐链相等；任意 prefix `as_of` 快照下两实现
  证书集相等（复用 p92 CKPT 侧信道机制，p92:459-499，每 K bar 快照装配）。
- **L5 生产等价**：T′ 接入 classify 主路径时「证书门关闭臂」逐字节不变（bit-exact 门
  先例：p101 报告:115 引 runner.rs:4426；p92-postfix 的 P92_BIT_EXACT 全 0）。
- **L6 已知差异白名单**：A 口径仅诊断（nest.rs:379-386）、D3 只计数（nest.rs:441-445）、
  盘背 diag 不入链（recursive_tower.rs:2075-2108；裁决②）——互验时这些差异须显式列入
  白名单而非静默吸收。

建议实装序：L1→L2（对象层）先行落地为 cargo 测试；L3/L4 在 T′ 出首版后跑全量
（btc_1m_full.json 4,613,599 bar）对 dump；L5 为接入主路径时的回归门。

## 7. 边界与纪律声明

- 本审计为纯文档：未跑 cargo、未改任何生产源码（divergence/bsp/signal/level_view/nest.rs
  等只读）、未做任何 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入；
  唯一新文件 = 本报告。
- 行号锚均为 worktree 内文件当前 HEAD；`p92:` 前缀 = `rust/src/bin/p92_nest_replay_postruling.rs`。
- 结论性判断均给出代码锚；「缺」的判断以全仓 grep 证据支撑（§2.G4）。
- 090 纪律：本报告不提出任何简化实装或补丁方案；N0–N7 全部是缺口陈述与裁定请求，
  不是设计承诺。

—— 塔内原生化缺口审计收口。
