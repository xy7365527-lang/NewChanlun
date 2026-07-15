# 区间套证书机器 N^δ_{ℓ↓e} 五裁定点建议书（#92 重放前深研）

- 日期：2026-07-16
- 性质：只读调研 + 建议书；不改代码、不改任务状态、不改 git 状态；本文件为唯一产出。
- 规范锚：`docs/formal-chain/proofs-full-strategy-20260703.md` 假设3 与 §C.3；统计先验同文 `:75-83, 235-243`（BTC 全历史 95% 区间套退化为深度=1，"小转大" 92%，深度≥2 几乎为 0）。
- 实现锚：`rust/src/theta_v0/classifier/nest.rs`（基例 `:324-325`、`is_sub :61-66`、`Λ≠∅ confirm :108`、deprecated 模糊入口 `:426,:462`、旧塔消费门 `:520`）。
- 已裁不可复活项（本文遵守）：确认窗口径、"子级 A 腿塞进父级 C 离开 episode"口径——均已被 `chanlun/review-results/dr-l1l2-zero-genealogy-20260711.md` §0.2 判为符号对象错配。

---

## 1. TL;DR 裁定建议表

| 裁定点 | 建议 | 置信度 | 一句话理由 |
|---|---|---|---|
| ② J^δ_ℓ 定位区间端点 | **双口径：B（背驰段口径）为判据核验，A（pair 全跨度）仅作结构上界过滤**；C2 域下 B=[leave.start, leave.end]、A=[leave.start, retest.end]，左端同源，157 条 A 链是 B 的真上界 | 高 | 第27课:44-46 区间套定理逐字嵌套对象是"背驰段"（=构成背驰的那段走势类型，027:22），不含回试段；genealogy 已判父区间=父级最后背驰段 c |
| ⑤ 背驰类型域 | **盘整背驰入链**：N^δ 链的背驰段域 = 趋势背驰 ∪ 盘整背驰；信号解释层维持 #145 承接门（盘背证书不冒充同级 B1/S1） | 高（域）/中（落地） | 背驰段定义逐字含盘背（027:22）；区间套旗舰例万科季度顶层即盘背（027:24,36-38）；趋势-only 把 92% "小转大"统计主通道结构性排除 |
| ① Cand^δ_ℓ 定义式 | **B：严格 C2 pair ∧ 背驰谓词命中**（谓词域按⑤参数化） | 高 | A 把候选降格为纯相邻性（非背驰对象）；C 挂旧塔——历史产量 0 且静默双核 3,169 窗（#89），输入不可信 |
| ③ 基例 Conf^δ_e | **维持 `BspBits::confirm_side`**；D5 CompletedFreeze 可作执行级完成资格前置门，不替换 Conf | 高 | Λ≠∅ 是买卖点 bit-vector 非空判定（Nest.lean `Confirm`）；FreezeEvent 无 bit-vector，改挂则 Λ≠∅ 语义直接丢失 |
| ④ 时序递降时间戳 | **snapshot 系 + 钟=背驰确认时点**（该级 pair 首次可证时点）；`retest.end` 仅当完成可证依赖回试结算时作显式保守钟；`CompletedFreezeEvent.created_at` 禁用 | 中 | 原文钟是"背驰的成立已经是确认"时点（027:30 语境）；已裁教义禁止终态回填（CandDeltaEvent snapshot 纪律）；created_at 是工程持久化时戳非语义钟 |

---

## 2. 详细论证

### 2.1 裁定点②（深挖）：区间套到底套的是什么区间

#### 原文依据

区间套定理原文（`docs/chanlun/text/blog/027-第27课.md:44-46`，逐字）：

> "定理：某大级别的转折点，可以通过不同级别**背驰段**的逐级收缩范围而确定。"
> "换言之，某大级别的转折点，先找到其背驰段，然后在次级别图里，找出相应背驰段在次级别里的背驰段，将该过程反复进行下去……相应的转折点就在该级别背驰段确定的范围内。"

被嵌套的对象**每一级都是"背驰段"**。背驰段的定义在同课 `:22`：

> "在某级别的某类型走势，如果构成背驰或盘整背驰，就把这段走势类型称为某级别的背驰段。"

即背驰段 = **构成背驰的那一段走势类型本身**。三条推论：

1. **不含回试段**。万科例（`027:38`）："月线……的背驰段，一定在季度线的背驰段里，而且区间比之小"——收缩的是背驰段对背驰段的包含，原文从未把背驰确认后的回试（retest）纳入定位区间。
2. **父级区间从 c 的结构起点起算**。第29课 `:30`："最后的背驰段，跌破该中枢后……"把趋势背驰的背驰段定位为最后一个中枢之后的最后一段（即 c）；第37课 `:22` 把递归入口放在"对 c 的内部进行分析……形成类似区间套"。此链条 `dr-l1l2-zero-genealogy-20260711.md` §1 已完整梳理并形成结论：父区间 = 父级最后背驰段 c 的完整走势类型区间，**非** A 起点、**非**整段趋势、**非**局部 reentry episode。本文核对原文行号无出入。
3. **右端 = c 完成/确认点附近**。`0016:62`（经 `d1-seed-anchor-d2-episode-fallback-ruling-20260715.md` 原文回查确证）："C段的走势类型**完成时**……"——背驰比较以 C 段完成为准，区间右端不越过 c 的完成。

#### 工程实证（C2 域映射）

- A 口径（#83 已测）：`[leave.start_index, retest.end_index]`，闭包含复用 `nest::is_sub`（`nest.rs:61-66`），得 157 条 L1–L5 完整链（`c2-yield-remeasure-20260715.md` §1.4）。#83 自我声明该口径"只回答结构闭包含，不重述为 `Chi::is_confirmed` 或交易信号"。
- B 口径（本建议）：leave move 本身就是一个完整 CompletedMove（完整走势类型，`first_retrace_replay.rs:31-55` 严格资格：不同、正向相邻、均 Completed），故 `[leave.start, leave.end]` 恰是"构成背驰的那段走势类型区间"——与旧塔 `enter_src` 局部 episode 左端（genealogy §2 判定偏右）不同，**C2 域的 leave.start 天然就是 c 的结构起点**，旧病灶（episode 局部化）在 C2 域已消失。
- 关键几何事实：**A 与 B 左端同源（leave.start），差异只在右端（retest.end vs leave.end）**。故 A ⊇ B 逐级成立，157 条 A 链是 B 链数的真上界；B 重放只会收缩不会新增。
- genealogy §3 的 L1→L2 三样本失败全是**左界**失败（子级证据早于父级左端 11504/19853/34445 bar）——那是旧塔"子 A 腿 vs 父 episode"错配的指纹，C2 域换对象后此指纹不可迁移，须重测而非外推。

#### 反方论证（A 单口径 / "157 即产量"）

- 支持 A 者可诉诸：retest 是背驰"被市场确认"的结构证据，纳入区间提高鲁棒性；157 条链已实测非零，避免再度归零。
- 驳：原文定理的收缩对象逐字是背驰段；把 retest 并入右端会使父区间右端右移，**放松**闭包含判据（假阳性方向的错误），且与 `0016:62` "完成时"口径冲突。retest 的正确位置是④的钟候选与①的资格门，不是②的区间端点。

#### "产量 0 是正确答案、157 只是结构上界"是否成立

**分层成立**：

1. 深度≥2 维度：成立。统计先验（proofs `:75-83`）深度≥2 几乎为 0 是市场几何事实；genealogy §0.5 预注册预测"按推荐定义重放 L1→L2 仍为 0 或极低"。B 口径重放后深度≥2 归零/接近零应读作**定义忠实的确认**，不是回归。
2. 深度=1 维度：**不成立为"0 正确"**。95% 案例是深度=1"小转大"（92%），其机制正是盘整背驰（见⑤）；若⑤维持趋势-only，深度=1 通道被结构性排除，产量 0 就是**定义偏窄的人工产物**而非市场事实。
3. 157 条 A 链的地位：结构上界过滤器（预筛/护栏），可入探针与审计，不得直接当判据命中或交易语义（与 #83 自我声明一致）。

**建议：B 为判据、A 为上界过滤的双口径；置信度 高；可逆性 好**（区间端点是纯参数，A/B 可并行重放对账，无持久化污染——D5 事件只 append 且 cache_key 含 version tuple，`level_view_store.rs:1-16`）。

### 2.2 裁定点⑤（深挖）：趋势背驰-only vs 盘整背驰入链

#### 原文依据（第27课前后课文）

1. **背驰段定义逐字含盘背**（`027:22`）："构成背驰**或盘整背驰**，就把这段走势类型称为……背驰段"。区间套定理（`:44-46`）的嵌套对象是背驰段——定义域上盘背天然在链内。
2. **区间套旗舰例顶层就是盘背**（`027:24,36-38`）：整课标题即"盘整背驰与历史性底部"；`:24` "盘整背驰最有用的，就是用在大级别上……往往就是历史性的大底部"；`:36` "这种从大级别往下精确找大级别买点的方法，和区间套是一个道理"——万科季度图三段（`:16` 语境：第一个中枢的背驰"只能算是盘整背驰"）是区间套示范的**父级入口**。
3. **区间套亦用于二买确认**（`027:60`）："600685 第二买点的确认方法（区间套）"；而 `:18` "多数的第二、三类买点，其实都是由盘整背驰构成的，而第一类买点，**多数**由趋势的背驰构成"（注意原文是"多数"）；`:20` 二三类买点的三段走势"形成盘整背驰"。
4. **盘背在大级别构成"类第一类买点"**（`027:66`）："第一类买点肯定是趋势背驰构成的，而盘整背驰构成的买点……在大级别里，这也构成一种类似第一类买点的买点……这个级别，至少应该是周线以上。"
5. **用法差异**：趋势背驰 → 同级第一类转折（第29课转折定理语境）；盘整背驰 → (a) 中枢离开受阻返回（`027:16`），(b) 二三类买点确认（`:18-20,60`），(c) 大级别类一买（`:24,66`），(d) 第37课 `:18` "完全可以用盘整背驰来处理" B 中枢的小级别波动——即**小转大机制的载体**。

#### 工程实证

- `divergence.rs` 已完整区分双证书：`trend_divergence`（A/C 跨相邻两中枢，`:20-22`）与 `consolidation_divergence`/`judge_pan_div`（A/C 同一中枢两次同向离开，`:23-24`）；`:27` 注明已结算裁决"第一类买卖点只由趋势背驰产生（beichi.md #4 + maimai.md:56）"。
- 旧塔 `CandDeltaEvent.cand_delta`（`recursive_tower.rs:1188`）= 趋势背驰确认 D only；`pan_div_diag`（`:1189-1191`）注释"裁决②：**不入谓词**"，测试 `:2266` 亦断言"趋势路径无盘整背驰诊断（裁决②不入链）"。
- C2 域 D2 provider（`level_view.rs:400-401`）："盘整 `None` 严格产零 pair"——当前 1,092 严格 pair **全部是趋势方向块**，盘背在 C2 域尚无 pair 对应物。
- 既有裁决 #145（`consolidation-protocol-decision-research-20260714.md` §0.5）："只有通过现有 **Nest/XZD 承接门**的 `PanDivCert` 才进入生产候选。它不变成同级第一类买卖点"——注意：该裁决**预设了 PanDivCert 参与 Nest 承接**，即盘背证书过区间套门是已定设计，与"盘背不入 cand_delta 谓词"的旧裁决②分属两层。

#### 反方论证（维持趋势-only）

- 旧裁决②（pan_div_diag 不入谓词）与 `beichi.md #4 + maimai.md:56` 已结算"盘背不产第一类"；放开域恐重开已闭裁决。
- 驳：两个裁决的辖域是**第一类买卖点谓词/first_class_bits**（信号解释层），不是 N^δ 链的背驰段域。原文证据（背驰段定义 + 万科例 + 600685 二买区间套）显示链域含盘背与"盘背不产同级第一类"**并行不悖**：盘背段可以是链的某一级定位区间，链终端的信号性质另由解释层判（#145 承接门恰是此分工）。维持趋势-only 反而与 #145 "PanDivCert 须由下级确认或 XZD 承接"矛盾——承接门无链可承。
- 统计压力测试：先验 92% "小转大"（proofs `:75-83`）机制上=次级别盘背引发大级别转折（第37课 `:18`）。趋势-only 域下该主通道不可达——这与"深度≥2≈0 是市场事实"不同，属于**定义人为截断**。

**建议：背驰段域 = 趋势 ∪ 盘整（链域），信号解释层维持已结算裁决与 #145 承接门；置信度 高（域的教义结论）/中（C2 域盘背 pair provider 尚不存在，工程距离非零）；可逆性 好**——谓词域做成 enum gauge（仿 `divergence.rs:434-448` 三套 Θ 预注册模式），趋势-only 与双域可并行重放对账。

---

## 3. 快评（①③④）

### ① Cand^δ_ℓ 定义式 → 建议 B（C2 pair ∧ 背驰谓词命中）

- 原文：候选对象是背驰段（027:22,44-46），不是任意相邻完成对——**A（pair 存在即候选）语义不足**：1,092 pair 中未过背驰谓词的对不构成背驰段，入候选即符号错配。
- 工程：**C（旧塔 cand_delta bit）不可用**——历史产量 0（任务背景）；且 #89 静默双核审计（`silent-dual-core-audit-20260715.md`）：3,169 窗（占可投影窗 27.1%）塔层继承核 ≠ econ 层投影核，下游 166/209 run 走势块已分叉——旧塔 bit 与 C2 对象不同源，挂接即双核污染。`nest.rs:520` 现行 `!base.cand_delta` 消费门即 C 路径，应随裁定迁移。
- B 的谓词实体已在产线：`DivergencePair`（`level_view.rs:385-398`，携带 `seg_a/seg_c`）由 `move-block-ac-v1` provider 产出（version tuple 锚 `level_view.rs:28-101`），背驰谓词 = `segments_diverge`（C<A 力度原语）。
- 注意 `nest.rs:12` 头注释本就标注 Cand^δ_ℓ "[需人工确认] 定义式，spec 疑点2"——本裁定即关闭该疑点。
- **置信度 高；可逆性 好**（谓词替换点单一，A/B 可对账）。

### ③ 基例 Conf^δ_e → 建议维持 BspBits::confirm_side

- 规范：假设3 基例 N^δ_{e↓e} = Conf^δ_e；spec P5 §6 `Conf^+_e(x)=⋁ B_{i,e}(x)`（`types.rs:183-203` 结果包引 line 283）；Λ≠∅ = bit-vector 非空（`nest.rs:108` confirm，契约锚 Nest.lean `Confirm`，`confirm_of_two_three` 不要求 |Λ|=1）。
- `CompletedFreezeEvent`（`level_view_store.rs:53-58`）字段为 `sequence/created_at/cache_key/snapshot`——是 D5 **完成冻结持久化事件**，无买卖点 bit-vector、无 side 语义。改挂 = 把"买卖点确认"偷换为"走势完成"，Λ≠∅ 语义**不保持**（freeze 上无 Λ 可言），且 `confirm_side`（`types.rs:216`）的方向化基例失去输入。
- 正确分工：D5 freeze 作为执行级 CompletedMove 的**资格前置门**（Cand 侧、①的 pair 完成性来源），Conf^δ_e 保持买卖点析取。二者合取而非替换。
- **置信度 高；可逆性 好**（不改动即零风险；若改挂后发现 Λ 语义丢失，回退成本高——又一条维持理由）。

### ④ 时序递降时间戳 → 建议 snapshot 系 + 背驰确认时点

- 原文钟：万科例（`027:30-38` 语境）"季度线跌破3.2元后，这个背驰的成立已经是确认了，而第三段的走势……可以一直分析下去"——父级钟是**背驰成立确认时点**（C 完成/破位可证时），随后向下细化；即 parent.confirm ≤ child.confirm 的钟是背驰确认钟，非回试完成钟、非持久化钟。
- 三候选评估：
  - **背驰确认时点**（推荐）：C2 域 = 该级 pair 首次可证时点（leave 完成 + 背驰谓词可判的最早 as_of）；对应旧塔 `divergence_confirm_src`（`recursive_tower.rs:1156`）的 C2 继承者。
  - **retest.end**：若 leave 的 Completed 判定在实现上依赖 retest 结算，则"首次可证时点"事实上等于 retest 确认时点——此时用 retest.end 作**显式保守钟**可接受，但须声明等价性而非混用（残余人裁点 R3）。
  - **freeze created_at**：禁用。D5 append 时戳是工程持久化时点，与语义钟无必然关系（重放/迁移会改变它，`level_view_store.rs:94-107` append-only 语义下同一语义事件可有不同写入时点）。
- snapshot vs terminal：选 **snapshot**。教义纪律已在旧塔字段上定型——`cp_certificate_confirm_src` "当时不可证必须保持 None"、`full_trend_c_qualified` "不得从终态对象回填"（`recursive_tower.rs:1176-1179`）；`nest.rs:426,:462` deprecated 注记本就要求显式二选一，terminal 系仅限离线审计对账。
- **置信度 中**（"首次可证时点"在 C2 域的精确实现位置未逐行核验）；**可逆性 好**（钟是入口参数，snapshot/terminal 双入口已并存）。

---

## 4. #92 重放前的只读证伪探针清单

全部 COUNTERFACTUAL_ONLY / 只读，不入生产路径（沿 #86 条款 3 先例）：

1. **P-②a 双口径链数对账**：在 #83 同一 run 集上把逐级区间从 A=[leave.start, retest.end] 收缩为 B=[leave.start, leave.end]，重算 `is_sub` 闭包含链数。预注册预测：B 链数 ≤ 157 且深度≥2 段大幅收缩（对齐 95% 深度=1 先验）；若 B 链数反升则②论证有洞。
2. **P-②b 右端敏感度**：统计 157 条链中每级闭包含边"仅靠 retest 段才成立"（即 inner.end ∈ (leave.end, retest.end]）的边占比——直接量化 A 口径右端放松制造的假阳性面。
3. **P-⑤a 盘背 pair 反事实计数**：用 `judge_pan_div`/`consolidation_divergence` 在 C2 Consolidation 块（1,615 个 None 方向 CompletedMove）上反事实计数 would-be 盘背 pair 与深度=1"小转大"候选数。预测：非零且集中在深度=1；若为零则⑤的产量论证失效。
4. **P-③ 基例命中率**：157 链（及 P-②a 的 B 链）执行级终端 BspBits 非零率。若大面积全零，则链归零的病灶在 bit 生产线而非基例挂接——先修上游再谈③改挂，任何改挂动议须先过此探针。
5. **P-④ 三钟通过率**：对每条链逐级分别用（背驰确认时点 / retest.end / freeze created_at）验 parent ≤ child 递降，输出三钟通过率矩阵与逆序样本——实证钟选择的判别力，并检验"leave 完成可证 ≡ retest 确认"是否成立（喂给残余点 R3）。
6. **P-① 双核阴影对账**：旧塔 `cand_delta=true` 事件（L2 有 13 个，genealogy §3）与 C2 域 B 谓词命中集的映射对账（仿 p89 方法）——量化 C→B 迁移丢弃/新增的候选面。

---

## 5. 必须人裁的残余点

- **R1（⑤辖域）**：旧裁决②"pan_div_diag 不入谓词"的辖域认定——本文论证其辖域为第一类信号层而非 N^δ 链域，但该裁决原文措辞挂在 CandDeltaEvent 上（`recursive_tower.rs:1189`）。链域放开盘背须人裁确认"不构成对已结算裁决的推翻，而是辖域澄清"，或走正式新裁定。
- **R2（②右端开闭）**：B 口径右端取 c 完成点（leave.end）还是背驰首次可证时点（可能早于 leave.end 的谓词命中点）；开区间/闭区间与 `is_sub` 的 ≥/≤ 边界等号（第三类边界谱系有等号敏感史）。
- **R3（④钟等价性）**：若 P-④ 显示 leave 完成可证时点 ≡ retest 确认时点，是否接受 retest.end 作实现钟并在文档声明等价；若不等价，取何者为准。
- **R4（A 口径地位）**：157 条 A 链作为上界过滤器是否允许进入生产预筛（性能护栏），还是仅限探针/审计——涉及生产路径引入非判据对象的先例。
- **R5（工程排期）**：⑤落地需要 C2 域盘背 pair provider（现 D2 provider 盘整严格产零，`level_view.rs:400-401`）与 version tuple 新分量——是否与 #92 重放同批或后置。

---

## 附：证据索引（关键 file:line）

| 主题 | 锚点 |
|---|---|
| 区间套定理/背驰段定义 | `docs/chanlun/text/blog/027-第27课.md:22,36,38,44-46` |
| 盘背地位/类一买/二买区间套 | 同文 `:16,18,20,24,60,66` |
| 背驰段=c / c 内递归 | `029-第29课.md:30`；`037-第37课.md:18,22` |
| 符号对象错配裁定 | `chanlun/review-results/dr-l1l2-zero-genealogy-20260711.md` §0-§3 |
| 157 链口径与自我声明 | `chanlun/review-results/c2-yield-remeasure-20260715.md` §0,§1.3-1.4 |
| 静默双核 | `chanlun/review-results/silent-dual-core-audit-20260715.md` TL;DR |
| PanDivCert 承接门（#145） | `chanlun/review-results/consolidation-protocol-decision-research-20260714.md` §0.5 |
| fail-closed 终局（D1/D2） | `chanlun/escalate/d1-seed-anchor-d2-episode-fallback-ruling-20260715.md` |
| 方向定版 central-ggdd-v1 | `chanlun/escalate/d3-direction-ruling-20260714.md` |
| 证书机器/基例/is_sub/deprecated | `rust/src/theta_v0/classifier/nest.rs:12,61-66,108,324-325,426,462,520` |
| CandDeltaEvent/pan_div_diag | `rust/src/theta_v0/classifier/recursive_tower.rs:1149-1191,2266` |
| 双背驰证书/第一类已结算 | `rust/src/theta_v0/classifier/divergence.rs:20-27,434-448,654-708,718-735` |
| 严格 pair 原语 | `rust/src/theta_v0/classifier/first_retrace_replay.rs:31-55` |
| DivergencePair/D2 provider | `rust/src/theta_v0/classifier/level_view.rs:28-101,385-398,400-418` |
| BspBits/confirm_side | `rust/src/theta_v0/types.rs:173-203,216` |
| CompletedFreezeEvent | `rust/src/theta_v0/classifier/level_view_store.rs:1-16,53-58,94-107` |
