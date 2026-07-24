# #172/T3 调研报告：严格链判定 shadow 双读全量 dump（阶段 B：四条件终判）

- 日期：2026-07-23
- 票据：issue #172（T3 严格链判定；parent map #106）；承接 #168 切分 ADR 裁定 4/5（`adr-chain-execution-split-emc-20260723.md`）、#163 裁定 1/3、#164 全量确证（同 corpus 对照面）。
- 性质：全量测量（非抽样推断）。阶段 A（代码实装 + 门开形态 shadow 双读重放，三窗 p3fold/wf7/wf8）已完成；本报告 = 阶段 B：dump 全量分析 + shadow 四条件终判。主仓零写入、git 零 mutation；rust 源码零改动（阶段 B 只读 `/tmp/kimi-nest-mainline` 源码做语义核对，零编辑）。
- 090 纪律：确证与未确证分项标注；不用统计估计；报告内全部数字可由交付脚本对 dump 重算复现（60 项断言全过）；与阶段 A 回报不符之处以 dump 重算为准并注明。方向见证分歧与换锚恢复=0 只呈现数据与机制解读，**不做裁定性结论**（编排者拍板）。

---

## 0. 总结论（四条件逐项，ADR 裁定 4「四条件全满足才开 T4」）

| # | 条件 | 判定 | 关键数字（三窗 p3fold/wf7/wf8） | 证据 |
|---|---|---|---|---|
| ① | 差异 100% 落入预期归因分类，零未解释 | **✓** | 全量 4875 候选逐行分类：迁移 186（94/65/27）= 断环拒 84（48/22/14）+ 缺环拒 38（16/17/5）+ 链顶谱系差 64（链下NoChain 32：17/12/3；链拒自回退出 32：13/14/5）+ 换锚恢复 **0**；unchanged 4689 行 admit/channel 双一致；**未分类 0、Xzd 族内漂移 0**；候选 admit 差逐候选合计 == admitted Δ（-76/-51/-22）平账 | §3、jsonl `migration` 字段、脚本断言全过 |
| ② | 链外一切路径与 T3 前逐字节一致 | **✓（实质判读；票面字面未满足处如实注明）** | 不变面全项一致（阶段 A：STATS 3 项、CHAIN 4 项、INDEX 13 项、tower MD5 三窗）；阶段 B 补强：dump 聚合 ↔ stderr 聚合全等（含 base 重构列对 baseline 的通道计数/typed_found/reuse/fallback 逐项精确）；Xzd 族内子通道**零漂移**（机制逐字节不变的直接证据）；Xzd 子通道计数 Δ∈[-2,+5] 全部逐候选归因（迁入=链下NoChain、迁出=链拒自回退出）。**票面「逐项一致」字面未被 Xzd 计数 Δ 满足**（阶段 A 简报口径 ≤±7；dump 实测 max\|Δ\|=5），按「机制不变 + 人口迁移 100% 归因」框架交编排者定夺 | §1/§3、脚本断言全过 |
| ③ | 方向守卫：链确认总数显著高于并集 17（1.03% 基线） | **✓（wf7 结算面；人口构成留档待裁）** | wf7 链确认 **39** vs 并集基线 **17** = **2.29×**（>1，未低于基线）。**人口构成（阶段 B 新数，只呈现不裁定）**：39 全部来自 single 桥命中人口（73）——39⊆single-73；并集 multi-only 的 17 个恢复在链下**零再确认**（12 no_chain + 5 reject）；#164 typed_none（n=1651）内严格链**新增恢复 0**。p3fold/wf8 只报告不设判：31/36（#164 无该两窗基线） | §5、脚本断言全过 |
| ④ | 汇总统计落报告（裁定 5，#164 同构三件套） | **✓** | 本报告 + 逐候选 jsonl（4875 行）+ 分析脚本（60 项断言全过） | 附录 A/B |

**链确认 vs 并集基线（wf7 正式结算面）**：严格链确认 39/1724 = 2.26%；并集时代 nest_pass 90（single 73 + multi 17）→ 链下 39（保留 39、缺环拒 17、断环拒 22、链下NoChain 12）。**换锚恢复 = 0（三窗一致）**：链确认集 ⊆ 并集确认集（39/39，逐候选），且 ⊆ 旧臂 nest_pass（cross `old_rej_new_pass` 15→0 逐候选版复核一致）。

---

## 1. 方法 + corpus 同一性证据

### 1.1 方法

阶段 A 已在 T3 门开形态下三窗重放 m8 E2E（`THETA_NEST_CERT_GATE=1`，`M8_WIN_FILTER=<tag>`），逐候选落 shadow 双读 dump：`{bar,level,source_index,dir,price, chain{verdict,top,closed_down_to,first_gap,levels[…]}, union{merged_pass,merged_rungs,hit_levels}, single, old_arm, xzd, base{admit,channel}, admit,channel}`。`base` = 并集时代判定重构列（`#[cfg(test)]` 装置，`t3_union_era_decision`：并集 merged 判定 + 既有 Xzd 回退三分支逐字重放）。阶段 B（本报告）对三窗 dump 全量 4875 行做：corpus 同一性核对 → dump 聚合 ↔ stderr 聚合对账 → 逐候选迁移归因（条件①）→ 闭合恒等式/NEST_GATE_T3 行重算 → 链×并集象限与分层 → 链谱系分布 → #164 三层对照与 nowhere 复核 → 方向见证 cohort。全部断言 60 项，0 失败（脚本退出码 0，输出留档 `/tmp/t3_evidence/phaseB_analyze.out`）。

### 1.2 corpus 同一性（wf7 ↔ #164；p3fold/wf8 ↔ T3 基线候选流）

| 证据 | 值 | 判定 |
|---|---|---|
| wf7 dump 行数 vs #164 candidates 行数 | 1724 vs 1724 | ✓ 精确 |
| wf7 ↔ #164 候选键 (bar,level,source_index,dir) **有序逐行一致** | 1724/1724 行，0 处不一致 | ✓（含 59 对重复键，见下） |
| wf7 ↔ #164 **single 桥读出逐位一致**（T3 `single` vs #164 `single_found`，1724 行） | 0 行不一致 | ✓（同一候选流 + 同一桥读的强证据） |
| wf7 tower_events MD5 vs #164 源（`/tmp/v4_C/wf7`） | 70073e449f10… 全等 | ✓ 逐字节一致 |
| 三窗 dump 行数 vs STATS.total（baseline/after） | 1633/1724/1518 全等 | ✓ |
| 三窗 tower_events baseline vs after MD5 | cfa80dee…/70073e44…/7119ffdb… 各自全等 | ✓ 逐字节一致（阶段 A 证据复算一致） |
| p3fold/wf8 逐键对照件 | **不存在**（T3 基线无 per-candidate dump；#164 只跑 wf7） | 候选流同一性 = 聚合不变面（STATS/CHAIN/INDEX 全项）+ 候选生成确定性，如实注明弱于 wf7 的逐行对照 |

**corpus 特征落账（重复键）**：键四元组非唯一——三窗各有 64/59/66 对 L0 同脚双发候选（同 bar/level/source_index/dir，两候选 `bits` 不同；dump 键不含 bits）。全部双发对同价、同链裁决；wf7 有 1 对、p3fold 有 1 对通道不同（cert_none vs xzd_pass），经核均为 unchanged 行且 base 已异——bits 差异在并集时代即导致不同 Xzd 读出，非 T3 迁移。因此本报告一切 T3↔#164 对照按**有序逐行（位置）**对齐。

### 1.3 dump 聚合 ↔ stderr 聚合（base 重构列保真度）

三窗逐项精确（脚本逐项断言）：after 通道计数 == after STATS（nest_pass/xzd_pass/xzd_gate_fail/cert_none/nest_n_delta_false/admitted/rejected）；**base 重构通道计数 == baseline STATS 全等**；`union.merged_pass=True` 数 == typed_found（112/90/58）；并集时代 reuse/fallback 可由 dump 列重构 == baseline CHAIN（reuse 351/512/315、fallback 1125/1099/1115）；after reuse/fallback == after CHAIN（343/502/312、1137/1107/1116）。base 列无独立 per-candidate 外部参照（T3 基线无 dump），其保真度 = 上述聚合全等 + 重构路径与基线决策逐字同码（§6 限制 3）。

---

## 2. 链 × 并集四象限裁决矩阵 + 按候选级分层

### 2.1 四象限（链裁决 × 并集 merged；并集只有 pass/none 两态——merged=false 三窗零发生，与并集时代 nest_n_delta_false=0 一致）

| 窗 | pass×并集pass | pass×并集none | reject×并集pass | reject×并集none | no_chain×并集pass | no_chain×并集none |
|---|---|---|---|---|---|---|
| p3fold (1633) | **31** | **0** | 64 | 13 | 17 | 1508 |
| wf7 (1724) | **39** | **0** | 39 | 14 | 12 | 1620 |
| wf8 (1518) | **36** | **0** | 19 | 5 | 3 | 1455 |

三窗「pass × 并集none」全为 0 ⟹ **换锚恢复 = 0**（链确认而并集 miss 的候选不存在；链确认集 ⊆ 并集确认集，逐候选确证）。

### 2.2 按候选级分层（表式沿用 #164 §3）

wf7（n=1724）：

| 候选级 | n | 链pass | 链reject | 链no_chain | 并集pass | 链pass∩并集pass |
|---|---|---|---|---|---|---|
| L0 | 1116 | **39** | 16 | 1061 | 67 | 39 |
| L1 | 275 | 0 | 26 | 249 | 18 | 0 |
| L2 | 178 | 0 | 9 | 169 | 5 | 0 |
| L3 | 73 | 0 | 2 | 71 | 0 | 0 |
| L4 | 82 | 0 | 0 | 82 | 0 | 0 |

p3fold（n=1633）：L0 1118（pass 31/reject 19/nochain 1068/并集 67）；L1 270（0/28/242/22）；L2 200（0/28/172/23）；L3 45（0/2/43/0）。
wf8（n=1518）：L0 1073（36/5/1032/44）；L1 297（0/16/281/13）；L2 65（0/3/62/1）；L3 83（0/0/83/0）。

**三窗一致的结构事实：链确认全部落在 L0 候选**（31/39/36 = L0 行的 pass 数；L1–L4 候选零确认），且与 §4 的「pass 全部链顶=0」互为印证。L3/L4 候选在链下同样结构性无确认（#164 的「塔顶无更深结构」论断在链语义下依然成立）。

---

## 3. 差异归因分类账（条件①：全量 4875 行，零未解释）

### 3.1 四类计数（base=并集时代重构 → after=链时代；谱系差按机制拆两亚型）

| 分类 | p3fold | wf7 | wf8 | 合计 | 机制 |
|---|---|---|---|---|---|
| 断环拒 | 48 | 22 | 14 | **84** | base nest_pass → nest_n_delta_false；链 reject 且首 gap=broken（高级有证而中间断） |
| 缺环拒 | 16 | 17 | 5 | **38** | base nest_pass → nest_n_delta_false；链 reject 且首 gap=missing（上方无闭合） |
| 链顶谱系差·链下NoChain | 17 | 12 | 3 | **32** | base nest_pass → Xzd 族；链 no_chain（并集键域有证而链键域 (方向,极值价,组锚) 区间零闭合） |
| 链顶谱系差·链拒自回退出 | 13 | 14 | 5 | **32** | base Xzd 族 → nest_n_delta_false；并集无证而链区间有闭合但缺/断（首因全为 missing，32/32 确证） |
| 换锚恢复 | 0 | 0 | 0 | **0** | base Xzd 族 → nest_pass（链确认而并集 miss）——三窗零发生 |
| unchanged | 1539 | 1659 | 1491 | **4689** | 通道相同且 admit 相同（逐行双一致断言过） |
| **未分类 / Xzd 族内漂移** | **0** | **0** | **0** | **0** | 任何落不了类的候选清单 = 空（条件①字面满足） |

对账：迁移 94/65/27 候选的 admit 差逐候选合计 == admitted Δ（-76/-51/-22，平账断言过）；reject 首因 missing/broken 计数 == NEST_GATE_T3 行（missing 29/31/10、broken 48/22/14）；断环拒计数 == broken 首因总数（自回退出全部 missing 首因 ⟹ broken 拒全部来自并集已确认人口）。逐候选清单 = 交付 jsonl 的 `migration` 字段（4875 行逐行标注，含 unchanged）。

### 3.2 代表例（wf7）

**教科书级断环例**（bar=138266，L2 候选，source_index=138199，Long）：

- 链谱系：链顶=2（脚存在最高账本级 L2）；L2 级 `closed`（键域 (事件级3, Long, 2716437000000, a*) 有 1 证、过因果守卫、n_delta 过）；L1/L0 级 `missing_existence`（恰好存在断裂）→ 首 gap=L1 **broken**（上方 L2 有闭合）；closed_down_to=2。
- 并集 shadow：`merged_pass=true`，hit_levels=[3]（端点精确匹配在事件级 3 有证），`single=true`——旧单桥/并集凭「L3 事件级一张过证」即准（base nest_pass）。
- 链判定：严格链要求 [L0, 链顶] 全链闭合到 L0，中间两级存在性断裂 ⟹ **断环即拒**（`nest_n_delta_false`）。并集「单点有证即准」与严格链「全链闭合才准」的语义差在此例完全显现。

**缺环拒例**（bar=15231，L0 候选，source_index=15175，Long）：链顶=1；L0 `closed`（1 证过）；L1 `missing_cert`（键域查无身份）→ 首 gap=L1 **missing**（上方无闭合）→ 缺环拒。并集 merged_pass=true（hit_levels=[1]，单桥同级命中）；链下该脚在 L1 的登记存在但无证书，全链不闭合。

**链下NoChain 例**（bar=16252，L0，Short，谱系差）：并集 merged_pass=true（hit_levels=[2]）而链区间 [L0,L2] 三级全缺（L0/L2 missing_cert、L1 missing_existence）→ 零闭合 → no_chain → Xzd 回退（xzd_pass）。换锚后该脚在链键域无证，而并集端点键有证——锚谱系差异的直接实例。

**链拒自回退出例**（bar=15231，L1 候选，source_index=15175，Long）：并集无任何证（base xzd_gate_fail）；链 L0 `closed`、L1 `missing_cert` → 有闭合但有缺 → reject（`nest_n_delta_false`），候选退出 Xzd 回退人口入硬拒。

---

## 4. 链谱系分布（「链即身份」首次实证，map #106 Destination 验收面）

### 4.1 链顶级别分布（top_dist，含 None=锚不可解）

| 窗 | top=0 | top=1 | top=2 | top=3 | top=4 | None |
|---|---|---|---|---|---|---|
| p3fold | 974 | 287 | 256 | 71 | — | 45 |
| wf7 | 976 | 310 | 223 | 64 | 128 | 23 |
| wf8 | 975 | 326 | 73 | 114 | — | 30 |

（与 NEST_GATE_T3 行逐项一致，dump 重算断言过；None = x 处无 confirmed 分型 / 本级层未载 / 脚未登记，全部 no_chain。）

### 4.2 闭合完整度（逐候选恒等式全部成立，三窗）

- **pass ⟹ 链顶=0 且 closed_down_to=0**：三窗 31/39/36 个确认**无一例外是单级链**（脚只在本向 L0 层存在，区间 [L0,L0] 单级闭合）。**没有任何候选的多级链（top≥1）全闭合并联到 L0**——多级链全部止步于 reject（有闭合但有缺/断）或 no_chain（零闭合）。
- reject ⟹ closed_down_to：首因 missing ⟺ None（链顶自身未闭合；三窗 29/31/10 例，合计 70 = 缺环拒 38 + 自回退出 32）；首因 broken ⟺ Some(k≥1)（自链顶连续闭合到 k，p3fold k=1:20/k=2:28，wf7 17/5，wf8 13/1）。`closed_down_to is None ⟺ first_gap.kind==missing` 对全部 reject 逐候选成立。
- no_chain ⟹ closed_down_to 全 None（零闭合级，结构性恒等，4875 行验证）。
- `len(levels)==top+1`、`pass ⟹ first_gap 无`：逐候选成立。
- reject 闭合级数分布：p3fold {1级:68, 2级:9}；wf7 {1:50, 2:3}；wf8 {1:24}——reject 以「仅 1 级闭合」为绝对主体。
- 谱系底质（链级条目 2600/3161/2302）：closed 117/95/60；missing_cert 1814/2014/1655；missing_existence 662/1046/577；missing_causal 7/6/10；**Broken 底质 0 发生**——「有因果干净证书但合并 n_delta 假」三窗零实例，与源码注记的「现装配只产过证、断底质结构性不可达」一致（确证）；「断环」全部来自位置极性（上方有闭合），不来自断底质。

### 4.3 「链即身份」实证读法

链顶账本级 = 脚的级别身份（恰好存在给出，非候选自报）：确认人口的级别身份全部为 L0（top=0）；top≥1 的身份存在性普遍存在（wf7 top≥1 共 725 例）但全链闭合率为 0——级别身份的「存在」与「闭合」在全量 corpus 上彻底分离。这是 map #106 Destination「链即身份」的首次全量实证落账：身份由链谱系给出（本报告 §4 全部数字），而闭合（身份的确证）在三窗只发生在 L0 单级链。

---

## 5. 与 #164 三层对照（wf7 = #164 同 corpus 正式结算面）

| 层 | 值 | 口径 |
|---|---|---|
| 代理上界（#164，区间包含代理） | 423/1651 = 25.62% | 「非桥但深」；若值桥放松为区间包含时的 multi 增益上界 |
| 并集实恢（#164，multi 真链命中） | 17/1651 = 1.03% | typed_none 内 single 之外的实际恢复（73→90） |
| **严格链（T3）** | **确认总量 39/1724 = 2.26%；typed_none（n=1651，逐位对读）内新增恢复 0** | 链全闭合到 L0 才准 |

**与并集基线的方向守卫（条件③）**：39 vs 17 = **2.29×** > 1，字面满足「显著高于并集 17」。**人口构成（阶段 B 新数，090 只呈现不裁定）**：

- 并集时代 nest_pass 90 = single 命中 73 + multi-only 17。链确认 39 **全部**来自 single 人口（single 73 → pass 39 / reject 34）；multi-only 17 在链下 **0 再确认**（12 no_chain + 5 reject）。p3fold/wf8 同型：single 82/53 → pass 31/36；multi-only 30/5 → 0 再确认（17/3 no_chain + 13/2 reject）。
- 即：2.29× 是两个**不相交人口**的计数比——链确认是旧单桥人口的严格子集（39⊆73），并非并集 multi 恢复人口（17）的再确认或放大。#164 的 17 个 multi 恢复全部命中在非桥级（hit_levels L2=8/L3=9 端点匹配），换锚 + 全链闭合双重要求下零幸存。

**换锚恢复 = 0 的解读（机制描述，非裁定）**：换锚收益侧（并集 miss 而链确认）三窗为零——三锚（方向, 极值价, 组锚）键域上没有出现旧端点键域之外的可闭合链；严格度代价侧（并集已确认被链剪掉）= 51（缺环拒 17 + 断环拒 22 + 链下NoChain 12，wf7），三窗 76/51/22。净效应：nest 通道确认数 112/90/58 → 31/39/36。换锚是否「值回」严格度代价，属 #163 裁定 1 已决语义（严格链即判定源），本报告只落账不重议。

**真 nowhere 100 个（#164 §4.3 平铺口径）链下状态复核**：以 #107/#164 同一代理（new_center 初始区间）对逐字节一致的 tower_events 重算，typed_none 内平铺 nowhere 恰为 100（与 #164 全等，逐位）。链下：**100/100 全部 no_chain**（零确认、零 reject——链区间同样零闭合），通道落 Xzd 回退：xzd_pass 51 / xzd_gate_fail 42 / cert_none 7。#164 的「结构真空」在链语义下同样是真空，两独立机制（区间包含代理 × 三锚链键域）交叉确证该 100 个候选无任何可攀附结构。

---

## 5.5 方向见证分歧 cohort（T2 resolution 遗留；只记录不裁定）

dump `chain.levels[].dir_witness`：同脚同价的 T1 键侧（`event.side`）与 T2 键侧（`bits.confirm_side` 注册）一致性见证。`layer_only_opposite` = 层仅在异向键下有该脚存在；`certs_only_opposite` = 层本向有该脚但证书仅在异向键域。

| 窗 | 分歧候选 | 占比 | 类型构成（layer_only / certs_only / 兼） | 分歧内链裁决 | 分歧内确认率 | 非分歧确认率 |
|---|---|---|---|---|---|---|
| p3fold | 130 | 8.0% | 76 / 51 / 3 | no_chain 122 + reject 8 | **0.00%** | 2.06%（31 个 pass 全部非分歧） |
| wf7 | 229 | 13.3% | 103 / 88 / 38 | no_chain 220 + reject 9 | **0.00%** | 2.61%（39 全） |
| wf8 | 110 | 7.2% | 56 / 48 / 6 | no_chain 102 + reject 8 | **0.00%** | 2.56%（36 全） |

- **规模**：三窗合计 **469** 例（与 NEST_GATE_T3 行 dir_witness_divergence 逐项一致）。
- **对链确认的压制量**：分歧候选中链拒/NoChain 占比 **100%**（469/469；0 个 pass）。机制通路（描述）：方向分歧在该级制造 missing_existence/missing_cert（本向查无），分歧级占分歧候选缺口级的 43.5%/36.5%/41.1%（其余缺口与分歧无关）。
- **留档待裁声明**：该分歧是 T1/T2 两侧对同脚同价的注册方向不一致（T2 resolution 遗留），**未裁定**；其对确认的压制是否「应然」（分歧脚本不该确认）或「误伤」（注册口径缺陷埋掉真链），本报告只呈现数据，交编排者裁定。分歧候选的链下完整谱系在交付 jsonl 逐行可查。

---

## 6. 限制声明（090）

1. **门关红线未在 m8 规模单独重放**（阶段 A 遗留）：本报告全部数字为门开形态；门关语义（本运行不消费链判定，与生产形态逐字节一致）未经本报告验证。
2. **exit 侧未在 m8 路径**（阶段 A 遗留）：四份 stderr 日志 NEST_GATE_EXIT 行合计 0（grep 确证）——exit 侧在 m8 路径无聚合行可逐项对照。阶段 A 简报所列「exit 侧逐项一致（三窗 ✓）」**无逐项对照件支撑**（090 如实注明）；exit 时序一致性实际证据 = trades 前缀逐字节一致（8/8/57 笔，本报告复算一致）+ tower_events 逐字节一致。trades 笔数变化（165→156 / 213→206 / 175→172）为变化面，由候选迁移（§3）经门后人口变化解释。
3. **base 列为重构列**（`#[cfg(test)]` 装置）：其保真度证据为聚合全等对账（§1.3）+ 与基线决策同码路径，**非**逐候选外部参照（T3 基线无 per-candidate dump）。若有逐候选级偏差而聚合抵消，本方法不可见——评估为低危（重构只读 multi/旧臂/Xzd 三既有读出，无新判定），如实声明。
4. **p3fold/wf8 无逐键对照件**（§1.2）：候选流同一性证据弱于 wf7（聚合不变面 + 确定性，非逐行）。
5. **方向见证分歧（§5.5）与换锚恢复=0（§5）**：本报告只呈现数据与机制描述，裁定权属编排者；条件③的 2.29× 与「multi-only 17 零再确认」并列表述，不以前者遮盖后者。
6. **#164 ±2 翻转括弧沿袭**：typed_none 分母用 #164 jsonl 实数 1651（v4_C 口径 1653 的 ±2 翻转不可个体识别，任何比例变化 ≤0.12pp，#164 §6 同款括弧）。
7. 阶段 A 简报称「Xzd 子通道计数 ≤±7 迁移」：dump 实测各子通道 Δ∈[-2,+5]（xzd_pass +5/0/0、xzd_gate_fail +1/-2/-1、cert_none -2/0/-1），在简报口径内但更紧；以 dump 重算为准。

---

## 附录 A：产物路径

- 本报告：`chanlun/review-results/typed-none-strict-chain-dump-20260723.md`（worktree `/tmp/kimi-nest-mainline`）
- 逐候选合并 dump（4875 行 = 1633+1724+1518，每行加 `window`/`migration` 字段，原列不动）：`chanlun/review-results/typed-none-strict-chain-dump-candidates-20260723.jsonl`
- 分析脚本（60 项断言全过，退出码 0）：`chanlun/review-results/typed-none-strict-chain-analyze-20260723.py`；运行输出留档 `/tmp/t3_evidence/phaseB_analyze.out`
- 阶段 A 输入件：三窗双读 dump `/tmp/t3_shadow_dump_{p3fold,wf7,wf8}.jsonl`；红线对照 `/tmp/t3_evidence/redline_report.md`（+`/tmp/t3_evidence/analyze.py`）；baseline/after 运行件 `/tmp/t3_baseline/<tag>/`、`/tmp/t3_after/<tag>/`（stderr.log + opsem/{trades,tower_events}.jsonl）
- #164 对照件：`chanlun/review-results/typed-none-joint-dump-20260722.md` / `-candidates-20260722.jsonl` / `-analyze-20260722.py`
- 切分 ADR：`chanlun/escalate/adr-chain-execution-split-emc-20260723.md`（裁定 4 四条件、裁定 5 报告形态）

## 附录 B：运行命令与改动清单

阶段 A 重放命令（引阶段 A 证据，本报告未重跑）：

```
THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=<dir> M8_WIN_FILTER=<tag> T3_SHADOW_DUMP_DIR=<dir> \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

阶段 A 改动清单（引阶段 A 回报；阶段 B 作者未独立 diff 改动面，只对 `admission.rs` 做语义核对零编辑）：`rust/src/theta_v0/backtest/admission.rs`（T3 严格链判定 `chain_lookup` 为唯一 nest 判定源、并集/single/旧臂降 shadow、`t3_union_era_decision` base 重构、`t3_shadow_dump_record` dump 装置，均 `#[cfg(test)]` 限定）；`rust/src/theta_v0/backtest/wverify_run.rs`（m8 窗过滤与 dump env 接线）；NEST_GATE_T3 聚合行新增。

阶段 B 改动清单：**零代码改动、零 git mutation**。仅新增本附录 A 所列三件报告产物（md/jsonl/py，均在 worktree `chanlun/review-results/`）与 `/tmp/t3_evidence/phaseB_analyze.out` 留档。

阶段 B 复核运行：

```
python3 chanlun/review-results/typed-none-strict-chain-analyze-20260723.py   # 60 项断言，退出码 0
```
