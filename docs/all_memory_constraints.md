---
name: bidirectional-always-in
description: 缠论不是只做多系统。无论牛熊震荡都应赚钱，绩效=涨跌幅绝对值之和，永远在市场里有方向
metadata:
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户2026-06-12明确纠正："你让我的缠论操盘变成了一个只做多的，这样不行。无论牛熊还是震荡，都应该是赚的。并且以涨跌幅的绝对值为准。"

**Why:** 当前hold26/fusion_tr全部long-only是实现缺口。缠论走势完全分类+六种买卖点=任何走势都有操作方向。卖点=反手做空入场，买点=反手做多入场。系统永远在市场里。BTC两周跌13.57%，正确系统应赚~13.57%而不是"少亏5.5%"。

**How to apply:**
- 绩效benchmark从"跑赢BH"改为"吃到Σ|各段涨跌幅|"
- master FSM从 LONG→FLAT→LONG 改为 LONG↔SHORT（无FLAT状态）
- 嵌套递归赋格中每层voice跟随自己级别走势方向
- 所有回测报告加双向口径
- 平仓≠卖空——平仓退出市场，卖空反向进入市场

Related: [[recursive-nested-fugue]], [[no-asset-specialization]], [[session3-fusion]]


---


---
name: Chanlun stroke types settled
description: 缠论笔只有两种：旧笔（第62课严版）和新笔（第81课宽版）。方向K线不是额外约束是推论。TV设置：宽笔+优化包含+严格延续。
type: feedback
originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---
缠论笔的定义（2026-04-22结算）：

只有两种笔（概念层面）：
1. 旧笔/严笔（第62课）：merged K线计数
2. 新笔/宽笔（第81课）：raw K线计数≥3，不考虑包含关系

方向K线约束（第62课L32"结合律"）：
- 不是额外约束，是从"结合律+分型定义+包含处理"推出的定理
- 缠师第81课答疑明确否定："那无所谓，只要是独立的就可以"
- 第62课L32的"上升K线"在后续三次再分辨中被逐步弱化直至否定

NewChanlun代码三模式（wide/strict/new）是过度区分，需收敛为两模式。strict(gap≥5)无原文依据。

TV指标设置（与NewChanlun一致）：
- 笔模式：**宽笔**
- 特征序列包含：**优化包含**（对应第67课+第71课）
- 特征序列延续：**严格延续**（对应第67课+第78课硬约束）

**Why:** 从第62课到第81课的完整概念演化链，5次课程澄清
**How to apply:** TV用宽笔+优化包含+严格延续。NewChanlun用stroke_mode="new"


---


---
name: Chunk long file operations
description: Break long edits into segments, add hardness enforcement to prevent task freezes
type: feedback
---

编辑长文件时必须分段，每次一步，每步后验证。

**Why:** 任务反复因单次处理长文件而冻结/卡住。
**How to apply:** task prompt中明确"ONE change at a time, verify after each write"；python-docx脚本每个section单独写；报告生成每次最多25行。


---


---
name: Dispatch as commander
description: Decompose before dispatching; Dispatch is the general, subtasks are worker bees; wave-based parallel execution
type: feedback
---

每次dispatch前必须分解任务。四步骤：能否拆分→粒度→权重（轻/重模型）→波次结构（独立任务并行、波间综合）。

**Why:** 用户明确的元编排原则，适用于所有领域。
**How to apply:** 模式"将军与工蜂"——Dispatch分解+综合，subtask执行。不做简单转发。


---


---
name: DOCX editing atomic rules
description: One fix per task, pre-built python scripts, PowerShell execution (not REPL)
type: feedback
---

三条强制规则：①每个task只做一个fix；②提前写好python脚本模板；③直接执行脚本，不用REPL。

**Why:** 七个结构性修复打包成一个Wave 1 task导致卡循环。
**How to apply:** docx编辑任务拆分为单一修复粒度。


---


---
name: Essay grounding in course materials
description: Thesis must emerge from readings, cite 8-10+ course sources, not impose external frameworks
type: feedback
---

选题和thesis必须从课程材料内部产生，不能把外部框架强加上去。最少引用8-10个课程内文献。

**Why:** 第一次HPSC0111论文只引了6个来源，完全没有engagement课程阅读，Studiosity也标记了来源问题。
**How to apply:** 论文写作开始前先列课程reading list，thesis从材料中涌现而非预设。


---


---
name: File write verification
description: Always verify file writes with get_file_info; VM paths don't reach host filesystem
type: feedback
---

task session写文件后必须立即用get_file_info验证非零大小。

**Why:** 9个WF1 task + WF2-7全部声称写入了文件，但目标路径上一个文件都不存在（写到了VM本地路径）。
**How to apply:** 每次Write后立即验证。沙盒路径 ≠ 用户本机路径。


---


---
name: necessity-accumulation
description: 纲领：必然性累积而非实证否定。回测不否定必然性推论，只否定拼接方式。每步揭示的必然性全部保留叠加成完整系统。
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户2026-06-14多次纠正后确立的方法论纲领：

**必然性是累积的，不被经验否定。** 每一步实装揭示的必然性内容应该全部保留，回测数字不好时丢弃的应该是"拼接方式"而不是必然性本身。

**具体原则：**
1. 从概念运动链推导出的必然性推论，不因回测P1不佳而丢弃
2. 回测否定的不是必然性，是必然性被拼接在一起的方式的不完整性
3. 必然性验证（prove_chain等逻辑检验）是验收标准，回测不是
4. 每个版本应该在前一个版本基础上扬弃（否定+保留+提升），不是patch也不是丢弃重来
5. "不是实证主义的实验"——不是发现问题→实验→改→再发现→再改的循环

**已揭示的必然性清单（全部保留）：**
1. 逐仓独立森林结构（第20环）
2. per-voice独立操作（第18环并发）
3. 全三类BSP消费（第12环）
4. 成本门终止递归（第16环）
5. 区间套自上而下定位（第14环）
6. 先势后定位时序（压缩→展开，540号双重性）
7. 降成本不需要pending——已有voice内部操作（第17环）
8. 双层会计多空嵌套（第22环）

**Why:** 这是方法论的根本转向——从实证主义（经验决定理论）到辩证法（概念自我运动决定实装）
**How to apply:** 每次实装后的验收用必然性检验（逻辑属性），回测只作为有效域读数。不因回测结果丢弃任何已确认的必然性推论。

Related: [[concept-movement-chain]], [[recursive-nested-fugue]], [[no-asset-specialization]]


---


---
name: no-asset-specialization
description: 缠论必须在任何标的任何市场下统一适用，per-asset配置=过拟合风险，regime切换必须从走势结构涌现
metadata:
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户2026-06-12明确表述："缠论应该在任何情形下的市场都适用，否则会有过拟合的风险。它不能被标的所特化。"

当前系统的三域三配置矩阵（趋势域fusion_t / 震荡域ht / BH域）本质是per-asset特化——虽然每个配置有原文课号，但"哪个标的用哪个配置"是回测盈亏拟合，无原文依据。

**Why:** 缠论第一性原理：走势完全分类（趋势/盘整），任何级别操作遵循同一套买卖点规则。不应存在per-asset配置选择。49课二相的切换是走势自身决定的，不是人决定的。

**How to apply:** 下一个session的最高优先级=找到统一配置，regime切换完全由走势结构驱动（运行时内生涌现）。不继续优化各域收益，回到根上。所有"白名单"类判决（osc域、fusion_t域、H1/H3域）都是形式化不到位的信号，不是最终形态。

Related: [[session3-fusion]], [[recursive-nested-fugue]]


---


---
name: no-workaround-bsp-lesson
description: 惨痛教训：不消费BSP导致整个操作层是workaround堆叠，promote/spawn/flip/chain全是替代品，消费BSP后全部消失
metadata:
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 2026-06-20 BSP不消费=一切workaround的源头

整个session追到底的教训：递归引擎（rec_engine.rs）计算了11281个缠论买卖点（type1/2/3 买卖各数千个），然后**全部丢弃**，操作层靠走势方向变化（chain[0]反转）做决策。

结果：
- 843个买点摆在那里没消费 → 空头骑120万bar穿越牛市 → -184%
- 50%空头不回补 → 不是信号问题，是消费问题
- promote/spawn/flip/emergent门控/clear_root/走势跟随 → 全是因为不消费BSP而造的workaround

**教训**：每多加一层间接机制（promote、spawn、flip、chain、extract_chain），就多一层可能出错的地方。如果直接消费BSP，操作跟BSP一一对应，不需要任何间接机制。

**正确形式**：每个T实例消费自己级别的BSP，买点→做多/平空，卖点→做空/平多。多级别同时运行=多重赋格。不需要中间层。

**Why:** 编排者反复强调"绝不能做任何workaround"。这次是最大的实例——整个操作层都是workaround堆叠，因为最底层（BSP消费）没做。
**How to apply:** 任何设计先问"这是不是在直接消费BSP？"如果需要间接机制（promote/spawn/flip/chain），先质疑：为什么不能直接消费BSP做操作？间接机制=潜在workaround。

Related: [[no-asset-specialization]], [[bidirectional-always-in]], [[recursive-nested-fugue]]


---


---
name: Options expiry date rule
description: 到期日 = T_target上限 + 45天theta buffer，禁止贴着事件/目标日期，不用凸性反推到期日
type: feedback
originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---
到期日计算规则（最终版）：

核心原则：用户只在缠论买卖点处置仓位。到期日的功能是为"等待卖点出现"提供足够的时间空间，不是为预测走势完成时间。

计算公式：到期日 = T_target_上限 + T_theta_buffer

输入要求：
1. T_target_上限：用户给出的"走势最晚完成时间"区间上限（用户必须提供，不可由 agent 推断）
2. T_theta_buffer：30-60 天，默认 45 天

事件驱动额外约束：到期日 ≥ 事件完成日 + 45 天，与 T_target_上限 + buffer 取较大者

建仓前必须与用户确认：驱动类型（纯缠论/纯事件/叠加）、T_target_上限的具体数值

禁止：到期日贴着事件完成日、到期日贴着 T_target_上限、用凸性最大化反推到期日

已明确接受的代价：premium 成本比最凸结构高 2-4 倍、杠杆相应降低、事件驱动行情中可能让 option 归零以等待缠论卖点。这些代价已由用户明确选择，agent 不需要优化掉。

**Why:** 之前多次因为到期日太近导致期权流动性问题（SI C79 七天到期没 bid）和时间压力
**How to apply:** 每次推荐期权前必须先问用户 T_target_上限，然后加 45 天 buffer


---


---
name: Options selector redesign
description: Strike从缠论结构位来，不从期权数学来。Agent只做翻译不做判断。杠杆是派生变量。
type: feedback
originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---
options-selector skill 彻底重写（2026-04-21）：

核心原则：三个strike全部落在缠论关键位置上。杠杆是结构派生变量，不是事前设定的目标值。

建仓前四个必答问题：
1. 缠论买/卖点（入场时机）
2. 当前级别止损位 → 卖put/call strike
3. 更大级别支撑/阻力 → 买低put/高call strike
4. 结构立足点 → ITM call/put strike

Agent职责：翻译缠论判断为期权订单参数 + 验证delta合规 + 报告派生杠杆
Agent禁止：自行选strike、按杠杆倒推strike、按delta/IV/凸性选strike

TV MCP辅助：Agent可从TV拉labels/boxes建议四个位置，但最终确认由用户决定。

自动风控：结构清晰→杠杆高→重仓；结构松散→杠杆低→轻仓；结构找不到→不建仓

**Why:** 期权书（Natenberg/McMillan/Bennett）从期权自身数学性质选strike。用户的做法是从市场结构出发，期权数学是派生的。
**How to apply:** 每次建仓先拉TV缠论数据识别结构位，提交用户确认后生成订单


---


---
name: Order pricing rule
description: Always LMT at mid/bid (buy) or mid/ask (sell), never MKT orders
type: feedback
---

买单：用MID或BID价格。卖单：用MID或ASK价格。绝对禁止MKT单。

**Why:** 用MKT单平仓原油combo仓位时滑点损失严重，期权价差很宽。
**How to apply:** TWS脚本下单前拉bid/ask/mid，只用LMT。未成交由用户手动调整，不自动升级为MKT。


---


---
name: Python output capture pattern
description: Never use stdout; write results to file; Desktop Commander can't capture Unicode output
type: feedback
---

强制模式：写结果到文件（RESULT_FILE），执行后读取文件内容。不用stdout。

**Why:** Desktop Commander无法可靠捕获python stdout（尤其含Unicode如Ørsted等）。不知道这个模式的task每次都浪费5-10轮诊断。
**How to apply:** 所有python脚本输出写文件，执行后read_file读取。


---


---
name: recursive-nested-fugue
description: 嵌套递归赋格的完整理解——同一笔交易双层记账，父降成本=子开仓，区间套=voice spawn，资金守恒无需配额
metadata:
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户2026-06-12多次纠正后的完整理解（"这才是嵌套递归多重赋格的真正意义"）：

**核心机制**：同一笔物理交易在递归的不同级别有不同的会计身份。
- 父级别卖出N股=降成本（短差的卖出端）
- 子级别开空N股=独立逐仓头寸（跟随子级别下跌走势）
- 物理上是一笔交易，会计上是两层记账
- 子空头P&L ≡ 父降成本金额（同一个数字的两个会计身份）

**资金守恒**：父卖出释放的现金=子开仓需要的资本。不需要额外资金，不需要配额分配机制。53课"仓位调配以后再说"可能就是因为在严格递归下不需要说。

**区间套=voice spawn的时序机制**：高级别candidate向下递归定位→到某层出现买卖点→在那层spawn voice。区间套和逐仓是同一件事的两面。

**平多≠开空**：
- 平多=该级别voice走势完美，关闭多头（仓位从有到无）
- 开空=子级别检测到反向走势，创建新的独立空头voice（新仓位从无到有）
- 两者发生在不同级别、不同头寸，不能混淆

**正则化**：每一层做的事完全一样（跟随走势→等完美→翻转/平仓），递归自相似。区别只在级别（涌现）和名义（资金守恒决定）。

**为什么吃到每一笔**：每笔交易同时在所有递归层产生会计效应。没有任何一段走势浪费——都是某一层的利润来源。绩效理论上限=Σ|涨跌幅|。

**为什么之前空头腿不赚钱**：之前的实装是净额翻转（一个仓位从多翻空），不是逐仓独立头寸。丢掉了递归嵌套的全部信息。

**Why:** 这是整个系统的核心——之前反复强调但一直没被正确实装
**How to apply:** 每笔物理交易触发多层ledger更新，不是两笔交易。逐仓独立不是额外特性是递归的物质形态。

Related: [[concept-movement-chain]], [[bidirectional-always-in]], [[no-asset-specialization]]


---


---
name: Real rubric scoring requirement
description: Must use actual course rubric for scoring, never self-invented dimensions
type: feedback
---

所有独立评分必须使用实际课程模块评分标准，禁止用自创维度。

**Why:** 第一次dissertation 80push基于自创分数，方向错误——真实评分标准中bibliography quality是最大扣分项（69-72分段），但自创评分完全没有识别出来。
**How to apply:** 编入philosophy-essay SKILL.md的WF8。评分前先读取课程rubric。


---


---
name: Tutor RJ dissertation feedback
description: Narrow RQ to Ørsted focus, hide source-template method, write typical HPS style
type: feedback
---

导师RJ四条核心指令：①source-template方法不能出现在文本中，只作底层分析逻辑；②RQ缩窄为"Schelling的自然哲学对Ørsted发现电磁的影响程度如何？"（Ritter作对比工具，不是并列案例）；③不要写方法论章节，直接进历史分析；④减少自创术语（disaggregation等），嵌入式使用而非宣告。

**Why:** 导师明确反馈，论文风格需要调整
**How to apply:** 论文写作时检查——方法论不独立成章，术语嵌入使用，RQ聚焦Ørsted


---


---
name: use-latest-model
description: "CC任务用opus级别模型（opus-4-8优先，不可用时opus-4-7），不用sonnet"
metadata: 
  node_type: memory
  type: feedback
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

用户明确要求开CC任务时用 opus 级别模型+ultracode深度。不要用 sonnet。

优先级：claude-opus-4-8 > claude-opus-4-7 > claude-opus-4-6
fable-5 曾经可用但2026-06-17不可用（start_code_task报错model not found）。

**Why:** 2026-06-17用户纠正："为什么你开任务都是sonnet而不是opus4.8的ultracode？你得开opus啊"。Sonnet推理深度不够，opus才能处理形式化+严格性要求。
**How to apply:** start_code_task 用 model="claude-opus-4-8"；如果超时用 model="claude-opus-4-7"作fallback；绝对不用 sonnet


---


---
name: alpha-diagnosis
description: M1三层alpha归因：纯信号≤BH，降成本是负alpha源，MACD门控治标不治本。提款机bug修复后全面无alpha
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 2026-06-06 三层alpha归因（L2决定性结果）

### 提款机bug修复
`if profit > 0` 截断在20个文件39处被修复。修复后所有"亮眼"回测数字崩塌。

### 三层归因结果

| 标的 | 纯进出场(A) | 含降成本(V0) | 精修门控(B) | BH |
|------|------------|-------------|------------|-----|
| QQQ | +40.59% | +23.69% | +26.03% | +174.64% |
| OKLO | +242.61% | +114.08% | +179.36% | +251.30% |

1. **纯进出场层无显著alpha** — OKLO接近BH(超额-8.69%)，QQQ差距大(-134%)
2. **降成本是负alpha源** — OKLO降成本亏128个百分点，QQQ亏17个百分点
3. **MACD门控有改善但治标不治本** — 通过率55-63%，减少失血但不翻正

### 挣股数阶段
FSM EARNING_SHARES状态已实现（金额守恒），但有效域为空——cost_basis降不到0，触发条件从未满足。

### 下一步
需先修进出场信号层（让纯信号跑赢BH），然后再叠降成本。

**Why:** 提款机bug掩盖了整个M1回测基线的虚假性
**How to apply:** M1所有历史回测数字不可信。新基线必须从零降成本版开始

Related: [[quant-system-progress]], [[costreduction-moneyprinter-bug]]


---


---
name: c-segment-fix-DONE
description: 引擎C段边界修复——已完成并验证（2026-06-11）。修复A level.rs:217-275 + B2 divergence.rs:405-417已提交（commit 5739ec4f/0359f0dc/36c6a033）。cargo test 74/0；447K ignored守卫显式跑过：L2 type1/type2=374/360, L4=8/8。唯一残留：BZ slow测试jetsam×2未跑完（替代覆盖已声明）。下游解锁：tranche entry 3→4、REV配对重做、全量回测重跑
metadata: 
  node_type: memory
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 根因
level.rs:258（Python: a_zhongshu_level.py:240）递归层move的seg_end ≡ last_zs.comp_end。
离开中枢后到转折前的C段被截断为空→趋势背驰永不触发→高级别type1/type2=0→tranche空定义域。
缠师24课C段终于转折点（走势完成处），代码截断在move工程边界——偏离原文。

## 修复方案B2
C段越界到下一中枢内的趋势极值段。具体：
- buysellpoint层的divergence_segments()或c_segment计算中，c_end不截止于comp_end
- 而是延伸到：走势的价格极值段（趋势方向最高/最低价的那个段）
- OKLO模拟：L2 type1从1暴增到373 confirmed，type2从0到360 confirmed

## 精确代码位置
### Rust
- rust/src/buysellpoint.rs：找divergence_segments或c_start/c_end计算
- rust/src/moves.rs：move的seg_end定义可能需要扩展
- rust/src/orchestrator.rs：递归层的走势构造

### Python（同步改保持oracle一致）
- src/newchan/a_buysellpoint_v1.py：_detect_type1里的c_segments/c_start/c_end
- src/newchan/a_zhongshu_level.py:240：seg_end定义

## 验证
- 笔/线段/中枢层输出必须完全不变
- L2 type1 > 100（修复前=1）
- L4 type1 > 0（修复前=0）
- type2 > 0（修复前=0）
- cargo test + 回测在Rust闭环里跑，不走Python链路

## 诊断报告
analysis/engine_bsp_gap_diagnosis.md（由引擎BSP缺口诊断任务产出）

**Why:** 这是"吃到每一笔"北极星的最大引擎层阻塞
**How to apply:** 新session第一优先级——直接改代码+验证

Related: [[session-organic-fugue]], [[recursive-regularization]]


## Wave1下游重验（2026-06-11本session，两工蜂并行，质询通过）
- **tranche**：旧"空定义域定理"证伪。OKLO buy1_ladders={2,3,4}，entry 3→4，(2,3]=1层非空（L4 buy=5/sell=3，与447K守卫side-split逐位吻合）。报告analysis/tranche_recheck_post_csegfix.md。仅证结构非空，n_rev_tranche_adds触发率待全量回测
- **REV配对**：9e0504c6实为完整矩阵且磁带=最终磁带；HEAD复现守卫通过（tape_fp逐位同）。V2o唯一跨标的稳健（ΣΔ+591.6pp，胜率45→52%）；V1f旧+607.5pp是C段bug伪影（新仅+57.6）；BRN最优翻转为V2p（逃逸型2/3否证）。θ_depth不需随type1重标定（度量中枢带宽，与触发位是不同几何对象），但θ=1%量级失配需ATR相对化扫描。报告analysis/rev_pairing_post_csegfix.md
- 两任务零引擎源码改动，未commit

## 下一波待办
- tranche全量回测（n_rev_tranche_adds实际触发率，M2/D3依赖）
- θ_depth ATR相对化扫描（预注册）
- 高层type1锚×REV闭腿（C段修复新打开的工位）
- 谱系补录：532子结算（tranche定义域兑现）+"C段越界极值定义"结算编号
- Git commit工作区遗留（analysis改动+本波两份报告）


---


---
name: concept-movement-chain
description: 缠论的《逻辑学》——从走势终完美出发，通过概念自我否定运动推导全部缠论元素，指导统一voice FSM
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

2026-06-12用户与我在讨论中确立的方法论突破：

**缠论的形式化不应该从实验归纳，也不应该从公理演绎，而应该从概念的自我否定运动——走势终完美这一个种子概念通过内在矛盾必然地展开为整个体系。**

这是缠论的《逻辑学》（黑格尔意义上的）：
- 起点=走势终完美
- 方法=概念自我否定+扬弃
- 每一步否定必须论证必然性（不这样就自相矛盾）
- 终点=所有缠论元素被涵盖，链自我闭合
- FSM是这条链的形式化

**扬弃的三个检验标准**（验收统一FSM）：
1. 矛盾保留（不是消解）
2. 零外部信息（零flag零白名单，走势结构决定一切）
3. 多矛盾复用（同一转移规则承载多个矛盾的解）

**四维特化消除**：标的/多空/regime/级别的差异全部从走势结构涌现。

**方法论否定链**：
- 消融臂（经验归纳）→ 否证（full 0/8，非线性交互）
- 公理演绎 → 否证（公理从哪来？不完备）
- 辩证螺旋（理论↔实践）→ 部分成立但仍滑回归纳
- **概念自我运动** → 当前方向：纯概念推导，实验仅做证伪检验

**Why:** 这决定了统一voice FSM的推导方法和验收标准
**How to apply:** FSM的每条状态转移必须对应概念链中的一步否定，实验只验证实装是否正确表达了概念运动

Related: [[session3-fusion]], [[no-asset-specialization]], [[bidirectional-always-in]], [[recursive-nested-fugue]]


---


---
name: Crude oil options strategy
description: S3 three-leg+kicker, 11×C130 existing, COIL/IPE execution rules, file locations
type: project
---

- **品种：** Brent原油COIL，Exchange=IPE，Multiplier=1000，数据源BZ/NYMEX
- **已有持仓（截至2026-04-08）：** 11×C130 Mar2027，账户总值~$118,112，BZ期货2手均价$109
- **推荐方案S3：** 三腿（Buy C80 + Sell P80 + Buy P70）+ 6手C150 kicker，总资本$97,020
- **执行规则：** 只用LMT，下单前拉bid/ask/mid，IPE单腿tif='DAY'，combo可GTC
- **凸性分析（真实TWS报价）：** Dec2026 C160最优(47x)，但现有C130(56x)成本更低不换仓

**Why:** 用户核心持仓，基于美元体系ω崩溃论题
**How to apply:** 交易相关指令优先参考此文件的执行规则


---


---
name: Dissertation conceptual genealogy
description: Full genealogy of discoveries, concepts (differential reception, level-selective uptake, deep uptake), RQ, and workflow progress
type: project
---

**RQ（2026-04-08确认）：** "康德后自然哲学内部的构成性/调节性张力如何结构性地决定了席林Naturphilosophie在Ritter和Ørsted实验实践中的差异性接受（1797-1820）？"

**核心发现：**
1. Di Giovanni (1979)：构成性/调节性区分在康德自身内部就不稳定
2. Breidbach (2004)："影响"概念预设了当时不存在的哲学/科学边界，应用"差异性接受"
3. Gardner (2019)：席林展开的是康德已潜在的内容
4. Harding Vol.1&2：Ørsted书信证明层级选择性吸收

**关键概念：** 差异性接受、层级选择性吸收（Ørsted）、深层吸收（Ritter）、构成性/调节性过滤器

**六章结构：** Ch1导言→Ch2席林模板→Ch3史学编史→Ch4 Ritter→Ch5 Ørsted→Ch6比较综合

**Why:** HPSC0041 dissertation核心内容
**How to apply:** 论文修改和答辩准备时参考


---


---
name: Dissertation final status
description: HPSC0041 all workflows complete, deAI cleaned version ready, viva prep pending
type: project
---

**已完成全部工作流：** WF0→WF1→...→WF12

**最终文件（E:\HPSC0041 Dissertation\final\）：**
- HPSC0041_WF7_final.docx — pre-Humbot原版
- HPSC0041_final_deAI_fixed.docx — Humbot修复版，25处损伤已修
- HPSC0041_final_deAI_cleaned.docx — 最终清洁版，删除3个Humbot幻觉引用

**剩余工作：** 答辩/viva准备

**Why:** 论文已提交，追踪最终状态
**How to apply:** 答辩准备时参考最终版文件位置


---


---
name: emergent-direction
description: Root方向由涌现最高级别决定，四步循环是跨级别+会计双重性，F建仓ε=+1硬编码是D∞截断，已实装ε对称但需regime门控
metadata:
  type: project
  originSessionId: cowork-2026-06-17
---

## 2026-06-17 级别涌现决定Root方向

### 核心洞察链（用户推动）

1. **四步循环是跨级别的**：本级别卖点→平多 / 次级别做空 / 次级别买点→平空 / 本级别买点→做多。每步有会计双重性（一个级别的了结=另一级别的开始）。

2. **同级别只有三类买卖点**：1买/2买/3买或1卖/2卖/3卖。没有同级别"翻转"操作。

3. **走势类型=上一级别的一笔**：级别递归定义。root方向的翻转→联系到上一级别。

4. **最大级别不是选的，是涌现的**：从1s/1min笔自下而上逐层涌现。Root方向=已涌现最高级别走势方向。

5. **根voice不应该存在**：没有预设root，方向从涌现结构中读取。

### ε对称性实装结果（2026-06-17）

F建仓从硬编码Long改为双向（buy_source→Long, sell_source→Short），结果：
- OKLO +116%→+495%（大幅改善，跑赢BH）
- ES +594%→-213%（崩了——Short root在强牛市做空）
- GC +219%→-122%（同上）
- 其余也变差

**诊断**：sell_source fire不等于"翻空"。正确的逻辑：sell_source=本级别走势结束=平多，下一步做什么取决于涌现的新走势方向，不是机械翻转。

### 架构改动

- docs/emergent_level_direction.md（形式化文档）
- axis.rs: root_direction() 从形态学轴读取
- operate.rs: prove_f_source_direction 守卫
- 形态学轴的真正级别涌现实装仍pending

**Why:** 解决了"root方向从哪里来"的根本问题——不是硬编码也不是信号翻转，是级别涌现的副产品
**How to apply:** MorphologyAxis需要实装真正的走势类型涌现（笔→段→走势类型递归），输出级别+方向

Related: [[session5-spiral-reinterpretation]], [[bidirectional-always-in]], [[orbit-enumeration-duality]]


---


---
name: located-direction-discovery
description: 区间套located武装方向发现：当前自下而上(错)应为自上而下(对)，但1min a0下高级别candidate稀缺导致source坍缩segment
metadata:
  type: project
  originSessionId: cowork-2026-06-14
---

## 2026-06-14 session关键发现链

### 1. 信号丢失12机制诊断 → isolated_fugue.rs森林引擎
- 栈→森林、per-voice acted、Type2接入
- P1 1/8（森林放松约束反而更差——栈纪律是特征非bug）

### 2. why_not_profitable.md 五维度诊断
- 方向正确率@bar ≈ 随机 → 用户纠正测法 → 配对重测70-83%强有效
- **摩擦地板按级别切割**：segment幅度0.02-0.19%<摩擦，recL2+ 0.23-75%>摩擦
- 99%交易在segment = 摩擦地板下噪声
- 高级别type1独占利润

### 3. nested_interval_fugue.rs (nif3) 固定floor
- min_trade_ladder=3固定参数 → OKLO+922%/BTC+1287% 但P1 1/8
- 固定参数=regime函数（强趋势改善震荡劣化）= 不严格

### 4. positioning_chain_fugue.rs (pcf) 自下而上→自上而下
- **发现located武装方向反了**：每层独立检测candidate→检查全层对齐=自下而上
- 区间套应该是：高级别candidate→级联武装下面所有层→低级别confirm=自上而下
- 实装了自上而下级联（cascade_arm）→ source覆盖3-4层（改善10pp）
- **但source仍83-87%坍缩segment**：根因=1min a0下高级别candidate本身稀缺
- 自下而上和自上而下两版夹逼证明：坍缩是candidate频率结构决定的

### 5. 下一步：1秒a0
- 用户选择1s a0方向（更细粒度→更多递归层→高级别candidate可能不那么稀缺）
- 用NautilusTrader框架+Databento数据
- 之前有1s实验(`1s_a0_nest_coverage_results.md`)双否证，但那是旧模式(fusion_v+自下而上)

**Why:** 这条链是整个session的核心推进——从信号丢失→摩擦地板→固定floor→located方向→candidate稀缺→a0粒度
**How to apply:** 下个session从1s a0 + NautilusTrader + 自上而下pcf引擎开始

Related: [[recursive-nested-fugue]], [[concept-movement-chain]], [[session3-fusion]]


---


---
name: m3-capital-flow-milestone
description: M3阶段性目标：资本流转流量/流速的OU估计+宏观持仓表设计
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## M3 宏观持仓表（阶段性目标，不阻塞M1/M2）

### 已完成的基础
- 方向 ✅：残差符号（强持续性，L2验证）
- 加速度 ⚠：残差MACD背驰（聚合无边际但高幅背驰命中4大regime）
- 流量 ❌：成交量proxy否证 + COT净持仓否证（7/7无一领先）
- 持续性/流速 ✅：残差缠论递归涌现L3多级别结构

### 待探索（M3时做）
1. **OU流量估计**：flow = θ×r，θ从残差自相关估计，不需外部数据
   - θ大=资本流动快（市场高效），θ小=有摩擦/管制
   - θ的regime变化点是否对应已知危机/转折
2. **持仓表设计**（Soros/Druckenmiller框架）：
   - 汇率腿（6E, DX）：残差方向决定
   - 折叠通道腿（GC, CL/BRN spread）：全局C↔M / C→P方向
   - 指数/个股腿（ES + M1+M2选出的标的）
3. **跨国总拓扑**：
   - 各国K4不是孤立的，是总拓扑的局部切片
   - 金/油是全局折叠（连接所有经济体），汇率是局部连接
   - R(不动产)是唯一局部顶点（锚定点）
   - 金计价下K4→K3（自由度3→2，自测不可能性）
4. **COT精细口径复验**：Legacy Non-Commercial已否证，Managed Money/Leveraged Funds待测

### 已完成的研究文档
- docs/architecture/capital_flow_ontology.md（548行）
- analysis/cross_national_fx_deep_dive.md
- analysis/verify_capital_flow_4d.py
- analysis/cot_capital_flow_verification.md
- /tmp/chanlun-soros/（用户之前的研究包，需对齐到当前体系）

**Why:** M3是M1+M2之上的宏观层，不阻塞但需要提前记录方向
**How to apply:** M1严格收尾 → M2选股验证 → M3宏观持仓表

Related: [[project_omega_research]], [[project_k4_fold_channel_model]]


---


---
name: MU short trade plan
description: MU做空方案：日线2卖预期，46%背驰，目标$311。Jul17到期。白银先平仓释放资金。
type: project
originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---
MU做空交易方案（2026-04-21制定）：

缠论结构（日线TV数据确认）：
- 1卖（趋势）$471.34，动能116
- 2卖预期 $470.97，动能53（背驰比46%——独一档）
- 线段级别中枢 $436.89-$392.71
- 3买位置 $392.71
- 前低 $311.49

到期日计算：T_target上限30天 + 45天buffer = 75 DTE → Jul 17, 2026（88 DTE）

TWS真实报价（盘后冻结）：
- $450P Jul17: bid=$60.80, ask=$61.60, IV=71.15%, delta=-0.43
- $380P Jul17: bid=$28.85, ask=$29.45, IV=72.18%
- $310P Jul17: bid=$10.95, ask=$11.30, IV=75.94%

策略对比（到$311）：
- 做空200股：赚$27,400（账户+24.9%），需保证金$27K
- 裸Put $450 1手：赚$7,780（+127%），成本$6,120
- Put Spread $450/$380 2手：赚$3,795×2=$7,590（封顶$380）
- Put Spread $450/$310 1手：赚$8,992（+180%）

执行前提：先平SI C79白银（后天到期），释放保证金后决定策略。
账户NetLiq $33K，AvailableFunds $1,946。

止损：$473（2卖高点上方）

**Why:** MU六标的扫描中唯一区间套背驰强度<50%的标的
**How to apply:** 明天开盘先平白银，看available funds决定做空手段


---


---
name: ω Paper-Physical Ratio Research
description: Multi-round research on capital circulation topology — paper-to-physical ratio, angular momentum L=Mω, persistent homology, W1 trading signals, regime detection, daily TDA diagnostics
type: project
---

研究目标：用持续同调（TDA）检测资本空转的拓扑结构，作为操盘判据。

理论框架（卢麒元 + K4拓扑 + 缠论）：
- 资本是能量（kg·m），MCM' 绕美元旋转抽走 P 的流动性
- ω = 金油比 = MCM' 空转速度 = 金融寄生对实体的剥削率
- 用户押注的不是油涨，是美元体系的 ω 维持能力在崩溃
- 战争是催化剂：减少物理供给 + 冻结金融通道，两面夹击美元压价能力
- 空转可以同时存在且不可维持——ω 在高位但结构在裂

八轮研究成果（2025-04-04）：

1. 纸实比：原油12x，黄金163x
2. 角动量 L=M2×ω：与GDP协整(p=0.007)，一阶差分负相关(r=-0.16)
3. 单变量TDA：检测变动幅度，分不清空转vs生产性
4. 双变量联合嵌入：成功区分！空转组max bar 0.276 vs 生产性组0.007 (p<0.005)
5. W1交易信号：领先6个月(r=0.526)，25年触发2次
6. 日线精细（GLD/USO）：ω加速，PROLIFERATING
7. 现货金油比（XAUUSD/Brent）：INTACT判定（但120天窗口掩盖了regime切换）
8. **断点检测+分regime TDA（最关键）**：
   - PELT检测3个断点：2024-07-30, 2024-09-04, **2025-02-20**
   - R4（2025-02-20至今）= Topological Collapse
   - H1 bar: 25→5, max bar: 0.893→0.270, 总持续度: 2.915→0.403
   - W₁ = 2.11（regime间拓扑完全不同）
   - 空转中枢被击穿，循环消失，射线状单向趋势

操盘判据：
- 当前处于空转结构解体阶段（bar缩短 + ω高位）
- 缠论三买后延伸段，不做反转，不用震荡思维
- 等R4积累40-50天后重跑TDA，看H1 bar是否回升判断新均衡
- max bar系统性回升 = 新循环形成 = 可能的转折信号

**Why:** 用户核心持仓逻辑是美元体系ω维持能力崩溃
**How to apply:** 需要定期（每周或关键价格变动时）重跑regime TDA监控H1 bar状态


---


---
name: orbit-enumeration-duality
description: D∞轨道枚举+H¹同调穷尽+物理重诠释：H⁰=核心仓(2/3)，H¹=机动仓(1/3短差)，Δr=-1=四步减仓-回补路径，单边上扬=H¹幅度→0，τ⊥减仓(范畴正交)
metadata:
  type: project
  originSessionId: cowork-2026-06-16
---

## 2026-06-16 轨道枚举+扬弃调研

### 核心发现

**穷尽不完整的原因**：58条推论只穷尽了H⁰（σ-不变量），没穷尽H¹（变化模式的不变量=扬弃）。

**H¹(D∞, ℝ₋) = ℝ（一维）**：唯一生成元 = φ(σ)=1 = "开仓r=k，闭合r=k-1"。角向Z/23贡献零，H¹纯在径向。

**cd依赖系数环**：
- cd_Q(D∞) = 1 → 实值观测H⁰+H¹闭合（两层辩证法）
- cd_Z(D∞) = ∞ → 整数股数H²=(Z/2)²不消失，量子化阻止综合

### 7条否定推论 NR-1 to NR-7
- NR-1：同级别闭合非必然（Δr=0是σ=e伪影）
- NR-2：move级降成本时间无界（Δt∝λ^{k-1}）
- NR-5：顶层σ越界
- NR-7：整数股数独立手性障碍（H²第二生成元，T51未捕获）

### A1定理（扬弃推论）
跨级别闭合率Δr≠0是导出不变量（1-cocycle，非上边界）= NR-1的正面对偶 = 否定之否定

### 实装方向
P-close：出场从同级别BSP改为向心跨级别φ=0（第64课+T49）

### 文档
- docs/orbit_enumeration_completeness.md（681行，轨道枚举+H¹）
- analysis/spiral_solution_to_underperformance.md（五方向+F裁决）

**Why:** 这解释了为什么58条推论穷尽后引擎仍然7/8跑不赢BH——H⁰不够，需要H¹
**How to apply:** 下一步实装P-close（跨级别闭合），验证是否消除E spawn全亏问题

Related: [[session4-strict-necessity]], [[necessity-accumulation]], [[recursive-nested-fugue]]


---


---
name: quant-system-progress
description: 量化交易系统M1/M2进展：完整版赋格回测+K4选股+全市场端到端
metadata: 
  node_type: memory
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## M1 操盘层回测进展（2026-06-05/06）

### 控制变量实验历程

| 版本 | QQQ | OKLO | HK700 | BTC | 关键变量 |
|------|-----|------|-------|-----|---------|
| 单层PH H组 | -19.8% | -99% | -84% | — | 基线 |
| 多级别PH双向 | +58.3% | +72.3% | +222.8% | — | PH分层 |
| 纯做多 | +71.7% | +228% | +292.9% | — | 去掉做空 |
| persistence过滤 | +58.7% | +194.4% | +313.0% | — | 高persistence才触发退出 |
| +区间套 | +60.7% | +192.4% | +325.3% | — | 进场精确化 |
| +去掉加仓 | **+138.9%** | **+652.3%** | **+1644.6%** | — | 加仓是负alpha根因 |
| +做空（个股） | +53.9% | +934.5% | +4086.3% | — | 指数不做空个股做空 |
| BTC完整版3年 | — | — | — | +486.6% (vs BH+345%) | 首次crypto验证 |

### 关键发现

1. **加仓是隐藏的负alpha因素**——扩大BSP ID集合引入额外退出触发器。去掉后所有标的暴增。
2. **做空效果取决于标的属性**——指数（QQQ）做空亏，个股（OKLO/HK700/BTC）做空赚。根因：指数有幸存者偏差向上漂移。
3. **persistence过滤有效**——约2/3走势settle是噪音，过滤后退出质量大幅提高。
4. **区间套通过过滤坏入场产生alpha**——不是找更好价位，而是放弃不好的入场。

### 最终基线

- 指数标的：D2无加仓版纯做多（QQQ +138.9%）
- 个股/crypto标的：D2无加仓版+做空（OKLO +934.5%, HK700 +4086.3%, BTC +486.6%）

## M2 选股层进展

### 已实现
- selection_pool.py / asset_classifier.py / omega_regime.py — 品种池+方向表
- K4六边比价递归（ES/GC/CL/SPY/GLD/USO 1min数据）
- ω L2验证：相关系数-0.22，单独预测力有限

### 全市场端到端回测（v1日线版，有bug）
- K4日线递归太粗，SPY 90%时间判为盘整
- OKLO/BTC组合回测与单标的不一致（-140%/-99% vs +934%/+486%），需诊断
- HK700 K4选股产生+274%增量（阻止灾难性做空）

### 待做
- 用1min递归做K4配置（已开任务）
- 诊断OKLO/BTC组合回测bug（已开任务）
- 用ES/GC/CL期货真实数据替代ETF代理（Databento已拉到）
- 比价关系递归驱动选股（六边各自递归→配置空间动态演化）
- 跨国K4拓扑（graph.py已有CROSS_NATIONAL边类型定义）

## 数据资产

| 标的 | 文件 | bars | 来源 |
|------|------|------|------|
| QQQ | qqq_1m_databento_full.json | 728K | Databento |
| OKLO | oklo_1m_databento_full.json | 333K | Databento |
| HK700 | hk700_1m_tws.json | 1.4M | TWS |
| BTC | btc_1m_3year.json | 1.8M | Binance归档 |
| SPY | spy_1m_full.json | 1.2M | Databento |
| GLD | gld_1m_full.json | 790K | Databento |
| USO | uso_1m_full.json | 726K | Databento |
| UUP | uup_1m_full.json | 158K | Databento |
| ES期货 | es_1m_databento.json | — | Databento |
| GC期货 | gc_1m_databento.json | — | Databento |
| CL期货 | cl_1m_databento.json | — | Databento |

## 文档资产

- docs/ROADMAP.md — 项目总纲领（358行）
- docs/architecture/complete_fugue_design.md — 完整版赋格设计（780行）
- docs/architecture/ibkr_quant_system.md — IBKR量化架构（1029行）
- docs/architecture/m2_selection_layer.md — M2选股层架构

**Why:** 这是整个量化系统的核心进展记录
**How to apply:** 新conversation接续时参考此记录了解当前状态和待做事项


---


---
name: recursive-regularization
description: 递归正则化=区间套反向应用：每级别操作必须由次级别结构锚定，多重赋格是此递归在级别塔上的逐层应用
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 核心命题

**递归正则化 = 区间套反向应用 = 缠论操盘的严格形式**

三个说法是同一件事的不同面：
- **区间套反向应用**（从上往下）：高级别中枢定义操作域，次级别走势类型定位操作点
- **递归正则化**（从下往上）：每级别的操作必须被次级别的结构锚定，否则是盲操作
- **缠论操盘的严格形式**：缠师原文的操作方法本来就是递归嵌套的

## 短差配对的结构性缺陷（已诊断）

BarSignalI在L237把BSP的kind/center_seg_start折叠成4个布尔——信息销毁点。
- type1卖→type1买：胜率78.9%（正确配对）
- type1卖→type3买：卖飞率67.9%（43%的配对，凶手）
- type3买点不是止损信号——它是一个回补位（中枢终结后的合理回补点）
- 卖飞67.9%的根因可能是type3 confirmed的时间滞后，不是type3本身错

## 用户洞察（2026-06-10）

1. **每个买卖点都要用次级别来把握**——否则没法递归，每个级别都会盲
2. **第三类买点是回补位不是止损信号**——中枢终结了，该接回来恢复暴露
3. **短差在每个级别上做该级别对应的高抛低吸**——不是全局串行
4. **这个结构是相当复杂且有机的**——需要深度思考，尤其结合多重赋格

## 多重赋格的递归结构（待深入）

每一层ladder的短差是同一个三元组的实例：
- 域(ℓ) = ladder ℓ 的存活中枢 [ZD,ZG] × lifetime
- 开/闭点(ℓ) = ladder ℓ-1 的走势类型完成点
- 失效(ℓ) = ladder ℓ 的第三类买卖点 → 回补位

递归组合：ladder ℓ 的一条卖腿本身构成 ladder ℓ-1 短差的运行窗口。
多重赋格 = 反向区间套在级别塔上的逐层应用。
每层的域来自上一层，每层的操作点来自下一层。

## 待思考

- 多重赋格的有机结构：多个级别并发做短差，各自有自己的域和锚，如何协调？
- 降成本→挣股数的跨越：正确配对后cost_basis能否真正降到0？
- 实盘正则化：代码的递归正则化如何直接映射到交易纪律？

**Why:** 用户指出这是操盘方法的核心结构，需要深度思考
**How to apply:** 所有涉及Version I多重赋格的改动必须保持递归正则化

Related: [[project_shared_position_fugue]], [[project_alpha_diagnosis]]


---


---
name: RTAS swarm infrastructure
description: Three-layer L1/L2/L3 architecture, extracted to ~/rtas/, dispatch skill installed
type: project
---

**三层架构：** L1 Dispatch（Cowork战略层）→ L2 CC Agent Team Lead（战术层）→ L3 Workers（执行层）

**文件位置：** ~/rtas/（CLAUDE.md + .claude/skills/ + agents/ + hooks/ + team-topology.json + settings.json）

**核心设计原则：** 构成性原则定义"是什么"，调节性规则管"做什么"；缠论递归/分形思维是构成性类比；Wave-based执行（侦察→执行→验证）；蜂群是矛盾显现机器

**Why:** 用户的多agent协作基础设施
**How to apply:** 使用meta-orchestration skill时参考此架构


---


---
name: session2-organic-fugue
description: 2026-06-11第二个session全成果：C段修复验证→30+实验→V2oa25_ht全局最优（OKLO+1800%/BRN+599%/CL+66%），大量否证链确立系统边界
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## Session成果总览（2026-06-11~12，~40个实验）

### 最优配置矩阵（两种配置覆盖全标的）
**V2oa25_ht**（osc正域：OKLO/BRN/PINS）：满仓入场+θ自适应P25+成本门+41课门+HoldTrend出场+REV配对
- OKLO +1800%（BH +307%），BRN +599%（BH +87%），PINS +52%（BH −7%）

**V2oa25_ht_scco**（osc负域：BTC/CL/ES/GC/QQQ/DX）：上述+中枢上移出口(sc)+盘整域限制(co)
- BTC +630%（BH +1380%），CL +250%（BH +28%），ES +304%（BH +594%），GC +203%（BH +257%）

### osc域严格化（session最大突破）
- **僵尸腿诊断**：osc亏损根因=type3确认不可达→空腿挂到master强平。BTC osc亏−281K
- **中枢上移出口(sc)**：49课"中枢向上移动时满仓"→新中枢ZD>旧ZG→回补。BTC +412.7pp，8/10正
- **盘整域限制(co)**：38课隐含盘整前提+49课满仓+26课单边不做短线。BTC +489.7pp
- **sc×co组合**：BTC +623.2pp（+6.6%→+629.8%），负域6/6全支配co
- **sign(Δ)=−sign(基线osc盈亏)**：10/10零例外的regime分裂。白名单尚需消除

### 已确立原则（否证链收敛）
1. **满仓入场**：递归建仓三档全负，收益严格单调随base_frac（暴露=alpha源）
2. **状态驱动出场HoldTrend**：sell1+趋势衰竭确认。事件驱动出场（signal）次优，级别升级出场（climb）否证
3. **θ自适应P25**：固定θ=1%四次量级失配（QQQ/BRN/CL/CL秒级），同层P50跨标的差21倍
4. **confirmed=次级别确认**：共现率100%，再加确认=纯延迟（R1/R3/P6/SC四独立否证收敛）
5. **级别是涌现非预设**：a0=递归构成性例外，47/53分割跨资产不变
6. **确认深度=状态范畴非事件范畴**：三次收敛（BSP slice饱和/卖点确认撤销/master入场）
7. **盘背卖回归出场位**：R2(ZG锚)让payoff翻正但信息含量≈0，R1+R2+R3全开仍负→回归原文位置
8. **逃逸型三标的一致负**
9. **38课循环串行否证**：C38pair劣于单次操作，死因=买回太快（区间套双向性）
10. **递归深度轴关闭**：depth=2仅1对腿N=1，OKLO/BRN域稀疏

### 关键发现
- **C段修复后基线漂移**：BRN P5从+488%坍缩到+28%（旧alpha=伪背驰伪影）
- **type2 Sell2开腿**：5/5全胜+145pp（N=5待验）
- **BRN分型递归depth=1**：+10.7pp首张有效票（但近似实现非严格形式）
- **38课程式≠分型进出**：严格形式=段间盘整背驰，分型是近似实现
- **O(N²)第三次修复**：事件流消费层delta化，CL 432s→16s（26.5×）
- **voice容器已是N个per-ladder**：缺的是操作对象和正确耦合
- **非阻塞递归**（用户分享对话）：赋格紧接段stretto=自我应用在自我尚未完成时刻
- **并发赋格深度思考**：控制状态串行化，资源状态并发安全。voice=有门的账本

### 数据资产
- CL秒级1年：5.96M bars / 309MB / $0（Databento套餐），引擎4.2秒
- BTC 1min全量：4.6M bars 2017-2026
- 期货7标的O(N)全量：29.3M bars ~10分钟
- 全部analysis/报告：~20份.md + data_cache JSON

### 额外完成项（session后半）
- **Sequence38**：38课严格形式跨regime全正（OKLO+10.2/BRN+4.1pp），已并main
- **HoldTrend出场**：4/4正，BRN +495pp。状态驱动>事件驱动
- **Climb爬梯出场**：4/4否证。等高级别=等高级别下跌走完
- **递归建仓/入场递归**：全否证。alpha来自暴露。确认深度=状态范畴
- **38课循环串行**：两轮否证。买回太快（区间套双向）
- **十标的V2oa25_ht**：正域6/负域4
- **僵尸腿诊断**：osc亏损根因=type3不可达→空腿挂到master强平
- **sc（中枢上移出口）**：49课满仓，8/10正，BTC +412.7pp
- **co（盘整域限制）**：38课+49课+26课，BTC +489.7pp
- **sc×co组合**：BTC +623.2pp（+6.6%→+629.8%）。sign(Δ)=−sign(基线osc)10/10
- **BTC/PINS数据**：在册。BTC REV胜率56.5%全表最高
- **操作判据审计**：17项原文对照（5严格/9近似/2外加/1缺失）
- **降成本实证**：十标的仅1笔触发earning

### 未解决
1. **osc regime白名单消除**——10/10反相关需从原理消解
2. **BTC超BH**——+630% vs BH +1380%
3. **并发赋格实现**——Part II有方向未验证
4. **秒级a0回测**——域打开但摩擦存疑
5. **Git commit收口**

**Why:** 下个session接续用
**How to apply:** 读此文件获取完整实验链和否证边界

Related: [[session-organic-fugue]], [[recursive-regularization]], [[quant-system-progress]]


---


---
name: session3-fusion
description: 2026-06-12~13 session3终局：NRF v4(3/8)+概念运动链23环+递归方法论辩证发展+BSP settle门(29.6%伪信号)+清仓真根因=top棘轮+合取稀疏+谱系536-539
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## Session终局（精确诊断收敛）

### 清仓问题的精确根因（最终确认）
**不是信号源问题**（递归层原生type1已正确实装sell1[k≥4]）。
**不是BSP质量问题**（settle门影响type2/3，和type1清仓不交）。
**真根因=top棘轮+located合取稀疏**：top只升不降卡在高层+同层同bar合取砍掉96%→清仓3次。
v5（换参照层top*）已L3否证（过度清仓踏空）。
开放轴=A'（合取选层）+ regime门（谱系539号登记）。

### NRF v4仍是最优基座
P1 3/8（OKLO+695%/QQQ+196%/GC+298%），MDD 8/8全优于BH，零强平。

### 概念运动链（docs/concept_movement_chain.md）
23环辩证推导：走势终完美→全部缠论元素。方法论：概念自我否定运动。

### 递归方法论辩证发展（谱系536号）
分型→特征序列→中枢=同一方法（包含处理）的量变→质变发展。

### BSP引擎层（谱系537号）
- 同一-差异双重性：线段=次级别走势（同一），走势完成需确认（差异）
- settle门实装：type2有29.6%伪信号被识别。type3自带保护0%
- 递归层BSP适配器已接通生产路径（bit-exact守卫通过）
- OKLO稀疏度：L2=43/L3=4 vs L1≈995→区间套必要性实证

### 会计双重性（谱系538号）
同一笔交易在父=降成本/子=独立头寸。资金守恒解锁深层嵌套。

### 嵌套递归赋格严格会计（docs/nested_fugue_accounting.md）
不清仓原则+earning多空对称+逐仓独立+区间套=voice spawn

### 量化交易系统
NautilusTrader骨架+Hyperliquid三通道PASS+Databento接入。HL钱包已连接。

### 回测历史
BTC btra_s34 +7368%（per-asset）/ fusion_tr +4298%（统一）/ NRF v4 OKLO +695%（严格会计）

### 下一session优先级
1. **清仓A'+regime门**：解决top棘轮+合取稀疏（谱系539开放轴）
2. **区间套定位型消费完整接通**：Piece 2C Step C（交易层+L3回测）
3. **统一voice FSM从修正信号层重跑**
4. **Hyperliquid实盘接入**

**Why:** 下个session接续
**How to apply:** 清仓真根因=top棘轮+合取稀疏（非信号源），从539号谱系A'/R6'出发

Related: [[recursive-nested-fugue]], [[concept-movement-chain]], [[bidirectional-always-in]]


---


---
name: session4-strict-necessity
description: 2026-06-14~16 session4终局：螺旋覆盖空间D∞+58条推论穷尽+向心回溯T49+零强平+NT回测P1=2/8+BTC踏空=E过度spawn+会计交易层3条遗漏
metadata:
  type: project
  originSessionId: cowork-2026-06-14-16
---

## Session核心成果

### 螺旋覆盖空间理论体系
- 递归自相似→对数螺旋（唯一共形不变曲线，spira mirabilis）
- 三坐标：φ角向(走势相位/背驰)、r径向(级别/区间套)、ε手性(多空/莫比乌斯)
- 群：D∞ = ⟨h,τ | τ²=e, τhτ⁻¹=h⁻¹⟩，h²³=σ（23环闭合=级别跃迁）
- 穷尽性：58个独立不变量槽位，58条推论全覆盖（T1-T59，含4条基础关系T56-T59）
- 覆盖空间基点无关（连通性保证）——信号/会计/交易是同一螺旋三个投影

### 关键代码改动（全部commit到main）
1. T49向心回溯（confirm从前向→回溯，展开恒等式Δt∝λ^{k-1}(λ-1)强制）
2. 成本门从confirm链删除（confirm是认知不受操作约束）
3. 否定线彻底删除（非缠师原文概念）
4. 观测态删除（非必然推论）
5. 三层prove体系（信号S5/S6/S7/S11/S12 + 会计A4/A5 + 交易N1-N8+T1+T14+T49-T59）
6. T52多义性+T53结合性+T54表里+T55比价（108课缺瓦补全）
7. NautilusTrader流式接口（UnnStream PyO3，backtest_unn_stream.py）
8. Fable 5系统提示词SessionStart hook注入

### 回测结果（NT流式，最新引擎）
P1=2/8 {OKLO+431%, DX+8%}。MDD 8/8全优BH。prove零panic 8/8 ~27M bar。

### BTC踏空根因（L3）
不是F没建仓不是C翻错——是E降成本spawn 333次子空头在2017暴涨年亏-45246。降成本alpha是regime函数：单边牛市里segment级别做空=稀释主升浪。

### 会计/交易层独立穷尽性
28条独立不变量。3条遗漏（T61/T62/T63，文档落盘不可信需重做）。

### 开放问题（下session）
1. 降成本E spawn在单边趋势中亏损——N7(E不受门控)和区间套(segment只做定位)的矛盾
2. 会计/交易层3条遗漏推论的严格定理化+prove实装
3. 代码清洗（以螺旋推论为基准删不必然代码）——大部分已清洁（零经验参数）
4. 全量NT 1秒回测
5. 541号谱系完整记录螺旋覆盖空间发现

### 方法论纲领（用户确立）
- 必然性累积不被经验否定
- 回测只否定拼接方式不否定推论
- prove函数（逻辑检验）是验收标准
- 不搞任何补丁/经验性参数
- 从覆盖空间穷尽性反推遗漏推论
- 基点无关（连通覆盖空间）

**Why:** 这是到目前为止最深的session——从实装层推进到理论层（螺旋覆盖空间D∞群+穷尽性枚举+108课全覆盖）
**How to apply:** 下session从E spawn矛盾+3条遗漏+代码清洗出发

Related: [[necessity-accumulation]], [[located-direction-discovery]], [[session3-fusion]]


---


---
name: session5-spiral-reinterpretation
description: 2026-06-16~17 session5：H⁰=核心仓(2/3)+H¹=机动仓(1/3)物理重诠释，τ⊥减仓范畴正交，四步循环=H¹ 1-cycle，递归赋格必然性证明，操作路线D∞word穷尽，三轴级别间关系（中枢构成/区间套超出D∞），segment黑洞修复+MACD开启
metadata:
  type: project
  originSessionId: cowork-2026-06-16-17
---

## Session核心成果链

### 1. theta σ-不变修复 + 会计层验证
- prove_theta_sigma_invariant补全542缺瓦（commit e2a78b0dd7）
- 会计层14/14规则有必然性依据

### 2. 辩证法穷尽
- docs/dialectical_exhaustion.md（1188行，94条张力逐条推导）
- 58闭合/34 gap/2须裁定，5个gap根（G1-G5）
- 代数验算全一致（Burnside=46, dim H¹=1, 68节点94边）

### 3. 螺旋引擎v2实装
- rust/src/spiral/ 13模块，38/38测试，bit-exact于unn
- P-close（Δr=-1跨级别闭合）激活后BTC更差（wins 9→0, 强平89→102）——因segment信号黑洞

### 4. Segment黑洞修复
- sub = max(p_ladder-1, PENDING_LO) → BTC +340%→+426%，max_kids 80→6，强平89→0

### 5. MACD开启
- take_trend_div_events panic→返回空Vec，enable_macd_divergence=True
- BTC +426%→+436.5%，轻微改善

### 6. 全面诊断：引擎偏离缠论
- 203笔中只有1笔多头——不是递归嵌套赋格
- 背驰用振幅代理不是MACD面积
- E spawn = 独立空头子voice，不是缠论的"先卖后买"减仓-回补

### 7. 物理重诠释（最深成果）
- H⁰ = 核心仓（2/3恒持，σ-不变Casimir）
- H¹ = 机动仓（1/3短差，减仓-回补循环）
- τ（翻转）⊥减仓（范畴正交）：τ是ε维全翻转，减仓是M维units变化
- 四步循环 = H¹的1-cycle，扭曲线积分=-2≠0（非平凡）
- 递归嵌套赋格 = H¹生成元沿σ-塔的σ-等变实现

### 8. 操作路线穷尽
- 字母表Σ={e,h⁺,h⁻,τ}，8个合法组合=F/C/D/E/A/hold
- D∞ word穷尽操作路线（句法侧）
- 但级别间关系三轴分布：中枢构成(H⁰,群外)/区间套(groupoid,逆极限)/仓位流动(H¹,完全)

### 9. 用户关键洞察序列
- "买卖点首尾相连→本级别只有三类买点，其他都是次级别买卖点"
- "根voice不应该存在——级别是涌现的"
- "翻转应该被扬弃——先卖后买是四步有时序的循环"
- "四步循环只是例子——还有空方版本和会计双重性"
- "级别之间的关系不止σ"

### 开放问题
- 中枢构成（涌现）和区间套（逆极限）超出D∞→需要形态学公理+范畴极限
- v3引擎实装（D∞ word处理器，非硬编码四步循环）在跑但可能卡住
- 1-chain的具体代表元（四步循环的空方/会计投影全部变体）需从原文完整提取

### 10. v3回测结果（2026-06-17）
- 4/8 T24 panic（OKLO/CL/ES/GC）——相邻层同向Long，prove过严
- 4/8跑完：BTC +760%（vs BH +1380%）、QQQ +23%（vs +175%）、DX +4.5% PASS、BRN +34%
- T24 prove已改为非panic计数器（count_chiral_violations），但caller未更新
- 下session需要：更新6处caller→build→重跑4个panic标的

### 待做（下session优先）
1. T24 caller更新（cycle.rs:17/46/77, operate.rs:31/181/266）→ build → 重跑OKLO/CL/ES/GC
2. 数据源路径统一（backtest_fugue_v3.py和backtest_unn_stream.py用不同parquet）
3. 全8标的v3 vs unn同数据对比

**Why:** 从"穷尽了58条但引擎跑不赢BH"出发，推进到发现H⁰⊕H¹两层闭合+物理重诠释+操作路线穷尽+三轴级别间关系+v3引擎实装+T24 panic诊断
**How to apply:** v3引擎=D∞ word处理器（不硬编码循环），每bar每级别选{h,τ,e}，双投影（操作+会计）

Related: [[session4-strict-necessity]], [[necessity-accumulation]], [[orbit-enumeration-duality]]


---


---
name: session-mental-model
description: 本session全局心智模型：引擎O(N)完成、谱系生产7命题、P1否证、P3待跑、Version I架构误解已发现
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 引擎层（已完成）

- **全链路O(N)达成**：bi_zhongshu路径75×（四层增量器）+ 主链segment层98×（SegCheckpoint resume）
- **Rust 8层bit-exact**：K线→分型→笔→线段→中枢→走势→背驰→买卖点，Python↔Rust逐位等价
- **L3跨标的验证**：OKLO/ES/GC/CL/DX 全部零mismatch
- **数据**：7期货10年1min（Databento）+ BTC 4.6M bars（Binance归档2017-2026）+ 中国IF/AU/SC（akshare 1023根/次）

## 谱系生产（核心概念层产出）

7个新命题从已结算否定中扬弃推导：

| 命题 | 状态 | 结论 |
|------|------|------|
| **P0** 谱系-实证断层 | L0成立 | 500条settled全偏概念层，实证否定未结晶 |
| **P1** ⋆=D（Hodge star=缠论D算子）| **L2否证** | 47.3%一致率≈抛硬币，退化bar上persistence≡amplitude |
| **P2** ω=配置驻留尺度参数 | L3待验 | 依赖K4转换矩阵+ω列 |
| **P3** alpha=暴露守恒 | **L2-L3待判决** | 脚手架完成，核心函数待确认后跑 |
| **P4** K4内禀信息下界 | L3待验 | 依赖固定全L3重算双熵 |
| **P5** ⋆=D推广 | **随P1否证降级** | 降为启发式 |
| **P6** 门控门regime切换 | L3待验 | 依赖P3+P4 |

## Version I架构误解（关键bug）

**正确理解**（用户纠正）：
- 一次满仓建仓100%暴露
- 区间套确认的最高级别买卖点决定出场
- 最高级别以下所有级别都是"多重赋格降成本"空间
- 降成本是在同一个仓位上叠加操作，不是独立slice
- 操作后仓位恢复100%

**当前实现的错误**：
- 每个级别独立FSM各管一个slice（机械化）
- sub-level卖点可能触发清仓（应该只触发降成本短差）
- 入场可能是ladder式逐级建仓（应该一次满仓）
- 审计任务在跑，精确到行号的报告即将完成

## 关键否定性结果汇总

| 否定 | 精确边界 | 保留 |
|------|---------|------|
| COT 7/7不领先 | 所有口径、所有标的 | 残差方向仍有效（内生反转点） |
| ω→美股方向 | p=0.069反向 | ω→金油比自身均值回归L2确认 |
| 门控毁alpha | K4门控932→128% | 排序保留96%（暴露守恒） |
| 2.074 bits不可约 | Γ→交叉边 | 交叉边必须直接读，六边各自跑 |
| ⋆=D | L1退化bar残差 | 高级别/非退化bar未关闭 |
| 流速空信号 | 2020饱和L3 | 流量（振幅）在内生反转点有效 |

## 正在跑的任务

- BTC V-I回测：I信号32%→1.5M减速
- Version I架构审计：在写报告
- P3随机门控：脚手架完成待确认
- V-I 7期货：后台nohup
- K4矩阵：数据完成写报告

## 度规问题（开放）

P1否证后度规问题回到开放状态。剩余候选：
- 方向4：K4转换矩阵平稳分布作为内禀度规（P4假说，不依赖⋆=D）
- 非退化bar残差上⋆=D重检（persistence≠amplitude时）
- 高级别走势方向的方向性力度（需可靠settle_ts锚定）

**Why:** 防止context compact后丢失关键结论和状态
**How to apply:** 新session开始时先读此文件获取全局心智模型

Related: [[quant-system-progress]], [[project_todo_master]], [[project_m3_capital_flow_milestone]]


---


---
name: session-organic-fugue
description: 有机赋格v2完整session成果：引擎O(N)全修+信号配对诊断+递归正则化+Rust交易层+C段缺陷诊断+逢亮盲测
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## Session全局成果清单（2026-06-10/11）

### 引擎层（全部完成验证）
- **全链路O(N)**：7堵墙全修（bi拷贝20×/segment resume/5堵O(N²)/segment级增量化）
- E: 14.67→1.96s, I: 49.15→3.36s (OKLO 447K)
- DX 2M bar: 18.7min→0.3min (62×加速)
- 37个Rust单测全过，三标的bit-exact

### 信号配对诊断（关键发现）
- **BarSignalI信息销毁**：L237把BSP的kind/center折叠成4个布尔
- type1卖→type1买：胜率78.9%（正确配对）
- type1卖→type3买：卖飞率67.9%（凶手，43%的配对）
- type3买点是回补位不是止损信号（用户纠正）
- 卖飞根因：不是信号滞后（卖在顶附近0.99%），是次级别上冲(2.24%)≫回调深度(0.68%)

### P5（已验证最优组件）
- 域腿：ZG高抛/ZD触线低吸/type3逃逸
- 三标的vs_B0全正：OKLO +450.6pp, QQQ +12.6pp, BRN +131.5pp
- 域腿胜率72-77%

### 有机赋格理论框架
- **递归正则化 = 区间套反向应用**：高级别中枢定义域，次级别走势类型定位点
- **北极星**："吃到每一笔"——每个方向、每个级别、每段走势都是利润来源
- 38课：向下段先卖后买，市场肢解为段的连接，无所谓牛熊
- 40课：N重层次操作，分区卷钱的机械
- 49课：利润最大定理——推导重构完成（8前提/7步推导）
- **49课vs38课冲突消解**：master执行49课（下跌段持币），voice执行38课（向下段先卖后买），FLAT vs LONG持仓前提不同

### REV腿（反向完整操盘）诊断
- V1f（修osc截断后）：OKLO +607.5pp翻正，QQQ -9pp, BRN -126pp
- REV本身净现金≈0，-626pp的根因是副作用（master频繁出场+osc强平）
- 532号裁决：master出场和voice REV彻底解耦
- QQQ/BRN负的根因：反向振幅太浅（MFE: OKLO 0.838% vs QQQ 0.090%）
- REV有效域=反向振幅充分的标的
- REV胜率45%——根因同配对问题（盲开盲闭）

### REV tranche递归建仓
- 空定义域是定理性的：buy1 ladder恒为{2,3}，entry≤3→区间(2,2]=∅
- 5min/30min换周期：6组全空转，涌现级别单调不增
- 空定义域和周期/标的无关，根因在引擎层

### 引擎C段缺陷（最根本的发现）
- **趋势背驰在递归层是不可能事件**：move的seg_end ≡ last_zs.comp_end → C段恒空 → 背驰永不触发
- 缠师24课C段终于转折点，代码C段截断在move边界——偏离原文
- type2缺失同根因：type1存在窗口是空集→type2没有锚
- buy1[2]的2559次触发全部是"生长中C段"伪背驰
- **修复B2**：C段越界到转折点→L2 type1从1暴增到373，type2从0到360
- 修复A：递归层seg_end对齐level-1→L4出现type1/type2，entry 3→4
- C段修复任务在跑（251轮），代码改动应已完成但验证卡住

### Rust交易层（已实现）
- rust/src/trading/ 目录：types.rs/level_operating_unit.rs/fatigue_gate.rs/ledger.rs/runner.rs
- O0≡P5 bit-exact三标的四面对账全PASS
- 编译器暴露10组矛盾（2组设计稿声明膨胀）
- PyO3接口暴露

### 两阶段守恒律（31/43课）
- cost>0：股数守恒（买卖等量）
- cost≤0：资金守恒（先卖后买挣股数，仓位递增）
- earning反作用：cost归零后master出场级别升级（设计存在但触发率=0）

### V-I多重赋格改动历史
1. slice模型→共享仓位（爆仓-99.88%，bar级短差满仓）
2. 用户纠正：多FSM各管各的架构是对的，但入场满仓+规模从结构来
3. v2均分总仓位→全标的回测
4. 递归正则化框架→P5验证→REV诊断→C段缺陷

### K4拓扑
- 48%级别异质性→分层Γ方案
- DX正典锚6维729态：48态/K4一致93.3%
- GC锚6维：120态/K4一致95.0%

### 谱系生产P0-P6
- P1 ⋆=D：全级别否证（persistence≡amplitude代数恒等式）
- P3暴露守恒：弱成立（54百分位，N=6功效不够）
- P2/P4/P6待验

### 逢亮盲测
- fable5一轮命中根因：settlement产物注入K_active无消费者→O(S²)膨胀→活性反馈环
- 方案：停止memory注入（和实际修复v264-swarm完全吻合）

### 其他完成项
- ROADMAP更新+Git commit（7个commit）
- 期货E+P3完成（7标的，P3联合54百分位弱正不显著）
- 闭合残差耗散结构验证（在跑）
- Γ→Δ v2成交量搜索（完成，边缘弱正）
- 跨国K4管线（数据产出，报告待补）

## 下一步（新session接续）

### 最高优先级
1. **C段修复验证**：任务可能已改好代码但验证卡住，需要新session确认改动+跑验证
2. **修复后全量重跑**：引擎信号层变了，所有回测都要重跑
3. **Rust回测闭环**：cargo test做集成测试，不走Python→PyO3→bash链路

### 待办
- REV配对修正（在新信号上重做）
- REV深度门槛
- tranche递归建仓（C段修复后entry可能升到4+）
- 全标的期货回测（新引擎）
- Git commit所有改动

### 任务控制
- 每个任务≤30轮
- 用cargo test不用pytest
- bash加timeout 60s
- 长计算用nohup后台

**Why:** 防止新session丢失本session的大量成果
**How to apply:** 新session开始时读此文件获取完整上下文

Related: [[recursive-regularization]], [[quant-system-progress]], [[project_todo_master]]


---


---
name: todo-master
description: 量化系统全局待办清单（2026-06-08），按依赖关系排序
metadata:
  type: project
  originSessionId: 437f75d0-be82-4f3c-912e-5aff8e6cea24
---

## 当前阻塞项（并行进行中）

### A. Databento 1min 10年数据拉取（进行中）
- ES/GC/CL/ZN/DX/BRN 全期货，ohlcv-1m
- CME + ICE Europe 订阅已生效
- 依赖：无 → 是下游所有任务的前置

### B. 引擎性能优化v2（进行中）
- attach_persistence 超线性增长 ~N^1.86 → 目标O(N)或O(N logN)
- 并行化6条边（multiprocessing）
- 比价OHLC构造向量化
- 依赖：无 → 是下游计算任务的前置

## 阻塞解除后的任务队列

### 1. 1min a0 K4配置转换矩阵（依赖A+B）
- 6条边1min比价OHLC → 缠论递归 → σ → Γ → 27态转换矩阵
- 10年数据看配置转换路径、吸收态、平均驻留时间

### 2. 闭合残差缠论递归（依赖A+B）— 验证耗散结构假说
- 4个三角形的走势结构层闭合残差时间序列
- 对每个残差做缠论递归：有结构（趋势/中枢/背驰）= 耗散结构成立
- 2020-2022历史验证：空转/走资/沉没的拓扑签名

### 3. Γ→Δ映射1min版重验（依赖A+B+COT数据）
- 1min Γ + COT持仓数据 → Δ
- 重验H(交叉边|Γ)和ω分辨能力
- 搜索补齐剩余熵的变量

### 4. 跨国K4管线实装验证（依赖A）
- cross_national_pipeline.py已写好
- 用真实1min数据（ES/BRN + EURUSD/USDJPY）验证跨国闭合
- 中国数据仍缺（akshare待拉）

### 5. 递归分解树纵向闭合实测（依赖A+B）
- P顶点（ES）分解为成分股
- 纵向闭合：ES/CL vs 各成分stock_i/CL
- 选股信号：哪些成分驱动宏观趋势

### 6. M1操盘层E版本1min全标的重跑（依赖A+B）
- 用优化后引擎+1min 10年数据
- E版本（背驰定位器）在ES/GC/CL/ZN上的表现
- 和BH对比，看不同宏观周期的alpha分布

### 7. M2选股C路径端到端回测（依赖1+2+6）
- 用1min a0的K4联合读数驱动选股
- 多品种组合：C路径排序→资金配置→E版本操盘
- 验证"排序优于门控"的结论在10年数据上是否稳健

## 独立任务（不阻塞）

### 8. 中国市场数据拉取
- akshare拉IF/AU9999/SC 1min数据
- 或TradingView拉取

### 9. Git全量commit
- 大量未提交的代码和文档
- K4折叠重构、递归分解树设计文档、分析脚本等

### 10. ROADMAP更新
- docs/ROADMAP.md 需要反映当前进展
- M1状态：E版本有alpha（OKLO +932%），降成本/挣股数/完整版待重验
- M2状态：K4折叠重构完成，C路径验证，递归分解树设计中

**Why:** 追踪所有待办，防止遗漏
**How to apply:** 每完成一项打勾，新增任务追加

Related: [[quant-system-progress]], [[alpha-diagnosis]]


---


---
name: zombie-short-diagnosis
description: 僵尸空头三条路径(A根仓做空方向错/B高层sink回补难/C强平)，σ-ascend无关，伤害=regime函数
metadata:
  type: project
  originSessionId: cowork-2026-06-17
---

## 2026-06-17 僵尸空头产生机制追查

### σ-ascend无罪
σ-ascend只搬核心仓多头层（operate.rs:193-202），对空头零作用。僵尸空头不是被推高的。

### 三条产生路径
- **路径A**：核心仓直接在高层F做空（ES bar=870220做空价1404→涨到7368=+425%，-146K）
- **路径B**：高层sink子层空头，recover需父层nf_buy极稀疏（rL2以上recover率=0%）
- **路径C**：低层空头撞2×basis强平线（ES seg liq_short -118K）

### nf_buy级别衰减
move(3)≈5000次 → recL2(4)≈150次 → recL3(5)≈12次 → recL4(6)≈1次
每升一级少一个数量级。recover率：seg层CL93%/BTC85%，rL2及以上全标的0%。

### 伤害=regime函数
CL（震荡BH+28%）：short PnL +1477（正），同样结构但震荡中空头赚钱
ES（强牛BH+594%）：short PnL -257K（灾难），单边涨空头结构性亏损

### 关键洞察链（用户推动）
1. "笔的走势类型是线段" → 考证：第65课否定（严格意义上不对）→ PENDING_LO=3正确
2. "停泊层≠触发层" → 空头停segment，recover用父层move信号，不死锁
3. "σ-ascend不推空头" → 代码确认，空头高层是sink在高层直接产生的
4. "多头也不该升级？" → 不，核心仓跟随涌现级别是缠论的（清仓等高级别卖点）
5. "涌现级别不可能减小" → 对，无论涨跌，级别涌现单调

### 零操作参数裁决
engine.rs:44 断言 floor_ladder==FIRST_BSP_LADDER，引擎是"零操作参数引擎"。加sink_ceiling等参数违背裁决。

**Why:** 从"ES -418%为什么"追查到σ-ascend无罪→三条路径→regime函数
**How to apply:** 修复应从路径A入手（根仓做空方向判断），路径B/C是结构性的regime依赖

Related: [[emergent-direction]], [[session5-spiral-reinterpretation]]


---


