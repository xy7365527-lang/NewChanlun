# Short 侧亏损归因审计（Long +23,826.69 / Short −16,465.00 的不对称来源）

- **审计工位**：只读审计 wave（分支 `kimi-nest-mainline-20260717`，worktree `/tmp/kimi-nest-mainline`）
- **数据源**：`/tmp/m8_opsem_fixed/trades.jsonl`（518 笔，M8 修复后重跑产物）；行情桥接 `analysis/data_cache/btc_1m_full.json`
- **审计日期**：2026-07-19
- **纪律**：090 严格否定；v3 硬禁令（不引入概率/统计推断作决策基础、不用回测验证策略、不假设 EMH）；本文件仅写 `.md`，未改 `rust/src` 一行，未做 git mutation，主仓只读。
- **pnl 口径声明**：`pnl_raw_unlevered = delta × (exit_px − entry_px)`（delta=+1 Long / −1 Short），是**每单位价差、费前、未乘 units**（runner.rs:2454 文档注释「费前方向盈亏……未扣费」；runner.rs:2464-2466 `pnl_raw = delta_sign * (t.exit_px - t.entry_px)`）。所有「占比」都是这个口径下的算术分解，不是账户 NAV 归因。units 两侧对称（§3.5），该口径不扭曲方向间比较。

---

## 0. 总量与桥接验证

| 指标 | Long | Short |
|---|---:|---:|
| 笔数 | 284 | 234 |
| pnl 合计 | **+23,826.69** | **−16,465.00** |
| 胜率 | 53.9% | 41.9% |
| 赢家 n / 均值 | 153 / +286.2 | 98 / +211.3 |
| 输家 n / 均值 | 131 / −152.4 | 136 / **−273.3** |

- 样本 bar 区间 [596, 264959] 为 **wf8 窗口局部索引**，全局偏移 3,146,239（`btc_1m_full.json`），对应 2023-08-17 09:56 .. 2024-02-16 23:59。桥接逐位验证：`entry_px == closes[3146239+entry_bar]` 抽查逐位相等。
- **窗口事实**：wf8 内 BTC 28,721.77 → 51,880.00（+80.6%），强单边上行。
- 不对称是**双因子**的：Short 胜率低 12 个百分点（频率因子），且 Short 输家均值是 Long 输家的 1.79 倍（幅度因子，−273.3 vs −152.4）。

---

## 1. Short 亏损分布表（交叉统计）

### 1.1 side × exit_type（核心表）

| side | exit_type | n | pnl 合计 | 均值 | 胜率 |
|---|---|---:|---:|---:|---:|
| Long | CloseRoot | 75 | +10,185.52 | +135.81 | 52.0% |
| Long | CloseShortDiff | 110 | +2,508.90 | +22.81 | 53.6% |
| Long | ReduceCore | 99 | +11,132.27 | +112.45 | 55.6% |
| **Short** | **CloseRoot** | 68 | **−11,721.32** | −172.37 | 39.7% |
| **Short** | **CloseShortDiff** | 94 | **−8,623.61** | −91.74 | 40.4% |
| Short | Hold | 1 | −162.31 | −162.31 | 0% |
| **Short** | **ReduceCore** | 71 | **+4,042.25** | +56.93 | 46.5% |

D3 复现确认：Short 在 CloseRoot/CloseShortDiff/Hold 全亏（合计 −20,507.24），仅 ReduceCore 幸存（+4,042.25）。

### 1.2 side × exit_type × level（pnl 合计）

| side | exit_type | L0 | L1 | L2 | L3 |
|---|---|---:|---:|---:|---:|
| Long | CloseRoot | −1,400.86 (42) | +708.98 (9) | −108.45 (1) | **+10,985.85 (23)** |
| Long | CloseShortDiff | +2,744.03 (91) | +325.59 (14) | −560.72 (5) | — |
| Long | ReduceCore | +10,256.13 (92) | +499.96 (6) | +376.18 (1) | — |
| Short | CloseRoot | −7,920.51 (55) | −3,800.81 (13) | — | — |
| Short | CloseShortDiff | −8,298.34 (83) | −325.27 (11) | — | — |
| Short | Hold | −162.31 (1) | — | — | — |
| Short | ReduceCore | +5,261.03 (70) | −1,218.78 (1) | — | — |

（括号 = 笔数。）Short 只落 L0/L1：L0 −11,120.13（67.5%）、L1 −5,344.86（32.5%）。Long 利润近半来自 L3 CloseRoot 23 笔 +10,985.85——Short 侧**根本没有 L3 仓位**，这是级别维的不对称（Short 大级别空头信号在本窗口未生成/未准入，原因不在本审计范围）。

### 1.3 Short × bsp_class_min（entry 质量维）

| class | n | pnl | 均值 | 胜率 |
|---|---:|---:|---:|---:|
| cls1（一卖） | 5 | −1,540.13 | −308.03 | 60.0% |
| **cls2（二卖）** | 85 | **−11,776.97** | −138.55 | 43.5% |
| cls3（三卖） | 144 | −3,147.89 | −21.86 | 40.3% |

对照 Long：cls2 107 笔 **+9,884.71**、cls3 177 笔 **+13,941.98**。**同一 class、反方向，盈亏完全镜像**——class 本身无方向性 alpha 差异，方向才是变量（与 m8-opsem-trades-breakdown-20260719.md §②b D2 一致）。Short cls2 是最大单一亏损组合（71.5% 的 Short 亏损）。

### 1.4 Short × 角色垂直轴 V（certificate.role 第二分量；语义见 coverage.rs:959-968）

| V 轴 | 语义 | n | pnl | 占 Short 亏损 |
|---|---|---:|---:|---:|
| **FollowParent** | 跟随父方向的级联核心仓（σ_p=−1，父亦空头） | 98 | **−12,453.12** | 75.6% |
| Ambient | 独立根（σ_p=0） | 88 | −2,974.34 | 18.1% |
| ShortDiff | 父多子空对冲腿（σ_p=+1） | 48 | −1,037.53 | 6.3% |

最坏单元：`SameReverse|FollowParent|Minus` × L0 × cls2 = 27 笔 −9,409.34（Short 总亏损的 57.1%）。对照 Long 侧 V 轴：Ambient +13,971.58 / FollowParent +5,739.19 / ShortDiff +4,115.92——**三个 V 轴仓位族全部盈利**，包括 ShortDiff 反弹腿（父空子多）。

### 1.5 Short × 退出触发（trigger_bsp_class_at_exit）

| 触发 | 含义（reverse_exit_type，interp.rs:250-258） | n | pnl |
|---|---|---:|---:|
| trig=1（买一） | 最深结构确认 → CloseRoot | 5 | −4,229.27（均值 −845.85） |
| trig=2（买二） | 深结构确认 → CloseRoot | 44 | −7,413.94 |
| **trig=3（买三）** | 最早触发 → ReduceCore | 88 | **+3,860.28** |
| null（无 BSP 触发） | §13 AncOK 连带剪/Stale prune/窗口边界 | 97 | −8,682.06 |

trig=1/2（深确认）合计 −11,643.21；trig=3（最早确认）为正。**退出触发要求的结构越深，Short 亏得越多；唯一幸存路径是最早触发的那条。**

### 1.6 Short × via_anc_ok_prune

| prune | n | pnl | 胜率 |
|---|---:|---:|---:|
| false（真信号平仓） | 138 | −7,945.24 | 43.5% |
| true（父失效连带剪枝，无独立信号） | 96 | −8,519.75 | 39.6% |

对照 Long：prune=true 91 笔 **+6,077.75**。同一条「父失效机械平仓」通道，Long 侧实现的是浮盈、Short 侧实现的是浮亏——prune 在漂移场中是「按市价兑现当前浮动盈亏」的镜子，本身无方向缺陷。

---

## 2. 机制链审计（代码锚）

### 2.1 exit_type 语义单源

`ExitType ∈ {CloseRoot, ReduceCore, CloseShortDiff, RiskExit, Hold}` 定义于 `rust/src/theta_v0/strategy/interp.rs:223-235`，typed 判据单源 `reverse_exit_type`（interp.rs:250-258）：

- `entry_v == ShortDiff` ⟹ CloseShortDiff（P7：短差子声部反向确认关闭，**子声部关闭语义压过触发类**）；
- `trigger_class == 3` ⟹ ReduceCore（P6：三类反向点=核心仓减仓）；
- 否则 ⟹ CloseRoot（P5：一/二类反向点=根清仓）。

**关键澄清**：`CloseShortDiff` 的 "ShortDiff" 是**角色名**（短差对冲腿，V 轴），不是持仓方向；Long 持仓同样可经 CloseShortDiff 退出（本样本 110 笔）。该判据**不读方向参数**——输入只有入场角色 `entry_v` 与触发候选类，方向不对称在代码层不存在。

### 2.2 closePred 四析取——方向对称性证明

退出谓词 `X_{v,t} = ¬ParentValid ∨ χ^{σ_p}（反向信号）∨ Stop ∨ RiskClose`（`rust/src/theta_v0/strategy/exit.rs:16-18`，契约锚 `Origin.SubVoiceOpenClose.closePred` line 552-562）：

- `close_pred` = 四析取直或（`rust/src/theta_v0/strategy/exec.rs:242-244`），无方向项；
- `reverse_signal` 完全镜像：持多遇卖侧 BSP、持空遇买侧 BSP（exec.rs:255-261）；
- `stop_hit` 完全镜像：多头止损在下方（low≤stop），空头在上方（high≥stop，exec.rs:263-275 注释「镜像」）；
- **四析取中没有任何止盈/目标价项**——退出只有「你错了」（反向信号）、「父没了」（失效连带）、「止损」、「强平」四种理由，对多空对称地**没有「赚够了」这个退出通道**。

### 2.3 μ 管线 fill loop 的四条退出通道（runner.rs）

trades.jsonl 由 opsem-dump（`rust/src/theta_v0/backtest/runner.rs:2364-2396`，env-gated 只读外化）从 TypedTradeLedger 写出，退出通道四条：

1. **反向信号平仓**（runner.rs:1701-1728）：`exit_type = reverse_exit_type(entry_v, trig.bsp_class)`，`via_prune=false`，trigger=候选类；
2. **静默离场**（runner.rs:1732-1766）：§13 AncOK 连带剪/Stale prune，无触发信号（trigger=null），非 Ambient 角色归 CloseShortDiff、Ambient 归 CloseRoot，`via_prune=true`；
3. **RiskExit 强平**（runner.rs:1769-1795）：本样本 **0 笔**；
4. **P2 overlay 关闭**（runner.rs:1797+）：TW StageII 重叠腿，归 CloseShortDiff，trigger=null。

如实登记：**Stop 析取项在本窗口 518 笔中未产出一笔 RiskExit**（另有 22 笔 `entry_stop_dist=null`）——止损通道空转，本审计不定案（止损价构造 vs 窗口未触及），列为 §7 未裁定项。

### 2.4 双账本 fill 方向处理（trading/ledger.rs）

`rust/src/trading/ledger.rs` 的双账本（`DirectionalBook::{Long(OrganicLedger), Short(ShortBook)}`，ledger.rs:468-472）方向处理：

- **极性分离**：空头单位 `units` 恒正、独立成书，禁止 `total_shares` 取负 + 下游截断（ledger.rs:463-467「符号污染编译不可达」）；
- **ShortBook 单相单律**（ledger.rs:351-368，引 `analysis/bidirectional_nested_accounting.md` §2.2/§2.4 非对称定理，L0）：**空头侧不存在 EarningShares 不动点**——多头可以靠短差利润把成本推到 ≤0 相变为「负成本免费持仓」，空头损失无界（c→∞）该不动点在 L0 不可构造，类型层不可表示；
- **空头书内降成本腿恒为 DiffSide::Long**（ledger.rs:363-364, 427）：先买后卖做反弹，利润 π 使 `proceeds_basis += π/U`（协变公式 d=−1 面，ledger.rs:454-458）单调上移、**无相变**。

这是**设计层已声明的多空不对称**：多头的「持有等待」有相变终点（免费持仓），空头的持有只是负债暴露的时间累积。而 §2.2 的 closePred 对两侧对称地允许「持有到反向确认」——**设计层的不对称没有传导到 μ 管线的退出纪律**（详见 §5 对照与 §6-F4）。

口径注意：trades.jsonl 产自 theta_v0 μ 管线 TypedTradeLedger（§2.3），不是本双账本；双账本是 O0≡P5 路径与 p120 嵌套声部设计的账本层。两者方向语义一致（ShortBook 利润公式 `(sell−buy)×shares` 与 `pnl=delta×(exit−entry)` 同构），本节的用途是提供**设计层多空不对称的权威表述**，不是主张本样本经过 ShortBook。

---

## 3. 行情级归因分解（btc_1m_full.json 逐 bar 桥接）

### 3.1 恒等式与窗口事实

Short 亏损的恒等式分解：`pnl_Short = −(exit_px − entry_px)`，即**每笔 Short 的亏损恰好等于其持仓期价格上移**。全部 234 笔 Short 持仓期的方向调整漂移合计 = +16,465.00（恒等，无残差）。wf8 窗口 +80.6% 单边上行是这个漂移的背景场。

但恒等式只是重述问题：**为什么 Short 会在上行窗口中开出 234 笔、并持有到漂移兑现为亏损？** 这才是机制问题，以下四节分解。

### 3.2 MFE / 捕获率——「浮盈坐成亏损」的直接证据

对每笔交易取持仓期 [entry_bar, exit_bar] 内逐 bar 高/低价，计算方向调整最大有利 excursion（MFE，Short = entry − min(low)）：

| side | exit_type | n | pnl | MFE 合计 | 捕获率 pnl/MFE |
|---|---|---:|---:|---:|---:|
| Long | CloseRoot | 75 | +10,185.52 | 29,680.83 | +0.343 |
| Long | CloseShortDiff | 110 | +2,508.90 | 23,898.32 | +0.105 |
| Long | ReduceCore | 99 | +11,132.27 | 32,955.18 | +0.338 |
| Short | CloseRoot | 68 | −11,721.32 | 15,743.84 | **−0.745** |
| Short | CloseShortDiff | 94 | −8,623.61 | 19,939.26 | **−0.432** |
| Short | ReduceCore | 71 | +4,042.25 | 24,151.31 | +0.167 |
| **合计** | Long | 284 | +23,826.69 | 86,534.33 | **+0.275** |
| **合计** | Short | 234 | −16,465.00 | 60,141.21 | **−0.274** |

三个硬事实：

1. **全部 234 笔 Short 都有 MFE>0**（无一笔入场后从未有过浮盈；Long 侧反而有 1 笔 MFE=0）。市场给了 Short 合计 60,141 点的可用浮盈（单边上行中回调段的累计深度），实捕获为 −16,465——**捕获率 −0.274，与 Long 的 +0.275 几乎完美镜像反对称**。同一套对称退出代码，在漂移场中产出镜像结果：Long 的浮盈方向与漂移同向，持有=兑现漂移；Short 的浮盈方向与漂移反向，持有=把回调浮盈坐回亏损。
2. 输家 MFE 很小：Short 输家 MFE 中位数仅 **61.1 点**（≈0.2% 价格），均值 150.9；Long 输家中位 62.0、均值 123.8——两侧输家画像相似，差别在输家均值幅度（§0：−273.3 vs −152.4），即 Short 输家被漂移带得更远。
3. mfe_lag（最佳退出点到实际退出的 bar 数）：Short 输家均值 361 / 中位 168，Long 输家均值 277 / 中位 145——Short 在浮盈高点之后平均多坐约 30% 的时间。

### 3.3 入场确认滞后——「追跌」的结构性来源

- **入场局部分位**（entry_px 在 ±1440 bar [min low, max high] 中的位置）：Long 均值 **0.553**（中位 0.584），Short 均值 **0.440**（中位 0.423）。Short 系统性在近两日区间的偏低位入场——卖侧 BSP 确认需要顶部结构完成，确认落地时回调已走过大半，**入场即回调末段**。
- **入场前一周漂移**：Long 均值 +909.8、Short 均值 +638.9（两侧都在上行背景中开仓——卖侧信号在上行窗口的每个回调顶持续形成）。
- 组合效应：确认滞后（约 0.11 分位差）× 单边漂移 = Short 入场价结构性偏高（相对其后的回调终点），MFE 中位 61 点 ≈ 确认滞后吃掉的回调余量。**cls2（二卖）确认滞后最深，对应最大亏损组合（§1.3，−11,776.97）。**
- 声明：确认滞后是 BSP 证书的构造属性（买卖点需后续结构确认），对多空对称存在；它不是独立 bug，但在漂移场中对 Short 是「滞后成本 + 漂移成本」叠加，对 Long 是「滞后成本被漂移覆盖」。此项与 §3.1 趋势项**共线**，不可加（§4 声明）。

### 3.4 退出时点——「持有到确认」在对冲角色下结构性太晚

- **post-exit +1 周漂移**（方向调整，正=留守更优）：Long 均值 **+1,035.4**（中位 +580）——Long 退出普遍偏早，退出后一周市场继续上行；Short 均值 **−782.2**（中位 −436）——**Short 退出后一周市场继续上行，退出的方向判断被后验 vindicate**。即：Short 的亏损不是「退错方向」，而是**入场→持有整段漂移→确认后退出**的完整链条里，亏损在持有期已经铸成。
- 触发深度单调性（§1.5）：trig=1/2（深确认）−11,643.21 → trig=3（最早）+3,860.28 → 唯一幸存出口 ReduceCore 就是「最早的你错了信号」。**「越早跑越赚」的单调 pattern 本身就是机制证据**：如果对称的退出谓词在角色上有效，不应出现存活率随退出深度单调排序。
- 止损通道空转（§2.3）：518 笔 RiskExit=0——Short 在上行窗口中没有一笔被止损救出，「你错了」的判定全部让位给「结构确认」。

### 3.5 排除项

- **sizing 对称**：units 非零均值 Long 183.2 / Short 181.2；units=0 笔数 37 / 27（depth≥3 零 sizing，见 m8-post-fix-comparison-20260719.md §4）。sizing 不构成方向不对称来源。
- **prune 通道无方向缺陷**（§1.6）：同一机械通道 Long +6,078 / Short −8,520，差异来自被剪时的浮动盈亏方向，不来自通道逻辑。
- **手续费**：pnl 为费前口径（§0），费率对多空对称，不改变方向归因。

---

## 4. 归因裁定

三个归因因子及其机制论证（090：因子间共线，**不给伪精确正交占比**；给出的是可证的算术分量与机制链）：

**因子 A：窗口单边趋势（背景场，必要不充分）**
机制链：wf8 +80.6% 单边上行 ⟹ 任何「对称信号 + 持有到确认」的策略，Short 侧持仓期漂移系统性为正 ⟹ pnl 为负（恒等式 §3.1）。**反事实结构论证**：代码层 reverse_signal/stop_hit/closePred 全部方向对称（§2.2），若窗口反转为单边下行，同一机制将给出 Long 侧镜像亏损——不对称不在代码，在「对称机制 × 非对称漂移场」。**趋势是必要条件，但不充分**：它不能解释为什么 Short 开了 234 笔（信号层）、为什么浮盈 60,141 点捕获为负（退出层）、为什么 cls2 占 71.5%（确认滞后）。

**因子 B：信号/角色配置——逆势方向性空仓是最大算术分量**
机制链：卖侧 BSP 在上行窗口每个回调顶持续形成，对称的准入谓词（子声部.pdf p22 P4/P5，无也不应有趋势概念）照单全收 ⟹ V=FollowParent 级联空头核心仓 98 笔 −12,453.12（75.6%）+ V=Ambient 独立根空头 88 笔 −2,974.34（18.1%）。**注意**：设计承担对冲角色的 V=ShortDiff 腿（父多子空）只占 6.3%（−1,037.53）——亏损的 93.7% 来自**方向性空仓**，不是对冲腿。最坏单元 SameReverse|FollowParent|L0|cls2 = 27 笔 −9,409.34（57.1%）。

**因子 C：确认滞后 × 退出无获利通道（机制不对称的实装缝隙）**
机制链：①入场端确认滞后使 Short 追跌（分位 0.44，MFE 中位 61 点 ≈ 回调余量被确认滞后吃掉）；②退出端 closePred 无止盈项（§2.2），Short 只能「持有到买侧确认」——而买侧确认在抬升低点结构中系统性高于 Short 入场价（trig=1 均亏 −845.85，§1.5）；③止损通道空转（RiskExit=0）使「你错了」无任何快速通道；④三重叠加 ⟹ 捕获率 −0.274（Long +0.275 镜像）、「越早跑越赚」单调 pattern、唯一幸存出口 = 最早触发（trig=3）。**这是实装层真实存在的机制不对称：不是代码对 Short 有分支，而是对称代码把 Short 角色（回调段操作 = 有限寿命敞口）当成 Long 角色（趋势段操作 = 可持有等相变）处理。**

三因子关系：A 是场，B 决定敞口放在哪，C 决定敞口怎么收。B、C 都与 A 共线（同一漂移场的不同切面），故 75.6%/57.1% 等算术分量**不可相加为总解释**，照实声明。

---

## 5. 与设计文档的角色对照

### 5.1 Short 侧的设计角色（文档锚）

1. **对冲腿（ShortDiff 子声部）**：子声部.pdf p17「ShortDiff 要求 σ_u = −σ_v：父多，子空；父空，子多」；p22 P4「父 voice active 且证书反父方向，开 ShortDiff」。多空对冲.pdf p9-10：对冲腿是父仓的 overlay（H_t = −σ_parent·h_t），**机械对冲 standalone PnL 预期 = 零减成本**，其价值在「降低最大回撤/波动率/尾部风险……先锁定下跌，再保留反弹的路径依赖收益」（p10），正确绩效指标是 hedge efficiency（prevented loss / hedge cost），不是 standalone Sharpe；且 p9 明确「voice-capture score > 0 ⇏ NAV alpha > 0」「这两个目标不能混用」。
2. **顺势方向性空头**：V=Ambient 独立根（子声部.pdf p22 P5「无 active parent，开 ambient voice」）与 V=FollowParent 级联核心仓——设计角色是**父向为空时的顺势跟随/独立趋势仓**。
3. **呼吸/降成本（账本层）**：trading/ledger.rs:363-364 ShortBook 内降成本腿恒 DiffSide::Long（反弹段低买高卖）——空头书的「呼吸」是**书内做多反弹**，不是加空。
4. **多空持有价值不对称（L0 定理）**：trading/ledger.rs:354-368 空头无 EarningShares 不动点，「负成本免费持仓」对空头不可构造。

### 5.2 实装 vs 设计

| 设计角色 | 当前实装对应 | 判定 |
|---|---|---|
| ShortDiff 对冲腿（父多子空） | V=ShortDiff|Minus 48 笔 −1,037.53（均值 −21.6/笔） | **大致相容但考核错位**：量级与「零减成本」的对冲成本相容；但 trades.jsonl 毛腿记分把对冲腿孤立读出，未配对父仓同窗口计算 hedge efficiency（多空对冲.pdf p10）——违反 p9「两个目标不能混用」。「先锁定下跌」的锁定机制不存在（退出等买点确认 = 反弹已开始，§3.4），对冲腿的路径依赖价值实装层未兑现。 |
| 顺势方向性空头 | V=FollowParent 98 笔 −12,453 + V=Ambient 88 笔 −2,974 | **角色落空**：σ_p=−1 的 FollowParent 空头意为「父向为空的顺势级联」，但父根本身逆势于窗口漂移——顺势是结构内自洽概念（顺父），与窗口漂移无关。对称准入（P4/P5）无趋势过滤，这是**设计语义使然，非实装 bug**；其实装后果是 93.7% 的 Short 亏损集中在方向性空仓。 |
| 空头书内呼吸（DiffSide::Long 反弹腿） | μ 管线无对应物（trades.jsonl 产自 TypedTradeLedger，§2.4） | **不可考核**：双账本呼吸层不在本样本管线上，照实登记为归因盲区。 |
| 多空持有不对称（无不动点） | closePred 对称无止盈（§2.2） | **设计-实装缝隙**：L0 已证明空头「持有等相变」无终点，但退出纪律允许 Short 与 Long 一样「持有到反向确认」——不对称的持有价值没有配上不对称的退出纪律。 |

---

## 6. 修复设计（针对因子 C 的机制不对称；均不涉及概率推断/回测验证）

- **F1 角色-退出谓词绑定**：V=ShortDiff 对冲腿的退出锚定**对冲窗口的结构性终点**（父级回调段完成 = 段级否定），而非买侧 BSP 确认（确认落地时回调已反转，锁定失效）。设计锚：多空对冲.pdf p10「先锁定下跌，再保留反弹」；实装点：`reverse_exit_type` 的 ShortDiff 分支（interp.rs:251-252）与 closePred 析取项（exit.rs:16-18）之间增加角色条件。验收 = 机制一致性（对冲腿退出时点 ≤ 父级回调段终点），**不是 pnl 转正**（v3：不回测验证策略）。
- **F2 对冲腿配对考核（归因纪律）**：trades.jsonl 增加父仓配对字段（parent voice 同窗口 realized + unrealized），ShortDiff 腿按 hedge efficiency = prevented loss / hedge cost 读出；毛腿 pnl 孤立归因在报告层禁止用于对冲腿。锚：多空对冲.pdf p9-10。
- **F3 逆势空仓准入裁定**：SameReverse|FollowParent|Minus（79 笔 −11,300.98，最大亏损单元）逐笔裁定其设计身份——若属 P4 ShortDiff 对冲腿，改走 F1/F2；若属 P5 级联核心仓，则其「反父向」与 FollowParent（跟随父向）语义自相矛盾，属**角色错配开仓**，应在准入谓词层否掉（coverage.rs 角色三轴与 interp 准入的对齐审计）。此项需设计侧裁定，本审计不定案。
- **F4 空头持有上界**：将 trading/ledger.rs:354-368 的 L0 非对称定理传导到退出纪律——Short 持仓寿命以结构为界（所属回调段/父级否定先到先走），不得复用多头的「持有等相变」语义。实装点：closePred 增加角色条件的持有上界析取项（方向非对称是**已证定理的传导**，不是引入预测）。
- **F5 止损空转排查**：518 笔 RiskExit=0 + 22 笔 stop_dist=null——裁定止损价构造（risk.rs structural_stop）在本窗口为何从未触及或未构造，Stop 析取项是否实质空转。若空转，因子 C 的「你错了无快速通道」成立到止损层。

---

## 7. 边界与未裁定项

1. **单窗口**：仅 wf8（2023-08-17..2024-02-16 单边上行）。p3fold/wf7 两窗未做同口径分解；Short 在下行窗是否镜像盈利，本审计不外推（v3：不对未来样本外推，也不以单窗归纳机制对错——机制结论只依赖代码对称性证明与恒等式，不依赖窗口代表性）。
2. **RiskExit 空转原因**未裁定（§6-F5）。
3. **SameReverse|FollowParent 角色错配**待设计侧裁定（§6-F3）。
4. **L3 级别 Short 缺席**：Short 无 L3 仓位而 Long L3 贡献 +10,985.85——级别维不对称的生成/准入原因不在本审计范围（与 p107-level-funnel-audit 衔接）。
5. **毛记分 vs NAV**：本报告全部数字为毛腿费前口径（§0）；按多空对冲.pdf p1-2 净额不可识别定理，同单位反向腿在净额 NAV 下互相抵消——Short 毛亏 −16,465 不等于 NAV 层损失 16,465（本样本 units 非完全对冲，净头寸变化存在，但 NAV 层归因需净头寸过程 N_t，非本数据源所能）。
6. 与既有审计的关系：D2/D3 复现自 m8-opsem-trades-breakdown-20260719.md §②b/§4；χ 退化为方向门（拒 45.2% 全部 Short）的并行发现见 l3-econ-gate-filter-rate-20260719.md——χ 若接入将改变 Short 准入，与本报告因子 B 直接相关，但 χ 量纲裁定（Gap-1）未完成前不动准入。

## 附：方法与可复现

- 统计：Python 3 逐行解析 `/tmp/m8_opsem_fixed/trades.jsonl`，`collections.defaultdict` 按 side/exit_type/level/class/role/trigger/prune 聚合；行情桥接 `analysis/data_cache/btc_1m_full.json`（全局偏移 3,146,239，桥接逐位验证 `entry_px == closes[offset+entry_bar]`）。MFE/MAE 取持仓期逐 bar 高/低价的方向调整 excursion；入场分位取 ±1440 bar 局部区间；post-exit 取 +10080 bar（1 周）。
- 代码阅读：`rust/src/theta_v0/strategy/{exit,exec,interp,coverage,ledger}.rs`、`rust/src/theta_v0/backtest/runner.rs`、`rust/src/trading/ledger.rs`（全部只读）。
- PDF：`pdftotext -layout` 提取 `docs/formal-chain/多空对冲.pdf`（16 页）、`docs/formal-chain/子声部.pdf`（27 页），引用带页码。
- 未做的事（v3 纪律）：未做胜率显著性/夏普等统计推断；未做参数寻优；未做策略回测结论；未假设 EMH；未对未来样本外推。
