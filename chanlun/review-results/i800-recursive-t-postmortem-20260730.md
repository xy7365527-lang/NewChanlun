# #800 recursive_t 失败复盘——17383 行平行引擎的立项意图、分叉点、卡点、自相似与模块化判别

- 日期：2026-07-30
- 票据：[#800](https://github.com/xy7365527-lang/NewChanlun/issues/800)（parent [#787](https://github.com/xy7365527-lang/NewChanlun/issues/787)，blocking [#799](https://github.com/xy7365527-lang/NewChanlun/issues/799)）
- 性质：**只查不改**。本报告未改动任何生产代码、未重构、不给实现建议。
- 分支：`probe/i800-recursive-t-postmortem`（自 `docs/grill-with-docs-entry` 切；`main` 是另一条无关谱系，不含 `rust/`）
- 标注约定：**【事实】**＝代码/commit/文档可直接核对；**【推断】**＝由事实推出但无直接留痕；**【未能判定】**＝证据不足。

---

## 0. 一页速查

| 问 | 结论 |
|---|---|
| 1 立项意图 | **【事实】** 就是「让每一级复用同一套判定」。设计文档首行：「用一个递归算子 T 取代 v3 引擎的手动分层 ladder 架构」 |
| 2 与 theta_v0 关系 | **【事实】** 同一件事的两套（实为三套）独立实现。五概念中 **中枢/背驰/走势类型/级别 同名异义，第二类买卖点定义直接冲突** |
| 3 分叉方式 | **【事实】** 两次分叉各自是有意的「standalone / 不依赖前代」决策；**两套并存从未被拍板**——退役裁定（ADR-0004）迟到 5 周，是事后名分追认，不是分叉时的设计决策 |
| 4 卡在哪一步 | **【事实】** 不是判据不对、不是性能、不是接不进生产（它接进了 NT）。是 **①核心可交易假设被自己跑的 L3 证伪 → ②三个可约靶子逐个被 L3 判「不可约」 → ③方法论换轨（预注册协议 + Θ 规格 + Lean 锚），旧引擎的迭代方式在新协议下不合法 → ④无人接着做**。四条同时成立，主因是 ③＋① |
| 5 现在谁在用 | **【事实】** 名分「现役」（#761/#762 机械判据），但 **6 个 python 调用方全部冻结于 2026-06-18~06-24**，与 theta_v0 调用方零重叠 |
| 6a 模块化吗 | **【事实】** **分层回答**：信号层（2611 行）模块化成立；操作层（12062 行）**不成立**——单体 `TRoot` 36 字段 + `EngineConfig` 22 个开关 + 24 处 `std::env::var`，变体以 env 分支塞进同一函数，零 trait 零泛型 |
| 6b 做到自相似吗 | **【事实】** 信号层 **做到了**（级别是纯参数，无按级别分叉）；操作层 **尝试过、被 L3 打回、由编排者裁决回退**成绝对 ladder ＋ 根角色特权 |

**对 #799 最关键的一句**：本次实验里「自相似」在**信号层被做到了并且一直保留至今**——所以失败原因不在「自相似做不做得到」。操作层是另一回事：它做不到不是因为架构表达能力，而是因为**仓位三阶段战役（降成本/退本金/增股数）本质是单例的**，per-level 复制它被裁决删除（`rec_engine.rs:41` 注释在案）。

---

## 1. 它当初想解决什么

**【事实】** 立项文档 `docs/unified_recursive_operator_T.md:1-24`，第一行副题逐字：

> 用一个递归算子 T 取代 v3 引擎的手动分层 ladder 架构。

三项任务同文档 `:18-22` 逐字列出：

1. 把「一组单元序列 → 找重叠 → 中枢 → 走势类型 → 成为上级单元 → 回到开始」写成**单一算子 T**；
2. 证明 v3 引擎的手动分层（`bi=1/segment=2/move=3/recL2=4/…`）是 T 的**展开式**，**每层之间的接缝 bug 是展开导致的，不是缠论本身的**；
3. 给出从 v3 迁移到 T 的路径。

原文依据锚在第 65 课 `aₙ=f(aₙ₋₁)`（同文档 `:56-62` 引原文三段：65 课 / 63 课 / 64 课）。

**首批落码 commit** `5856c191a1`（2026-06-18 16:09），message 逐字：

> standalone 架构（不依赖 v3 nucleus）证明 T 形式不变性（第65课 aₙ=f(aₙ₋₁)）可独立自洽：**所有级别共用同一对纯函数**（`center::find_centers` + `trend::segment_into_trends`），**递归深度由数据涌现上界 r\* 决定，非 MAX_LADDER 常量**。

**结论（Q1）**：**【事实】** 票面假设成立——立项目的**就是**「让每一级复用同一套判定」，而且动机明确是「消灭手动分层的接缝 bug」。这与 #799 待裁的问题是同一个问题。

同文档还预先划了诚实边界（`:32-40`），值得原样带给 #799：

- L0 骨架（有原文）：T 的存在、形式不变性、a₀ 唯一性；
- **【综合层包装，缠师原文不直接支持】**：把 a₀ 内部（包含处理，取 max/min 合并）与 a₀ 之上（重叠，区间求交）统一为同一个 `f`。文档自述「**统一的只是「找单元→找结构→升级」的循环外壳**」（`:40`）。

---

## 2. 它跟 `theta_v0` 是不是同一件事的两套实现

**【事实】** 是。而且实际是**三套**：顶层 v3 散件（`rust/src/{bi_engine,zhongshu,buysellpoint,divergence,moves}.rs` 等）→ `recursive_t/`（2026-06-18）→ `theta_v0/`（2026-06-25）。三套互不调用：census `.chanlun/review-results/legacy-generation-census-20260729.md:23` 实证「**theta_v0 全自包含，不依赖任何前代族代码**」。

### 2.1 五概念对照表

| 概念 | `recursive_t`（standalone T 层） | `theta_v0`（π） | 判定 |
|---|---|---|---|
| **中枢** | `center.rs:85 find_centers`：`ZD=max(三段低)/ZG=min(三段高)`，成立条件 `zg > zd`（严格）；延伸用**闭区间** `overlaps`（`center.rs:60`，相切=延伸，#322 裁定改弱）；**无方向交替要求**；扫描 `i = last + 2`（跳过突破段） | `classifier/center.rs:1-16`：成立要**两条全部**——(1) `DirAlternates` 方向交替 `s1.dir≠s2.dir ∧ s2.dir≠s3.dir`，(2) `computeZD < computeZG` 严格；锚 Lean `Origin.CenterComplete` | **同名异义**。成立谓词强度不同：theta_v0 多一条方向交替硬条件，recursive_t 无。延伸口径经 #322 已对齐（相切=延伸） |
| **背驰** | `divergence.rs`：第 37 课**趋势背驰 5 条件**全实装 ＋ 第 24 课力度衰减；力度代理 = `inner_zhongshu_count`（嵌套深度）或振幅，**MACD 是可选项**（`PerfectionMode::{Structural, And, Or}` 三模式，`operator.rs:94`） | `classifier/divergence.rs:1-30`：**MACD 面积为力度原语**（`d.forceC.area < d.forceA.area`，锚 `Origin.Divergence.IsDivergence`）＋ A/B/C 框架层区分趋势背驰 / 盘整背驰 | **同名异义**。力度度量的**主源不同**：recursive_t 主结构、MACD 可关；theta_v0 主 MACD 面积、结构作前提门 |
| **走势类型** | `trend.rs:27 group_centers_by_direction`：按「依次同向中枢」分组，1 中枢=盘整、≥2 同向=趋势；方向源自中枢外缘 `same_direction_step`（`next.dd > prev.gg`） | `classifier/decompose.rs` ＋ `classifier/recursive_tower.rs`：走势单元升级为携 subs 的 `RMove::Compose`（`descend.rs` port `Origin.SubLevelDescent.RMove`，Lean `Move` μF 镜像）；τ 门控在 `decompose.rs`（task #143） | **同名异义**。recursive_t 的走势 = 中枢分组的切片；theta_v0 的走势 = 携次级别 subs 的递归代数对象（μF），后者能 `descend` 取回次级别序列，前者不能 |
| **级别** | `mod.rs:96 iterate`：**深度由数据涌现**——`out.trends.is_empty()` 或 `next.len() < 3` 即停；`k > 64` 只是防无限循环的安全阀（`mod.rs:121`）。操作层另有硬顶 `rec_engine.rs:81 MAX_LEVEL = 8` | `docs/reference-theta-v0.md:27-30`（Θ_level）：`L0 = 1分钟线段账本`；`L(k+1)` 只由 `Lk` 已完成走势构造，**禁跳级混级**；**`Lmax=6` 是预注册常量** | **同名异义**。recursive_t 级别是涌现量、无先验上界；theta_v0 级别是冻结参数（Lmax=6）。此外 recursive_t 内部自相矛盾——信号层无上界、操作层硬顶 8 |
| **买卖点** | `types.rs:191 BSPKind` 六值互斥枚举；type1=步骤 c 迭代不变量，type3=中枢离开派生（`operator.rs:54 detect_type3`），**type2 = 跨级投影**（`mod.rs:134 project_type2`：`type2_k` ≔ 紧跟 level-k type1 之后、同极性的第一个 level-(k−1) type1） | `classifier/bsp.rs:1-18`：**bit-vector 非互斥** `BspBits`；`IsType1 = brokeCenter ∧ IsDivergence`；**`IsType2 = afterTypeOne ∧ ¬brokeCenter`**（同级别时序谓词，非跨级投影）；`no_exclusive_trichotomy` 已在 Lean 证明 **2B/3B 可共存** | **同名异义 ＋ 定义直接冲突**。①代数结构相反：互斥 sum vs 非互斥 subset（theta_v0 明写「把 E 做成互斥 sum 是错误」，`level_state.rs:14-16`）；②第二类买卖点：recursive_t 定义为**次级别 type1 的跨级投影**，theta_v0 定义为**同级别「1 类之后 ∧ 未破中枢」**——两者不等价 |

**同名同义的只有一处**：第三类买卖点的几何判据（离开中枢后回抽不破 ZG/ZD）——`operator.rs:65,74` vs `bsp.rs:9` 语义一致。

**【事实】** 两套实现的教义口径被**同一批裁定票同时改**，即维护税是真实支付过的：`36092eab8e`（2026-07-26）message 逐字「#322 中枢延伸谓词改弱对齐全域——**第四实现分歧消解**，相切=重叠=延伸」；`3186bd104d`（#338）为其收尾。recursive_t 在裁定书里被计为「第四实现」。

---

## 3. 什么时候、以什么方式分叉的

**【事实】** 时间线（commit 锚）：

| 日期 | commit | 事件 |
|---|---|---|
| 2026-06-18 | `5856c191a1` | recursive_t 落地，自述「**standalone 架构（不依赖 v3 nucleus）**」——对 v3 的分叉是**有意的、写在 message 里的** |
| 2026-06-18 | `aa45a76d1f` | 操作层重写「每级别独立自我复制（**删 root 方向**）」——自相似推到操作层 |
| 2026-06-20 | `782be90dca` | 架构 v2 设计文档 `docs/recursive_t_architecture_v2.md`（740 行），编排者三裁决在案 |
| 2026-06-21 | `docs/recursive_t_deadlock_escalation.md` §10 | 编排者**第五方案**裁决：「对照 flat 引擎，把它的逻辑递归化。**不要自创约束。**」——操作层自相似被回退 |
| 2026-06-25 | `066ebcbea9` | `docs/reference-theta-v0.md` ＋ `docs/backtest-protocol-v0.md` 落地（codex gpt-5.5 代理裁决，`.chanlun/review-results/codex-decide-20260625-1008.md`） |
| 2026-06-25 | `cc108f6c53` / `0c499e6fbb` | `theta_v0/` 落地，**第三套引擎**，同样自包含 |
| 2026-06-25 | `a0c879b9c7` | recursive_t **最后一次功能性 commit**（`#84 逐级配额+G轴+L_confirm instrumentation — 全 env-gated OFF`） |
| 2026-07-26 | `36092eab8e` / `3186bd104d` | 跨实现教义订正扫到 recursive_t（「第四实现」） |
| 2026-07-28 | `docs/adr/0004-generation-constitution.md` ＋ `docs/agents/generation-constitution.md:35` | **首次**明文裁定「**π 是唯一现役引擎**」 |
| 2026-07-30 | `c1f1cb5e7f` / `bd5666c8c6` | #761/#762 名分标记，改判 recursive_t 全簇**现役不可删** |

**结论（Q3）**：**【事实】**

- **每一次「另起一套」都是有意的设计决策，且都有留痕**：recursive_t 自述 standalone；theta_v0 由 2026-06-25 codex 裁决（`.chanlun/review-results/codex-decide-20260625-1008.md`）开的 Phase 0–3 相变计划落地。
- **但「两套并存」这件事本身从未被拍板**。**【事实】** 2026-06-25 的裁决书全文只谈形式化路径（νF / LTS / bisimulation / Θ 规格），**通篇未提 `recursive_t`，未裁其存废，未安排迁移**。恰恰在同一天，recursive_t 打下最后一个功能 commit 后停摆。
- 明文退役裁定（ADR-0004「π 唯一现役」）**迟到 5 周（2026-06-25 → 2026-07-28）**。这 5 周里两套代码同时在仓、同时被教义票扫到、同时编译进同一个 `.so`。

**【推断】** 因此 Q3 的答案不是「有意决策」也不是「纯漂移」，而是第三种：**分叉有意、并存无人裁定**——每次都合法地开了新线，但没有任何一张票负责关旧线，直到一个月后由「名分」类票据补裁。这与 #787 map 的既有判断（「根因非『重复造轮子』而是『没人知道已经有什么』」）同构。

---

## 4. 它卡在哪一步没能取代原有的（本票最重一问）

先排除三个常见解释：

- **「判据不对」——不成立。** **【事实】** 判据一直在被修，而且修得比 theta_v0 还早：第 37 课五条件全实装（`5856c191a1`）、c 段归属修复（`docs/c_segment_attribution_reasoning.md`，把无 c 段比例从 77.6% 打下来）、相切口径 #322。判据不是它停下来的原因。
- **「性能不行」——无证据。** **【未能判定】**：全仓无一份 recursive_t 的性能否决报告；`docs/rec_t_8x3_l3_verification.md:11` 记录 8 标的 × 3 模式 24 组全量跑通耗时 796s（约 1500 万 bar），未见性能被点名为阻碍。
- **「接不进生产」——反证成立。** **【事实】** `597f423b21`（2026-06-20）message 逐字「递归 T 引擎接入 NautilusTrader（PyO3 + Strategy + BTC 回测）」；`trading_system/strategy/rec_t_strategy.py:45` 是 NT Strategy 实体；`trading_system/backtest_t_fugue.py:166` 走 `nr.TFugueStream`。它**接进去了**。

真正的四条，按证据强度排序：

### 4.1 【事实】核心可交易假设被它自己跑的 L3 证伪（2026-06-20）

`docs/session_handoff_2026-06-20_t_redesign_and_falsification.md:16-22, 44`：

> **决定性证伪**：type1 实时确认锚下 1d 前向收益 **−0.27%**，劣于随机 **+0.13%**（p=0.93），**8 标的一致** → 「最危险的墙」正面攻后立着。
> 幸存的是 cost_basis（仓位会计，与信号质量正交）和离场信号；**塌掉的是「实时抄底」**。

同文档 `:146-157` 给 BTC 4,625,119 bar 的结构数字：趋势走势 76 个、无 c 段 59 个（**77.6%**）、全塔 type1 单调衰减 `165→68→14→3→0`，`level3` 起 `type1_buy=0`。

**这条直接掏空了立项的操作前提**：T 塔越往上，一类买卖点越稀疏到零，而一类买卖点又被证明实时无 alpha。

### 4.2 【事实】三个「可约靶子」被逐个 L3 判为不可约（2026-06-24）

`#164` 声明四个可约靶子（R1–R4），实测：

- `e822bcfab0`（R4）：「**TC 瞬时 churn 频率不可约** — L3 否证 #164「频率可约」声明（否定性结果，**无修复实装**）」。铁证：TC 平仓触发源 100% 是否定线止损，`reason=0`（信号反转 churn）命中 **0%**；唯一能区分亏损腿与等量盈利腿的特征 = 下一 bar 价格方向 = look-ahead = 非法。
- `86f4c14269`（R2）：「T3 出场 leak 可约性诊断 — **修复 L3 否证（出场侧不可约）**」。实验变体 8 标的 L3 全否证（T3 不降反增 +0~+76），**修复逻辑被整体回退**（`enable_t3_exit_on_flip` 全删）。
- `cca134d918`（R3）/`ecbf7f5557`（R1）落码，但 message 未给 L3 净正收益。

净效果：**四个可约靶子实测剩三个，两个被证「不可约」，修复实装被回退**。commit message 自己写「修正 #164 §4：TC「频率可约」→「不可约」；四可约靶子实为三」。

### 4.3 【事实】收益侧从未超越基线

`docs/rec_t_8x3_l3_verification.md:16-27`（8 标的 × 3 模式 = 24 组，vs Buy-Hold）：

> **超 BH 仅 1 组（CL/OR）**。7/8 标的全模式 < BH。

同表：BTC 引擎 +37.3% vs BH +1380.4%；ES +7.1% vs +594.3%；GC −28.3% vs +257.3%。报告自己标注「7/8 标的全模式 <BH = 已知基线性质（确认滞后税 + 强牛踏空，非 bug）」。

**【推断】** 该报告的诚实标注（`:27`「超长上行数据上『无 alpha』结论信息增量低」）说明当时并未把这当成否决——但它也确实从未提供过「取代原有」的收益理由。

### 4.4 【事实】方法论换轨——旧引擎的迭代方式在新协议下不合法（2026-06-25，主因）

同日落地的 `docs/backtest-protocol-v0.md:3-4`：

> **状态**：预注册（pre-registered）。本文档在**看任何回测结果之前**冻结。
> 任何在看到结果后才提出的判据、阈值、品种排除、时段裁剪一律**无效**。

`:296` 更进一步：

> 在看到回测结果之后才追加的、放松失败判据/排除不利品种/裁剪不利时段的 change-request，**本身就是事后解释**……必须显式标注 "post-hoc" 并在结果报告中作为方法论污点披露。

而 recursive_t 的整个开发方式，从 commit 序列上看，**正是被这条禁止的形状**：2026-06-18 至 06-25 的 60+ 个 commit 里，`cc017e4e61`（「BTC +1424% 首超 BH（+1380%）」）、`70948a9605`（「递归归还子树（−238%→+502%）」）、`fc3a5f680e`（「−848%→−175%」）——**改代码 → 看收益 → 再改**，收益数字直接写进 commit message 作为验收。

同时，`docs/reference-theta-v0.md:5` 给出了新的合法性链条：

> 缠论结构公理 + Θ ⟹ C_Θ ⟹ π_Θ ⟹ **Rust 实装** ⟹ L2/L3 检验（可证伪）。

新线要求实装**逐字段锚到 Lean `Origin.*`**（`theta_v0/classifier/center.rs:5-16` 每条判据都写着契约锚）。recursive_t 没有、也没人给它补这层锚。

**【推断】** 这一条是主因：它不是被判「跑得不好」，而是被换掉了**验收语法**。旧引擎的全部产出（那 24 组收益读数）在新协议下既不能作为通过证据，也不能作为失败证据——它们是未预注册的。

### 4.5 【事实】然后就没人接着做了

- recursive_t 最后一个功能 commit：`a0c879b9c7`（2026-06-25），且是「**全 env-gated OFF**」的 instrumentation。
- 此后仅 4 次触碰，全部是外部推进：`77040317f2`/`36092eab8e`/`3186bd104d`（2026-07-26，跨实现教义订正）、`4d993bd646`（2026-07-29，全仓 rustfmt）、`c1f1cb5e7f`/`bd5666c8c6`（2026-07-30，名分注释）。**零功能演进。**
- 6 个 python 调用方最后修改日期全部在 2026-06-18~06-24（见 §5）。

### 4.6 与编排者定性的对照

**【事实】不冲突，但需要一处订正**：编排者定性「失败」成立（它没有取代任何东西、核心假设被证伪、收益不超基线）。但**「不成熟的尝试」这个措辞与证据有出入**——它并非半途夭折：

- 它跑完了 8 标的 × 3 模式 × 约 1500 万 bar 的 L3 验证；
- 它接进了 NautilusTrader 生产回测；
- 它在四个 prove 守卫上做到 8 标的零 panic，把 L0 不变量的有效域从 L2 提升到 L3（`docs/rec_t_8x3_l3_verification.md:12`）；
- 它自己把自己的核心假设证伪了，并且**照实记录了否定性结果、回退了无效修复**（`e822bcfab0` / `86f4c14269` 均标「无修复实装」「回退修复逻辑」）。

**【推断】** 更准确的描述是：**一次执行到位、方法论过时的实验**——失败在验收语法与市场假设，不在工程完成度。这一区别对 #799 有直接后果：不能用「上次不成熟」来给这次的自相似尝试打折。

---

## 5. 它现在被谁在用

**【事实】** 名分与调用面（#762 报告 `.chanlun/review-results/issue762-impl-20260730.md` §2.3/§2.4 已核，本票复核一致）：

| PyO3 导出名（`lib.rs:2694-2703`） | 走哪支 | python 调用方 | 调用方最后修改 |
|---|---|---|---|
| `run_recursive_t` | (a) standalone T | `analysis/t_vs_v3_comparison.py:65` | 2026-06-18 |
| `TFugueStream` / `run_t_fugue` | (b) flat 支 | `trading_system/backtest_t_fugue.py:166` | 2026-06-21 |
| `RecTStream`（`ffi.rs:291` `#[pyclass(name="RecTStream")]`） | (b) rec 支 | `trading_system/strategy/rec_t_strategy.py:45`（**NT Strategy**）、`analysis/capture_ratio_matrix.py:84`、`analysis/t3_exit_trigger_diag.py:35`、`backtest/rec_backtest.py:8` | 2026-06-21 / 2026-06-24 / 2026-06-24 / 2026-06-21 |

**路径性质**：`rec_t_strategy.py` 是 NautilusTrader Strategy 类（生产路径形态），其余五个是研究/诊断脚本。

**与 theta_v0 调用方的重叠**：**【事实】零重叠**。census `:23` 实证 `theta_v0` 对五族（含 recursive_t）**零引用**；反向亦然——`grep "crate::theta_v0" rust/src/recursive_t/` 无命中。两套走完全不相交的入口（theta_v0 走 `theta_backtest` bin / `wverify_run` harness）。

**要点（不要误读「现役」）**：**【事实】** 「现役」是 ADR-0004 的**机械判据**（有非测试调用者 ∧ 无 `#[deprecated]` ∧ 在 main 上），不是「在被使用」。全部 6 个调用方冻结于 5 周前，且其中 4 个（`t3_exit_trigger_diag` / `capture_ratio_matrix` / `t_vs_v3_comparison` / `rec_backtest`）是当年那批被证伪实验的诊断脚本本身。

唯一仍判 `deprecated 待退役` 的是 `backtest.rs` + `backtest_run.rs`（约 1400 行，`#[cfg(test)]` 门控，生产零可达）。

---

## 6a. 它是不是模块化的（编排者追加，先答此层）

**【事实】** 必须分两层答，两层结论相反。

### 6a-1 信号层（standalone T，2611 行，6 文件）——模块化成立

| 判据 | 证据 |
|---|---|
| 模块边界 | 一层一文件，依赖单向且无环：`operator.rs:16-19` 只 `use` `center` / `divergence` / `trend` / `types`；`center.rs` / `trend.rs` / `divergence.rs` 互不反向依赖 |
| 输入输出契约 | `operator.rs:94` `pub fn apply_t(units: &[Unit], level: usize, mode: PerfectionMode) -> TLevelOutput` —— 显式、值语义、无 `&mut self` |
| 判定 vs 调度分离 | **分离**。判定在 `center.rs:85` / `trend.rs` / `divergence.rs`（纯函数，零状态）；调度在 `mod.rs:96 iterate`（只做 `loop { apply_t; 终止判断; 升级 }`，28 行，`mod.rs:96-128`）。`iterate` 不含任何缠论判据 |
| 级别是参数还是分支 | **是参数**。`apply_t(units, level, mode)` 的 `level` 只作为**输出标签**流入 `Zhongshu.level` / `BSP.level` / `Unit.level`；`grep "level ==\|level >\|level <" center.rs trend.rs operator.rs types.rs` 命中 **0**。全目录仅一处按 level 分支：`divergence.rs:389 let diag_down = t.direction == Direction::Down && t.level < 9;`，且是**诊断计数开关**，不参与判定 |
| 可单独测试 | **可以**。`operator.rs:133-233`、`center.rs`、`divergence.rs:700-1180` 的单测直接手搓 `Vec<Unit>` 调用被测函数，**不需要引擎、不需要数据文件、不需要 env**。`operator.rs:190` 甚至有专门的纯性测试 `apply_t_是纯函数_不改输入` |
| 可单独删除（deletion test） | **不可**，但方向相反：#761 核查证实删 (a) 会立刻打断 (b) 编译（`stream.rs:225,384` / `rec_stream.rs:241,313,353,375,644,688` / `backtest.rs:329` 直接调用）。即 (a) 是干净的**下层**，(b) 依赖它、它不依赖 (b) |

### 6a-2 操作层（T 引擎，12062 行，10 文件）——模块化**不成立**

| 判据 | 证据 |
|---|---|
| 抽象边界（trait / 泛型） | **零**。`grep "^pub trait\|^trait "` 全目录命中 **0**；唯四个 `impl` 全是 `impl Default`（`rec_engine.rs:537,932`；`t_engine.rs:104,253`）。无泛型函数（除 PyO3 生命周期参数）。两支（flat / rec）**不是**同一 trait 的两个实现，而是**两份互相抄写的具体代码**——`rec_engine.rs:36-44` 自述是「flat 的**逐行对照**重新实装」 |
| 单体规模 | `rec_engine.rs` 3665 行 / 135 个 `fn`；核心结构 `struct TRoot`（`rec_engine.rs:941`）**36 个字段**，含仓位 (`instances`)、战役状态 (`stage`/`core_cost_basis`)、会计、守卫、以及大量诊断计数器（`buy_sink`/`buy_recover`/`buy_noop`/`buy_core`/`flip_log`/`n_flips`…）全塞一处 |
| 判定 vs 调度 vs 会计分离 | **缠在一起**。`rec_engine.rs:1890 fn route_bsp` 一个函数里同时做：角色判定（`nearest_active_parent`）、策略选择（sink/drain/recover/add）、门控（`enable_orbit9_nodevec` / `enable_orbit9_dispatch`）、诊断埋点（`self.buy_sink += 1` 等 4 处内联计数器）。`rec_stream.rs:199 push_bar` 单函数跨 199→612 共 **413 行、63 个分支点**，同时负责喂 bar、重跑信号塔、投影视图、驱动引擎 |
| 变体是模块还是开关 | **开关**。`struct EngineConfig`（`rec_engine.rs:135`）**22 个字段**，几乎全是 `enable_*: bool`；`rec_engine.rs` 内 `std::env::var` **24 处**。每个新假设（读法乙 / LegPair / 牛熊对称 / Face B 定仓 / 9 轨道分派 / 节点向量路由 / 三种开空触发器 / 三种开多门控…）都是**塞进同一批函数的 `if flag`**，而非独立模块。文档反复出现的验收句式「`OFF ⇒ … 逐字不动（bit-exact）`」（如 `rec_engine.rs:157`「`false ⇒ g_pair 行为与 committed #69 逐字一致（bit-exact）`」、`:257`「`OFF=false ⇒ … 逐字 bit-exact`」）正是这种形状的自白 |
| 级别是参数还是硬编码分支 | **既不是纯参数，也不是 `match level`**：级别是**数组下标 + 全局硬顶**。`rec_engine.rs:943 instances: Vec<TInstance>`（按 level 索引）＋ `rec_engine.rs:81 pub const MAX_LEVEL: usize = 8`；flat 侧同构 `t_engine.rs:214 layers: Vec<Layer>` ＋ `[bool; MAX_LADDER]` 定长数组（`t_engine.rs:73,75,79,81`）。**没有 `match level { 0 => … }` 这种形状**——这一点票面担心的最坏情况**不成立** |
| 可单独测试 | **弱**。`rec_stream.rs` 的测试（`:803-1554`）几乎全是端到端驱动（`fn drive(s: &mut RecStream) -> f64`）＋ bit-exact 对拍，多数带 `#[ignore]` 并依赖 `analysis/data_cache/*.json` 真实行情。无法在不构造整个 `RecStream` 的前提下单测 `route_bsp` / `sink` / `recover` |

**6a 结论**：**【事实】** `recursive_t` 是**半模块化**的——判定层（信号）做到了模块化的全部四条判据；操作层没有做到任何一条抽象边界判据，是一个 36 字段的单体状态机 ＋ 22 开关的变体矩阵。

**【推断】** 这个半分裂本身有解释：信号层是从设计文档（`unified_recursive_operator_T.md` §1.3 步骤 a/b/c/d）一次性推导出来的；操作层是 60+ 个 commit 一路试错长出来的，每次假设被否都留下一个 `enable_*` 开关（因为 bit-exact 回归锁要求旧路径逐字不动，删不掉）。**开关只增不减是 bit-exact 验收纪律的直接副产品。**

---

## 6b. 它有没有真的做到自相似

（6a 判信号层模块化成立 ⟹ 对信号层此问可答；操作层 6a 判不模块化，但它**不是**票面担心的 `match level` 形状，级别仍是下标而非分支，因此仍可作有限回答，下面显式标注这层限制。）

### 6b-1 信号层：**做到了，而且保留至今**

**【事实】** 三条独立证据：

1. **同一份代码作用于每一级**。`mod.rs:96-124 iterate` 的循环体只有一句 `let out = apply_t(&current, k, mode);`——`k` 从 0 递增，函数体不变、`mode` 不变。上一级的 `next_units` 直接喂下一级（`mod.rs:117 current = next`）。
2. **判定函数内无按级别分叉**。见 6a-1：`center.rs` / `trend.rs` / `operator.rs` / `types.rs` 对 `level` 的比较**零命中**；唯一命中 `divergence.rs:389` 是诊断计数门。
3. **有一条专门为此写的回归测试**。`operator.rs:198` `fn apply_t_泛型_同一套代码作用于level1单元()`，注释逐字：「验证「所有级别用同一套代码」：把 level-1 单元（带 `inner_zhongshu_count`）喂给**同一个** `apply_t`，走嵌套深度档背驰，产出 level-2 结构」。

**一处必须照实标注的裂缝**：**【事实】** 判定确实有一个二分支——`divergence.rs:347 let has_nest = t.units.iter().any(|u| u.inner_zhongshu_count > 0);`，`has_nest` 为真走第 37 课**五条件全检（真背驰）**，为假走**条件 1+4+力度（类背驰）**（`divergence.rs:334-336`）。它按**数据属性**分叉，不按级别索引分叉——但**实践上 level 0 的 a₀ 单元恒有 `inner_zhongshu_count = 0`（`types.rs:147`），level≥1 的封装单元恒携真中枢数（`operator.rs:42`）**，所以这个数据分支在实际运行中与「level 0 vs level≥1」一一对应。

**【推断】** 这是「用数据表达级别差异」的正确形态（升级不需要改代码、深度可涌现），但它同时说明：**即使在最干净的信号层，第 0 级也确实是特殊的**——不是因为架构妥协，而是因为 a₀ 由分型/笔/线段确认、没有次级别中枢可数。这与立项文档 `:40` 自己划的边界（「a₀ 内部用包含处理、a₀ 之上用重叠，是两套合并算子」）完全一致。

### 6b-2 操作层：**尝试过 → 被 L3 打回 → 编排者裁决回退**

**【事实】** 完整的三段式留痕：

**① 尝试**（2026-06-18，`aa45a76d1f`），commit message 逐字：

> 把 T 算子的形式不变性 aₙ=f(aₙ₋₁)（第65课）**从信号层延伸到操作层**：每个级别独立运转完全相同的操作逻辑，**没有 root 方向、没有 core 核心仓、没有「最高级别给方向」**。同一个函数 `act(k, want)` 对每个级别调用 = T 在操作层的自我复制。
> 删除（no-patch-mentality 删除而非保留）：`root_direction` / `core_polarity` / `core_ladder` / `core_emergent_ladder`。

**② 被 L3 打回**（2026-06-21，`docs/recursive_t_deadlock_escalation.md` §8/§9）：

纯递归（`level = 父 − 1` 连续下钻、无跳级、无 root 特权）在 3 标的实测 `sink = 0`、`short_pnl = 0`——做空腿**从未激活**（§8:74）。穷尽验证 4 角度全 `infeasible`（§9:103）：

> **不是 candidate 定义问题（AND/MACD/Structural 都不解），是 C1（core 持多骑牛升最高涌现永不见顶）与「sink 必须 core 级别确认」的内在矛盾。** rec 连续架构（level=父−1 不跳级）下做空腿**结构性不可能**激活。

**③ 编排者裁决回退**（2026-06-21，同文档 §10:123）逐字：

> 编排者否定 A/B/C/D，给出**第五方案**：「对照 flat 引擎，把它的逻辑递归化。**不要自创约束。**……逐行对照 flat `route_bsp` 用 `TInstance`/`TRoot` 重新实装。」

回退后的形态（`rec_engine.rs:36-44` 自述逐字）：

> flat `layers: Vec<Layer>`（**绝对 ladder 数组**）→ `instances: Vec<TInstance>`（按 level 索引）。
> flat **单核心 campaign 三阶段**（`stage`/`core_cost_basis`/`withdrawn`/`earning_cash`）→ 在 `TRoot`（**非 per-instance，per-instance 三阶段是递归自创，已删**）。

**结果**：**【事实】** 死锁解除，`sink` 从 0 升到千次级，且与 flat **bit-exact 复现**（§10:134-137）。同文档 §10:139 编排者结论：「第五方案证明前述『C1 vs 级联到 core』矛盾是**伪矛盾**——它建立在『C1 永不翻空』**自创前提**上。」

**回退后操作层的自相似程度（精确读数）**：**【事实】**

- ✅ **一份代码管所有级别**：`rec_engine.rs:1890 route_bsp(j, is_buy, node, c)` 对任何 `j` 调用同一函数体，无 `match j`。
- ❌ **但存在根角色特权**：`route_bsp` 按 `nearest_active_parent(j)` 是否为 `None` 二分（`:1892`）——有活跃祖先走 sink/drain/recover/add（`:1894-1938`），无活跃祖先（= 核心级）走 enter/ascend/**flip**（`:1940-1968`）。**只有核心级能翻转**（`:1959` 注释「核心**可翻空**」）。
- ❌ **绝对上界回来了**：`MAX_LEVEL = 8`（`rec_engine.rs:81`），且其取值理由是「= flat `MAX_LADDER(11) − BASE_LADDER(3)`」——纯粹为了与 flat 对拍，不是数据涌现。这与信号层的「深度由数据涌现、`k>64` 只是安全阀」（`mod.rs:120`）**在同一个 crate 里自相矛盾**。
- ❌ **仓位战役是单例**：三阶段（降成本/退本金/增股数）挂在 `TRoot`（`rec_engine.rs:949 stage` / `:951 core_cost_basis`），**per-instance 版本被明文删除**。

**【推断】** 角色特权（`Some(p)` vs `None`）是**动态角色**而非静态级别硬编码——同一个 level 在不同时刻可以是核心也可以是子级。所以操作层不是「按级别分了叉」，而是「按**在树中的位置**分了叉」。这在结构上比按级别分叉干净得多，但它确实**不是**立项时想要的「每级别逐字相同」。

---

## 7. 对 #799 的三格定位（照 6a/6b 矩阵）

**【事实】** 本次实验落在**两格**，因为两层结论不同：

| 层 | 6a 模块化 | 6b 复用 | 落格 |
|---|---|---|---|
| 信号层（2611 行） | ✅ 成立 | ✅ 复用了 | **「模块化了、复用了」⟹ 失败原因不在自相似上** |
| 操作层（12062 行） | ❌ 不成立（单体 + 22 开关，但**非** `match level`） | 尝试过→回退，现为「一份代码 + 动态根角色 + 绝对上界 8」 | **「不模块化」⟹ 这一层的功课是模块化本身** |

**【推断】** 因此 #799 的问题形状建议按层重问，而不是整体问「架构该不该递归自相似」：

- 在**结构判定层**（中枢/走势/背驰/级别构造），自相似**已被本仓实证做到过**，代价可控（2611 行、纯函数、可单测、级别是参数），且它不是失败的来源——失败来自市场假设（type1 实时无 alpha）与验收语法换轨。
- 在**仓位/操作层**，本仓有一次带 L3 铁证的否定性结果：纯递归（连续下钻、无根特权）会让做空腿结构性不可能激活；且三阶段资金战役被裁定为**单例语义**、per-level 复制已被删除。这一层若要重来，需要先回答的不是「能不能自相似」，而是「三阶段战役是不是本来就只有一个」。

---

## 8. 未能判定 / 缺什么才能判

1. **【未能判定】** recursive_t 是否被明确「停止开发」过。全仓无一份「停做 recursive_t」的裁定/交接文档；2026-06-25 的 codex 裁决书通篇未提它。要判需要该日期前后的会话 transcript（`.kimi-code/sessions/` 或当时的 CC transcript），本票范围内未查。
2. **【未能判定】** 性能是否曾构成阻碍。无 recursive_t 侧的 profile 报告；`docs/` 下的性能票（#60/#65/B2/B3/B4）全部针对 theta_v0。要判需要一次同口径对拍（同数据、同 bar 数、墙钟）。
3. **【未能判定】** `rec_t_strategy.py` 是否真在实盘/定期回测中被跑过（还是仅存在于仓库）。要判需要运行日志或 CI 记录；`.github/workflows/` 未见其入口。
4. **【事实但需下票承接】** 两套实现的第二类买卖点定义冲突（§2.1）目前无人裁——recursive_t 的「跨级投影」与 theta_v0 的「1 类后未破中枢」不等价，而 #322/#338 那批教义票只扫了中枢延伸口径，没扫买卖点定义。

---

## 9. 结果包六要素

1. **结论**：见 §0 速查表。核心两条——(a) 失败原因**不在**「自相似做不做得到」，信号层做到了且保留至今；(b) 卡点是「核心可交易假设被自证伪 ＋ 验收语法在 2026-06-25 换轨（预注册协议 + Lean 锚），旧引擎全部读数在新协议下既不能算通过也不能算失败」，随后无人接手。
2. **定义依据**：commit 锚 `5856c191a1`/`aa45a76d1f`/`066ebcbea9`/`cc108f6c53`/`e822bcfab0`/`86f4c14269`/`bd5666c8c6`；代码锚 `rust/src/recursive_t/{mod.rs:96, operator.rs:94, center.rs:85, divergence.rs:347,389, rec_engine.rs:81,132,941,943,949,1890, rec_stream.rs:199}`、`rust/src/theta_v0/classifier/{center.rs:1-16, divergence.rs:1-30, bsp.rs:1-18, level_state.rs:1-16, recursive_tower.rs:1-30}`；文档锚 `docs/unified_recursive_operator_T.md:1-40`、`docs/recursive_t_deadlock_escalation.md:74,103,123,134-139`、`docs/session_handoff_2026-06-20_t_redesign_and_falsification.md:16-22,44,146-157`、`docs/rec_t_8x3_l3_verification.md:12,16-27`、`docs/backtest-protocol-v0.md:3-4,296`、`docs/reference-theta-v0.md:5,27-30`、`docs/agents/generation-constitution.md:35`、`.chanlun/review-results/legacy-generation-census-20260729.md:23,54-70`、`.chanlun/review-results/issue762-impl-20260730.md:§2.3-2.4`。
3. **边界条件**：本报告只读 `docs/grill-with-docs-entry` 线的仓库状态（`5984ab0d8e` 起）；未跑任何回测、未编译；§8 四条为未判定项。若发现 2026-06-25 前后存在未入仓的裁定记录，§3/§4 的「无人拍板」判断需按其订正。
4. **下游推论**：#799 的问题形状建议按「结构判定层 / 仓位操作层」分开裁（§7）；#789 的重复实现普查应把「第二类买卖点定义冲突」补为一条待裁项（§8.4）。
5. **谱系引用**：`project_recursive_t_architecture_v2`、`project_unified_recursive_operator_T`、`project_t_engine_btc_baseline`、`project_unified_architecture_map_787`、`project_interval_nesting_not_called_in_backtest`；票据 #761/#762/#763/#787/#789/#799。
6. **影响声明**：**零代码改动**。新增本报告一份文件。未动生产代码、未重构、未给实现建议。
