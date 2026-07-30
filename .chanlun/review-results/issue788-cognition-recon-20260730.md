# 认知对账：模块自我描述 vs 实际状态偏差表（issue #788，map #787）

## 0. 方法与边界

- 本 worktree（`agent-a4f7cf707afe12f32`）checkout 在一个远早于 main 的分支上，磁盘上没有
  `rust/` `formal/` `analysis/` `trading_system/` 目录。改用 `git archive main | tar -x` 把
  main 分支的完整树只读展开到 `/tmp/nc-main-788/` 后在那棵树上勘察（未在 worktree 内做任何
  checkout/switch，未修改本仓任何文件）。main 取自本次勘察发起时的 HEAD
  `f6d000fed26ba9ab3f7d94f10e9253c0c864b8c3`。
- 硬判据：Rust 侧写脚本沿 `mod`/`#[path]` 声明链从 `rust/src/lib.rs` 递归展开，得到真正进编译树
  的文件集合；`src/bin/*.rs` 按 Cargo 惯例自动各自成二进制，不需要 `mod` 声明，未计入"孤儿"。
  Python 侧用 `grep` 查实际 import 调用点。
- 疆域内四区全部触达：`rust/src/*.rs`、`rust/src/theta_v0/`、`formal/`、`src/newchan`、
  `analysis/`、`trading_system/`。`formal/`（`lakefile.toml` 的 `Origin` 库 96 个 roots vs
  `formal/Origin/*.lean` 92 个文件，仅差 `ParityFixtureExport.lean`——CLAUDE.md 明确它是按需
  `lake env lean` 直跑的导出器，不进 `defaultTargets` 是设计而非缺口）核查未发现偏差，未列入表。
- **090 照实**：本轮对 `analysis/`（658 文件）、`trading_system/`（94 文件）做了关键词命中 + 抽样
  核查，未做逐文件调用图核查——这两区若有更深偏差，本报告**查不到**，不代表"不受影响"。

## 1. 偏差表

| # | 模块/文件 | 自称 | 实际 | 方向 | 证据 |
|---|---|---|---|---|---|
| 1 | `rust/src/theta_v0/nautilus/bar_adapter.rs` | 文件头（L1,L10）：「★骨架（nautilus 依赖未加，不编译）……不 `use nautilus_*`」 | `L75` `#[cfg(feature = "nautilus")] pub fn from_real_bar(bar: &nautilus_model::data::Bar, ...)`——真实 `use nautilus_model::data::Bar` 且被 `theta_strategy.rs:159` 真实调用 | 以为没做，其实做了 | `nautilus` feature 在 `Cargo.toml` 有 5 个真实 v0.60.0 依赖；`mod.rs`（无 cfg 门控）`pub mod bar_adapter;` 无条件编译 |
| 2 | `rust/src/theta_v0/nautilus/order_adapter.rs` / `account_adapter.rs` / `strategy.rs` | 三个文件头均写「★骨架（nautilus 依赖未加，不编译）：真实接口锚以注释 + TODO 标注」 | 三者均被 `theta_strategy.rs`（`use super::account_adapter::PortfolioSnapshot; use super::order_adapter::{OrderIntent, OrderSideLike}; use super::strategy::ThetaCore;`）真实消费，且三者本身在 `mod.rs` 里无 `#[cfg(feature="nautilus")]` 门控（`theta_strategy`/`backtest_engine` 才有门控）——即这三个文件本来就**始终编译**，从未处于"未加依赖不编译"的状态 | 以为没做，其实做了 | `rust/src/theta_v0/nautilus/mod.rs` L57-60：`pub mod account_adapter; pub mod bar_adapter; pub mod order_adapter; pub mod strategy;`（无门控）；同文件顶部有 #524 订正段，明确说「三项自称均已为假」——但订正只写在 `mod.rs`，四个子文件各自的文件头一字未改 |
| 3 | `docs/ROADMAP.md` 支柱 IV 组件表 | 「风控模块 ❌ 未实现」「下单执行器 ❌ 未实现」 | `rust/src/theta_v0/strategy/risk.rs`（2230 行，结构止损 + sizing，契约锚 `Origin.RiskProj`）、`rust/src/theta_v0/strategy/exec.rs`（492 行，延迟成交/费用/止损成交/不可交易过滤）、`rust/src/theta_v0/nautilus/order_adapter.rs`（S_Θ Order↔Nautilus order_factory 映射）均已实装且进编译树 | 以为没做，其实做了 | 三文件均在 `theta_v0` 的 `mod` 链上（`strategy/mod.rs`→`pub mod risk;`/`pub mod exec;`，`nautilus/mod.rs`→`pub mod order_adapter;`），非孤儿；`risk.rs`/`exec.rs` 头部各自写明契约锚与规格行号，非占位 |
| 4 | `docs/ROADMAP.md` 支柱 I | 全篇仅描述 Python 五层管线 + 「Rust 引擎（8 层逐位等价）」，未提及 `theta_v0`/`nautilus` 生产链路的存在 | `theta_v0::nautilus` 是可驱动真实 `BacktestEngine`/`LiveNode` 的生产适配层（`backtest_engine.rs` 240 行，`theta_strategy.rs` 184 行，均有真实 `use nautilus_*`），是比 ROADMAP 描述的 8 层 bit-exact 引擎更新一代的架构层，ROADMAP 完全未提 | 以为没做，其实做了 | `rust/src/theta_v0/nautilus/mod.rs` 头部「goal `g-l2-nautilus-production`」+ 「本模块已过骨架期、是实装适配层」段 |
| 5 | `rust/src/theta_v0/classifier/cp_replay_diagnostics.rs`、`oracle_probe.rs`（`classifier/` 顶层，非 `classifier/diag/` 子目录下的同名文件） | 两文件无任何"孤儿/废弃"自称（无模块头状态声明） | 未被任何 `mod`/`#[path]` 声明引用，是 `#748(C4)` 把这两个文件"移入 `diag/` 子目录"后遗留在原路径的**字节几乎相同的旧副本**（`diag/cp_replay_diagnostics.rs` 比顶层版本多两行迁移说明，其余相同）；顶层版本从未进编译树 | 以为做了，其实没做（严格讲：自身不做自称，但目录结构制造了"这里也有一份实现"的错觉） | `theta_v0/classifier/mod.rs:140` 只 `pub mod diag;`，顶层两文件不在任何 `mod` 声明里；`diff` 顶层版本与 `diag/` 版本仅差 2 行迁移注释 |

## 2. 按严重度排序（最上面最严重）

1. **#2**（三个 nautilus 适配子文件头「不编译」自称，实际始终编译且被真实调用）——最严重：这正是
   issue #788 拿来做标本的 #524 案例的**未收尾部分**。#524 订正了 `mod.rs` 这一层的三项自称，
   但没有传导到 `account_adapter.rs`/`order_adapter.rs`/`strategy.rs` 三个子文件自己的文件头——
   编排者如果只读子文件（比 `mod.rs` 更细粒度、更容易被当作"就近权威"），会重新踩回 #524 想订正
   的那个坑。
2. **#1**（`bar_adapter.rs` 头写「不 use nautilus_*」，本体第 75 行就是 `use nautilus_model::...`）
   ——同一批遗留，且是四个文件里自相矛盾最直接的一个（本文件内部头尾互相打脸，不需要跨文件比对）。
3. **#3**（ROADMAP「风控/执行器 ❌ 未实现」，实际 2230+492 行真实实装）——最贴合 issue #788 举例
   的模式（骨架自称 vs 真实现），且是编排者最可能直接读的文档（ROADMAP 是纲领文件，不是深埋的
   模块注释）。
4. **#4**（ROADMAP 支柱 I 完全不提 `theta_v0`/`nautilus` 生产链路）——比 #3 更系统性：不是某个
   状态标记错了，是整份文档的认知框架停留在"8 层 Rust bit-exact 引擎"这一代，没有反映"再往上
   还有一层可驱动真实 BacktestEngine/LiveNode 的 theta_v0 适配层"这个事实，与 map #787 起源段
   点名的"编排者判断 NT 生产路径没接进去线"是同一根因的另一处症状。
5. **#5**（`classifier/` 顶层两个孤儿副本）——最轻：不是自我描述错误，是 `#748` 重构遗留的死重复
   文件，唯一风险是未来有人对着错误路径改代码、改了不生效（因为没进编译树）。

## 3. 计数汇总

- **以为没做，其实做了**：4 条（#1 #2 #3 #4）
- **以为做了，其实没做**：1 条（#5，且严格说不算"自称"错误，是孤儿重复文件而非自我描述违背事实）
- **查不到 / 未穷尽**（090 照实声明）：`analysis/`（658 文件）与 `trading_system/`（94 文件）
  仅做了关键词抽样，未做逐文件调用图核查，可能还有偏差未被本轮捕获；`src/newchan`（204 文件）
  同样仅做了关键词抽样 + 少量 import 链核查（`a_segment_v0.py` 自称"占位"经查实际只有其
  `Segment` 数据类型被广泛复用，算法本体 `segments_from_strokes_v0` 仅 `ab_bridge_newchan.py`
  一处消费，自称与实际**吻合**，排除出偏差表）。

## 报告路径

`/Users/silencehan/Projects/NewChanlun/.claude/worktrees/agent-a4f7cf707afe12f32/.chanlun/review-results/issue788-cognition-recon-20260730.md`
