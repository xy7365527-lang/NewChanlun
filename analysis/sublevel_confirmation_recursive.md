# 次级别确认完整递归 — 盘点、实现与 OKLO 判决

> 任务（2026-06-11 编排者）：27课区间套"每个买卖点都要用次级别来把握"——盘点当前
> 次级别确认覆盖、为全部缺口操作点设计并实现确认链路（config 逐点消融）、OKLO 回测裁决。
> 认识论等级：**L2**（OKLO 447,739 bars 单标的真实数据；基线对账在册逐计数相等）。
> 数据：`analysis/data_cache/_sublevel_confirm_OKLO.json`（纯 Rust 链路落盘）。

---

## 0. 判决（先行）

**"次级别确认完整递归"假说在 confirmed 事件层被否证。** 六个新增确认点：
4 个负贡献（SCm/SCo/SCr/SC7），2 个空约束（SCe/SCc——确认条件在触发 bar 恒已满足）。
无一转正。

| 消融位 | 操作点 | 复利% (基线+1499.2) | 确认拒/持次数 | 裁决 |
|---|---|---|---|---|
| SCe | 入场超时 fallback 取消 | +1499.2（=基线） | 0 | **空约束**——OKLO 全部 ARM 在 60 bar 内获次级别确认，超时通道从未触发 |
| SCm | master type1 卖出场 | **+310.1（−1189pp）** | 177 holds | **强否证**——出场延迟在强趋势顶毁灭性；下游 osc 净 +15.4K→−20.1K |
| SCo | main 降成本腿卖开 | +1028.0（−471pp） | 161 rejects | **否证**——被拒的卖开是正确操作（main 净 +70.7K→+18.6K） |
| SCc | main 同锚 Buy1 闭腿 | +1499.2（=基线） | 0 | **空约束**——confirmed Buy1 出现的 bar 上次级别买证据共现率 100% |
| SCr | REV 开腿（全触发源） | +1345.5（−154pp） | 422 rejects | **否证**——rev 开 185→89，砍掉的腿净赚（rev 净 +51.4K→+24.7K） |
| SC7 | confirmed Buy3 回补 | +1248.9（−250pp） | 54 holds | **否证（预注册判据触发）**——与 R3 同构：t7 29→4，回补延迟，rev 净 −13.8K |
| SCall | 全开 | +264.8（−1234pp） | — | 负贡献叠加 + trade 边界漂移（195→204 笔，maxDD 46.9→63.3%） |

## 1. 盘点：次级别确认的现有覆盖（任务第1步）

| # | 操作点 | 位置 | 现有确认 | 状态 |
|---|---|---|---|---|
| 1 | ARMED→入场（type1 买开仓） | runner | `buy_any & sub_mask`（任意次级别买点）+ SUB_EXPIRY=60 超时 fallback | **有**（fallback 是缺口 → SCe） |
| 2 | master 出场（type1 卖清仓/升级） | runner | 无——sell1 行直接出 | **缺口 → SCm** |
| 3 | main 降成本腿开（type1/2 卖） | runner | 无——confirmed Sell 事件 + center_gate | **缺口 → SCo** |
| 4 | main 腿闭（同锚 confirmed Buy1） | runner | 无 | **缺口 → SCc** |
| 5 | main/REV hard（confirmed Buy3） | runner/LOU | 无 | **缺口 → SC7** |
| 6 | REV 开腿 | LOU | G1 `sub_anchor`（方向锚定，预注册否证）；R1 盘背窗口（否证） | **缺口（统一形式）→ SCr** |
| 7 | REV 闭腿 T5 | LOU | `nesting_buy_level` 区间套逐级读出 + `RevClose::Nested` 轴 | 有 |
| 8 | REV 闭腿 T6（candidate Buy3） | LOU | R3（否证：抑制 t6 漏到 t7 更差） | 有先例，**设计排除**（不重测） |
| 9 | osc 域腿开（ZG 高抛） | LOU | `sub_sell`（次级别 sell_any）硬性合取，P5 逐字 | 有（结构内置） |
| 10 | osc 域腿闭（ZD 低吸） | LOU | `osc_buy_sub` 开关（P6 BRN −204.7pp 否证，默认关） | 有先例 |
| 11 | type3 卖逃逸开腿 | LOU | 无独立确认 | V2r 基线 `rev_escape_open=false` ⇒ 空有效域；SCr 实现上覆盖（rev_paired 全触发源），消融不可测 |

## 2. 设计：统一确认谓词（任务第2步）

`BarRows::sub_confirm(k, side)` = **同 bar 事件证据 ∨ D3 方向行结构证据**：

- **事件证据**：次级别（k−1）本 bar `buy_any`/`sell_any` 掩码 ∪ div 事件——
  T5 `nesting_buy_level` 的已验证共现形式（27课"大级别买点必然伴随小级别买点共现"），
  **非 R1 的窗口记忆形式**（R1 判决：窗口反选坏腿）。
- **结构证据**：`dir_row[k−1]` 已翻向操作方向（Sell→Down / Buy→Up）。方向行是状态
  非事件；**bi 层（ladder 1）无 BSP/div 事件流但有 D3 行**——这消除 R1 的空定义域
  陷阱（floor=segment 的 k=2 操作点在纯事件证据下必然保守拒绝，盘背消融 §3.1）。

各买卖点的确认语义映射（任务清单 → 谓词实例化）：
type1 买（次级别下跌背驰确认）= sub_confirm(k, Buy)；type1 卖 = sub_confirm(k, Sell)；
type2 买/卖（次级别反弹/回调结束）= 同形式（方向行翻转即"结束"的可观测信号）；
type3 买（次级别不破回补位）= sub_confirm(k, Buy)（24课"回抽不破"的次级别形式）；
type3 卖（次级别突破逃逸位）= sub_confirm(k, Sell)；域腿 = 现有 sub_sell/osc_buy_sub（不重复）。

事件流可用性（任务第3步问题）：bi 层（1）仅 D3 方向行；segment（2）以上有 BSP+div
事件流 + 方向行。capability guard：任意 SC 位 ⇒ 磁带 `dir_flips` 行（fail-fast，
不提供纯事件降级——那是 R1 陷阱的重演路径）。

## 3. 实现（任务第3步）

| 文件 | 改动 |
|---|---|
| `rust/src/trading/config.rs` | SC 六位（默认全关）+ 变体 V2rSCe/SCm/SCo/SCc/SCr/SC7/SCall |
| `rust/src/trading/level_operating_unit.rs` | `BarRows::sub_confirm` 统一谓词；SCr 落点（rev_paired_open，全触发源）；SC7 落点（step_down_paired + legacy DownLeg 两路径） |
| `rust/src/trading/runner.rs` | capability guards（SC ⇒ dir_flips；sc_rev_open ⇒ rev_paired）；rows 构造前移至 master 段；SCe（ARMED 超时不强制入场）；SCm（master 三处出场点）；SCo（main 卖开）；SCc/SC7（main 闭腿 normal/hard） |
| `rust/src/trading/master.rs` | `from_sell1_row_sub_confirmed` 显式入口——532号类型隔离不破（输入仍是布尔行，确认是出场时机细化非新触发源） |
| `rust/src/trading/types.rs` | 6 个 SC 计数器（入 py_items） |
| `rust/src/trading/sublevel_confirmation_ablation.rs` | 消融 harness（`cargo test --release sublevel_confirm_oklo -- --ignored`）+ 4 个 SC 单测（谓词三态/master 入口/SCr 拒-开/SC7 持-闭） |

守卫：V2r 基线零侵入（harness 内 assert 在册 enginefix 逐计数 + SC 计数全零，PASS）；
trading 单测 52 passed / 0 failed。t6 不加确认位——R3 判决先例的设计排除，非遗漏。

## 4. 机制分析：为什么全部否证/空约束

**confirmed 事件的"confirmed"语义已经是次级别结构完成的编码。** 引擎的
candidate→confirmed 确认机制本身就是 27课"用次级别把握"的系统落点——confirmed
Buy1 的定义（次级别离开后回拉不创新低等）内含次级别走势的完成判定。在 confirmed
事件之后再加一道次级别确认 = 双重确认 = 纯延迟。

SCc 的零效应是此命题的实证：**同锚 confirmed Buy1 出现的 bar 上，次级别买证据
共现率 100%**（SCc 持有计数恒 0）——确认谓词对买侧 confirmed 事件是恒真冗余。
而谓词有差异的位置，差异方向全为损失：

- **SCm（177 holds）**：卖侧不对称——顶部 bar 上次级别方向行翻 Down 滞后于价格顶
  （D3 确认滞后），sell1 行触发时谓词常假。延迟的每个 bar 都在强趋势顶下方兑现更差
  价位；osc 腿净由 +15.4K 翻 −20.1K 是 trade 边界漂移的下游级联（出场晚 ⇒ trade
  切分变化 ⇒ 域腿锚不同）。
- **SCo/SCr**：被拒的卖开/REV 开是正确操作——卖点 bar 上次级别证据缺失不预示腿
  质量差（与 R1 反选同构：确认条件与腿盈利性无相关或负相关）。
- **SC7（54 holds，t7 29→4）**：与 R3 完全同构——type3 通道的出场延迟恒变差，
  因为三买回补位之后是"定义内合法上涨"（24课"否则怎么会有第三类买点"），
  每延迟一 bar 买回价更高。预注册判据（REV 净恶化即否证）触发：rev 净 −13.8K。

**正确的次级别确认拓扑**（本判决后的系统陈述）：次级别确认属于**等待型**操作点
（入场 ARM 等待、T5 区间套读出、osc 开腿的 sub_sell 合取——这三处确认是"等到
次级别证据再动"，不阻断已确认的结构信号）；不属于**信号型**操作点（confirmed
BSP 事件本身——结构已完成，再等是延迟）。R1/R3/P6/本判决四案收敛于同一边界。

## 5. 与 R1 教训的对照（任务关键参考的回应）

R1 失败的两个根因，本设计均已修正，仍然否证——说明失败不在形式而在概念：

| | R1（盘背消融） | SC（本任务） | 结果 |
|---|---|---|---|
| 证据形式 | 窗口记忆（sub_sell_bar ≥ run 锚） | 同 bar 共现 | 仍否证 |
| bi 层空域 | 纯事件证据 ⇒ k=2 恒拒（1145 拒/41→3 腿） | + D3 方向行 ⇒ 有效域非空（SCr 拒 422/185→89，非全拒） | 仍否证 |

即：修复了 R1 的"实现缺陷"（窗口+空域）后，确认仍是负贡献——否证的是
"confirmed 事件后加次级别确认"这个概念本身，不是某个实现形式。

---

## 结果包六要素

1. **结论**：六个 SC 消融位已实现（config 独立开关，默认全关）并在 OKLO 447K 完成
   8 变体消融。判决：4 否证（SCm −1189pp / SCo −471pp / SC7 −250pp / SCr −154pp）+
   2 空约束（SCe/SCc 触发零次/恒满足）。**次级别确认完整递归被否证——27课区间套
   在系统中的正确落点是已有的等待型确认（入场 ARM / T5 读出 / osc sub_sell）+
   引擎 candidate→confirmed 机制本身；对 confirmed 事件再加确认 = 双重确认 = 纯延迟。**
   生产配置维持 V2r（SC 位全关）。
2. **定义依据**：第27课区间套（"大级别买点必然伴随小级别买点共现"——SCc 共现率
   100% 是其实证）、第24课三买（"否则怎么会有第三类买点"——SC7 延迟恒亏的定义内
   性质）、第17课 type1/2 定义（confirmed 语义内含次级别完成判定）。代码依据 §3 表。
3. **边界条件**：(a) L2 单标的——若 QQQ/BRN 复验中某 SC 位转正且账户级传导，该位
   判决翻转（鉴于 4 位全负且量级大，方向翻转概率低）；(b) SCe 空约束条件于 OKLO
   的入场密度（60 bar 内必有次级别买点）——稀疏标的上 SCe 可能非空，但其效应方向
   （错过入场）在强趋势标的上先验为负；(c) D3 方向行的确认滞后是 SCm 损失的机制
   成分——若磁带升级为更低滞后的方向信号，SCm 量级会变但"出场延迟为负"的方向
   由强趋势数据性质决定；(d) 消融在 floor=segment 口径——floor 升层后 bi 层结构
   证据通道失去意义，但事件证据通道仍在。
4. **下游推论**：(a) V2r 生产配置零改动；(b) "等待型/信号型"操作点二分是新的设计
   语法候选——后续任何"加确认"提案先问落点类型（信号型 ⇒ 先验否证方向）；
   (c) R1/R3/P6/SC 四案收敛 ⇒ 卖侧/出场侧的次级别确认全谱系否证，该方向关闭；
   (d) SC 计数器与变体保留在代码中（可观测面 + QQQ/BRN 复验零成本）。
5. **谱系引用**：R1/R2/R3 盘背消融判决（`consolidation_div_ablation_results.md`——
   R1 反选与 R3 延迟先例）、区间套配对修复 P1-P7（买侧 Nested 闭腿有效 + P6 osc_buy_sub
   否证）、532号（master 出场类型隔离——SCm 经显式入口不破隔离）、527/090号（严格性）。
   "次级别确认"专属谱系条目未核对——与盘背调研报告同一声明。
6. **影响声明**：改动 6 个 Rust 文件（§3 表）+ 新增 harness 文件与本报告 +
   `_sublevel_confirm_OKLO.json`。SC 位默认全关 ⇒ 在册回测（V2r/V2of/O0 全系）零行为
   漂移（harness 基线守卫验证）。**并发声明**：本任务与 type2（sell2_open/buy2_close）、
   θ 自适应（theta_mode/DepthRef）两工位同 session 并行改动同组文件——本报告判决
   基于 ThetaMode::Fixed + type2 位全关路径（SC 变体不开启它们），互不污染；
   代码未 commit（共享文件含并行工位未完成改动，混表 commit 违反磁带指纹纪律）。
