# 票 #1189：trading 域两文件职责考古与拆分建议（map #1186 架构深化第 4 轮）

- 日期：2026-08-22；性质：只读考古，零代码/零文件改动（本报告为唯一写入）。
- 对象：
  - `rust/src/trading/level_operating_unit.rs`（5,837 行）
  - `rust/src/trading/positional_fusion.rs`（4,423 行）
- 证据范围：`rust/src` 与 `rust/tests` 全量 grep；补充 `analysis/*.py` 回测守卫与仓内测试基础设施核对。
- 证据等级声明：消费面/测试面为静态事实；拆分建议中的「零行为可验收」按现有测试与回测守卫如实标注。trading 域无拆分先例，未对未见过的回归锁作任何推测。

## 0. 结论速览

| 文件 | 结论 | 刀数 | 每刀边界 |
|---|---|---|---|
| `level_operating_unit.rs` | **建议切 2 刀**（1 刀测试外置 + 1 刀 SubLou 生产拆分）；VoiceUnit 内部 phase 块本轮不切 | 2 | ① 测试模块整体外置到 `level_operating_unit_tests.rs`；② `SubLou` 类型 + `impl SubLou` 迁 `level_sub_lou.rs`，原路径 `pub use` 重导出 |
| `positional_fusion.rs` | **建议不切**（生产侧是单入口单循环体，拆文件 = 搬复杂度；deletion test 反用详见 §3.3） | 0 | — |

零行为验收标注：
- level 刀①：**零行为可验收 = 是（构造性）**；纯 `#[cfg(test)]` 移动，不触生产 token。无对拍锁/金标准涉及。
- level 刀②：**零行为可验收 = 是（可证）**；54 个 in-file 单测 + `sublevel_confirmation_ablation` 的 V2r 硬计数基线 + SubLou 相关 Python 回测零漂移守卫。无 cargo golden fixture 直接覆盖本域。
- positional：本轮无刀，无验收问题。若未来强行切，必须跑 `analysis/hold26_counterseg_fusion_backtest.py` / `fusion_t_eight_asset_backtest.py` 的 guard-a/guard-b 与各模式在册对照（0.05pp / 逐位），cargo 侧无该域金标准。

---

## 1. `level_operating_unit.rs`

### 1.1 文件体量与职责块边界

- 总行 5,837；生产 1–2,496（2,496 行）；测试 2,497–5,837（3,341 行，占 57.2%）。
- 全文件有 5 个 impl 块：生产侧 `RevLeg`、`SubLou`、`BarRows`、`VoiceUnit` 4 个 + 测试夹具 `Fixture` 1 个。

| 行段 | 类型/fn | 职责 |
|---|---|---|
| 1–42 | 模块头/use | GUARD-ROLE 现役声明；v2 转换表 T1–T7、C2–C6 修正落点 |
| 43–50 | `pub enum VoicePhase` | UpLeg / DownLeg 声部相位 |
| 51–59 | `pub struct RevTranche` | 单个 REV tranche 坐标（level / last_dir） |
| 60–188 | `pub struct RevLeg` + `impl RevLeg` | REV 腿载体：tranche 前缀截断、开腿字段、T4b 标记、Seq38n 冻结低点 |
| 190–824 | `pub struct SubLou` + `impl SubLou` | **递归子 LOU**：4 模式（Zhongshu / Fractal / Sequence38 / CounterSeg）+ 级联强闭 |
| 825–835 | `pub enum RevOpenVerdict` | REV 开腿守卫枚举（Open / RejectedSubAnchor / RejectedGate / RejectedFrozen） |
| 836–918 | `pub struct BarRows` + `impl BarRows` | 每 bar 行视图；`sub_confirm` 次级别确认统一谓词 |
| 891–918 | `pub struct VoiceUnit` | 声部操盘单元：相位、REV 腿、区间套证据记忆、Cycle38/Seq38n 状态 |
| 919–2,496 | `impl VoiceUnit` | 主 FSM；内部按注释横幅已分 5 块（见下） |

`impl VoiceUnit` 内部职责块（方法起止按实际行号）：

| 行段 | 方法 | 职责 |
|---|---|---|
| 919–1,209 | `new` / `seg_end_trigger` / `buy_trigger_at` / `nesting_buy_level` / `step` | 公开入口与主转换表编排；legacy T1/T5/T5b/T6/T7、T4b、recursive sub 调用 |
| 1,210–1,509 | `osc_amp_admitted` / `step_osc` / `try_open_osc_at` | **osc 域腿（P5 + H1/H2/H3/H4/41 门）**，C2 正交路径 |
| 1,510–1,787 | `step_cycle38` / `c38_close_leg` | **38 课循环模式**（Cycle38，与 Single 互斥替换 REV 相位路径） |
| 1,788–2,327 | `rev_open_signal` / `rev_paired_open` / `effective_theta` / `step_down_paired` / `step_sub_tree` / `close_all_paired` | **配对修正机制**（rev_paired：震荡型/逃逸型、R1/R2/R3、Seq38n 主腿、子 LOU 驱动） |
| 2,328–2,496 | `rev_open_verdict` / `t4b_addons` / `t5b_struct_close` / `close_all_tranches` | 内部机制：G1/G2/G3 守卫、tranche 加码、防悬挂兜底、T6/T7 公共闭合 |

### 1.2 pub 面

| 符号 | 行 | 可见性 | 说明 |
|---|---|---|---|
| `VoicePhase` | 43 | `pub` enum | 相位 |
| `RevTranche` | 51 | `pub` struct | 字段 `level`/`last_dir` 均 pub |
| `RevLeg` | 60 | `pub` struct | 公开字段 11 个（`tranches/open_bar/open_kind/anchor_cs/zd/zg/ext_allowed/open_trigger/low_since_open/seg1_low/sub`），私有 T4b 标记 3 个 |
| `RevLeg::max_level` | 148 | `pub` fn | tranche 最高级别 |
| `RevLeg::close_upto` | 156 | `pub` fn | 前缀截断回补 |
| `SubLou` | 190 | `pub` struct | 公开字段 `ladder/path/child`，私有状态 4 个 |
| `RevOpenVerdict` | 825 | `pub` enum | 开腿拒因 |
| `BarRows` | 836 | `pub` struct | 9 个公开借用字段 |
| `BarRows::dir` | 858 | `pub(crate)` fn | D3 方向行读出 |
| `BarRows::sub_confirm` | 873 | `pub` fn | 次级别确认统一谓词 |
| `VoiceUnit` | 891 | `pub` struct | 公开字段 `ladder/phase/rev`，私有记忆字段 5 个 |
| `VoiceUnit::new` | 920 | `pub` fn | 构造 |
| `VoiceUnit::seg_end_trigger` | 939 | `pub` fn | legacy 段终结三触发谓词 |
| `VoiceUnit::buy_trigger_at` | 954 | `pub(crate)` fn | T5 单层买侧三岔 |
| `VoiceUnit::nesting_buy_level` | 980 | `pub` fn | 区间套确认级别纯函数 |
| `VoiceUnit::step` | 1,002 | `pub` fn | 每 bar FSM 主入口 |
| `VoiceUnit::rev_open_signal` | 1,788 | `pub` fn | 配对/legacy 开腿谓词统一入口 |

### 1.3 仓内消费点（grep `rust/src` + `rust/tests`）

生产消费者只有 `runner.rs`；测试消费者只有 `sublevel_confirmation_ablation.rs`。`rust/tests` **零直接引用**（无任何 `level_operating_unit` / 上述符号命中）。

| 消费点 | 位置 | 消费对象 |
|---|---|---|
| `runner.rs:32` | import | `{BarRows, VoicePhase, VoiceUnit}` |
| `runner.rs:172` | `VoiceUnit::new(k)` | voice 声部构造 |
| `runner.rs:841` | `BarRows { .. }` 组装 | 每 bar 行视图 |
| `runner.rs:868` | `VoiceUnit::buy_trigger_at` | master REV 腿 T5 |
| `runner.rs:879/927/1004/1184/1199/1245` | `rows.sub_confirm` | SCe/SCm/SCo/SCc/SC7 等次级别确认门 |
| `runner.rs:1267` | `run.voices[vi].phase == VoicePhase::UpLeg` | REV 开腿预判 |
| `runner.rs:1269` | `VoiceUnit::rev_open_signal` | 开腿信号谓词 |
| `runner.rs:1274` | `run.voices[vi].step(...)` | 每 bar 声部步进 |
| `sublevel_confirmation_ablation.rs:69` | import | `{BarRows, VoicePhase, VoiceUnit}` |
| `sublevel_confirmation_ablation.rs:103/222/232/259/295/305/331/357` | 测试直调 | `BarRows` 构造、`VoiceUnit::new/step`、`phase` 断言 |

零直接外部消费者的 pub 项：`RevTranche`、`RevLeg`、`RevLeg::{max_level,close_upto}`、`SubLou`、`RevOpenVerdict`、`BarRows::dir`、`VoiceUnit::{seg_end_trigger,nesting_buy_level}`（`RevLeg/SubLou` 仅有注释提及，无代码消费）。这些 pub 项是 `pub mod` 暴露面，但实际只由本文件与上述两个文件消费——拆分时用 `pub use` 重导出即可保持路径兼容。

### 1.4 测试面

- **in-file tests：54 个**（全部无 `#[ignore]`），测试模块 2,497–5,837 行；共享 `Fixture`/`empty_rows`/`ev*` 等夹具。
- 测试按覆盖对象分组：

| 组 | 数量 | 行段（近似） | 覆盖对象 |
|---|---|---|---|
| core legacy REV/T1/T4b/T5/T7 | 5 | 2,572–2,622；3,616–3,824 | `step` legacy 分支、`close_all_tranches`、`nesting_buy_level`、`close_upto`、`rev_open_verdict`、`t4b_addons` |
| osc 域腿 P5/H1–H4/41/domain/upshift | 12 | 2,623–2,789；5,243–5,837 | `step_osc`、`try_open_osc_at`、`osc_amp_admitted` |
| Cycle38 | 8 | 2,790–3,222 | `step_cycle38`、`c38_close_leg`、`RevCycleClose` 七臂 |
| SubLou 直接（Fractal/Sequence38/双门） | 8 | 3,223–3,287；3,322–3,421；3,487–3,615 | `SubLou::step/step_fractal/step_sequence38`、成本门/41课门 |
| SubLou 经 VoiceUnit 级联 | 4 | 5,073–5,242 | `step_sub_tree`、`cascade_close_with_self`、存在论下限 |
| rev_paired/R1/R2/R3/Seq38n/T2o/T2c | 17 | 3,835–4,528；4,541–4,995 | `rev_paired_open`、`effective_theta`、`step_down_paired`、`close_all_paired`、R 轴记忆 |

- **仓内同域外部测试**：
  - `sublevel_confirmation_ablation.rs`：5 个测试。其中 4 个快测直接调用 `BarRows::sub_confirm` / `VoiceUnit::step`；1 个 `#[ignore]` 长回测 `sublevel_confirm_oklo` 内嵌 **V2r 基线硬计数守卫**（`n_rev_open=185`、`n_rev_close_t6=116`、`n_rev_close_t7=29`、`n_rev_zd_close=38` 等）——这是本文件最接近「金标准」的仓内锁。
  - `runner.rs` 自带 9 个测试走 `run_organic`，覆盖 ledger-voice/守卫面；仅间接经过 `VoiceUnit` 装配与 `step`，不以 LOU 计数为主断言。
  - `rust/tests`：无直接/间接针对本文件的集成测试。
- **回测守卫（非 cargo，补充证据）**：`analysis/cycle38_close_ablation.py`（O0/V2oa25 带/不带 trend_flips 逐位相同）、`sequence38_backtest.py`（V2of/V2ofF1 逐位零漂移）、`p1_dual_gate_backtest.py`（V2ofF1 基线）、`seq38_main_backtest.py`（V2oa25_ht 基线对账）等均自带「旧路径零接触/在册数字复现」守卫。

---

## 2. `positional_fusion.rs`

### 2.1 文件体量与职责块边界

- 总行 4,423；生产 1–1,702（1,702 行）；测试 1,703–4,423（2,721 行，占 61.5%）。
- **全文件没有任何 `impl` 块**（生产侧与测试侧均无；结构与 level 文件完全不同：它是自由函数 + 单一巨型入口）。

| 行段 | 项 | 职责 |
|---|---|---|
| 1–87 | 模块头/use | GUARD-ROLE 现役声明；三机制、44 课铰链、会计不变量、资金解耦 doc |
| 88–133 | `pub(crate)` 常量 | `SUB_COST_K` / `SUB_FRICTION_RT` / `LAMBDA` / `SUB_SPAWN_FRAC`（成本门与配额常数） |
| 134–145 | `pub(crate) enum PhaseView` | Osc/MoveUp/MoveDown 相位三值视图 |
| 146–197 | 私有类型 | `LayerPhase` / `SubOut` / `NestWin` |
| 198–239 | 私有谓词 | `nest_sub_evidence` / `sub_open_trigger` / `sub_close_trigger` |
| 240–278 | `nav_fusion` | NAV 恒等式（层 + 短差 escrow + osc 在外腿） |
| 279–348 | `try_restore` | 短差回补（义务优先、推迟重试、escrow 释放） |
| 349–1,702 | `pub(crate) fn run_fusion` | **唯一生产入口**：15 参数模式矩阵守卫 + 单循环 FSM |

`run_fusion` 内部边界（按现有注释横幅）：

| 行段 | 块 |
|---|---|
| 349–626 | 参数守卫矩阵：floor/磁带/各轴合取合法性（约 278 行，全是预注册组合显式拒绝） |
| 627–673 | 状态初始化：layers/subs/pool/book/depth/osc/flips/trend/gate41/b3/phi/nest/seq38 |
| 674–745 | 每 bar 磁带推进：dir/trend 翻转、CenterBook/DepthRef 摄入、osc 参照、H1 否定、seq run_low |
| 746–806 | P6 相位机更新 |
| 807–934 | 区间套正向定位（nest_forward） |
| 935–1,045 | gate41 / b3_start 事件更新 + 全部相位/趋势/anc/R2/双向谓词闭包 |
| 1,068–1,314 | **阶段 A**：Long 出场/停削/铰链升级/回补/空头翻面 |
| 1,315–1,359 | **阶段 B**：counter_sub 短差开腿 |
| 1,360–1,397 | **阶段 B''**：seq38_sub 子腿开 |
| 1,398–1,426 | **阶段 B'**：统一 osc 开腿 |
| 1,427–1,614 | **阶段 C**：Flat/Pending 入场回复；Short/Gated 出口集 |
| 1,615–1,701 | eod 全层收口与 final_nav |

### 2.2 pub 面与仓内消费点

| 符号 | 行 | 可见性 | 直接消费者（`rust/src`，全部） |
|---|---|---|---|
| `SUB_COST_K` | 88 | `pub(crate)` | `recursive_nested_fugue.rs:78`、`nested_interval_fugue.rs:68`、`unified_necessity.rs:195`、`unified_osc.rs:94`、`nested_fugue.rs:67`、`unified_recursive.rs:77`、`positioning_chain_fugue.rs:111`、`isolated_fugue.rs:69`（共 8 文件） |
| `SUB_FRICTION_RT` | 89 | `pub(crate)` | 上述 8 文件 + `axiom_voice.rs:60`（`C_ROUND_TRIP`） |
| `LAMBDA` | 101 | `pub(crate)` | **零外部直接消费**（仅 `SUB_SPAWN_FRAC` 派生；`spiral/params.rs` 刻意各自持有） |
| `SUB_SPAWN_FRAC` | 121 | `pub(crate)` | `unified_necessity.rs:195`（σ-不变配额 `p_units * SUB_SPAWN_FRAC` 及其单测） |
| `PhaseView` | 134 | `pub(crate)` | `dual_voice.rs:78`、`axiom_voice.rs:60`、`unified_voice.rs:66`、`unified_osc.rs:94`（相位闭包接口） |
| `run_fusion` | 349 | `pub(crate)` | `positional.rs:1093`（`run_positional` 对 `PolarityMode::Fusion{..}` 的唯一分派） |

- `rust/tests`：**零直接引用**（无 `positional_fusion` / 上述符号命中）。
- 间接面：`run_positional` → `PolarityMode::parse("fusion_t"/"fusion_s"/"fusion_u"/…")` → `run_fusion`，是 `analysis/*.py` 60+ 处消费链的 Rust 端点（证据见 `trading/mod.rs` GUARD-ROLE 块）。

### 2.3 测试面

- **in-file tests：77 个**，全部无 `#[ignore]`；入口经 `run_positional`（`run`/`run_with_rows` 夹具）或直接 `PolarityMode` 构造。
- 按模式矩阵分组：

| 组 | 数量 | 行段（近似） | 覆盖对象 |
|---|---|---|---|
| parse/守卫/常数 | 13 | 1,821–2,036；2,114–2,176；2,280–2,346；2,992–3,501；3,391–3,501；3,649–3,744；3,806–4,198 等 | 模式串解析、各轴互斥/磁带行守卫、常数 bit-exact |
| 基座 fusion_t（trend_hold） | 2 | 2,037–2,086 | `in_trend`/停削/熊市保持 |
| nest_forward | 5 | 2,177–2,279 | `nest_sub_evidence`、NestWin 武装/触发/否定/lead |
| anc 祖先趋势豁免 | 3 | 2,347–2,453 | `anc_up`/`TrendScope` |
| counter_sub/铰链/回补/decoupled/NAV | 15 | 2,454–2,814 | `sub_open_trigger`/`sub_close_trigger`/`try_restore`/`nav_fusion`/escrow |
| T 轴严格化（gate41/div_exit/b3） | 6 | 2,848–2,991 | `gate41_pass`、`b3_anchor`、`div_exit_hit` |
| 统一 U（osc 递归路由） | 11 | 3,091–3,372 | `osc_layer.try_open/step_exit`、`phase_view` |
| P6 相位机 | 6 | 3,502–3,648 | `phi` 锁定/候选/相位恢复 |
| P7 R2 位置门 | 3 | 3,745–3,805 | `r2_blocked` |
| 双向 S1–S4 / btrg / btra | 8 | 3,866–4,094 | Short/Gated 出口集、强平、anc_down、翻转会计 |
| btran（nest × short） | 1 | 4,157–4,198 | nf 触发与翻空/平空合取 |
| btrau/btrauf/btraq（消融矩阵） | 4 | 4,293–4,410 | osc 白名单分区、H1 冻结、seq38 子腿 |
| 合计 | 77 | — | — |

- 仓内没有针对 `run_fusion` 的独立外部测试文件；`rust/tests` 零覆盖。该域的长期回归主要靠 `analysis/*.py` 的 guard 模式（见 §3.3）。

---

## 3. 拆分建议（deletion test 反用）

### 3.1 level：建议切 2 刀

**刀①：测试模块整体外置（纯测试治理）**
- 迁移：`level_operating_unit.rs` 行 2,497–5,837（`#[cfg(test)] mod tests { … }`）整体搬到新文件 `rust/src/trading/level_operating_unit_tests.rs`；原文件只留：
  ```rust
  #[cfg(test)]
  #[path = "level_operating_unit_tests.rs"]
  mod tests;
  ```
- 边界：0 个生产类型/fn 移动；生产文件降到约 2,496 行，测试宿主单独成文件（与 runner.rs「测试宿主非 god module」先例同构）。
- 为什么是集中而非搬复杂度：该文件 57% 是测试。删除 `level_operating_unit_tests.rs` 后生产语义完全完整（只丢测试），说明测试不是生产循环的一部分；`#[path]` 子模块保持 `use super::*` 的私有可见性，不需要 `pub` 扩面。
- 零行为验收：**是，构造性零行为**。验证 = `cargo test trading::level_operating_unit`（54 全绿）+ 全仓编译。无对拍锁/金标准涉及（本域无 cargo golden fixture）。

**刀②：`SubLou` 类型 + `impl SubLou` 迁 `level_sub_lou.rs`**
- 迁移：行 190–824 的 `pub struct SubLou` + `impl SubLou`（含 `new/side/open_trigger/close_trigger/step/step_fractal/step_sequence38/step_counterseg/counterseg_close_trigger/close_one/cascade_close_children/cascade_close_with_self`）迁到 `rust/src/trading/level_sub_lou.rs`。
- 边界与可见性：
  - 新文件是 `level_operating_unit` 的兄弟私有模块（`mod level_sub_lou;`），旧路径兼容用 `pub use level_sub_lou::SubLou;`。
  - 最小可见性改动：`SubLou::{new, step, cascade_close_with_self}` 加 `pub(super)`（当前仅父模块 `VoiceUnit::step_sub_tree/close_all_paired` 与父模块测试调用）；外部 pub 路径 `trading::level_operating_unit::SubLou` 不变。
  - 留在原文件：`VoiceUnit::step_sub_tree`（驱动端，行 2,235–2,275）与 SubLou 经 VoiceUnit 的 4 个级联测试（行 5,073–5,242）；测试共享夹具不动。
- deletion test 反用：`SubLou` 是**独立类型 + 独立 4 模式状态机**，不是 `VoiceUnit` 循环体内的一个步骤。删除新文件后，父文件必须重新嵌入整套递归子 LOU（4 个 step 变体 + 级联会计），而不是从某处「找回一个被搬走的步骤」——这是真集中。其对外契约只有 `new/step/cascade_close_with_self` 三个 `pub(super)` 口，消费面不扩大。
- 零行为验收：**是（可证）**。验收序列：
  1. `cargo test trading::level_operating_unit`：54 全绿（SubLou 直测 12 个 + 级联 4 个都在父测试里原位保留）；
  2. `cargo test trading::sublevel_confirmation_ablation`（4 快测；`sublevel_confirm_oklo` 需数据/按 ignore 流程重跑）——V2r 基线硬计数是现有最强锁；
  3. 相关回测守卫复跑：`analysis/sequence38_backtest.py`（V2of/V2ofF1 逐位零漂移）、`analysis/p1_dual_gate_backtest.py`（V2ofF1 基线）、`analysis/seq38_main_backtest.py`（V2oa25_ht 基线对账）。
  - 对拍锁/金标准：**无 cargo golden fixture 直接覆盖本文件**；`rust/tests/fixtures/*.golden*` 全部属于 theta_v0/issue533，不触碰。靠上述计数断言 + Python 回测守卫。

**本轮不切：VoiceUnit 的 osc / Cycle38 / rev_paired 三块**
- 理由：三者都是**同一个 `impl VoiceUnit` 的私有方法族**，共享 `ladder/phase/rev/sub_*_bar/cycle38_on/rev_run_low` 等私有状态；迁移需要 `pub(super)` 扩展 + 子模块 impl 拆分，但读者理解 FSM 仍须按 T1–T7 优先级横跨这些块。deletion test 反用：删除其中任何一块，`step` 只是少一个 phase/mode 分支，core 反而要保留「该模式不可达」的调用残片——不像 SubLou 那样是一个可整体删除的独立类型。属于搬复杂度 > 集中复杂度。
- 重启条件（如实记录，不预拍）：若 `rev_paired` 块继续增长且出现第二个生产调用者，优先把行 1,788–2,327 迁 `level_rev_paired.rs`；osc 块（1,210–1,509）与 Cycle38 块（1,510–1,787）同理，但当前证据不支持本轮切。

### 3.2 positional：建议不切（生产侧）

- 结构事实：生产侧只有 6 个 `pub(crate)` 项（4 常量 + `PhaseView` + `run_fusion`）和 5 个私有辅助 fn（另有 3 个私有类型），核心是 **1 个 1,354 行的 `run_fusion` 单循环**。内部「阶段 A/B/B''/B'/C」不是独立子系统，而是同一 bar 循环内对共享 `layers/subs/pool/osc_layer/res` 的顺序写步骤。
- 候选刀逐个用 deletion test 反用：
  1. **守卫矩阵（349–626）**：纯校验，但签名就是 run_fusion 的 15 个参数；迁到新文件只得到一个「单调用点、15 参数、无复用」的 helper，删除新文件后剩余循环仍完整，不构成真集中。若要改善，正确动作是**在原文件内抽 `validate_fusion_args(...)`**，不是拆文件。
  2. **相位机/nest/阶段 A/B/C**：均捕获或可变借用 `dir_state/trend_state/phi/nest_*/layers/subs/pool/osc_layer/book/res` 等 10+ 项循环局部状态；拆出去必须发明状态 struct 或 20+ 参数闭包。删除新文件后，原循环留下一堆洞和闭包上下文，读者仍需跨文件重装配——与 issue749 `fill.rs`「同循环体单编排」判定同构。
  3. **测试外置**：可以做（2,721 行是模式矩阵 harness），但它不改变生产复杂度；本报告不把它计为生产拆分。
- 结论：**不切生产侧**。现有文件 1,702 生产行低于此前架构勘察对 god module 的警示线，且 pub 面已经极窄（1 入口 + 4 常数 + 1 枚举）；拆分收益主要是行数，成本是刺穿单循环状态与 77 个模式矩阵测试的验收面。
- 若未来强行切：验收必须执行 `analysis/hold26_counterseg_fusion_backtest.py` / `fusion_t_eight_asset_backtest.py` / `_s1s4_guard_baseline.py` / `unified_recursive_routing_backtest.py` / `p6_phase_machine_backtest.py` 等脚本内的 guard-a/guard-b（在册数字 0.05pp / 逐位复现）。cargo 侧无该域金标准，现有 77 单测是唯一仓内快速锁。

---

## 4. 证据不足处（如实声明）

- trading 域没有拆分先例，`level_sub_lou.rs` / `level_operating_unit_tests.rs` 的文件名与模块组织是建议形态，不是仓内既有约定。
- `rust/tests` 对两文件零直接引用；无法从 cargo 集成测试面给出更多零行为证据。theta_v0 的 golden fixture 与这两个文件无交集。
- `sublevel_confirm_oklo` 与 `analysis/*.py` 守卫需要 OKLO 等数据文件才能实跑；本报告只核对其源码中的基线常量与守卫逻辑，未实跑（本任务只读且未运行 cargo/python 回测）。
- `LAMBDA` 零外部消费、`RevLeg/SubLou/RevOpenVerdict/RevTranche` 零外部直接消费均为 grep 事实；不据此建议删除（GUARD-ROLE 现役、公开面兼容与测试面在案）。

本报告绑定 #1189，工作草稿
