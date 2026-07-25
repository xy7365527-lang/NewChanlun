# #207/T5a 调研报告：方向退役后链判定全量 dump（阶段 B：新旧链逐候选对照 + 四条件重新结算）

- 日期：2026-07-24
- 票据：issue #207（T5a 方向退役 + shadow 重跑）；承接三裁 ADR（`adr-chain-identity-colocation-and-direction-retirement-20260723.md`，裁定 1「身份判据=同点递归；方向整体退役出身份层」）、T3 报告（`typed-none-strict-chain-dump-20260723.md`，旧链=方向过滤语义基线）、#164 全量确证（同 corpus 对照面）。
- 性质：全量测量（非抽样推断）。阶段 A（代码实装 + 三窗 p3fold/wf7/wf8 重放 + 红线对照）已完成；本报告 = 阶段 B：新链 dump（4875 行）× 旧链基线 dump（4875 行）逐候选对照分析 + 四条件按 T5a 语境重新结算。主仓零写入、git 零 mutation、rust 源码零改动（阶段 B 只读数据与阶段 A 证据，零编辑）。
- 090 纪律：确证与未确证分项标注；不用统计估计；报告内全部 dump 数字可由交付脚本对两个合并 dump 重算复现（114 项断言全过，退出码 0）；与阶段 A 回报不符之处以 dump 重算为准并注明（本次结算未发现不符，见 §6 限制 1）。A 类回归实例与链顶延伸 cohort 只呈现数据与机制描述，**不做裁定性结论**（编排者拍板）。

---

## 0. 总结论（四条件逐项，按 T5a 语境重述）

| # | 条件 | 判定 | 关键数字（三窗 p3fold/wf7/wf8） | 证据 |
|---|---|---|---|---|
| ① | 迁移 100% 归因零未解释 | **✓** | 全量 4875 候选逐行分类：**方向自由化恢复 3**（1/1/1）+ **链顶延伸致拒 17**（7/7/3）+ **存在性解放但闭合仍缺 82**（34/41/7）+ unchanged 4773（1591/1675/1507）；**未分类 0**；逆向迁移（reject→no_chain / pass→no_chain）三窗 0；每类机制断言逐个成立（恢复 3 例旧 dump 均带方向压制见证；延伸 17 例全部旧 top=0→新 top≥1 且首缺即新顶、L0 仍闭合；解放 82 例全部旧零闭合→新 ≥1 闭合）；admit 翻转 42/49/11 全部伴随裁决迁移、净差 -23/-33/-7 与阶段 A after STATS.admitted Δ 平账 | §3、脚本 §2/§3 断言 |
| ② | 链外机制逐字节一致 | **✓（引阶段 A 证据，本报告复跑确认）** | 阶段 A 红线全项一致：候选流（逐行键对齐 1633/1724/1518 + STATS.total/flat_dir/no_level）、INDEX 内容 13 项、tower_events MD5×3、NEST_GATE_EXIT 零行、exit 侧无聚合行可差；本报告复跑 `redline_compare.py` 退出码 0。阶段 B 补强：price 逐行一致 4875/4875、逐级 certs/clean 单调不减 + 底质格序不降（反单调 0）、探针逐字节不变候选（1431/1382/1345）admit/channel 双一致。**足迹列账（预期差异）**：INDEX.index_builds 186→279（+93）/ 234→344（+110）/ 140→217（+77）——链查询 hint 触发的构建次数增加，构建产物（INDEX 内容列）逐项不变 | §1、§6 限制 2；阶段 A `/tmp/t5a_evidence/redline_report.md` |
| ③ | 方向守卫：对 #164 并集基线 17（wf7） | **✓（结算面字面成立；对 T3 链净降照实落账）** | wf7 新链确认 **33** > 并集基线 **17**（**1.94×**）成立。对 T3 链 39 **净降 -6**（+1 方向自由化恢复 − 7 链顶延伸致拒），p3fold 31→25、wf8 36→34 同型（各 +1 恢复）；**本报告不使用「上升」预期措辞**。人口构成：33 ⊆ single-73（single 73 → pass 33 / reject 40）；multi-only 17 → **0 再确认**（11 no_chain + 6 reject）；typed_none（n=1651）内新增恢复 0 | §5、脚本 §5 断言 |
| ④ | 汇总落报告（新三件套） | **✓** | 本报告 + 新链合并 dump（4875 行）+ 分析脚本（114 项断言全过，退出码 0） | 附录 A/B |

**阶段 A 已确证数字经 dump 复算全部一致**：裁决迁移矩阵（no_chain→reject 34/41/7；pass→reject 7/7/3；reject→pass 1/1/0；no_chain→pass 0/0/1）、chain_pass 31→25 / 39→33 / 36→34（净 -6/-6/-2）、探针机制计数（存在性解放 L0 66/85/53、L1 16/44/9 等 10–14 项/窗）、trades 156→149 / 206→196 / 172→169 及逐笔归因——逐项复算一致（§3/§4 断言）。

**阶段 B 新数（阶段 A 未报，090 照实）**：

1. **多级全链闭合首次非零**：新链 pass 92 中 **4 例 top=1**（p3fold 2 / wf7 2 / wf8 0）——T3 时代 106 个确认全为 top=0 单级链；4 例 = 2 个同点跨型候选对（§4.2）。
2. **链确认突破 L0 候选层**：2 例 L1 候选 pass（p3fold bar=190306、wf7 bar=122626，即候选对中的 Long 侧）。
3. **并集包含性出现 1 例例外**（wf8 bar=23249 Short，xzd_pass→nest_pass）：T3 时代「链确认集 ⊆ 并集确认集」三窗无例外，新链下 91/92 ⊆ 并集、wf8 1 例在并集外新确认（§5）。
4. **旧方向分歧 cohort 469 例去向**：方向压制解除后仅 3 例转 pass（= A 类回归全部），确认率 0.64%——供给约束（ADR 裁定 2 / map #126）在压制解除后仍是主约束（§5.5）。

---

## 1. 方法 + 对齐验证

### 1.1 方法

阶段 A 已在去方向实装（T1 键域 / T2 层索引 / T3 链同向过滤去方向分量，身份锚简化为两元（极值价, 合并组锚)）下三窗重放 m8 E2E，逐候选落新链 dump：`{bar,level,source_index,dir,price,anchor, chain{verdict,top,closed_down_to,first_gap,levels[…]}, admit,channel, window}`（levels 无 `dir_witness`——方向见证装置按裁定 1 退役为非概念；无 union/single/old_arm/xzd/base shadow 列）。阶段 B（本报告）以 T3 阶段 B 交付的 4875 行旧链 dump 为基线，做：corpus 对齐验证 → 裁决迁移矩阵 + admit/channel 平账 → 条件① 四类归因（逐类机制断言）→ 单调性/探针复算 → 链谱系新旧对照 → #164 三层对照重述 → 链顶延伸 cohort + 方向分歧 cohort 去向。全部断言 114 项，0 失败（脚本退出码 0，输出留档 `/tmp/t5b_analyze.out`）。

### 1.2 对齐验证（条件①前置）

| 证据 | 值 | 判定 |
|---|---|---|
| 新/旧 dump 行数 | 4875 vs 4875（分窗 1633/1724/1518 各自相等） | ✓ |
| 候选流逐行键对齐（bar,level,source_index,dir） | 1633/1633、1724/1724、1518/1518 | ✓（阶段 A 已核，本报告复算一致） |
| price 逐行一致 | 4875/4875 | ✓（供给线未动的直接证据） |
| 键四元组唯一性 | 非唯一：64/59/66 对重复键（L0 同脚双发，T3 §1.2 同数） | 一切新旧对照按**窗内行序（位置）**对齐 |
| 新链 top=None（锚不可解/存在性全无） | 36/17/30（旧 45/23/30；None→Some 9/6/0，Some→None 0） | ✓ 无锚丢失 |
| anchor 字段形态 | anchor is None ⟺ top is None（4875 行）；已解行 anchor == source_index（4792/4792） | ✓（本 corpus 合并组锚=组内首根序号=脚 x） |
| wf7 ↔ #164 候选键逐行对齐 | 1724/1724（沿 T3 §1.2 结论复算） | ✓（§5 结算面前提） |

corpus 同一性的聚合级证据（候选流 STATS、INDEX、tower MD5）由阶段 A 红线逐项给出，本报告复跑 `redline_compare.py` 退出码 0 确认（§6 限制 2）。

---

## 2. 新旧链裁决矩阵 + 按候选级分层

### 2.1 裁决迁移矩阵（旧裁决 × 新裁决；== 阶段 A 逐窗数字）

| 窗 | no_chain→no_chain | pass→pass | reject→reject | no_chain→reject | pass→reject | reject→pass | no_chain→pass |
|---|---|---|---|---|---|---|---|
| p3fold (1633) | 1491 | 24 | 76 | **34** | **7** | **1** | 0 |
| wf7 (1724) | 1591 | 32 | 52 | **41** | **7** | **1** | 0 |
| wf8 (1518) | 1450 | 33 | 24 | **7** | **3** | 0 | **1** |

逆向迁移（reject→no_chain、pass→no_chain）三窗 0——方向退役只增证据不撤证据（单调性断言 §3.4），不存在「旧有链而新无链」的候选。

chain_pass：31→**25** / 39→**33** / 36→**34**（净 -6/-6/-2）。admit/channel 翻转 42/49/11 例全部伴随裁决迁移（同裁决翻转 0），净差 -23/-33/-7 与阶段 A after STATS.admitted Δ 逐窗平账；翻转分类（裁决迁移 × 通道迁移）与阶段 A 逐项一致（脚本断言）。

### 2.2 按候选级分层（n / 旧pass / 新pass / 恢复 / 延伸 / 解放 / unchanged）

| 窗 | 候选级 | n | 旧pass | 新pass | 恢复 | 延伸 | 解放 | unchanged |
|---|---|---|---|---|---|---|---|---|
| p3fold | L0 | 1118 | 31 | 24 | 0 | 7 | 10 | 1101 |
| | L1 | 270 | 0 | **1** | 1 | 0 | 13 | 256 |
| | L2 | 200 | 0 | 0 | 0 | 0 | 5 | 195 |
| | L3 | 45 | 0 | 0 | 0 | 0 | 6 | 39 |
| wf7 | L0 | 1116 | 39 | 32 | 0 | 7 | 11 | 1098 |
| | L1 | 275 | 0 | **1** | 1 | 0 | 13 | 261 |
| | L2 | 178 | 0 | 0 | 0 | 0 | 6 | 172 |
| | L3 | 73 | 0 | 0 | 0 | 0 | 7 | 66 |
| | L4 | 82 | 0 | 0 | 0 | 0 | 4 | 78 |
| wf8 | L0 | 1073 | 36 | 34 | 1 | 3 | 2 | 1067 |
| | L1 | 297 | 0 | 0 | 0 | 0 | 1 | 296 |
| | L2 | 65 | 0 | 0 | 0 | 0 | 3 | 62 |
| | L3 | 83 | 0 | 0 | 0 | 0 | 1 | 82 |

结构事实：延伸致拒 17 例全部落在 L0 候选（旧确认本就在 L0）；**新链确认首次突破 L0 候选层**（2 例 L1 候选，即 §3.2 的 Long 回归例）；解放 cohort 遍布 L0–L4（存在性解放在各候选级普遍发生，但闭合仍缺）。

---

## 3. 条件①：迁移归因分类账（全量 4875 行，零未解释）

### 3.1 四类计数 + 机制断言

| 分类 | p3fold | wf7 | wf8 | 合计 | 机制（逐类断言过） |
|---|---|---|---|---|---|
| 方向自由化恢复（A 类回归） | 1 | 1 | 1 | **3** | 旧 reject/no_chain → 新 pass；3 例旧 dump 缺口级均带方向压制见证（2×layer_only_opposite + 1×certs_only_opposite）；新链全级闭合、cdt=0 |
| 链顶延伸致拒 | 7 | 7 | 3 | **17** | 旧 pass（top=0 单级链）→ 新 reject；全部旧 top=0 → 新 top≥1，首缺恰在新顶、首因全 missing（16 missing_cert + 1 missing_causal）、cdt None、L0 仍 closed |
| 存在性解放但闭合仍缺 | 34 | 41 | 7 | **82** | 旧 no_chain（82/82 零闭合）→ 新 reject（82/82 ≥1 级闭合但未全闭）；新首因 missing 68 / broken 14 |
| unchanged | 1591 | 1675 | 1507 | **4773** | 裁决相同；其中探针逐字节不变 1431/1382/1345（⟹ admit/channel 双一致，断言过），探针变但裁决同 160/293/162 |
| **未分类** | **0** | **0** | **0** | **0** | 任何落不了类的候选清单 = 空（条件①字面满足） |

对账：admit 翻转净差 -23/-33/-7 == 阶段 A after STATS.admitted Δ（平账断言过）；reject 首因 missing/broken == redline NEST_GATE_T3 行（旧 29/48、31/22、10/14 → 新 64/53、75/25、16/18）；新 broken 拒 96 = 旧 reject 延存 82 + 解放 cohort 14（延伸 cohort 贡献 0——17 例首因全 missing）；reject→reject 152 例首因零漂移。

解放 cohort 残余缺口的方向相关性（机制描述）：新首缺级在旧 dump 的见证 = agree 62 / 旧级别不可见（链顶延伸新冒出的级，旧 dump 无该级见证）19 / certs_only_opposite 1——**残余缺口 98.8%（81/82 = 62+19）与方向无关**，与 ADR 裁定 2「多级零闭合归因=证书产量不足」一致：方向退役解放了存在性，但塔侧证书供给没有随迁增产。

### 3.2 A 类回归实例（3 例全清单；「同点跨型合法递归」实例证据）

**例 1（p3fold，bar=190306，L1 候选，source_index=190252，Long，price=2682801000000，anchor=190252）**：

- 旧链：reject，top=1，cdt=1，首 gap=L0 **broken**——L1(ev2) `closed`（1 证过，wit=agree）；L0(ev1) `missing_existence`，**wit=layer_only_opposite**（该脚在 L0 层只注册在异向键下——同一点在 L0 是异型分型）。
- 新链：**pass**，top=1，cdt=0——L0 `closed`（certs=1 clean=1）、L1 `closed`，区间 [L0,L1] 两级全闭合到 L0。
- 通道 nest_n_delta_false→nest_pass（admit False→True）；旧 shadow：single=True、union merged_pass=True（hit_levels=[2]）。

**例 2（wf7，bar=122626，L1 候选，source_index=122572，Long，price=2682801000000，anchor=122572）**：谱系与例 1 逐格同构（旧 reject top=1、L0 layer_only_opposite、L1 closed → 新 pass 两级全闭合；通道 nest_n_delta_false→nest_pass；single=True、merged=True hit=[2]）。与例 1 同价同构（见 §4.2 候选对）。

**例 3（wf8，bar=23249，L0 候选，source_index=23215，Short，price=2575247000000，anchor=23215）**：

- 旧链：no_chain，top=0，首 gap=L0 **missing**——L0(ev1) `missing_cert`，**wit=certs_only_opposite**（本向键域查无证书，证书只在异向键域）。
- 新链：**pass**，top=0，cdt=0——L0 `closed`（certs=1 clean=1）。
- 通道 **xzd_pass→nest_pass**（admit True→True，准信源换成链）；旧 shadow：single=None、union merged=None——**并集时代对该候选无任何桥证据**（§5 的并集包含性 1 例例外即此例）。

三例共同结构：同一价格点在不同级别/不同侧分别为顶/底分型，方向过滤时代该点的跨型证据被键域方向分量裁掉（层只在异向键、或证只在异向键），方向退役后证据回摆、链条闭合——裁定 1「同一点可在不同级别分别为顶分型/底分型，各级分型类型是各级自己的结构事实」的全量语料中的全部 3 个实例。

### 3.3 阶段 A 探针机制计数复算

按 redline_compare.py 同口径（top/区间/锚未动的候选内，逐级比对）：存在性解放 L0 66/85/53、L1 16/44/9（另有 L2 2/16/4、L3 0/6/0）；证书池合并 L0 30/38/20、L1 22/27/6（另有 L2 13/17/31、L3 0/64/0）；底质迁移 missing_cert→closed 6/13/3、missing_cert→missing_causal 38/109/51（各级合计）——与阶段 A 回报逐项一致（脚本断言）。注（090）：该口径只覆盖 top/区间未动候选；top 延伸候选的机制在 §3.4 top 迁移矩阵落账。

### 3.4 单调性与 top 迁移

- **反单调 = 0**：逐候选逐级（zip 公共区间）certs/clean 不减、底质格序（missing_existence < missing_cert/missing_causal < closed）不降；price 逐行一致——方向退役只增不撤（阶段 A(b) 复算一致）。
- **top 迁移（含 None）**：None→Some 9/6/0（锚/存在性新解）；Some→None 0（无锚丢失）；同有延伸（新顶 > 旧顶）75/123/63；top=0 占比降（974→918 / 976→914 / 975→924）——存在性解放把大量候选的链顶向上推（§4.1）。

---

## 4. 链谱系分布新旧对照（链顶延伸实证）

### 4.1 链顶级别分布（top_dist，含 None=锚不可解；== redline NEST_GATE_T3 行）

| 窗 | 语义 | top=0 | top=1 | top=2 | top=3 | top=4 | None |
|---|---|---|---|---|---|---|---|
| p3fold | 旧 | 974 | 287 | 256 | 71 | — | 45 |
| | 新 | 918 | 302 | 293 | 84 | — | 36 |
| wf7 | 旧 | 976 | 310 | 223 | 64 | 128 | 23 |
| | 新 | 914 | 306 | 231 | 90 | 166 | 17 |
| wf8 | 旧 | 975 | 326 | 73 | 114 | — | 30 |
| | 新 | 924 | 347 | 82 | 135 | — | 30 |

top=0 减少、top≥1 与 None 齐动方向一致：方向退役让脚在更高级别的存在性显形（链顶延伸）并新解一部分锚（None 降 9/6/0）。

### 4.2 pass 谱系：多级全链闭合首次非零

- 旧链 pass 106 例**全部 top=0**（T3 §4.2 复核一致）：单级链 [L0,L0] 闭合。
- 新链 pass 92 = top=0 88 + **top=1 4**（p3fold 2 / wf7 2 / wf8 0），cdt=0 无一例外。
- **多级闭合 4 例 = 2 个同点跨型候选对**（坐标事实）：price=2682801000000，p3fold（si=190252）与 wf7（si=122572）各一对——Short L0 候选（旧 pass top=0 → 新 pass top=1，延伸段 L1 也闭合）+ Long L1 候选（旧 reject → 新 pass，§3.2 例 1/2）。同一点对 Short 侧是 L0 底分型锚、对 Long 侧是 L1 顶分型锚，方向退役后两侧链同时全闭合——「同点跨型合法递归」的完整实例对。两窗 bar 索引不同系，是否同一市场脚本报告不断言（090），仅落坐标同价同构事实。
- 恒等式（新链逐候选成立，断言过）：len(levels)==top+1（None ⟹ 空）；pass ⟹ cdt=0 且无 first_gap；reject ⟹（cdt None ⟺ 首因 missing）；no_chain ⟹ cdt None；no_chain 且 top None ⟹ first_gap None。

### 4.3 reject 首因与闭合级数

| 窗 | 旧 reject（missing/broken） | 新 reject（missing/broken） | 新 reject 首因×来源 |
|---|---|---|---|
| p3fold | 77（29/48） | 117（64/53） | 旧 reject 延存 29+47；解放 cohort 28+6；延伸 cohort 7+0 |
| wf7 | 53（31/22） | 100（75/25） | 旧 reject 延存 31+21；解放 cohort 37+4；延伸 cohort 7+0 |
| wf8 | 24（10/14） | 34（16/18） | 旧 reject 延存 10+14；解放 cohort 3+4；延伸 cohort 3+0 |

reject→reject 152 例首因零漂移（broken→broken 82 + missing→missing 70；旧 broken 84 中 2 例转入 pass=例 1/2）。reject 闭合级数分布（新）：p3fold {1级:106, 2级:11}；wf7 {1:97, 2:3}；wf8 {1:34}——仍以「仅 1 级闭合」为绝对主体。

### 4.4 谱系底质（链级条目）

| 窗 | 语义 | 条目 | closed | missing_cert | missing_existence | missing_causal |
|---|---|---|---|---|---|---|
| p3fold | 旧 | 2600 | 117 | 1814 | 662 | 7 |
| | 新 | 2737 | 155 | 1905 | 622 | 55 |
| wf7 | 旧 | 3161 | 95 | 2014 | 1046 | 6 |
| | 新 | 3409 | 138 | 2137 | 984 | 150 |
| wf8 | 旧 | 2302 | 60 | 1655 | 577 | 10 |
| | 新 | 2404 | 68 | 1722 | 548 | 66 |

读法（机制描述）：条目总数上升（链顶延伸 ⟹ 区间变长）；closed 全面上升（存在性+证书池解放）；missing_existence 下降（存在性解放的直接刻痕）；**missing_causal 大幅上升**（7→55 / 6→150 / 10→66——合并后的证书池把更多证送进因果守卫，守卫照常在岗）；broken 底质新旧均 0（「断环」仍全部来自位置极性，不来自断底质，与 T3 §4.2 同）。

---

## 5. 与 #164 三层对照（wf7 = #164 同 corpus 正式结算面）

| 层 | 值 | 口径 |
|---|---|---|
| 代理上界（#164，区间包含代理） | 423/1651 = 25.62% | 引 #164，本报告未重算 |
| 并集实恢（#164，multi 真链命中） | 17/1651 = 1.03% | 引 #164，本报告未重算 |
| 严格链（T3，方向过滤语义） | 39/1724 = 2.26%；typed_none 内新增 0 | T3 报告 |
| **新链（T5a，去方向）** | **33/1724 = 1.91%；typed_none（n=1651，逐位对读）内新增恢复 0** | 链全闭合到 L0 才准（语义不变），键域去方向 |

**方向守卫（条件③）**：新链 33 vs 并集基线 17 = **1.94×** > 1，字面成立（T3 时代为 39 vs 17 = 2.29×）。**对 T3 链 39 净降 -6，机制平账：+1 方向自由化恢复（reject→pass）− 7 链顶延伸致拒（pass→reject）**；p3fold/wf8 同型（各 +1 恢复：31→25 为 +1−7，36→34 为 +1−3）。按 090 纪律照实记录净降，不沿用「上升」预期措辞。

**人口构成（只呈现不裁定）**：并集时代 nest_pass 90 = single 73 + multi-only 17。新链确认 33 **全部**来自 single 人口（single 73 → pass 33 / reject 40；reject 40 = 延伸致拒 7 + 缺/断拒 33）；multi-only 17 在新链下 **0 再确认**（11 no_chain + 6 reject；T3 时代为 12+5——方向退役使 1 例从 no_chain 移入 reject，未改零确认结论）。typed_none（n=1651）内新链确认 0（与 T3 同型：新增恢复 0）。

**并集包含性（三窗，090 显著标注）**：T3 时代「链确认集 ⊆ 并集确认集」三窗无例外（39/39 等）。新链下：p3fold 25/25、wf7 33/33 ⊆ 并集；**wf8 33/34——例外 1 例**（bar=23249 Short，§3.2 例 3：并集无任何桥证据、旧 xzd_pass，方向退役后链确认、升 nest_pass）。即方向自由化在 wf8 产生 1 例并集外新确认；wf7 结算面的「确认集 ⊆ 并集确认集」与「typed_none 内新增 0」不受其影响（该例外不在 wf7）。

---

## 5.5 链顶延伸 cohort（17 例）+ 方向分歧 cohort 去向

### 链顶延伸致拒 17 例（pass→reject；全清单在脚本输出，此处全部分布与机制）

- **链顶分布**：新 top {1:9, 2:3, 3:3, 4:2}（旧全部 top=0）。p3fold 7 例顶 1/1/1/1/2/3/3；wf7 7 例顶 1/1/1/1/2/4/4；wf8 3 例顶 1/2/3。
- **首因**：全部 17 例首缺**恰在新顶**、首因全 **missing**（顶自身未闭合；16 missing_cert + 1 missing_causal——wf8 bar=242881 顶 L2 有证但因果守卫不过）、cdt 全 None、**L0 全部仍 closed**（旧证据保持，通道 nest_pass→nest_n_delta_false）。
- **机制描述**：方向退役后脚在更高级别的存在性显形 ⟹ 链顶升高 ⟹ 严格链要求区间 [L0, 新顶] 全闭合 ⟹ 延伸段各级塔侧无（过）证 ⟹ 缺环拒。拒因纯在延伸段（L0 未失证），即「确认口径从『本向 L0 单级』变为『存在性所及全区间』」的严格化代价；延伸段的证书供给属 map #126 战线（ADR 裁定 2）。只呈现不裁。
- 代表例：wf7 bar=198989（si=198932 Short，top 0→4，L4 missing_cert）；wf7 bar=9699（si=9636 Long，top 0→2，L2 missing_cert）；wf8 bar=242881（si=242813 Short，top 0→2，L2 **missing_causal**——唯一首因有证而守卫不过例）。

### 旧方向分歧 cohort（469 例，T3 §5.5）新链去向

方向见证装置已随裁定 1 退役（新 dump 无 dir_witness 字段），469 例分歧候选的压制源（键域方向分量）整体移除。去向（旧裁决→新裁决）：

| 窗 | cohort | 新链转 pass | 迁移明细 |
|---|---|---|---|
| p3fold | 130 | **1** | no_chain→no_chain 92；no_chain→reject 30；reject→reject 7；reject→pass 1 |
| wf7 | 229 | **1** | no_chain→no_chain 182；no_chain→reject 38；reject→reject 8；reject→pass 1 |
| wf8 | 110 | **1** | no_chain→no_chain 96；no_chain→reject 5；reject→reject 8；no_chain→pass 1 |

**确认率 3/469 = 0.64%**（T3 时代 0.00%）：方向压制解除后，分歧候选的绝大多数仍卡在存在性/证书供给（no_chain 主体 370/469）或闭合不全（reject 96/469 中转入 73）。A 类回归 3 例全部出自本 cohort（逐窗各 1，与该窗回归数相等，断言过）——「方向过滤误伤真链」在全量语料中的实测规模就是 3 例；其余 466 例的被拒首因与方向无关。与 ADR 裁定 2「方向自由化可挽回上界 ≤40，非主因」同向且更紧。只呈现不裁。

---

## 6. 限制声明（090）

1. **与阶段 A 回报的对账**：本报告全部可重算数字（迁移矩阵、探针机制计数、翻转分类与平账、trades 归因引文外的 dump 面）经脚本从两个 dump 复算，与阶段 A 回报**未发现不符**；阶段 A 探针口径仅覆盖 top/区间未动候选，本报告以 top 迁移矩阵补全延伸面（§3.3 注），属口径互补非矛盾。
2. **条件②的不变面证据为阶段 A 产出**（STATS/INDEX 13 项/tower MD5/EXIT 零行/trades 逐笔归因、index_builds 足迹），本报告复跑 `/tmp/t5a_evidence/redline_compare.py` 退出码 0 确认可复现，但未独立重跑 m8；dump 级补强（price 逐行、单调性、探针不变 ⟹ admit/channel 一致）为本报告新算。index_builds +93/+110/+77 为 stderr 面数字，不可由 dump 复算，按足迹列账引阶段 A。
3. **门关红线与 exit 侧**（沿 T3 遗留）：本报告全部数字为门开形态；NEST_GATE_EXIT 四份日志零行，exit 侧无逐项对照件；trades 变化（156→149 / 206→196 / 172→169）已由阶段 A 逐笔归因（直接 admit 翻转 + 级联）。
4. **p3fold/wf8 的对齐为候选流行序对齐**（键四元组非唯一，64/59/66 对重复键），候选流同一性的外部证据 = 阶段 A 聚合不变面 + 候选生成确定性，弱于 wf7 另有的 #164 逐位对读（沿 T3 §6 限制 4 同款括弧）。
5. **A 类回归 3 例与链顶延伸 17 例的裁定**：本报告只呈现数据与机制描述；「方向过滤误伤规模=3」「延伸严格化代价=17」是否需后续动作（如 map #126 增产线的优先级），裁定权属编排者。多级闭合 4 例中两窗同价同构候选对是否同一市场脚，未断言（§4.2）。
6. **#164 ±2 翻转括弧沿袭**：typed_none 分母用 #164 jsonl 实数 1651（同 T3 §6 限制 6）。
7. **旧 dump 的 union/single shadow 列为 T3 时代读出**（方向过滤语义的装置列）：§5 的并集包含性/人口构成以之为参照系，其保真度沿 T3 §1.3 聚合全等对账；新链不再产出该族列，无新侧可校。

---

## 附录 A：产物路径

- 本报告：`chanlun/review-results/typed-none-colocation-chain-dump-20260724.md`（worktree `/tmp/kimi-nest-mainline`）
- 新链合并 dump（4875 行 = 1633+1724+1518，逐字节合并自阶段 A 三单窗 dump，每行含 `window` 字段，原列不动）：`chanlun/review-results/colocation-chain-dump-candidates-20260724.jsonl`
- 分析脚本（114 项断言全过，退出码 0）：`chanlun/review-results/colocation-chain-analyze-20260724.py`；运行输出留档 `/tmp/t5b_analyze.out`
- 阶段 A 输入件：新链三单窗 dump `/tmp/t5a_chain_dump_{p3fold,wf7,wf8}.jsonl`；红线对照 `/tmp/t5a_evidence/redline_report.md`（+`redline_compare.py`，本报告复跑退出码 0）；baseline/after 运行件 `/tmp/t5a_baseline/<tag>/`、`/tmp/t5a_after/<tag>/`
- 旧链基线（T3 交付）：`chanlun/review-results/typed-none-strict-chain-dump-20260723.md` / `-candidates-20260723.jsonl` / `-analyze-20260723.py`
- #164 对照件：`chanlun/review-results/typed-none-joint-dump-20260722.md` / `-candidates-20260722.jsonl` / `-analyze-20260722.py`
- 三裁 ADR：`chanlun/escalate/adr-chain-identity-colocation-and-direction-retirement-20260723.md`（裁定 1 方向退役、裁定 2 供给侧挂 map #126、裁定 3 出场同构→T5b）

## 附录 B：运行命令与改动清单

阶段 A 重放命令（引阶段 A 证据，本报告未重跑）：

```
THETA_NEST_CERT_GATE=1 OPSEM_DUMP_DIR=<dir> M8_WIN_FILTER=<tag> T5A_CHAIN_DUMP_DIR=<dir> \
  cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

阶段 B 改动清单：**零代码改动、零 git mutation**。仅新增本附录 A 所列三件报告产物（md/jsonl/py，均在 worktree `chanlun/review-results/`）与 `/tmp/t5b_analyze.out` 留档；历史报告（T3/#164 三件套）未动。

阶段 B 复核运行：

```
python3 chanlun/review-results/colocation-chain-analyze-20260724.py   # 114 项断言，退出码 0
python3 /tmp/t5a_evidence/redline_compare.py                          # 阶段 A 红线复跑，退出码 0
```
