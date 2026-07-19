# p109：#83 结构嵌套链逐门对账（gate attrition）

日期：2026-07-17 ｜ 数据：btc_1m_full.json（4,613,599 bar，as_of=4,613,598）｜ 状态：**定稿**
探针：`rust/src/bin/p109_chain157_gate_attrition.rs`（新写，只读；bin 自动发现，未动 Cargo.toml）
探针输出：`/tmp/p109_full2.txt`（全量含 P109_CHAIN_DETAIL）；`/tmp/p109_full3.txt`（复跑，除 DETAIL 行序外逐字节一致）
输入 dump：`/tmp/p92_ckpt_dump.txt`（只读，下称 dump，行号引其）

## 0. 结论先行（对主线问题的直接回答）

**在现行生产语义下，全深度结构嵌套链是 6,482 条（不是 157），0 条全门通过；逐门对账显示
击杀主力不是市场几何，而是 provider 身份覆盖（58.5%）与 BSP 终端背书（25.2%）。**

- 「157」不可复现：#83 量于 #105 Trend C 终段修复**之前**＋V1/V2 fail-closed 断点语义
  （`c2-yield-remeasure-20260715.md:29-32` 的 tuple 为 `extended-to-exact-three-v1`）。同一定义
  在现行代码下：V2 口径 5,092 条、V3 生产口径 **6,482 条**（§2）。本报告对账对象是现行 6,482 条。
- 门级 attrition（6,482 条链，管线序首个空候选门 = 击杀门，∃ 语义与生产装配一致）：

| 门 | 击杀数 | 占比 | 击杀位置 | 归因分层 |
|---|---:|---:|---|---|
| 身份门（provider 无对应事件） | 3,790 | 58.47% | L1×3,781，L2×9 | Trend-leave 2,552＝Cand(Extreme) 定义口径；Cons-leave 1,238＝pan 结构定位，**待裁定（聋度候选）** |
| 终端门（无 confirm_side BSP） | 1,631 | 25.16% | 全在 L1 | 754 含趋势背驰确认基例却无同级同 source BSP＝**谓词分歧（聋度/装配工件）**；877 全 Pan 基例＝盘整背驰不入 BSP 账本（定义口径） |
| 背驰确认门（divergence_confirmed=false） | 1,020 | 15.74% | 全在 L1 | 全部 Pan 基例（Trend 基例构造性零杀，已证）＝MACD 面积谓词未过，**待裁定（聋度 or 市场事实）** |
| 包含门（is_sub@B 判负） | 30 | 0.46% | 全在 L1→L2 边 | **裁定②口径产物**：A 口径构造性全过（不变量 0 违例）；p102 机制逐字吻合 |
| 方向门（跨级方向不一致） | 11 | 0.17% | 全在 L1→L2 边 | **数据事实** |
| 时钟门（全通但无因果见证） | 0 | 0% | — | 无从触发：零全通链；50 条前缀证书 dump 见证 50/50 |
| **全门通过（应成证）** | **0** | **0%** | — | — |

- 50 条链通过基例三门（身份/背驰/终端），涉及 **11 个不同基例事件，全部在 66 张证书的基例
  身份集内**（`P109_CROSS base_in_66=50`）；这 50 条**全部死于 L1→L2 边**（身份 9／方向 11／
  包含 30），**没有一条链到达深度 2**（`P109_PREFIX_DEPTH depth=0×6,432 / depth=1×50`）。
- 对编排者主张的判定：「每个级别都该有对应证书」在**全深度链**层面被证伪——0/6,482；
  但证伪的主要责任不在「市场事实」：明确属市场几何/教义定义的只有方向门 0.17%＋身份门中
  Trend-leave 的 Extreme 口径（合计 39.5%）；**46.5% 落在未经逐案审计的聋度/工件候选**
  （pan 结构定位、盘整背驰 MACD 谓词、BSP 背书与身份桥），裁定② 口径直接代价仅 0.46%（§5）。

## 1. 口径、管线与硬锚（先固定『探针 = 生产路径』）

### 1.1 链定义（与 #83 逐字同构）

- 严格 C2 pair：同 run `view.moves.windows(2)` 两相邻均 Completed，区间 `[leave.start_index, retest.end_index]`
  （`rust/src/bin/p83_yield_remeasure.rs:408-417`；本探针 `p109_chain157_gate_attrition.rs:172-186`）。
- 相邻级边：闭包含 `is_sub(child, parent)`（`rust/src/theta_v0/classifier/nest.rs:65-67`）；
  完整链 = L1–L5 每级恰取一个 pair 且每条相邻边成立（`c2-yield-remeasure-20260715.md:53-62`）。
- 枚举数 = DP 计数：`P109_ENUM chains_enumerated=6482 dp_complete=6482`（枚举与 p83
  `chain_stats` 语义 DP 互验，`p83_yield_remeasure.rs:455-516` 复刻于探针 `:330-384`）。

### 1.2 门定义（与生产装配逐门对应）

管线序 = `assemble_typed_certificate` 的判定序（`rust/src/theta_v0/classifier/nest.rs:465-522`）：

1. **身份门**：链级 pair 在 provider 宇宙有对应事件。匹配键 = `interval_a` 精确相等——合法键：
   `AssembledMove.{start,end}_index` 与 `structural_pair_span` 同取 `projection.seeds` 坐标
   （`level_view.rs:513-526,838-841`），严格 pair 区间与 `interval_a` 同源同义。
   自检：`P109_MATCH` 各级 `miss_with_endpoint_anchor=0`（无端点错位，缺失是真缺口）。
2. **背驰确认门**：基例 `divergence_confirmed`（nest.rs:476；provider 判据 level_view.rs:574-580,633-639）。
   rung 级不要求（#97 裁定，nest.rs:550-556 注释）。
3. **终端门**：基例 BSP `confirm_side`（nest.rs:479-481；查法同 p92 `terminal_bits_new`，
   `p92_nest_replay_postruling.rs:931-944`；`types.rs:216-221`）。
4. **包含门**：逐边 `is_sub(child_b, parent_b)` @B 口径（`interval_b`＝背驰段 C，裁定②
   `chanlun/escalate/nest-migration-ruling-20260716.md:14`）；同侧边筛选并入此门，
   方向不一致单列「方向门」。
5. **时钟门**：全通链的身份向量在 dump CERT/CKPT 任一钟点在场（因果见证），缺席＝幽灵/快照伪象
   （#103 口径，`p103-truncation-replay-doubletest-20260717.md:15-23`）。

### 1.3 硬锚（全部 PASS）

- `P109_ANCHOR_EVENTS candidates=3683 trend=521 pan=3162 divergence_confirmed=1235 trend_confirmed=429 pan_confirmed=806`
  —— 与 p92 postfix 回归逐字一致（`p92-postfix-regression-20260717.md:16`）。
- `P109_ANCHOR_CERT reproduced=66 dump=66 missing_in_repro=0 extra_in_repro=0`
  —— 终态快照双口径证书集与 dump CERT 66 行（dump:753-818）双向 diff=0，复现 p102 的 66/66。
- `P109_INVARIANT a_edge_violations=0`——匹配事件 `interval_a`==pair 区间时 A 边构造性成立
  （对账全程零违例）：**包含门的 30 次击杀全部是 B 口径相对 A 的收窄所致**。
- 冒烟锚：25 万 bar 复跑 reproduced=11 = dump 首检查点 A=7/B=4（dump:8,13；`/tmp/p109_smoke.txt`）。
- 复跑确定：全量二进制连跑两次，stdout 除 DETAIL 行序外逐字节一致；单遍耗时 2.64s real
  （batch 分类，非增量重放；`/tmp/p109_time.txt`）。

## 2. 链人口：157 → 5,092 → 6,482（版本归因）

| 口径 | L1 pairs | L2 | L3 | L4 | L5 | 完整链 |
|---|---:|---:|---:|---:|---:|---:|
| #83（2026-07-15，V1 tuple＋#105 前） | 827 | 212 | 38 | 10 | 5 | 157 |
| 本探针 V2（inherited-core＋#105 修复） | 2,302 | 509 | 134 | 42 | 8 | 5,092 |
| 本探针 V3（生产 CarriedOnly） | 2,473 | 552 | 143 | 42 | 8 | **6,482** |

- **#105 修复是主放大器**：V2-current 的 L1 moves=2,730 与 #83 逐字相同（D1/块结构未变），
  但 Completed 1,701→2,498——C 终段 `rev().find→find`（commit `1a377f68a6`）让约 800 个
  L1 趋势块完成（p104 口径 confirmed 1→429/521）。strict pair 与链随 Completed 同步放大。
- **V3 收复断点**：V2-current InvalidSeed=165+43+7+0+0=**215**，与 #83 的 215 逐字一致
  （`c2-yield-remeasure-20260715.md:75`）；V3 全部收复（invalid=0，每级单 run，
  `P109_LEVEL_V3 runs=1`），runs 合并后 pair 数再增（L1 +171）。
- 链丛形态：1,303 个不同 L1 pair 衍生 6,482 条链（均值 5.0，最丛 24 条/pair）；
  塔级 = 6（L1–L5），classification_levels=6。
- **L5 provider 覆盖为零**：`P109_LEVEL_V3 L5 ... strict_pairs=8 events=0`——即使链爬到
  L4→L5 边也必死身份门；各级 pair→事件命中率 L1 43.3%／L2 37.9%／L3 37.1%／L4 26.2%／L5 0%
  （`P109_MATCH`），provider 覆盖随级别单调塌陷，与 p105「L4 零样本」互为表里。

## 3. 各门机制证据与代表样本

### 3.1 身份门（3,790，58.47%）

- **Trend-leave 2,552（其中经背驰完成 2,552）**：leave 是趋势块且**经 D2 背驰对完成**
  （`P109_GATE_SUB identity leave_trend=2552 （其中经背驰完成=2552）`）——背驰真实存在，
  但 provider 的 Cand 门 `dir ∧ Comparable ∧ Extreme`（level_view.rs:567-572：Long 要求
  C 段创新低、Short 创新高）拒绝发事件。这是**已裁定定义**（#97 D1 宽候选），按教义
  无新极值即无一类点 → 记**定义口径/数据事实**，不算聋度。
- **Consolidation-leave 1,238**：盘整块的 `locate_pan_div_structure` 未定位或
  `pan_div_structure_extreme` 未过（level_view.rs:598-612）。盘背检测是否有结构漏检，
  本探针不展开逐案几何 → **待裁定（检测聋度候选）**。
- identity@L2 共 9 条（7 Trend-leave／2 Cons-leave）：链基例已过终端门，L2 pair 无事件。
  样本：`P109_CHAIN_DETAIL id=3213 L2 pair=(2077627, 2110244) leave=Trend/Up via_div=true`；
  `id=5406 L2 pair=(3610040, 3628392) leave=Consolidation`。
- L1 样本：chain=0 `L1=(699448,700968) leave=Consolidation`；chain=1
  `L1=(700164,706492) leave=Trend/Down via_div=true`（`/tmp/p109_full2.txt` P109_SAMPLE）。

### 3.2 终端门（1,631，25.16%）——聋度嫌疑最重的一扇

- 构造性事实：**Trend 基例不可能死背驰确认门**（趋势块完成 ⟺ 同一 `segments_diverge`
  通过，level_view.rs:864-877 vs 574-580；`P109_GATE_SUB divergence base_trend=0` 实证），
  但**可以死终端门**：`has_trend_divergent_base=754`——754 条链的 L1 基例趋势背驰已确认，
  同级同 source 却无 confirm_side BSP；`all_pan_bases=877`。
- `any_bsp_at_turn=62`（turn 上有 BSP 但方向不确认）vs `no_bsp_at_turn=1,569`：
  绝大多数是**该 source 上 BSP 账本根本无点**。
- 样本：chain=15 `Trend/Short turn=726571 interval_b=(726527,726571) any_bsp=false`；
  chain=66 `Trend/Long turn=808164`；chain=678 `Consolidation/Long turn=1219003 any_bsp=true`。
- 归因：877 条全 Pan 基例＝盘整背驰**按设计不产 B1/S1**（`NestDivergenceKind::Consolidation`
  「不冒充同级 B1/S1」，level_view.rs:461-466）→ 定义口径；754 条趋势基例＝D2-MACD 谓词
  与 BSP 生成器（`extract_first_third_for_level`）在 L1 的**谓词分歧**——塔侧审计已记
  「塔无背驰事件↔BSP 稳定身份边」（`nest-tower-native-gap-audit-20260717.md` §2.G1 偏差①）。
  是 BSP 生成器聋、还是 D2-MACD 谓词过宽 → **待裁定**（建议逐案对审，§6-1）。
- 全局旁证：p92 全宇宙 terminal_confirmed=24/1,235（1.9%）——终端门对全体候选同样苛刻，
  非链特有现象。

### 3.3 背驰确认门（1,020，15.74%）

- 全部 Pan 基例（`base_pan=1020`）：盘整背驰结构的 seg_a/seg_c 上 MACD 面积
  严格 `curr<prev` 未过。样本：chain=24 `turn=739707/739886 seg_a=(739416,739512)`；
  chain=37 `turn=761938 等 5 基例 seg_a=(761171,761333)`。逐案面积审计未做 → **待裁定**
  （聋度 or 市场事实；坐标已落 `/tmp/p109_full2.txt` P109_CHAIN_DETAIL）。

### 3.4 包含门（30，0.46%）——裁定② 口径的直接测量

- 30 条击杀全在 L1→L2 边；A 口径构造性全过（§1.3 不变量），故**全部是 B 口径收窄所致
  （口径产物）**。形态：子在父右（背了又背/回试段）17、子在父左（前 C 区）13
  （`P109_EDGE_SHAPE`），与 p102 机制分类一致。
- 与 p102 逐边互洽（`p102-b-long-zero-attribution-20260717.md:67-73`）：

| 基例 | p102 left/right gap | 本探针 P109_EDGE_GAP | 关系 |
|---|---|---|---|
| 706241 | +2719/−1883 | chain=2：+2719/−1883（parent_turn=704358） | **逐字一致** |
| 1588321 | +13870/−13690 | chain=1944-1946：同值（1574631） | **逐字一致** |
| 1834972 | +2786/−2215（A 吸收父 1832757） | chain=2638-2644：+2786/−1418（最近父 1833554） | 同一子、不同候选父，left_gap 一致 |
| 3745754 | −4458/+4831 | chain=5450：−4458/+4831（3750585） | **逐字一致** |

- 越界幅度 295–20,450 bar：最小 295（chain=2313-2318，parent 1743920，left_gap=−295，
  1m 约 5 小时的擦线），最大 20,450（chain=2310-2312）。多数是大位移判负，非边界擦线。
- 新发现（p102 未覆盖的 Short 侧）：chain=4788/4789（base 3047378，−12637/+12710）、
  chain=4868/4869（3096382，−8393/+8809）、chain=5408/5409（3625589，−3255/+3549）
  ——Short 侧同样以前 C 区形态被 B 门判负。

### 3.5 方向门（11，0.17%）与时钟门（0）

- 方向门：L2 pair 有事件但与基例异侧（样本 chain=2645-2648 base 1834972、chain=3215-3220
  base 2106003、chain=6476 base 4359660）——跨级背驰方向不一致，**数据事实**。
- 时钟门：零全通链，无从触发。50 条前缀（深度 1）证书全部在 dump 在场
  （`prefix_cert_in_dump=50 prefix_cert_absent=0`），无 #103 式幽灵/身份迁移个案；
  #103 幽灵（750000→1000000 的 3:725489→3:689893 迁移）涉链 base 706241 在本集仅 1 条
  （chain=2，死于包含门@L2），其前缀证书两口径均在场。

## 4. 与 66 张证书的交叉对账

- 50 条过终端门的链 → **11 个不同基例**，全部 ∈ dump B 口径 exec=1 基例集（21 个）：
  706241×1、1588321×6、1743555×9、1834972×11、2106003×8、3047378×4、3096382×2、
  3625589×4、3745754×2、4128368×2、4359660×1（`/tmp/p109_full2.txt` P109_CHAIN ids 字段）。
- 11 个基例的死法分布：inclusion 30（1588321×6、1743555×9、1834972×7、706241×1、
  3047378×2、3096382×2、3625589×2、3745754×1）／direction 11（1834972×4、2106003×6、
  4359660×1）／identity@2 9（2106003×2、3047378×2、3625589×2、3745754×1、4128368×2）。
- 其余 10 个 B exec=1 基例（147126、1655063、2116573、2168349、2254823、2560565、611277、
  664175、79510、915689）**不在任何全深度链上**——包括唯一 B 多级证书基例 147126
  （dump:806）：其结构链向上不到 L5，与 p105「全样本无 top≥5」一致。
- ⟹ 66 张证书与 6,482 条结构链的交集恰好是「深度 1 证书基例被链触及」的 11 个身份；
  结构链宇宙与证书宇宙的连接面只有 50 条链 × 11 个基例，且全部止步 L1→L2 边。

## 5. 对主线归因四分的判定

| 层 | 判定 | 占比 |
|---|---|---|
| 市场事实 | 方向门 11 条＋身份门 Trend-leave 2,552 条（Extreme＝教义新极值要求） | ~39.5% |
| 裁定口径 | 包含门 30 条（裁定② B）＋终端门全 Pan 877 条（盘整背驰不入 BSP）＋身份门含 Extreme 部分已左列 | ~14.0% |
| 检测聋度候选（**待裁定**） | 终端门趋势基例 754（谓词分歧）＋身份门 Cons-leave 1,238（pan 定位）＋背驰确认门 1,020（pan MACD） | ~46.5% |
| 装配工件 | 时钟门 0；身份桥缺（turn_source↔BSP source）已并入终端门待裁定项；幽灵 0 | ~0% |

- 「定义不苛刻、每个级别都该有对应证书」不成立：即便把待裁定项全部按有利方向裁定，
  当前定义下 6,482 条链仍有 ≥39.5% 死于教义本身（Extreme/方向），且 L5 provider 覆盖为 0
  使全深度证书在结构上不可达。
- 但「低产量＝市场事实」同样不成立：**最大单一击杀层是 provider 身份覆盖与 BSP 背书**，
  其中约一半（终端门趋势基例＋pan 定位＋pan 力度）是未经逐案审计的聋度候选；
  裁定② 的直接代价只占 0.46%（30 条）。

## 6. 复核建议（不执行裁决）

1. 终端门谓词分歧对审：754 条趋势基例逐案比 D2-MACD 确认 vs `extract_first_third_for_level`
   的 L1 BSP 产出，判定 BSP 生成器漏检还是 D2 谓词过宽（样本坐标已在 P109_CHAIN_DETAIL）。
2. 身份门 Cons-leave 1,236 条（@L1）逐案查 `locate_pan_div_structure` 失败原子；
   identity@L2 9 条同上。
3. 背驰确认门 1,020 条 pan 基例做 MACD 面积复核（`segments_diverge` 严格 curr<prev 的边界样本）。
4. L5 provider 零覆盖：8 个 L5 strict pair 无事件的原因（D2 pair 缺失 vs Extreme 未过）。
5. 若要把结构链升级为证书候选，需先裁定：(a) Cand(Extreme) 宽严；(b) 终端背书是否可用
   nest 自定义确认位替代 BSP 账本；(c) pan 基例的终端确认语义。在此之前本报告不停留在
   「结构链 ≈ 应发证书」的任何推论。

## 7. 复现与纪律

- 复跑：`cd rust && cargo run --release --bin p109_chain157_gate_attrition -- \
  ../analysis/data_cache/btc_1m_full.json /tmp/p92_ckpt_dump.txt`（batch 单遍约 3 秒；
  全量连跑两次除 DETAIL 行序外逐字节一致）。
- 生产源码零改动（divergence/bsp/signal/level_view/nest.rs 等只读）；新增文件仅
  `rust/src/bin/p109_chain157_gate_attrition.rs` 与本报告；未改 `rust/Cargo.toml`
  （bin 自动发现）；未做任何 git mutation；主仓 `/Users/silencehan/Projects/NewChanlun` 零写入
  （数据经 worktree 内 symlink 只读消费；dump 只读）。
- 声明与能力一致：本报告做终态快照逐门对账＋dump 因果见证核对；未做 prefix 首证钟重放
  （时钟门以 dump 18 检查点见证裁决，零全通链故无未覆盖对象）、未做逐案 MACD 面积/结构几何
  审计（列为待裁定并给出坐标）、不声明任何择时 alpha。
