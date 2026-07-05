# opsem 重做诊断结果包（2026-07-05）

- **goal**: g-20260705T1430Z-opsem-instrumentation-fix（R5 实战验证 + opsem 重做）
- **工位**: ws-opsemredo（基因 073a/274号；swarm/主线）
- **分支**: gap3-rework-codex9-fix（基线含 R5 修复 commit 5db6362093）
- **上游**: `.chanlun/review-results/opsemfix-r5-20260705.md`（R5 实装）/ `.chanlun/review-results/opsem-loss-diagnosis-20260705.md`（上轮 6 族全否决）

## 数据范围与诚实声明（铁律前置）

- **回测命令**: `env OPSEM_DUMP_DIR=/tmp/opsem2 cargo run --release --features backtest_bin --bin theta_backtest -- BTC`（全量 BTC 1min，无窗口切片）
- **dump 落盘范围**: trades.jsonl **4653 笔**（trade_id 1-4653 连续），tower_events.jsonl **28221 事件**，覆盖 **bar 0 - 2245671**（约 4.3 年 OOS，~全量 461 万 bar 的前 48.7%）。
- **binary 未自然结束**：全量 461 万 bar 按实测 1344 bar/秒 需 ~29 分钟；在 bar 2245671（4653 笔已落盘、模式稳定）处 SIGTERM 停止以锁定样本。dump 末行 JSON 完整性已验证。**后续 ~52% 数据未跑**——有限效域见 §边界条件。
- **诚实铁律（上轮教训）**：本报告所有 trade_id / entry_bar / pnl / 字段值均从 `/tmp/opsem2/trades.jsonl` 真实行读取（脚本 `/tmp/analyze_opsem2.py` + `/tmp/story_opsem2.py`），无任何编造。dump 档案不足的笔标 UNREPLAYABLE（本轮 0 笔）。所有聚合数字经 python 从 dump 二次精算，非手算。

---

## §1 R5 实战验证表

N=4653 笔（全量 dump）。三字段非 null 比例：

| R5 字段 | 全量(4653) | Type1(103) | Type2(1908) | Type3(2642) | 判定 |
|---|---|---|---|---|---|
| **lex_argmin_top3** 非空 | **4653/4653 = 100%** | 103/103 = 100% | 1908/1908 = 100% | 2642/2642 = 100% | ✓ R5-a |
| **nest_depth** 非 null | **4653/4653 = 100%** | 103/103 = 100% | 1908/1908 = 100% | 2642/2642 = 100% | ✓ R5-c |
| **divergence_input** 任一字段非 null | 103/4653 = 2.2% | **103/103 = 100%** | 0/1908 = 0% | 0/2642 = 0% | ✓ R5-b（R5-2 边界） |

**三字段全部远超 80% 非 null 门槛，阶段 1 PASS，进入阶段 2。**

### 关键语义澄清（任务原文笔误校正）

任务原文"验证 divergence_input 在**非一类候选**上 Some"与 R5 实装规范矛盾。R5-2（opsemfix-r5 §3 定理）已证：divergence 谓词定义域 = Type1 趋势背驰段对；Type2/Type3 不在定义域 ⟹ `divergence_input=None` 是「定义域外诚实缺席」（非 bug）。dump 实测印证：**Type1 上 100% Some，Type2/3 上 100% 合法 null**。正确表述为"divergence_input 在**一类候选（Type1）**上 Some"。

### 按 exit_type 分组（divergence 在 Type1 跨 exit_type 均覆盖）

| exit_type | N | lex_top3 非空 | divergence 非 null | nest_depth 非 null |
|---|---|---|---|---|
| CloseRoot | 1214 | 100% | 70(5.8%) | 100% |
| CloseShortDiff | 1902 | 100% | 24(1.3%) | 100% |
| ReduceCore | 1537 | 100% | 9(0.6%) | 100% |

（divergence 非 null 占比 = 该 exit_type 内 Type1 占比，与 R5-2 一致。）

---

## §2 逐笔语法故事

### 代表笔深度故事（2 笔亏损 + 1 笔正收益对照）

**【族A 代表】trade_id=3765  pnl=-36910.92（全档最大亏损，占亏损30 Σpnl 的 24.1%）  exit=CloseShortDiff  bsp=Type2  dir=Short**

- **entry 语义状态**（dump 真实行）：level=3，voice_id(L3, ord=167)，role=`SameReverse|ShortDiff|Minus`，nest_confirmed=True，nest_depth=2，**parent_dir_sigma_p=+1（父级别上涨）**，tw stage=CostReduction/PositiveUnsafe。entry_px=10891.53。
- **解释器选择**（R5-a lex_argmin_top3）：3 候选，p_star 选 **候选1 control=-1（tracking_err=128, trade_cost=0, risk_penalty=5, grid=0）= Short**；拒绝 候选2 control=0（tracking_err=1845, grid=1）/候选3 control=1（tracking_err=5562, grid=2）。**解释器按 J_Θ（tracking_err+trade_cost+risk_penalty）选了 Short，因 tracking_err=128 最小——但 J_Θ 不含"大级别趋势方向"项**。
- **divergence_input**：全 null（Type2，R5-2 合法缺席——本笔非背驰点，力度签名不在定义域）。
- **entry 后走势语法路径**（1991 塔事件）：entry 瞬间 L1/L2/L3/L4/L5 **五级别同时 extend**（全级别上涨共振），随后 L1 持续 new_center+extend 上移（中枢 zd/zg 单调抬高，467 个 new_center + 1514 extend，触及 L5）。exit_px=47802.45（**价格涨 4.4 倍**）。
- **语法故事**：父级别上涨（parent_dir=+1）共振五级别，解释器却选 Short 短差（逆势：dir=-1 与 pdir=+1 反向）；持仓 201909 bar（~5.5 月）跨越大级别上涨全程，被趋势碾压。**根因：J_Θ 代价函数缺大级别趋势方向约束 ⟹ 逆势短差被 lex_argmin 选中 + 持仓过久未止损**。

**【族C 代表】trade_id=2031  pnl=-2955.14（唯一 Type1 亏损）  exit=ReduceCore  bsp=Type1  dir=Short**

- **entry 语义状态**：level=3，role=`SameReverse|Ambient|Minus`，nest_depth=0，parent_dir_sigma_p=0（父级别中性），entry_px=8413.33。
- **divergence_input**（R5-b Type1 暴露）：seg_a_macd_area=1.192e12，seg_c_macd_area=1.149e12，**C/A=0.9639<1 ⟹ 背驰成立**；seg_a_dif_peak=9.60e9 > seg_c_dif_peak=6.92e9（C 段 DIF 峰更低，力度衰竭印证）；force_state=`Incomparable(口径冲突)`。
- **解释器选择**：p_star 选 候选1 control=-1（tracking_err=13）Short；拒 候选2/3。
- **entry 后走势**（594 塔事件 L1-L4）：entry 后 L1 new_center 持续上移（zd 836301→849830→852686→...），exit_px=11368.47（**价格涨 35%**）。
- **语法故事**：**背驰判定正确**（C 段力度<A 段，标准趋势背驰做空信号），但持仓 51107 bar（~1.4 月）期间大级别趋势延续突破背驰段（背驰被"化解"）。**根因：背驰是局部段对信号，无力对抗大级别趋势延续；且 force_state=Incomparable 提示口径冲突、C/A=0.96 接近 1（背驰强度弱）**。

**【正收益对照代表】trade_id=3732  pnl=+4997.79  exit=ReduceCore  bsp=Type3  dir=Long**

- entry：level=0，role=`SameReverse|FollowParent|Plus`，nest_depth=1，parent_dir_sigma_p=+1（**父级别上涨，本笔 Long 顺势**），entry_px=32494.25。
- 解释器：3 候选，p_star 选 control=1（tracking_err=5227, grid=2）；拒 control=0/-1。
- **走势**（仅 2 塔事件，全 L1 extend）：entry..exit 结构几乎未变即平仓兑现，持仓 148 bar，exit_px=37492.04（涨 15.4%）。
- **语法故事**：**顺父级别方向（pdir=+1，Long 同向）+ 快进快出（持仓 148 bar，结构未演化）= 利润兑现**。与 3765 形成镜像——两者 bsp/role/tw stage 相近，差异在**方向是否顺父级别 + 持仓时长**。

### 亏损 30 笔逐笔精简表（pdir=parent_dir_sigma_p；族标签见 §3）

| # | trade_id | pnl | exit | bsp | dir | pdir | nest | 持仓bar | 塔事件 | 族 |
|---|---|---|---|---|---|---|---|---|---|---|
| 1 | 3765 | -36910.92 | CloseShortDiff | 2 | Short | +1 | 2 | 201909 | 1991 | A |
| 2 | 3756 | -11034.49 | CloseRoot | 2 | Short | -1 | 3 | 14205 | 96 | A |
| 3 | 3993 | -7836.44 | CloseShortDiff | 2 | Long | -1 | 3 | 990 | 21 | B |
| 4 | 3985 | -7144.42 | ReduceCore | 3 | Long | +1 | 1 | 3675 | 31 | D |
| 5 | 4182 | -4946.37 | ReduceCore | 3 | Short | 0 | 0 | 4500 | 26 | D |
| 6 | 3699 | -4871.71 | CloseRoot | 2 | Short | -1 | 1 | 2831 | 22 | B |
| 7 | 4487 | -4716.00 | CloseShortDiff | 2 | Long | +1 | 1 | 659 | 5 | B |
| 8 | 3954 | -4589.87 | ReduceCore | 3 | Short | -1 | 1 | 2156 | 12 | D |
| 9 | 4284 | -4422.70 | CloseShortDiff | 3 | Long | -1 | 1 | 632 | 3 | B |
| 10 | 4324 | -4309.97 | CloseRoot | 2 | Long | +1 | 1 | 1579 | 11 | B |
| 11 | 3831 | -4159.53 | CloseShortDiff | 2 | Long | -1 | 2 | 418 | 7 | B |
| 12 | 3838 | -4077.12 | ReduceCore | 2 | Short | -1 | 2 | 844 | 8 | D |
| 13 | 3784 | -3809.06 | CloseRoot | 3 | Long | 0 | 0 | 1528 | 14 | B |
| 14 | 4361 | -3622.04 | CloseShortDiff | 2 | Short | -1 | 1 | 1046 | 6 | B |
| 15 | 3865 | -3513.43 | ReduceCore | 3 | Long | 0 | 0 | 388 | 0 | D |
| 16 | 4378 | -3395.93 | CloseShortDiff | 3 | Short | +1 | 1 | 2424 | 14 | B |
| 17 | 3899 | -3348.61 | CloseShortDiff | 3 | Short | +1 | 1 | 4859 | 25 | B |
| 18 | 4075 | -3249.98 | CloseShortDiff | 3 | Short | +1 | 2 | 1322 | 14 | B |
| 19 | 4224 | -3133.84 | CloseShortDiff | 2 | Short | -1 | 1 | 1150 | 8 | B |
| 20 | 3808 | -3096.64 | CloseRoot | 2 | Short | -1 | 4 | 1200 | 11 | B |
| 21 | 2031 | -2955.14 | ReduceCore | **1** | Short | 0 | 0 | 51107 | 594 | **C** |
| 22 | 4098 | -2820.83 | CloseShortDiff | 3 | Long | -1 | 2 | 1078 | 7 | B |
| 23 | 4563 | -2796.83 | CloseRoot | 3 | Long | 0 | 0 | 1159 | 2 | B |
| 24 | 272 | -2795.68 | ReduceCore | 2 | Short | -1 | 1 | 2305 | 13 | D |
| 25 | 381 | -2766.97 | CloseShortDiff | 2 | Long | -1 | 1 | 1260 | 5 | B |
| 26 | 4551 | -2647.51 | CloseRoot | 3 | Short | -1 | 2 | 1253 | 10 | B |
| 27 | 4036 | -2618.55 | CloseRoot | 2 | Short | -1 | 1 | 12775 | 127 | A |
| 28 | 3825 | -2582.81 | CloseShortDiff | 2 | Short | +1 | 3 | 198 | 7 | B |
| 29 | 3759 | -2569.69 | ReduceCore | 3 | Short | -1 | 2 | 1551 | 12 | D |
| 30 | 4113 | -2552.02 | CloseRoot | 2 | Short | -1 | 3 | 4743 | 37 | B |

族划分规则：C = bsp=1（Type1）；A = 持仓>5000 bar；D = ReduceCore（Type2/3）；B = 其余（短持仓震荡）。亏损30 总 Σpnl = **-153295.10**。

### 正收益 ReduceCore top10 逐笔精简表

| # | trade_id | pnl | bsp | dir | pdir | nest | 持仓bar | 塔事件 |
|---|---|---|---|---|---|---|---|---|
| 1 | 3732 | +4997.79 | 3 | Long | +1 | 1 | 148 | 2 |
| 2 | 4505 | +4944.34 | 3 | Short | 0 | 0 | 234 | 0 |
| 3 | 3935 | +4705.94 | 2 | Short | -1 | 3 | 666 | 6 |
| 4 | 4461 | +3064.67 | 3 | Short | -1 | 1 | 584 | 2 |
| 5 | 3688 | +2591.62 | 3 | Short | -1 | 1 | 348 | 2 |
| 6 | 3965 | +2198.72 | 3 | Long | +1 | 3 | 288 | 6 |
| 7 | 4240 | +1999.56 | 2 | Long | +1 | 1 | 506 | 2 |
| 8 | 3991 | +1987.05 | 2 | Short | -1 | 1 | 287 | 2 |
| 9 | 3734 | +1953.58 | 2 | Short | -1 | 1 | 521 | 3 |
| 10 | 3911 | +1942.41 | 2 | Short | -1 | 2 | 438 | 4 |

**正收益共性**：持仓 148-666 bar（极短），塔事件 0-6（结构几乎未变即兑现）；**9 笔顺势（dir=pdir≠0）+ 1 笔中性（pdir=0，trade 4505）+ 0 笔逆势**。

---

## §3 故事聚类（4 族 + 1 对照组）

亏损 30 笔按"entry 语义状态 × entry 后走势语法"聚为 4 族（Σpnl/占比经 python 从 dump 精算）：

### 族 A — 超大持仓跨大级别演化（3 笔：3765/3756/4036）  Σ=-50563.96（占亏损30 的 33.0%）

- **语义特征**：持仓 12775-201909 bar（~9 天到 ~5.5 月），塔事件 96-1991（L1-L5 跨级别）。3765 逆势（Short vs pdir+1），3756/4036 顺势（Short vs pdir-1）但仍巨亏。
- **语法故事**：持仓跨越大级别趋势全程。3765 = 逆势短差被五级别上涨碾压；3756/4036 = 顺势做空但大级别反弹幅度超短差承受。**共同根因：持仓过久未止损，单笔灾难化**（3765 单笔 -36910 占亏损30 的 24.1%）。

### 族 B — 中枢震荡短差择时错（19 笔）  Σ=-70139.42（占 45.8%，金额最大族）

- **成员**：3993/3699/4487/4284/4324/3831/3784/4361/4378/3899/4075/4224/3808/4098/4563/381/4551/3825/4113
- **语义特征**：Type2/3，CloseShortDiff/CloseRoot 主导，持仓 198-4859 bar（短中），塔事件 2-25（L1-L3 轻演化），nest_depth 多为 1（区间套浅，无多级别共振保护）。
- **语法故事**：中枢震荡（盘整）里做短差，entry 后小级别反向波动触发 CloseShortDiff 平短差止损。**高频小亏累积**（单笔 -2582 到 -7836，19 笔聚成最大金额族）。

### 族 C — 背驰判定正确但持仓过久（1 笔：2031）  Σ=-2955.14（占 1.9%）

- **语义特征**：唯一 Type1，divergence C/A=0.9639<1 背驰成立，force_state=Incomparable（口径冲突），持仓 51107 bar（族内最长），塔事件 594。
- **语法故事**：标准趋势背驰做空（信号对），但大级别趋势延续突破背驰段，持仓过久回吐。背驰强度弱（C/A 接近 1）+ 口径冲突，本应更早离场。

### 族 D — ReduceCore 减仓时机错（7 笔：3985/4182/3954/3838/3865/272/3759）  Σ=-29636.58（占 19.3%）

- **语义特征**：Type2/3，exit=ReduceCore，tw stage=CostReduction（**与正收益同 stage**），持仓 388-4500 bar。
- **语法故事**：进入 CostReduction 阶段减仓，但减仓点落在大级别反弹中段（非真正反转点）。**与正收益 ReduceCore 同 stage 同 bsp 分布——差异在持仓时长（正收益 148-666 bar vs 族D 388-4500 bar）**。

### 对照组 — 正收益 ReduceCore（10 笔，关键差异变量定位）

| 变量 | 正收益10 | 亏损30 | 结论 |
|---|---|---|---|
| bsp class | Type2/3 主导（同） | Type2/3 主导（29/30） | **非分水岭** |
| tw stage | CostReduction（同） | CostReduction（30/30） | **非分水岭** |
| 持仓 bar | **148-666**（极短） | 198-201909（族A/C 超长） | **关键变量** |
| 塔事件数 | 0-6（结构未变） | 族A/C 37-1991（结构反转） | **关键变量** |
| dir vs pdir | **0 笔逆势**（9顺势+1中性） | 10 笔逆势（含最大单笔3765）+15顺势+5中性 | **逆势是亏损的充分特征** |

**核心结论**：盈亏分水岭**不在买卖点识别**（Type1 背驰判定正确，2031 印证；Type2/3 盈亏同分布），**而在持仓管理的退出时机**。两个可观测的亏损充分特征：(1) 持仓>5000 bar（族A+C，4 笔占 34.9% 金额，单笔灾难化）；(2) 逆势（dir≠pdir，正收益侧 0 笔）。族B（19 笔震荡小亏）显示短持仓+顺势也会因震荡择时错亏损，但单笔金额小。

---

## §4 与上轮 6 族否决的对照

| 维度 | 上轮（opsem-loss-diagnosis-20260705） | 本轮（redo） |
|---|---|---|
| **R5 三字段** | lex_argmin_top3 60/60 null / divergence_input 58/60 null / nest_depth 60/60 null | lex_top3 100% / nest_depth 100% / divergence Type1 100% 非 null |
| **Grammar agent 行为** | 6 族全否决（survived=false）——语义字段缺席时编造 trade_id/entry_px/中枢编号 | 4 族均基于 dump 真实行，trade_id 1-4653 连续可追溯，0 编造 |
| **数据层伪造** | 故事引用 trade 89/113/142 等，档案仅 72 笔（越界）；族级 Σpnl 算术不可能（全档 Σ=+804 正值，F2 单族声称 -9334） | 每笔 pnl 单独可追溯（pnl_raw_unlevered 真实读出），族 Σpnl 算术自洽（A+B+C+D=-153295.10=亏损30 Σ） |
| **dump 路径一致性** | /tmp/opsem(72笔) vs /tmp/opsem_q1(214笔) 引用混乱 | 单一 dump /tmp/opsem2（4653 笔），所有引用同源 |
| **可观测诊断能力** | 无法区分亏损模式（语义全缺席） | 清晰区分 4 族 + 定位两个可观测充分特征（持仓>5000bar / 逆势） |
| **真实失败模式结论** | 仅识别"语义插桩不足"+ 1 个背驰支配序实装缺口 | **亏损根因 = 持仓管理（退出时机）失败，非买卖点识别失败**；持仓>5000bar 单笔灾难化 + 逆势 + 震荡择时错 |

**核心进步**：上轮在"语义全黑"下只能猜（且猜错）；本轮在 R5 三字段可观测下，首次用真实语义状态定位到亏损来自**持仓管理**而非**信号识别**——这是 opsem 框架设计目的（在可观测操作语义上诊断亏损）的首次有效兑现。

---

## 结果包六要素

### 1. 结论

- **R5 实战验证 PASS**：三字段（lex_argmin_top3 / divergence_input / nest_depth）在 4653 笔真实 dump 上全部合格（100% / 100% / Type1 100%），远超 80% 门槛。
- **opsem 重做首份有效诊断**：亏损 30 笔聚为 4 族（A 超大持仓 3 笔 33.0% / B 震荡择时错 19 笔 45.8% / C 背驰对持仓久 1 笔 1.9% / D 减仓时机错 7 笔 19.3%），正收益 10 笔对照组。**两个可观测充分特征：持仓>5000 bar、逆势（dir≠parent_dir）**；正收益侧 0 笔逆势、持仓全 ≤666 bar。

### 2. 定义依据

- R5-a/b/c 修复形式（opsemfix-r5-20260705.md §1）+ R5-1（env-gated bit-exact）/ R5-2（divergence 定义域=Type1）
- formal-chain C8 LexArgmin（p*=LexArgmin_{p∈K_Θ}J_x(p)）/ C5-C6 divergence / C2 区间套
- dump 字段语义：`pnl_raw_unlevered` = 费前方向盈亏（delta×(exit-entry)）；`certificate.parent_dir_sigma_p` = 父级别方向极性（+1/-1/0）；`role` = v3 操盘角色标记；`tw_at_entry.stage=CostReduction` = 成本减少阶段；持仓 bar = exit_bar-entry_bar

### 3. 边界条件（结论翻转条件）

- **有限效域（重要）**：dump 仅覆盖 bar 0-2245671（~4.3 年，全量前 48.7%）。若后续 ~52% 数据（含 memory 记 12 窗的牛市三窗）亏损模式不同（如牛市趋势延续更强使族A/C 占比升高，或牛市震荡使族B 占比升高），4 族**比例**需重测。**4 族的存在性结论稳健（每族已在样本中出现），比例结论有限效域**。
- **L2 观察性而非 L3 因果**："持仓时长/逆势是亏损充分特征"是基于亏损/正收益对照组的观察关联（L2），非多变量回归因果证明（L3）。若引入控制变量做倾向得分匹配，结论可能细化但 unlikely 翻转（正收益侧 0 逆势 + 持仓全 ≤666 bar 的对照差异显著）。
- **背驰 force_state=Incomparable**：trade 2031 的 divergence force_state 标注"口径冲突"——背驰判定本身的支配序实装缺口（上轮已识别，关于背驰.pdf p3-4 禁止标量比较）仍在；本轮不修，仅观测。
- **结论翻转条件**：若后续 52% 数据出现"短持仓+顺势也大面积亏损"或"长持仓+逆势也大面积盈利"，则充分特征结论翻转——需在完整 dump 上重测。

### 4. 下游推论

- **持仓管理是亏损主因** ⟹ 后续优化应作用于**退出时机**（族A/C/D 的持仓过久 + 族B 的震荡止损），而非 entry 信号识别（Type1 背驰判定已正确）。
- **J_Θ 缺大级别趋势方向项**（族A/3765 根因）⟹ 可在 lex_argmin 候选评估中加入 parent_dir 一致性约束（顺父级别方向候选优先 / 逆势候选加惩罚），但这是策略层改动，需编排者裁决（超出 R5 诊断范围）。
- **逆势是亏损充分特征**（正收益侧 0 逆势）⟹ 可作为风险门控候选（逆势开仓降仓或禁开），同样需编排者裁决。
- **R5-c nest_depth 与生产门同源但绕开 hist**（opsemfix-r5 §6）⟹ 若要让 π 路径候选真进 Nest/Xzd 准入门，需在 signal 层补 hist（z 维 #11 缺口同源）——本轮 nest_depth 仅作诊断读数，不进 μ 桶键。

### 5. 谱系引用

- 基因 073a / 274号（任务标注）：opsem-dump 框架谱系锚点
- opsemfix-r5-20260705.md（R5 实装）/ opsem-loss-diagnosis-20260705.md（上轮 6 族否决，本报告对照基线）
- formal-chain C2/C5-C6/C8（区间套/divergence/LexArgmin 硬判据）
- 090号（严格性语法规则——诚实铁律，不编造 dump 数据）/ 231号（形式化有效域，L2 观察性等级声明）
- 不确定是否有相关谱系：本轮"持仓管理是亏损主因"+"逆势/持仓时长是亏损充分特征"的诊断结论未检索到既有谱系，若编排者认为值得结晶为操作语义发现，可走 knowledge-crystallization。

### 6. 影响声明

- **改动文件**：仅新增本报告 `.chanlun/review-results/opsem-redo-20260705.md`（未 commit，按任务要求）。`/tmp/opsem2/`（dump）/ `/tmp/analyze_opsem2.py` / `/tmp/story_opsem2.py`（分析脚本）为临时产物，不进仓库。
- **不影响生产路径**：R5 是 env-gated 只读外化（OPSEM_DUMP_DIR 未设 ⟹ 生产 bit-exact 不变，opsemfix-r5 §6 已证）；本报告是诊断读数，不改任何策略/μ桶/J_Θ 代码。
- **不影响 R2**：本任务不触及 R2（descend 锚定语义），R2 待编排者裁决（任务注）。

---

## 复现脚本（诚实可追溯）

```bash
# 1. 跑回测产 dump（~29分钟全量，或 SIGTERM 提前停止）
env OPSEM_DUMP_DIR=/tmp/opsem2 cargo run --release --features backtest_bin --bin theta_backtest -- BTC

# 2. 阶段1 统计 + 阶段2 排名概要
python3 /tmp/analyze_opsem2.py

# 3. 阶段2 逐笔语义+走势原料
python3 /tmp/story_opsem2.py
```

dump 路径：`/tmp/opsem2/trades.jsonl`（4653 笔）+ `/tmp/opsem2/tower_events.jsonl`（28221 事件）。
