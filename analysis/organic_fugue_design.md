# 多重赋格的有机结合 — 完整递归操盘框架设计

> 状态：设计稿（不含实现）。设计精确到可直接实现。
> 认识论等级：本设计的结构推导为 **L0**（原文+已结算谱系推导）；所有数值预期引用
> **L2/L3** 已落盘回测（`interval_nesting_reverse_backtest.md`、`fugue_version_i_results.md`
> 及 memory 笔记）；框架本身的有效性为 **待验证假设**，验证设计见 §8。
> 日期：2026-06-10。

---

## 0. 统一原则（有机性的核心命题）

**多重赋格 = 同一个操盘程式（第38课同级别分解程式）在每个中枢承载层上运行；
各声部之间的全部差别只有两个：账本对操作的解释（master=真实仓位，voice=共享
仓位上的短差）和该层投入的资金筹码（第40课）。**

原文锚定（第40课，blog/040 第19-22行）：

> "对于大资金来说，这种级别的操作可以一直延伸下去，可以变成N重层次的操作，
> 每一重都对应着一定的资金与筹码，而相应对应着不同的节奏与波动。……这如同赋格曲，
> **简单的动机、旋律在N个层次上根据不同的转位、移位、对位等原则运动着**，合成统一的乐曲。"

赋格曲的数学本质是**单一主题的多声部转位**，不是多个主题并发。因此正确架构只有
**一个** FSM 类型（LevelOperatingUnit，下称 LOU），实例化 N 份；不是"中枢震荡短差
FSM"+"进出场 FSM"+"做空 FSM"三套异质逻辑的拼接。这是对现状代码（master 进出场
三态机 + voice 短差配对逻辑分离）的结构性统一。

> "在这种同级别分解的多重赋格操作中，**可以在任何级别上进行操作，而且都遵守该级别
> 的分解节奏与波动，只是在不同级别中投入的筹码与资金不同而已**。"（第40课 第22-23行）

---

## 1. 经验基础（已裁决命题清单）

设计建立在以下已落盘的 L2/L3 裁决之上，不重新争论：

| # | 命题 | 裁决 | 来源 |
|---|------|------|------|
| E1 | 级别隔离架构（每 ladder 独立事件流+独立配对上下文） | **确认**（P1-P5 重构后逐位复现，是机制不变式） | interval_nesting_reverse_backtest §结果包(一) |
| E2 | 域腿（锚 ZG×次级卖点高抛 / 锚 ZD 触线低吸） | **确认**（胜率 72.7-77.6%，三标的净现金全正，全机制质量最高） | 同上，P5 腿分解表 |
| E3 | 买回侧次级别确认合取 | **否证**（P6：BRN −204.7pp；确认滞后+悬挂尾部） | 同上，P6 判决 |
| E4 | type3 买点 = 纤维死亡逃逸（回补信号非低吸） | **确认**（P3>P2，candidate 左侧 > confirmed 右侧） | 同上 |
| E5 | kind 盲配对（t1卖→t1买，无中枢锚） | **否证**（P1 三标的全负，劣于盲配对基线） | 同上 |
| E6 | bar/bi 级短差 | **否证**（I_bar0 爆仓；印证第53/35/31课"太小级别有害"） | project_shared_position_fugue / fugue_version_i docstring |
| E7 | slice 固定比例分仓 | **否证**（限额掩盖真实 alpha，已废） | fugue_version_i 重写说明#1 |
| E8 | 机械均分（1/N_sub） | **否证**（无原文依据的归一化，均分不防爆仓、不携带结构信息；用户裁定为临时方案） | fugue_version_i `_open` 注释 |
| E9 | 同锚 type1 买正常回补 | **空有效域**（n_close_normal=0×3标的：type1 买必然异锚） | interval_nesting 边界条件(b) |
| E10 | 信号层不折叠 kind/center 信息 | **确认**（bsp_events 结构化事件流已是磁带标准字段） | 同上，下游推论(b) |

---

## 2. 概念框架：完整循环 vs 中枢震荡（原文锚定）

### 2.1 现状的缺口

现状（P5/P7）的 voice 只做两类腿：离开段腿（P4 main：c>ZG 盘背高抛→type3/预逃逸回补）
和域腿（P5 osc：中枢内高抛低吸）。两者都是**中枢尺度**的操作。缺失的是第38课的
**段尺度完整循环**：

> "一旦向上段的运作结束后，就进入向下段的运作。**向下段的运作刚好相反，是先卖后买，
> 从刚才向上段结束的背驰点开始，所有操作刚好反过来就可以**。"（第38课 第36-37行）

即：高级别的一段反向走势，对该层声部而言不是"等待下一个买点"的空窗，而是一个
**完整的反向操盘机会**——从向上段终结的背驰点卖出，持币（期货：持空）穿越整个
向下段，在向下段完成点（38课三岔判定）买回。

### 2.2 第38课操作程式（LOU 的原文规格）

> "不妨从一个下跌背驰开始……必然首先出现向上的第一段走势类型，根据其内部结构可以
> 判断其背驰或盘整背驰结束点，先卖出，然后必然有向下的第二段，这里有两种情况：
> 1、不跌破第一段低点，重新买入，2、跌破第一段低点，如果与第一段前的向下段形成
> 盘整背驰，也重新买入，否则继续观望，直到出现新的下跌背驰。"（第38课 第34-36行）

映射到引擎事件语言：

| 38课语句 | 引擎事件 |
|----------|---------|
| "从一个下跌背驰开始"（买入） | 本级别 confirmed **type1 买** |
| "背驰或盘整背驰结束点，先卖出" | 本级别 confirmed **type1 卖** ∨ **div(up, kind="consolidation") 卖侧** |
| "不跌破第一段低点，重新买入" | 本级别 confirmed **type2 买** |
| "跌破……与第一段前的向下段形成盘整背驰，也重新买入" | 本级别 **div(down, kind="consolidation") 买侧** |
| "否则继续观望，直到出现新的下跌背驰" | 等待下一个 confirmed **type1 买** |
| （33课补充）三卖必须走 | 本级别 confirmed **type3 卖** 也触发"向上段结束" |

第39课对节奏的强调（韵律即语法）：

> "其中最大的就是**向上段先买后卖与向下段先卖后买的韵律**，如果这个韵律都错了，
> 那操作就一团糟。"（第39课 第19-20行）
> "高位没走，低位去回补等于加仓，这样不好，一定要搞清楚向下段与向上段。……
> **只要是先卖的，回补起来就不会害怕了。所以节奏是第一的**。"（第39课 第145-147行）

### 2.3 第41课硬约束（反向操盘的前提）

> "买卖点是有级别的，大级别能量没耗尽时，一个小级别的买卖点引发大级别走势的延续，
> 那是最正常不过的。但如果一个小级别的买卖点和大级别的走势方向相反，**而该大级别
> 走势没有任何衰竭，这时候参与小级别买卖点，就意味着要冒着大级别走势延续的风险，
> 这是典型的刀口舔血**。"（第41课 第22-24行）

设计推论：段尺度反向腿（REV）的**开腿**必须有上级别衰竭证据守门。注意约束的精确
范围——41课约束的是"与大级别方向相反的完整参与"，**不约束域腿**（域腿两腿都在
中枢 [ZD,ZG] 内，中枢震荡是第49课明示该做的操作，不构成反向走势参与），也不约束
已有中枢门的离开段腿。门只加在 REV 上。

### 2.4 两阶段守恒律（第31/43课）与 master 循环的反作用

> "当成本为0以前，要把成本变为0；当成本变成0以后，就要挣股票，**直到股票见到历史性
> 大顶，也就是至少出现月线以上的卖点**。"（第31课 第24-25行）
> "成本为0前，只补进相同的数量，仓位不增加。成本为0后，抛出后，跌回来，就把抛出的
> 钱，全补进去，这样买回来的数量一定多了。"（第31课 第169-170行）
> "成本为0后，可以用先卖后买的方法，例如20卖1万，19就可以回补1万多股了，这样股数
> 越来越多。"（第43课 第136-137行）

关键发现：earning 切换**不只改守恒律，还反作用于 master 循环**——成本为0后
"直到历史性大顶"意味着 master 的出场级别**升级**（原 entry 级别的 type1 卖不再清仓，
而是降格为一次 REV 腿先卖后买），清仓权上移到更高级别的卖点。这是第31课原文的
严格形式，现状代码完全没有这一层（earning 只改 close_diff 的算术）。

### 2.5 持股持币（第45课，master 循环的存在论）

> "在一个30分钟的买点买入后，就进入一个持股的操作中……一个30分钟的卖点必然在前面
> 等着……在这个卖点到来之前，你就只在持股这唯一的操作里。当这个30分钟的卖点出现时，
> 卖出，然后就进入持币的操作里，直到一个30分钟的买点出现。"（第45课 第15-17行）

master 的完整循环（持股↔持币）与 voice 的完整循环（RIDE↔REV）是**同一个 FSM**：
master 的 REV 态 = 持币（仓位为零），voice 的 REV 态 = 该声部的卖腿开放。
这就是 §0 统一原则的落点。

---

## 3. 架构总览

```
信号层（每 bar 一条 BarSignalI 磁带，compute-once）
│   现有字段：buy1/sell1/sell_any/buy_any[ladder], max_ladder, bsp_events[ladder]
│   新增字段：div_events[ladder]（背驰事件流：kind/direction/side/seg锚，§5.1）
│
├─→ CenterBook[ladder]（中枢生命周期账本——现有 last_center/dead/frozen，不变）
│
├─→ FatigueMonitor（41课守门员，§5.3）
│      输入：上级别 div_events + bsp_events（卖侧）
│      输出：fatigue[k] ∈ {None, 证据集}（k 层反向操作的上级别衰竭状态）
│
├─→ SizeAllocator（40课规模，§5.4）
│      输入：各层存活中枢 (ZD,ZG) + 当前价
│      输出：frac[k]（中枢振幅归一的声部资金占比；中枢生死事件时重算）
│
├─→ LOU[k]  k ∈ [FIRST_BSP_LADDER, entry_ladder]（38课程式 FSM，§5.2）
│      master（k = entry_ladder）：RIDE=持股满仓 / REV=持币空仓
│      voice（k < entry_ladder）：RIDE=无开放腿 / REV=段尺度卖腿开放
│      RIDE 内子循环：域腿（P5 osc，原样保留）
│      产出：操作事件 {OPEN_REV, CLOSE_REV, OPEN_OSC, CLOSE_OSC, MASTER_OPEN, MASTER_CLOSE}
│
└─→ OrganicLedger（共享账本，_SharedFugue 的扩展重定义，§5.5）
       两阶段守恒律 per-leg（was_earning，原样保留）
       槽空间：osc=ladder+100（保留）/ rev=ladder+200（新增）
       市场语境：stock（先卖后买，Σ卖出≤持仓）/ futures（REV 可真实做空，扩展 F1）
       earning 反作用：earning=True → master 出场级别升级（§5.6，变体 O4）
```

数据流方向是单向的：磁带 → 账本/监视器 → LOU → 账本操作。LOU 之间**零通信**
（级别隔离不变式 E1）；唯一的跨级别读取是 FatigueMonitor 读上级别事件（这不破坏
隔离——隔离禁止的是配对上下文跨级别混流，41课门本来就是跨级别约束，是定义的一部分）。

---

## 4. 设计问题逐一回答

### a) 级别递归操盘单元

每个中枢承载层一个 LOU，状态机见 §5.2。完整的买-持-卖-持币循环 = RIDE↔REV 主循环；
中枢震荡短差降格为 RIDE 态内的子循环（域腿）。38课程式的封装方式是把"段终结判定"
（卖出）和"段完成三岔判定"（买回）都用本级别事件表达（§2.2 映射表），不依赖
次级别确认（E3 教训）。

### b) 反向走势做空

高级别向下段 = voice 的 REV 腿：在段终结卖点开（先卖），段完成买点关（后买）。
- **股票语境**：REV 腿就是一个段尺度的 ShortDiffCycle——先卖后买的合法性来自第26课
  （"先卖后买也是可以挣钱的"），卖出量受持仓约束（不变量 INV-1，§5.5）。
- **期货语境**：除先卖后买外可叠加真实空头（卖出超过持仓的部分成为净空头，独立
  PnL 记账）。这改变守恒律语义（空头不参与 cost_basis 递推，profit 直接入现金），
  作为扩展实验 F1 单独验证，不进主线 O 系变体。

### c) 两阶段切换的代码形式

**守恒律属于账本，不属于 LOU。** LOU 只产语义操作（OPEN_REV/CLOSE_REV…），
OrganicLedger 按腿 open 时刻的阶段（was_earning 标记，现有机制原样保留）决定该腿
close 时执行股数守恒还是金额守恒。一个 voice LOU 的 REV 段恰好是一个 ShortDiffCycle；
域腿子循环也是 ShortDiffCycle——`ShortDiffCycle` 与 `profit` 公式零改动。

衔接的新增部分只有一个：**earning 反作用**（§2.4/§5.6）——cost_basis≤0 时
master 的 RIDE→REV 转换从"清仓"降格为"开一次 entry 级 REV 腿（金额守恒，挣股数）"，
清仓权上移。这使 EARNING_SHARES 不再只是 close_diff 里的一个算术分支，而是改变
整个赋格的声部结构（master 自己变成一个 voice，最高声部空缺给更高级别卖点）。

### d) 操作规模的递归确定

frac[k] = A_k / Σ_j A_j，其中 A_k = (ZG_k − ZD_k) / c（k 层当前存活中枢的相对振幅），
求和遍历当前有存活中枢的声部层。无存活中枢的层 frac=0——**没有结构域就没有操作权**
（与 E2 域腿要求 center_alive 同构）。重算时机 = 任一层中枢生死事件（不逐 bar，避免
churn）；开腿时读取当时的 frac 快照，腿存续期内不变。

依据：中枢振幅是该层震荡的结构空间 = 该层短差的单次期望捕获量；第40课"每一重都
对应着一定的资金与筹码"+ chan99/0033"量基本只和级别有关"——量由级别（即中枢
振幅尺度）决定，高级别振幅大自然分得多，与第27课大资金配大级别一致。
认识论标注：该公式是 **L0 推导**（"一定"的结构化解释中最简的振幅正比形式），
非原文逐字（原文无公式），其优于均分是**待验证假设**（变体 O3 vs O2，§8）。

### e) 41课约束的代码化

FatigueMonitor 维护 fatigue[k] = 上级别 u(k) 的衰竭证据集。u(k) = k 之上最近的有
结构（曾出现 move）的承载层，封顶 entry_ladder；u(k) 不存在 → 门恒关。
衰竭证据（任一即门开）：

1. u 层当前向上 move 上的卖侧背驰事件（div_events，kind ∈ {"trend","consolidation"}
   ——盘整背驰即可算"有衰竭迹象"，引擎 `divergences_from_moves_v1` 的
   kind="consolidation" 直接可用）；
2. u 层 type1 卖（candidate 即可——E4/P3 证明左侧信号 > 右侧确认）；
3. u 层 confirmed type3 卖（中枢向下离开 = 上级别自身转入向下段）。

时效：证据自事件 bar 起生效，至 u 层**新的向上 move settle**（创新动力 = 衰竭被
市场否定）时清空。门只约束 OPEN_REV，不约束域腿与离开段腿（§2.3 范围论证）。

### f) 和 P5 的关系

**P5 是组件，不是特例。** 精确地说：P5 域腿 = LOU 在 RIDE 态、本级别中枢存活时的
子循环，原样嵌入（参数、触发、逃逸逻辑零改动——E2 是全机制质量最高的腿，动它没有
依据）。完整框架在 P5 之上加了四件 P5 没有的东西：段尺度 REV 腿（38课）、41课门、
结构化规模（40课）、earning 反作用（31课）。P4 离开段腿作为可选第三类腿保留
（P7 形态），由消融裁决去留。

---

## 5. 模块精确定义

### 5.1 信号层扩展：div_events[ladder]

**现状**：`BarSignalI.bsp_events[ladder]` 已携带 BSP 事件
（kind/side/seg_idx/confirmed/center_seg_start/center_zd/center_zg/price）。
背驰事件在 per_level_bsp / Rust `divergences_from_moves_v1` 内部已经算出，
但被折叠掉（只剩 type1 BSP 间接携带趋势背驰）。盘整背驰（kind="consolidation"）
完全不可见——这是 E10（不折叠信息）尚未覆盖的最后一块。

**新增字段**：`BarSignalI.div_events: tuple = ()`，
`div_events[ladder]` = 该层本 bar 新背驰事件流，每个事件 =
`(kind, direction, side, seg_idx, force_a, force_c, price)`：

- `kind` ∈ {"trend", "consolidation"}（引擎原生字段，透传）
- `direction` ∈ {"up", "down"}（背驰所在 move 方向）
- `side`：direction="up" → "sell"（向上段力度衰竭=卖出语义）；"down" → "buy"
- `seg_idx`：背驰段锚（去重键成分）
- `force_a/force_c`：进入段/离开段力度（透传，供诊断与 fatigue 强度扩展）

去重：per-ladder seen-set，键 = (kind, direction, seg_idx)。
默认 `()`：不消费该字段的全部现有路径（含 run_version_i 的 P1-P7）**逐位不变**。
产出方：`compute_i_signals_rust_events`（interval_nesting_reverse_backtest.py 中的
Rust 驱动事件信号层）抽出为公共模块并扩展。
力度口径沿用引擎价格振幅 fallback（非 MACD，O(N²) 不可行——既有诚实声明继承）。

> **成本声明修正（实装时勘误，2026-06-10）**：原稿"divergences 已是中间产物，
> surfacing 而非新计算，每 bar 增量成本≈0"仅对 **ladder≥4**（递归层，
> `_level_bsps` 路径）成立。实装发现：ladder2 的 Rust 增量 BSP 引擎
> （IncrementalBiZhongshuBsp）在内部计算 divergences 但**不暴露**中间产物——
> 引擎零改动约束下的严格形式 = stroke 增长 bar 用 Rust 纯函数全链重算
> （current_strokes marshal O(S)/次，摊还 O(S²)）；ladder3 在 bsp_epoch 门控点
> 用 current_segments/zhongshus/moves 重算（廉价）。实测 OKLO 447K 信号层
> 17s→402s。详见 `organic_signals.py` docstring 成本声明。

### 5.2 LevelOperatingUnit（LOU）

每层一个实例。`__slots__ = ("ladder", "role", "state", "osc_leg", "frozen_center")`。

**状态**：`IDLE`（仅 master：持币未建仓）/ `RIDE`（持多）/ `REV`（向下段：
master=持币，voice=段尺度卖腿开放）。voice 在 master 建仓时刻全部初始化为 RIDE。

**输入**（每 bar，全部本级别 k，除门外）：`bsp_events[k]`、`div_events[k]`、
`sell_any[k−1]/buy_any[k−1]`（域腿定位）、`c`、CenterBook[k]、`fatigue[u(k)]`、`frac[k]`。

**转换表（voice）**：

| # | 当前态 | 触发（本级别事件） | 守卫 | 动作 | 次态 |
|---|--------|--------------------|------|------|------|
| T1 | RIDE | confirmed type1 卖 ∨ div(up,*) 卖侧 ∨ confirmed type3 卖 | fatigue[u(k)] 非空 ∧ k∉frozen | 先 CLOSE_OSC（若开），OPEN_REV(k, frac[k], c) | REV |
| T2 | RIDE | 同 T1 | 门关 | 无（域腿照常） | RIDE |
| T3 | RIDE, osc 未开 | c ≥ ZG_k ∧ sell_any[k−1] ∧ 中枢存活 ∧ k∉frozen | — | OPEN_OSC（锚=(cs, ZD)） | RIDE |
| T4 | RIDE, osc 开 | c ≤ 锚ZD ∨ 锚中枢死亡 | — | CLOSE_OSC | RIDE |
| T5 | REV | confirmed type1 买 ∨ confirmed type2 买 ∨ div(down,consolidation) 买侧 | — | CLOSE_REV | RIDE |
| T6 | REV | candidate type3 买 | — | CLOSE_REV（预逃逸） | RIDE |
| T7 | REV | confirmed type3 买 | — | CLOSE_REV + frozen[k]=当前中枢（至新存活中枢解冻） | RIDE |

T3/T4 = P5 域腿逐字（E2/E3：开腿次级别定位、买回纯触线）。
T5 = 38课三岔的直译（§2.2 映射表）。T6/T7 = E4（type3 逃逸，candidate 优先）。
同 bar 事件优先级：逃逸（T6/T7）> 正常关腿（T5）> 开腿（T1/T3）——与现行
pairing 分支的"先平后开"事件序一致。

**REV 关腿触发消融轴 R**（§8）：R_conf = T5 原样（confirmed）；
R_cand = type1 买放宽为 candidate；R_nested = candidate type1 买 ∧ buy_any[k−1]
（27课区间套）。⚠ R_nested 与 E3 张力：E3 否证的是**域腿触线之上**加合取
（回归触达已是定位），REV 腿无 ZD 锚可触（价格已离开中枢），区间套定位是 38课
"根据其内部结构判断"的正格用法——两者域不同，故 R_nested 合法入消融、由数据裁决。

**转换表（master，role 差异仅在动作解释）**：

| 当前态 | 触发 | 动作 |
|--------|------|------|
| IDLE→RIDE | 现行 ARM+区间套入场逻辑（buy1 最高层 ARM → 次级别 buy_any 精确入场，逐字保留） | MASTER_OPEN（满仓建仓，初始化全部 voice 为 RIDE） |
| RIDE→REV | 同 T1 触发，**门恒开**（master 是最高声部，u(k) 不存在；其卖点本身就是全局衰竭判定） | cost>0：MASTER_CLOSE（清仓，全 voice 腿强制回补）；earning：OPEN_REV(entry 级)（§5.6） |
| REV→RIDE | master 为 IDLE 语义：等待下一次 ARM（现行 _FLAT 逻辑） | — |

> **532号修正（2026-06-10，已结算）**：RIDE→REV 行的"同 T1 触发"单轴声明被
> OKLO+QQQ 双标的数据否证——master 出场从 sell1（confirmed type1 卖）扩展到
> 38课三触发携带独立大额负贡献（Δ(O1v)−Δ(O1)：OKLO −562.5pp / QQQ −39.0pp）。
> master 出场判定与 voice 反向腿触发是两个概念：master 保持 sell1，voice 保留
> 三触发（其正增量未确立，regime 依赖）。拆解位：OrganicConfig.master_seg_end。
> 见 `.chanlun/genealogy/settled/532-seg-end-trigger-axis-split.md`。

止损（stop_mode A/B）语义不变，作用于账本层，与 LOU 正交。

### 5.3 FatigueMonitor

`fatigue: dict[int, set]`，键 = ladder。每 bar：

1. 对每层 u，扫 `div_events[u]` 卖侧 + `bsp_events[u]` 中 type1 卖（candidate 含）
   与 confirmed type3 卖 → 加入 fatigue[u]（证据 = (类型, seg_idx)）。
2. u 层新向上 move settle（需信号层透传 up_move_settled[u]——走势级已有
   `up_move_settled`，递归层从 move 状态 diff 得出，新增轻量布尔行）→ fatigue[u].clear()。
3. 查询接口 `gate_open(k) = bool(fatigue[u(k)])`，u(k) = k 之上最近曾涌现的承载层
   （用 max_ladder 与各层 move 出现史判定），无 → False。

### 5.4 SizeAllocator

`frac: dict[int, float]`。重算触发 = 任一声部层中枢生死事件（CenterBook 变更）：
`A_k = (ZG_k − ZD_k)/c`（存活中枢），无存活中枢 → A_k = 0；
`frac[k] = A_k / Σ_j A_j`（Σ=0 时全 0——无任何结构域则无任何声部操作）。
腿 open 时读取快照存入腿记录，存续期内固定。master 不参与分配（满仓建仓不变）。
对照模式 `sizing="equal"` 保留现行 1/N_sub（变体轴）。

并发暴露上界：三类腿独立槽（main/osc/rev）下单层最大并发 = 3×frac[k]，
全声部最坏 Σ ≤ 3。由不变量 INV-1 钳制（见下）。

### 5.5 OrganicLedger

**新模块定义**（不修改 `_SharedFugue`——它是 P1-P7 回归基线的组成部分；
no-patch：语义扩展用新账本完整重写，复用 `ShortDiffCycle` 与 profit 公式）。

字段 = `_SharedFugue` 字段 + `market_mode: str`（"stock"/"futures"）。
槽键空间：main 腿 = ladder（P4 保留时）；osc 腿 = ladder+100；**rev 腿 = ladder+200**。
归因报告按三类腿分解（腿分解表是 P5 判决的关键工具，必须保留）。

**守恒律**（零改动继承）：腿 open 时记 was_earning；close 时
was_earning=False → 股数守恒（profit 降共享 cost_basis）；True → 金额守恒
（卖 V 买 V，total_shares 净增，cost_basis 锁 0）。

**不变量**：
- **INV-1（stock 模式）**：任意时刻 Σ(开放腿 shares) ≤ total_shares——先卖后买
  只能卖持有的（第26课语境）。open_* 在违反时拒绝开腿（计数器记录拒绝次数，
  报告可见，非静默）。
- **INV-2**：earning 阶段腿的 total_shares 增减与 cost_basis=0 锁定
  （现行 close_diff 逻辑）。
- **INV-3（futures 模式，扩展 F1）**：REV 腿允许 Σ 超持仓，超出部分为净空头，
  PnL = shares×(sell−buy) 直接入 cumulative_recovered，不进 cost_basis 递推
  （空头不是持仓成本的一部分）；INV-1 对 osc/main 腿仍生效。

### 5.6 earning 反作用（master 出场升级，变体 O4）

`earning=True` 后：
- master 的 RIDE→REV 不再 MASTER_CLOSE，改为 OPEN_REV(entry_ladder, frac=全仓
  可卖余量, c)——entry 级卖点变成最大的一次先卖后买（43课程式的字面执行）；
  CLOSE_REV 按 T5-T7。
- 新清仓条件 = `exit_ladder_earning` 层的 confirmed type1 卖，
  `exit_ladder_earning = min(entry_ladder+1, 当前 max_ladder)`；若该层从未涌现
  或从未给出卖点 → 持有至数据尾（eod_close）。这是 31课"至少月线以上卖点"的
  级别相对化（绝对周期口径在 1min 递归涌现体系中无对应物，相对化为"entry 上一级"）。
- 边界（诚实声明）：complete_fugue_v2 实测挣股数曾零触发（持仓太短 cost_basis
  未≤0）。O4 验证前置条件 = earning 触发率 > 0；触发率本身先行报告（§8）。

---

## 6. 与现有代码的 diff 点

### 保持不变（零改动）

| 对象 | 理由 |
|------|------|
| Rust 引擎（src/rust 全部） | 全部所需事件（BSP kind/center、divergence kind、move settle）已可从状态/纯函数得出 |
| `src/newchan/trading/cost_reduction_fsm.py` | 通用单槽 FSM 语义独立；`ShortDiffCycle` 被复用（import，不改） |
| `fugue_version_i.py` 的 `run_version_i`/`_SharedFugue`/`PairingConfig` | P1-P7 回归基线，逐位保护 |
| `m1_i_rust_engine.compute_i_signals_rust` | 布尔磁带路径（pairing=None 消费者）不动 |

### 修改

| 对象 | 改动 |
|------|------|
| `interval_nesting_reverse_backtest.py` 的 `compute_i_signals_rust_events` | 抽出为公共模块 `analysis/organic_signals.py`；增 `div_events[ladder]`（§5.1）+ `up_move_settled[ladder]` 布尔行；原脚本改 import（其 P1-P7 结果须逐位复现作回归守卫） |
| `fugue_version_i.BarSignalI` | 增 `div_events: tuple = ()` 与 `up_move_settled: tuple = ()` 默认空字段（旧构造方/消费方 bit-exact） |

### 新增

| 文件 | 内容 |
|------|------|
| `analysis/organic_fugue.py` | LOU / FatigueMonitor / SizeAllocator / OrganicLedger / `run_organic(tape, config)` 主循环 |
| `analysis/organic_fugue_backtest.py` | §8 变体矩阵跑批 + 腿分解报告 + 结果包 |
| `analysis/test_organic_fugue.py` | LOU 转换表逐条单测（合成事件序）+ INV-1/2/3 不变量测试 + O0≡P5 逐位等价守卫 |

**O0≡P5 等价守卫**（最重要的 diff 验证）：`run_organic` 在配置
{REV 关闭, 门关闭, sizing=equal, main 腿关闭} 下必须与 `run_version_i(pairing=P6′)`
——确切说与 P5 的域腿子集——逐笔对账。若 LOU 重写连 P5 都复现不了，框架无效。
（实现注记：P5 含 main 腿，故等价目标取 P5 配置图中 osc-only 投影 + main 腿开启
两档分别对账。）

---

## 7. 设计决策的原文锚定总表

| 设计决策 | 课文 | 锚句 |
|----------|------|------|
| 单一 FSM 多声部转位 | 第40课 | "简单的动机、旋律在N个层次上……合成统一的乐曲" |
| 任何级别可操作、仅资金不同 | 第40课 | "可以在任何级别上进行操作……只是投入的筹码与资金不同" |
| REV 腿（向下段先卖后买） | 第38课 | "向下段的运作刚好相反，是先卖后买……所有操作刚好反过来" |
| REV 关腿三岔 | 第38课 | "1、不跌破第一段低点，重新买入，2、……盘整背驰，也重新买入，否则继续观望" |
| 韵律 = 段方向与买卖次序锁定 | 第39课 | "向上段先买后卖与向下段先卖后买的韵律" |
| 41课门（REV 开腿守卫） | 第41课 | "大级别走势没有任何衰竭……就是典型的刀口舔血" |
| 先卖后买合法性（stock 模式） | 第26课 | "先卖后买也是可以挣钱的" |
| 两阶段守恒律 | 第31课 | "成本为0前，只补进相同的数量……成本为0后……全补进去" |
| earning 后出场升级 | 第31课 | "直到股票见到历史性大顶，也就是至少出现月线以上的卖点" |
| 挣股数程式 | 第43课 | "20卖1万，19就可以回补1万多股" |
| master 持股/持币二元 | 第45课 | "在这个卖点到来之前，你就只在持股这唯一的操作里" |
| type3 卖强制离开（T1 触发之一） | 第33课 | "次级别回拉不能重新回到中枢里……第三类卖点出现，必须走" |
| 域腿（保留 P5） | 第49课 | "中枢震荡中，本质上是应该全仓操作的……中枢上方全部抛出筹码，在下方如数接回" |
| 入场区间套（master IDLE→RIDE 保留） | 第27课 | "不同级别背驰段的逐级收缩范围而确定" |
| 规模与级别挂钩 | 第40课+chan99/0033 | "每一重都对应着一定的资金与筹码"；"量基本只和级别有关" |
| floor=segment（排除 bar/bi） | 第53/35课 | "最小也不应该小于5分钟"；"太小的级别……没有意义" + E6 实测爆仓 |

---

## 8. 回测验证设计

### 8.1 变体矩阵（消融轴正交分解）

| 变体 | REV 腿 | 41课门 | sizing | earning 反作用 | main 腿 | 验证目标 |
|------|--------|--------|--------|---------------|---------|----------|
| O0 | ✗ | — | equal | ✗ | P5 同 | ≡P5 逐位守卫（基线锚定） |
| O1 | ✓(R_conf) | **✗** | equal | ✗ | P5 同 | 段尺度反向本身是否有 alpha |
| O2 | ✓(R_conf) | **✓** | equal | ✗ | P5 同 | **41课门的因果增量**（O2−O1） |
| O3 | ✓(R_conf) | ✓ | **structure** | ✗ | P5 同 | 结构规模 vs 均分（O3−O2） |
| O4 | ✓(R_conf) | ✓ | structure | **✓** | P5 同 | earning 反作用（前置：触发率>0） |
| O2c/O2n | ✓(R_cand / R_nested) | ✓ | equal | ✗ | P5 同 | REV 关腿触发消融（vs O2） |
| F1 | ✓ 真实空头 | ✓ | structure | ✗ | P5 同 | 期货语境扩展（仅期货标的） |

标的：OKLO（447K，主验证）+ QQQ（728K，低波动域）+ BRN（2.4M，期货/高频出场域）
——与 P5 实验同三标的，vs_B0/vs_P5 可直接对表。F1 加 ES（既有数据）。
成交口径与 P5 一致（信号 bar 收盘价，无滑点），费率敏感性单列（见 8.3-c）。

### 8.2 可证伪判据（预注册，先于跑批写定）

1. **REV 假设**：定义 Δ(X) = vs_B0(X) − vs_B0(P5)。
   - O1 在 ≥2/3 标的 Δ>0 → 段尺度反向独立成立；
   - O1 全负 **且** O2 转正 → 41课门是 REV 的必要条件（最强结果：原文约束被数据确认）；
   - O1≈O2 均正 → 门在该数据无增量（41课约束在此有效域内不被激活，**不是**41课被
     否证——门闲置≠门错误，需检查门开率）；
   - O1、O2 全负 → 段尺度反向被否证，框架收缩为 P5+规模轴（O3 仍独立可验）。
2. **门开率前置量**：报告 REV 触发尝试数 / 门拒数 / 门开率 per ladder。门开率≈0 时
   O2−O1 无统计意义（结论降级为"无定义域"，引 E9 空有效域先例）。
3. **规模假设**：O3 在 ≥2/3 标的 Δ(O3)>Δ(O2) → 结构规模成立；
   附内部对账：per-ladder 净现金与 frac[k] 的秩相关（结构分配应把钱配到产钱的声部）。
4. **earning 假设**：先报 earning 触发率；触发率=0 → O4 无定义域（诚实落盘，
   引 complete_fugue 先例）；>0 → O4 vs O3 同 Δ 比较。
5. **REV 腿独立质量**（腿分解表）：rev 腿胜率/卖飞率/avg_diff/净现金独立列出；
   预期 avg_diff 显著大于 osc 腿（段振幅>震荡振幅）→ 对费率更鲁棒；若 rev 腿
   avg_diff ≤ osc 腿且净现金为负 → REV 在执行摩擦下劣于纯域腿，判据 1 的正结果
   也要降级。
6. **多重比较防护**：主判据只有 1/3/4 三条（各一对预注册对比）；O2c/O2n/F1 为
   探索性（结论标注 exploratory，不进主判决）。

### 8.3 已知边界条件（继承+新增）

- (a) θ=1% 不跨波动率域（P5 边界 (a) 继承，main 腿在 QQQ 仍可能被门杀）；
- (b) 收盘价成交假设：REV 腿关腿在 confirmed 事件 bar 收盘成交，confirmed 滞后
  已计入（这正是 R_cand/R_nested 消融的动机）；
- (c) 费率：osc 腿 QQQ/BRN 优势 ~4bps（P5 边界 (c)），REV 腿预期振幅更大但笔数更少，
  全变体报告附 2/5/10 bps 费率敏感表；
- (d) 力度口径 = 价格振幅 fallback 非 MACD（继承既有诚实声明）——div_events 的
  kind="consolidation" 判定继承同一口径；
- (e) 强趋势满仓标的上 E 系（无降成本）整体占优的可能仍在（P5 边界 (f)）——本框架
  裁决的是声部层内部结构，不裁决"要不要声部"；
- (f) fatigue 的"上级别"在高层稀疏时退化（u(k) 长期不存在 → REV 恒关），高层稀疏
  分布按 ladder 报告。

---

## 9. 结果包（六要素）

1. **结论**：完整递归操盘框架 = 单一 LOU（38课程式 FSM）× N 声部 + 41课衰竭门 +
   中枢振幅结构规模 + 两阶段守恒律账本（含 earning 对 master 循环的反作用）。
   P5 域腿原样嵌入为 RIDE 态子循环。设计精确到转换表/不变量/槽空间/验证矩阵，
   可直接实现。
2. **定义依据**：§2/§7 原文锚定表（第26/27/31/33/38/39/40/41/43/45/49/53课，全部
   一级权威 blog 逐字引用）；引擎事件语义（type1/2/3、divergence kind、move settle）
   取自 `a_buysellpoint_v1`/`divergences_from_moves_v1` 现有定义，零新定义。
3. **边界条件**：§8.3 全列；最关键三条——REV 假设可被 O1/O2 全负否证（框架收缩为
   P5+规模）；earning 反作用可因触发率=0 而空有效域；fatigue 门可因高层稀疏恒关
   而无定义域。设计中任何"L0 推导"标注项（规模公式、出场级别相对化）的经验有效性
   均待 §8 裁决。
4. **下游推论**：若 REV+门成立 → 41课首次获得代码化的因果验证，且"声部完整循环"
   取代"声部=短差配对器"成为生产路径架构；若规模假设成立 → E8 均分被结构公式
   正式替换；若 earning 反作用可触发且为正 → 31课三段式资金管理首次完整落地，
   master/voice 边界变为动态（阶段依赖）。事件磁带增 div_events 后，盘整背驰
   对所有下游策略可见（E10 完成最后一块）。
5. **谱系引用**：E1-E10（§1 表，interval_nesting_reverse_backtest 结果包 +
   project_shared_position_fugue + project_complete_fugue_v2 + 525号笔中枢 +
   521号纯拓扑无动量 + 267/268a号操作方法论 FSM）；概念分离史：slice→共享仓位
   （E7）、盲配对→结构域配对（E5→E2）、买回侧合取否证（E3）。不确定谱系：
   "fatigue/衰竭"作为显式概念此前未在谱系中单独立条——若实现中发生概念分离
   （如盘背衰竭 vs 趋势背驰衰竭的分辨），需新谱系记录。
6. **影响声明**：本产出为设计文档（新文件 `analysis/organic_fugue_design.md`），
   未改动任何代码/定义/磁带。声明的未来 diff 面见 §6（两处修改四处新增，
   全部带逐位回归守卫设计）。
