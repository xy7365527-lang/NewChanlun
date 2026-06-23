# 强牛「假空腿」根因诊断：穿仓 + 杠杆是否同根 = 多重赋格在强牛开了完全分类不该有的空腿

> 工位：缠论线 friction-model（topo_address: swarm/friction-model）。日期：2026-06-23。
> 编排者重定优先级：主诊断不是事后摩擦，是「强牛里开了完全分类不该有的空腿」根因（穿仓+杠杆同根）。
> 实装：隔离 worktree `/tmp/friction-current`（分支 prop4-nest-readingB detached @ d18b083a03，含 efcea91ba1 多重赋格 reading_b）。新增 per-级别腿方向时序埋点（`leg_bars_long/short[k]`、最高活跃级别 Short 占比、liq_log 级别×方向），全观测、bit-exact 不变。
> 认识论：腿方向/失血归属/穿仓落点 = **L3**（BTC/OKLO/ES × RB_DTOP/RB_DIVERGE 真实数据）；「为何 OKLO 顶层标 Down」的 trend-label 内因 = **L2 假说**（未 dump 标注内核，开放轴）。
> 原始数据：`.chanlun/review-results/fake-short-diag-raw-20260623.txt`。

---

## 0. 一句话判决

**编排者假说被 L3 证实并精化：穿仓 = 强牛里持有「冻结的空腿」（开空后 switches≈0–1、bars_short=10⁵⁺、做空整段牛市）无界失血；穿仓与杠杆同根 = reading_b 多重赋格「每级别独立开空腿（按该级别 trend 标记），无升跌完备性/父级门控」。失血空腿出现在 trend 标记冻结为 Down 的任意级别——OKLO 是最高级别本身（塔矮，top L2 做空 100% 时间），BTC 是次级别（top L2 正确 Long，但 L1 冻结空腿失血 −98k~−132k > 顶层多头收益 → 穿仓）。ES 不穿仓因为最高级别 L3 正确 Long（+694k 主导），sub 空腿有界。修法 = honor 第21课:40 升跌完备性（确认上涨趋势下不持有持续空腿，sub 空腿只能是有界回调即完成即平），即给 reading_b 补 instances/route_bsp 路径已有的父级门控——不是 ANCHOR（非硬编码 long），不是摩擦。**

---

## 1. 诊断方法（L0）

reading_b 多重赋格机制（`rec_engine.rs:g/consume_legs`）：每级别 k 的腿独立骑 `levels[k]` 走势，方向 = `node.direction`（ride 分支按 trend 方向开腿）；`d_top[k]`（走势完成链 RB_DTOP / 背驰段链 RB_DIVERGE）触发 → close+反向 open（switch）。**关键：每级别独立翻转，无 `nearest_active_parent` 父级门控**（547 隔离的代价）。

新增埋点（纯观测）：
- `leg_bars_long[k]` / `leg_bars_short[k]`：级别 k 腿活跃在多/空的 bar 数。
- 最高活跃级别（`max_active_lvl_seen`）腿为 Short 的 bar 占比（`top_leg_short_bars/top_leg_active_bars`）= 最高级别假空腿强度。
- `liq_log`（已有）：穿仓时各腿 (级别, 方向)。
- `per_level_long/short_pnl[k]`（已有）：哪级别腿赚/亏。

---

## 2. L3 数据（BTC/OKLO 穿仓 + ES 不穿仓对照）

| 标的 | 变体 | strat% | BH% | liq | 最高活跃lvl | **顶层腿方向** | 顶层腿pnl | **失血空腿（级别/pnl）** | 顶层Short占比 |
|------|------|--------|-----|-----|-----|------|------|------|------|
| ES | RB_DTOP | +681.4 | +594.3 | **0** | L3 | **Long** | **+694,733** | 无（L1/L2 小额） | 7.4% |
| ES | RB_DIVERGE | +678.4 | +594.3 | **0** | L3 | **Long** | **+706,359** | 无 | 7.4% |
| BTC | RB_DTOP | −100.6 | +1380.4 | 1 | L2 | **Long** | +75,377 | **L1 短 −131,787** | 6.1% |
| BTC | RB_DIVERGE | −100.4 | +1380.4 | 1 | L2 | **Long** | −7,581 | **L1 短 −98,165** | 4.1% |
| OKLO | RB_DTOP | −101.0 | +307.1 | 3 | L2 | **Short** | **−101,552** | **L2(顶层) 短 −101,552** | **100.0%** |
| OKLO | RB_DIVERGE | −100.1 | +307.1 | 3 | L2 | **Short** | **−91,760** | **L2(顶层) 短 −91,760** | **100.0%** |

### 2.1 OKLO（塔矮，top=L2）：最高级别本身假空腿（编排者假说字面成立）
- 顶层 L2 腿：opens=1、**switches=0**、bars_long=**0**、bars_short=37,296–37,307、short_pnl **−91k~−102k**（穿仓主因）。
- **顶层腿 100% 时间做空**（牛市 BH+307%）。开空后从不翻转（switches=0）= **冻结空腿**。
- 穿仓时 3 条腿全是 Short（DTOP: L0/L1/L2 全空）。**第21课:40 字面违反：确认上涨趋势下最高级别持有持续空头。**

### 2.2 BTC（塔到 L2）：顶层正确 Long，次级别 L1 冻结空腿失血（假说机制精化）
- 顶层 L2 腿：opens=1、**switches=0**、bars_long=415k–637k、bars_short=**0**、long_pnl +75k（DTOP）/−7.6k（DIVERGE）。**顶层正确 Long（捕捉主升浪）。**
- 次级别 L1 腿：opens=1、**switches=1**、bars_long=0、bars_short=110k–131k、short_pnl **−98k~−132k**（穿仓主因）。**L1 开空后仅翻一次 = 冻结空腿，骑整段牛市做空失血。**
- 穿仓时仅 L2 Long 腿在场（L0/L1 空腿先失血触发账户级强平）。
- **穿仓来源 = 次级别冻结空腿（不是最高级别）**——编排者假说的级别需精化：失血空腿在 trend 冻结 Down 的级别，BTC 是 L1 非顶层。

### 2.3 ES（不穿仓对照）：顶层 Long 主导 → 杠杆「存活」但仍杠杆膨胀
- 顶层 L3 腿：opens=1、switches=0、bars_long=**5,143,513**、bars_short=0、long_pnl **+694k~+706k**（主导收益）。
- sub 空腿（L1/L2）：bars_short 大但 short_pnl 小额（−19k~−30k，被顶层多头碾压）。
- **liq=0**：顶层多头 5.1M bar 主导 → 不穿仓。但仍是 gross 2.61×>net 1.74× 杠杆（见 §3）→ 去杠杆收益 ≈ BH（见摩擦杠杆判决 friction-leverage-verdict）。

---

## 3. 穿仓 + 杠杆同根（编排者核心判据，YES）

### 3.1 杠杆来源 = 同时持「顶层 Long + 次级别 Short」（多腿叠加）
ES gross 2.61× > net 1.74×：顶层 L3 Long（5.1M bar）+ L1/L2 Short **同时持有** ⇒ 毛敞口 = 多腿名义之和 > 净。**杠杆不是借贷，是多空双开同时在场**（做空腿卖空 refund free → 可同时开多腿，563 裂隙2 机制）。

### 3.2 同一机制，两种结局——同根证实
| 标的 | 顶层 | 主失血空腿 | 净结局 |
|------|------|------|------|
| ES | Long(+694k) | sub 小额 | 顶层多头占优 → **存活**（杠杆膨胀，去杠杆≈BH） |
| BTC | Long(+75k) | **L1 冻结空(−132k)** | sub 空头 > 顶层多头 → **穿仓** |
| OKLO | **Short(−102k)** | 顶层即空头 | 净空头 → **穿仓** |

**穿仓 + 杠杆 = 同一个多腿架构**：reading_b 在每级别独立开腿，按该级别 trend 标记开空腿（无父级门控）。该机制**既造杠杆**（多空同时在场）**又造穿仓**（强牛里冻结空腿无界失血）。顶层多头占优则杠杆「productive」存活（ES），空头占优则穿仓（BTC/OKLO）。**编排者「穿仓+杠杆同根 = 多空双开在强牛开了完全分类不允许的空」= 证实。**

### 3.3 21:40 违反 vs 合法回调短差（编排者判据2）：判别量 = 是否「冻结」
- **冻结空腿（=bug，违反 21:40）**：switches≈0–1、bars_short=10⁵⁺、short_pnl 大负。OKLO L2 / BTC L1 = 骑整段牛市做空，从不关闭。
- **有界回调短差（=合法，cc-coverage spec）**：switches 多、bars_short 小、short_pnl 正（捕捉下跌即平）。BTC L0（DIVERGE：opens=9 switches=9 short_pnl +4771）/ OKLO L1（DIVERGE short_pnl +5951）= 接近合法形态。
- **穿仓来自冻结空腿，非有界回调短差。** 判别量：`switches/bars_short` 比 + short_pnl 符号。冻结空腿的根 = `d_top[k]`（走势完成/背驰段）在该级别稀疏不触发（556 冻结）→ 空腿开了关不掉 → 无界失血。

---

## 4. 修法方向（验证编排者：升跌完备性父级门控，非 ANCHOR 非摩擦）

### 4.1 根因 = reading_b 缺升跌完备性/父级门控
- **第21课:40**「上涨趋势确定后只可能出现第三类买点」⟹ 确认上涨趋势下最高级别无卖点 ⟹ 不该有持续空腿。
- reading_b 每级别**独立**骑 trend 标记开空腿（547 隔离删了 `nearest_active_parent`）⇒ 任意级别 trend 标记冻结 Down 就开无界空腿，**无「父级是否 Up？则本级空腿只能是有界回调」的门控**。
- 对比：instances/route_bsp 路径**有**父级门控（`Some(p)` 分支「子级永不独立翻转」，反父向 BSP 走 sink/drain 而非翻空）。**reading_b 缺的正是这个门控。**

### 4.2 修法（方向，非本工位实装）
- **honor 升跌完备性**：最高活跃级别走势 Up（确认上涨趋势）⇒ (a) 顶层腿不开空（OKLO 修复：top L2 不做空）；(b) 次级别空腿只能是**有界回调**（完成即平，不冻结）——父级 Up 时 sub 空腿必须随 sub-走势完成关闭，禁止跨主级别冻结（BTC 修复：L1 空腿不骑整段牛市）。
- 方向**由结构涌现非硬编码**：确认上涨趋势 ⟹ 无顶层卖点 ⟹ 顶层涌现保持 Long（区别于 ANCHOR 硬编码 long=已否决）。
- **不是摩擦**：摩擦/杠杆调整是附带验证（friction-leverage-verdict），根因是假空腿。

### 4.3 残留开放轴（诚实）
- **为何 OKLO 顶层 L2 标 Down**（塔矮 → 最高涌现级别不够高捕捉牛市为 Up？vs OKLO 真有大级别下跌结构？）= 未 dump trend-label 内核，**L2 假说**。需进一步钻取 levels[2] 的 node.direction 时序 + 中枢结构。OBSERVABLE（顶层做空 100% → 穿仓）是 L3 实证；内因待查。
- **556 冻结与假空腿的耦合**：冻结空腿关不掉的根 = d_top[top] 稀疏（556）。修升跌完备性门控可能不足，须同时解 556（完成信号稀疏）——否则有界回调空腿也可能因完成稀疏而冻结。两者关系待结晶。

---

## 5. 六要素结果包

1. **结论**：强牛穿仓（BTC −100.4%/OKLO −100.1% vs BH +1380/+307%）根因 = reading_b 多重赋格在强牛持有**冻结空腿**（switches≈0–1、bars_short=10⁵⁺、做空整段牛市）无界失血。失血空腿在 trend 标记冻结 Down 的级别：**OKLO=最高级别本身（top L2 做空 100% 时间，−92k~−102k）；BTC=次级别（top L2 正确 Long +75k，但 L1 冻结空 −98k~−132k 失血更多）**。ES 不穿仓因顶层 L3 Long(+694k) 主导。**穿仓+杠杆同根 = 多空双开（每级别独立开空腿，无父级门控）**：同一机制造杠杆（多空同在场）+ 造穿仓（强牛空腿无界失血）。

2. **定义依据**：
   - 第21课:40「上涨趋势确定后只可能出现第三类买点」⟹ 确认上涨趋势下最高级别无卖点 ⟹ 不该有持续空腿（编排者引用，本诊断输入：OKLO 顶层做空 100% = 字面违反）。
   - cc-coverage spec「次级别 type1 卖触发 sink 不属翻空」= 合法有界回调短差（判别：BTC L0 switches 多、short_pnl 正 ≈ 合法 vs L1 冻结 = bug）。
   - reading_b 架构（`rec_engine.rs:g/consume_legs`，547 隔离删父级门控）= 假空腿的结构来源；instances/route_bsp（`nearest_active_parent`「子级永不独立翻转」）= 对照有门控路径。

3. **边界条件（结论翻转处）**：
   - (a) 若 OKLO 顶层 L2 标 Down 是**真大级别下跌结构**（非塔矮误标），则 OKLO 顶层空腿不违反 21:40（合法骑下跌）——但 OKLO BH+307% 整体上行，顶层 100% 做空仍与「确认上涨趋势」矛盾。须 dump levels[2] node.direction 时序裁定（开放轴）。
   - (b) 若加升跌完备性门控后穿仓不减（冻结空腿因 556 完成稀疏仍关不掉），则根因升级为 556（完成信号稀疏）而非纯门控缺失。门控 + 556 解的相对贡献未测。
   - (c) ES「存活」的边界：顶层多头主导是 regime 函数（ES/BTC 塔到 L2-L3 + 顶层标 Up）；OKLO 塔矮顶层标 Down → 顶层即空。塔高 × 顶层标注方向决定存活，非门控单独。

4. **下游推论**：
   - **吃跌探索的核心难点从「事后摩擦」转移到「假空腿门控」**：reading_b 多重赋格 5/8 正（563）的存活标的（ES/QQQ/GC/BRN/DX）= 顶层正确 Long + sub 空腿被顶层碾压；穿仓标的（BTC/OKLO）= 空腿（顶层或冻结 sub）失血主导。**修升跌完备性门控可能把 BTC/OKLO 从穿仓救回**（与 ANCHOR 救踏空正交——ANCHOR 硬编码 long，门控让 long 涌现）。
   - **reading_b 的 547 隔离（每级别独立无父级门控）是双刃**：解了 cascade 越级翻空（547），但开了无界假空腿（本诊断）。父级门控需在「隔离」与「升跌完备性」间重新平衡。
   - **杠杆是假空腿的孪生症状非独立 alpha**：ES 杠杆 gross>net = 多空双开，去杠杆≈BH（friction-leverage-verdict）。杠杆不该被当作 alpha，是假空腿机制的副产物。

5. **谱系引用**：
   - **563号**（吃跌 L3，多重赋格 5/8 正 + 裂隙2 杠杆 1.3-2.4×）：本诊断**精化裂隙2**——杠杆 = 多空双开（顶层 Long + sub Short 同在场），穿仓标的（BTC/OKLO）= 空腿失血主导。563 的「5/8 正解 556 产生收益非暴露 539」对存活标的成立，但**穿仓标的（3/8）的 539 暴露 = 假空腿（冻结空腿在强牛失血）**，与 539 同根深化。
   - **556号**（最高级别完成稀疏冻结）：冻结空腿关不掉的根 = d_top 稀疏（556）。假空腿 = 556 冻结 + 无父级门控的合成症状。
   - **539号**（做空腿 regime 失血不可约）：本诊断给 539 一个结构机制——失血来自**冻结空腿**（非有界回调），可由升跌完备性门控解（539 不是「做空本身亏」，是「无界冻结空腿在牛市亏」）。
   - **547号**（cascade 级别错配）：reading_b 删父级门控（547 隔离）正是假空腿的结构来源——547 与升跌完备性门控张力（隔离 vs 父级约束）待裁。
   - **第21课:40 / cc-coverage**（升跌完备性）：本诊断 L3 证实其操作含义——确认上涨趋势下持续空腿 = bug。
   - **231/formalization-validity-domain**：穿仓根因有效域 = {强牛 ∩ trend 标记冻结 Down 的级别存在}；ES「存活」= 顶层标 Up 的有效域，非门控普适。

6. **影响声明**：
   - **改动文件（隔离 worktree `/tmp/friction-current`，未合主树）**：`rec_engine.rs`（+`leg_bars_long/short`/`top_leg_short_bars`/`max_active_lvl_seen` 埋点，reading_b on_bar 块累计，纯观测）、`rec_stream.rs`（新诊断 harness `prop4_fake_short_diag`）。bit-exact 不变（纯观测埋点）。
   - 原始数据：`.chanlun/review-results/fake-short-diag-raw-20260623.txt`。
   - **修法（升跌完备性父级门控）= 方向建议，未实装**（route_bsp/consume_legs 改动需 escalate 选择：父级门控严格度 + 与 547 隔离的张力）。
   - 不改任何已结算定义；为 563 裂隙2 + 539 提供结构机制；**不触发概念矛盾中断**（假空腿是实装缺门控，非定义冲突——修法明确：补 instances 路径已有的父级门控）。
   - **直接回答编排者**：(1) 为什么还在「骑牛」= 没骑牛，是 reading_b 在强牛误开冻结空腿（顶层 OKLO/次级别 BTC）；(2) 能不能多空双开 = 能，但必须有升跌完备性门控（确认上涨趋势下空腿只能有界回调，不能冻结骑整段牛市）。

---

## 6. 认识论等级标注

| 命题 | 等级 |
|---|---|
| 穿仓主因 = 冻结空腿（switches≈0-1，bars_short=10⁵⁺，short_pnl 大负）| **L3**（BTC/OKLO×2变体）|
| OKLO 穿仓 = 最高级别(L2)假空腿（做空 100% 时间）| **L3**（顶层 Short 占比 100%）|
| BTC 穿仓 = 次级别(L1)冻结空腿（顶层 L2 正确 Long）| **L3**（liq_log 仅 L2 Long + L1 short_pnl −98k~−132k）|
| ES 存活 = 顶层 L3 Long(+694k) 主导 | **L3**（liq=0 + 顶层 bars_short=0）|
| 穿仓+杠杆同根 = 多腿独立开空（无父级门控）| **L3**（杠杆=多空同在场 + 穿仓=空腿失血，同一机制）|
| 21:40 违反 vs 合法回调判别 = 是否冻结 | **L3**（switches/bars_short/pnl 三量判别）|
| 修法 = 升跌完备性父级门控（非 ANCHOR 非摩擦）| **L1**（架构对照：instances 有门控 vs reading_b 无）|
| 为何 OKLO 顶层标 Down（塔矮 vs 真下跌结构）| **L2 假说，未 dump trend-label 内核（开放轴）**|
| 门控 vs 556 解的相对贡献 | **未测（开放轴）**|
| 异质审计（Codex/Gemini）| **未执行**（数值实证非概念质询，缺口标注）|
