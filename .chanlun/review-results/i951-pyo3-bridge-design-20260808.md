# theta_v0 PyO3 出口接口设计——跨范畴缝合方案（#951）

**只设计，不实装。** 本文未改任何既有代码文件，未修 #947，未碰 `rust/src/bin/p938_bound_center_openness.rs`。

- 工作 HEAD：`161202ce26`（`docs(#932): ADR 0021 补测节`）＝ 本地 `main` 顶点。
  worktree 初始 HEAD 是 `19b4015927`（祖传 `origin/main` 线，与 `main` **零共同祖先**，`git merge-base` 空输出），
  已 `git reset --hard main` 纠正。本文全部读数基于 `161202ce26`。
- 实测编译：`cargo build --lib`（default features）exit 0，14.93s。**未跑 `cargo check`**。
- 标注约定：【实测】= 本会话亲手打开/跑出；【推断】= 由已读事实推出但未验证。

---

## 0. 结论先行

**核心缝不需要新造，它已经在 `theta_v0` 里，只是没被导出。**

`theta_v0` 的 π_Θ 决策环内部**本来就是目标净敞口机器**：
`pi_theta_position(...) -> f64` 产 **`p_star` ＝ 目标净持仓（有符号、lot 对齐）**
（`rust/src/theta_v0/strategy/coverage/sizing.rs:468`），
紧接着 `schedule_order(p_star, p_t, exec_index) -> Order`（同文件 `:524`）
把 `Δ = p_star − p_t` 翻译成 `StrictAction` 订单。
两者串在 `pi_theta_step`（同文件 `:616`，`pub`），返回 `(A_{t+1}, p_star, (Order, ProtocolEvent))`。

⟹ **`recursive_t` 的 `lu − su`（`ffi.rs:319`）与 `theta_v0` 的 `p_star` 是同一范畴的量**；
`Order`/`StrictAction` 在 `theta_v0` 内部**不是并列范畴，而是 `p_star` 的派生物**——
`schedule_order` 就是「目标敞口 → 订单」的那个已实装的翻译器，含穿零、含 Add/Reduce、含 Wait/Hold 全七态。

**⟹ #808 的 B1「范畴差是硬的」需要按对象分层订正**（见 §1.4）：

- 在 **`theta_v0` 决策核心层**（`coverage/sizing.rs`）：范畴差 **不成立**，`p_star` 与 `lu−su` 同范畴，翻译器现成。
- 在 **`theta_v0` 当前唯一的流式出口层**（`nautilus::strategy::ThetaCore::plan_for_bar`，`rust/src/theta_v0/nautilus/strategy.rs:138`）：范畴差 **成立**——
  该路径走的是**另一条订单生产链**（`strategy::plan_orders`，逐声部 Buy/Sell/Close），
  **根本不经过 `p_star`**，且把目标敞口这个量在返回前就丢掉了。
- ⟹ 真正的硬障碍**不是**「敞口 vs 订单」，而是 **`theta_v0` 内部有两条互不相通的订单生产链**，
  而 D1 要的两样东西（目标净敞口 ＋ per-leg 账本）**只在其中一条上同时存在，且那条不是流式的、也不在默认构建里**。

**一句话方案**：
> 把生产 π 回路的**单 bar 步**从批量 fill loop 里抽出来做成 `ThetaPiStream`（与 `recursive_t` 的
> `TFugueStreamCore` 同款「流式/批量共核」形态），PyO3 出口 `ThetaStream.push_bar(o,h,l,c) -> f64`
> **直接返回 `p_star`**——与 `RecTStream.push_bar` 签名逐字同形；
> 订单不由 Rust 侧产，由 Python 侧照 `rec_t_strategy.py:_rebalance` 的老办法算 `delta = target − portfolio.net_position`，
> **成交回执不回填 Rust**（`p_t` 由调用方每 bar 显式传入，见 §1.3 的状态裁定）。
> per-leg 账本走 `StepTrace.sep_legs → OverlayState`（`rust/src/theta_v0/strategy/overlay_state.rs:170`）＋
> `LevelLedgerMirror`（`rust/src/theta_v0/strategy/level_ledger.rs:217`）——两者**都已实装且已在生产 fill loop 里被驱动**。

---

## 1. 核心缝：目标净敞口 ↔ 订单契约

### 1.1 两边现在各是什么（逐行确认）

| | `recursive_t` | `theta_v0` |
|---|---|---|
| 流式入口 | `PyRecStream::push_bar(o,h,l,c) -> f64`（`rust/src/recursive_t/ffi.rs:316`） | `ThetaCore::plan_for_bar(Bar, &PortfolioSnapshot) -> Vec<OrderIntent>`（`rust/src/theta_v0/nautilus/strategy.rs:138`） |
| 返回值语义 | `lu − su`＝目标净敞口 signed units（`ffi.rs:317-319`，`root().exposure()` 的差） | 本 bar 下单意图列表（`OrderSideLike` ＋ qty ＋ reduce_only ＋ 可选限价） |
| 当前持仓从哪来 | **不需要**——引擎自持内部模拟账本，`target_net_units()`（`ffi.rs:323`）随时可读 | **调用方传入**——`PortfolioSnapshot.net_position`（`rust/src/theta_v0/nautilus/account_adapter.rs`，`PortfolioSnapshot` 定义在该文件） |
| 有无 `p_star` 式的目标量 | 有（就是返回值本身） | **有，但没导出**：`pi_theta_step` 的第 2 个返回值（`coverage/sizing.rs:616`） |

⚠️ **#951 票面写的 `ffi.rs:314-318` 在本 HEAD 是 `316-319`**（`fn push_bar` 在 `:316`，`lu - su` 在 `:319`）。
断言本身成立，行号有 2 行漂移，本文按 HEAD 实测行号引用。【实测】

### 1.2 已实装的翻译器：`schedule_order`

`rust/src/theta_v0/strategy/coverage/sizing.rs:524-566` 逐行读出（非转述）：

```
Δ = p_star − p_t;  qty = |Δ| 四舍五入
qty==0  ∧ 当前平 → Wait        （qty=0，不下单）
qty==0  ∧ 当前有仓 → Hold      （qty=0，不下单）
当前平 → Buy(p_star>0) / Sell(p_star<0)，qty=|Δ|
目标平 → Close，qty=|Δ|（＝|p_t|）
同号且|p_star|>|p_t| → Add
同号且|p_star|<|p_t| → Reduce
反号穿零 → Buy(p_star>0)/Sell(p_star<0)，qty=|Δ|   ← 单张净额订单跨零
```

⟹ 票面要求回答的**四个边界情形，三个已有现成答案**：

1. **目标 +3 → −2 穿零**：`schedule_order` 走最后一支，产**单张** `Sell(qty=5)`。
   这是**净额账户（NETTING）语义**下的正解；`rec_t_strategy.py:_rebalance`（`:90-106`）用的也是同一算法
   （`delta = target − cur`，单张 market 单），两边**行为一致**，不需要新设计。
   ⚠️ 有效域：仅在 NT `OmsType.NETTING` 下成立。HEDGING 账户下单张跨零单不合法——**本设计不覆盖 HEDGING**（见 §5.3）。
2. **部分成交**：`schedule_order` 是**无状态纯函数**——下一 bar 用新的 `p_t`（＝真实持仓）重算 `Δ`，
   未成交部分**自动被下一 bar 的目标追平**。⟹ 部分成交**不需要专门处理**，这是目标跟踪范式相对信号范式的结构性好处。
3. **拒单**：同上——拒单 ≡ `p_t` 没变，下一 bar 重算出同样的 `Δ` 再下一次。
   ⚠️ **但会无限重试**（每 bar 一次）。`rec_t_strategy.py` 用 `min_rebalance_units`（`:31`，默认 `1e-4`）
   挡微小 churn，**不挡重复拒单**。本设计**照抄这个缺口，不假装补上**（见 §5.4）。
4. **反向回填（成交回执 → 当前净敞口）**：见 §1.3，这是唯一需要裁定的一项。

### 1.3 状态放哪（票面第三问）：**裁定 = 无状态（`p_t` 每 bar 由调用方传入）**

两套的现状（【实测】）：

- `recursive_t`：**有状态**。`RecTStream` 自持完整内部模拟账本（`op_counts()` `ffi.rs:337` 数
  enter/sink/recover/ascend/flip；`finish()` `:349` 出引擎自算 `final_nav`），
  Python 侧 `rec_t_strategy.py` **同时**读 `portfolio.net_position`（`:96`）——
  ⟹ **两本账并行且从不对账**：引擎的 `final_nav` 与 NT 真账本只在日志里并排打印（`:114-117`），无断言。
- `theta_v0`：**已经是无状态的**。`ThetaCore::plan_for_bar` 显式吃 `&PortfolioSnapshot`，
  文档 `nautilus/strategy.rs:117-121` 逐字写「持仓**数量**真相源 = Nautilus portfolio ……**不用** S_Θ
  `plan_and_fill_mtm` 的内部模拟台账」；`account_adapter.rs` 的 `to_account_state` 把 `net_position` 灌进 sizing。
  `pi_theta_step` 同样把 `p_t` 作为入参（`sizing.rs:616` 的第 4 参）。

**裁定：新出口沿用 `theta_v0` 的无状态口径，`push_bar` 签名带 `p_t`。**

```
ThetaStream.push_bar(o, h, l, c, p_t, nav) -> f64      # 返回 p_star
```

理由三条：
1. **不新增双账**。`theta_v0` 已经明文消过一次双源（上引 `strategy.rs:117-121`），
   为了跟 `RecTStream.push_bar(o,h,l,c)` 签名对齐而在 Rust 侧再养一本影子仓位，等于把已消掉的双账重新装回来。
2. **拒单/部分成交自愈**（§1.2 第 2、3 条）只在 `p_t` 是**真实持仓**时成立；
   若 `p_t` 用 Rust 内部影子值，这两条性质立刻失效。
3. `p_t` 无论如何都得进来——`schedule_order` 与 `pi_theta_position` 的可行集 `𝒦_Θ` 都消费它。

**代价（照实）**：签名与 `RecTStream.push_bar(o,h,l,c)` **不逐字兼容**（多两个参数）。
⟹ 消费者迁移不是「改个类名」，最少要改一行调用。这个代价是**故意付的**，不是疏漏。
提供便利重载 `push_bar_selfstate(o,h,l,c)`（内部用上一次 `p_star` 当 `p_t`，模拟「全成交无滑点」）
**仅供研究脚本对拍用，且必须在 docstring 里写明它不是生产语义**——不设默认参数，避免生产误用。

### 1.4 那 B1 到底错在哪、对在哪

- **对的部分**：`rec_t_strategy.py:8`（【实测】原文：「与 ChanlunStrategy（BSP 信号→订单）的范畴差：
  递归引擎产出**仓位**不是信号，故走目标敞口跟踪（NETTING 镜像），不走 BSP→BUY/SELL 映射」）
  ——作者自陈的对象是 **`ChanlunStrategy`（BSP 信号链）**，不是 `theta_v0`。这句话本身没错。
- **被外推错的部分**：#808/#807 把这句话搬到 `theta_v0` 头上，得出「`theta_v0` 是订单契约 ⟹ 范畴差」。
  但 `theta_v0` 的 π_Θ 环**恰恰不是 BSP→BUY/SELL 映射**，它是 `p̃ → LexArgmin → p_star → Schedule_Θ`
  ——**与 `recursive_t` 同构的目标跟踪范式**。⟹ 这一步外推**不成立**。
- **仍然成立的硬障碍（真名字）**：`theta_v0` 内部**两条订单链并存**，
  ① `nautilus::strategy::ThetaCore::plan_for_bar` → `strategy::plan_orders`（`rust/src/theta_v0/strategy/mod.rs:302`）→ 逐声部 `build_open_order`/`build_exit_order`（`:440`/`:462` 造 `Order`）。**无 `p_star`，无 `sep_legs`。**
  ② 回测 π loop `pi_theta_fill_loop_overlay`（`rust/src/theta_v0/backtest/fill.rs:4369`）→ `pi_theta_step_traced_with_risk_seeds`（`:5142` 调用点）→ `p_star` ＋ `StepTrace.sep_legs`。**有 `p_star`，有 per-leg。**
  ⟹ **生产 NT 路径（①）跑的不是 D1 要导出的那条链（②）。** 这是本票真正要处理的东西。

  ⚠️ 这一条是**新发现，#808 未记载**——#808 的 B1 把矛盾定位在「`recursive_t` vs `theta_v0`」的跨模块面，
  实际矛盾在 `theta_v0` **模块内部**。【实测：两条链的调用图逐个函数打开确认】

---

## 2. D1 两项出口具体长什么样

### 2.0 前置：`theta_v0/strategy/ledger.rs` **不是** per-leg 账本（票面要求逐条打开确认，已做）

`rust/src/theta_v0/strategy/ledger.rs`（1775 行）文件头 `:1-60` 逐行读：
它是**双账本分量**——`LedgerComp`（`:675`，R = Π − A − W 收益表）＋ `TwState`（`:175`，
取本金三阶段 free/holding/withdrawn）。两者锚 `Origin.FullDefinitionStrategy.LedgerState` /
`Origin.TotalWealth`，且 `:90` 明写两者**不同构**。
**与「腿」无关，靠文件名猜会猜错。**【实测】

**真正的 per-leg 账本在别处，且已实装**：

| 结构 | 位置 | 是什么 |
|---|---|---|
| `SepLeg` | `rust/src/theta_v0/strategy/coverage/leg.rs:23` | 逐声部**目标**腿：`id: ElementId` / `side: VoiceSide` / `q_units: f64` / `role_v: Vertical` / `parent_id: Option<ElementId>` |
| `VoiceBook` | `rust/src/theta_v0/strategy/overlay_state.rs:62` | 逐声部**持仓**簿行：`id` / `side` / `q: i64`（整数手数）/ `role_v` / `parent_id` / `entry_bar` / `entry_px` / `pnl_v` |
| `ClosedVoice` | 同文件 `:83` | 已离场声部归因：上述 ＋ `exit_bar` / `exit_px` / 冻结 `pnl_v` |
| `OverlayState` | 同文件 `:102`，`step()` 在 `:170` | 持久逐声部簿；`step(sep_legs, px, bar, lot) -> OverlayStep{net_before, net_after, order}`，`order = ΔN` |
| `LevelLedgerMirror` | `rust/src/theta_v0/strategy/level_ledger.rs:136`，`step()` 在 `:217` | 按 `id.level` 分桶的**只读**级别账本镜像；`lee_net_witness()` 见证 `Σ_ℓ net_ℓ ≡ N` |

⟹ **D1 的第 ② 项「per-leg 账本」不需要新建，需要的是「导出」。**
它们已经被生产回测臂 `run_theta_v0_pi_overlay`（`rust/src/theta_v0/backtest/runner.rs:476`，
`:500`/`:503` 构造两者）逐 bar 驱动。

### 2.1 出口 ①：目标净敞口

```rust
// 新增：rust/src/theta_v0/ffi.rs
#[pyclass(name = "ThetaStream")]
pub struct PyThetaStream { core: ThetaPiStream }

#[pymethods]
impl PyThetaStream {
    /// symbol/config 预设名；不传 ⟹ ThetaConfig::default()
    #[new]
    #[pyo3(signature = (preset=None, initial_nav=1.0e6))]
    fn new(preset: Option<String>, initial_nav: f64) -> PyResult<Self>;

    /// 推一根 bar。`p_t` = 调用方真实净持仓（手数，有符号）；`nav` = 账户净值。
    /// 返回**目标净持仓 p\*** （有符号手数，lot 对齐）——与
    /// `RecTStream.push_bar` 的 `lu−su` 同范畴同量纲。
    fn push_bar(&mut self, o: f64, h: f64, l: f64, c: f64, p_t: f64, nav: f64) -> f64;

    /// 上一步的 p\*（不推进 bar）。对位 `RecTStream.target_net_units()`（ffi.rs:323）。
    fn target_net_units(&self) -> f64;

    /// 上一步 Schedule_Θ 的订单形态（诊断用，**不是**下单指令——生产下单由
    /// Python 侧 delta 提单，见 §1.3）。
    /// 返回 (action: str, qty: int, exec_index: int)；action ∈
    /// {"buy","sell","add","reduce","hold","close","wait"}（StrictAction 七构造子，types.rs:345）
    fn last_order(&self) -> (String, i64, usize);

    /// 快照：(cur_bar, p_star, n_active_legs, n_levels)
    fn snapshot(&self) -> (i64, f64, usize, usize);
}
```

**语义锚**：返回值 = `pi_theta_step` 的第 2 个返回值（`coverage/sizing.rs:616` 签名的 `f64`），
即 `p* = LexArgmin_{p∈𝒦_Θ(x)} J_x(p)`。
`last_order` 的三元组直接来自 `theta_v0::types::Order`（`rust/src/theta_v0/types.rs:359`：
`action: StrictAction` / `qty: i64` / `exec_index: usize`），
`StrictAction` 七构造子在 `types.rs:345-357`。

**为什么 `last_order` 只作诊断**：`Order.exec_index` 是**回测的延迟成交 bar**
（`types.rs:363` 注释：「触发该订单的 bar（执行延迟后的成交 bar，reference-theta-v0.md:50）」，字段本体 `:364`），
生产路径没有延迟队列——`nautilus/strategy.rs:129-133` 已明文裁过这一条
（「回测 runner 用**内部延迟成交队列**，生产路径**无延迟队列**……撮合时序差异归 venue」）。
⟹ 把 `exec_index` 当下单指令用会引回一个假的时序模型。**导出它、但降级为诊断字段。**

### 2.2 出口 ②：per-leg 账本

```rust
    /// 逐声部目标腿快照（本 bar 的 P^sep_{t+1}）。
    /// 每项 = (level: u32, elem_id: (u32,usize,...), side: str, q_units: f64,
    ///         role_v: str, parent_id: Option<...>)
    fn sep_legs(&self, py: Python<'_>) -> PyResult<PyObject>;

    /// 收尾 → 完整 per-leg 账本 dict（对位 RecTStream.finish_full，ffi.rs:360）：
    ///  - "active_voices" : [VoiceBook 行]  逐声部在飞：
    ///        (level, side, q, role_v, parent_level, entry_bar, entry_px, pnl_v)
    ///  - "closed_voices" : [ClosedVoice 行] 逐声部已离场：
    ///        (level, side, q, role_v, parent_level, entry_bar, exit_bar,
    ///         entry_px, exit_px, pnl_v)
    ///  - "account_price_pnl" : f64    账户级 Σ_t N_t·ΔP_t
    ///  - "total_voice_pnl"   : f64    Σ_v pnl_v
    ///  - "reconcile_residual": f64    |前两者之差|（PDF §11 对账，应 < eps）
    ///  - "level_nets"        : [(level, net_ℓ)]  LevelLedgerMirror::nets()
    ///  - "lee_net_witness"   : (n_observations, max_abs_residual, max_abs_net,
    ///                           identity_witnessed: bool)
    ///  - "n_orders"          : usize  ΔN 非零步数
    fn finish_full(&mut self, py: Python<'_>) -> PyResult<PyObject>;
```

**字段来源全部是既有 getter，零新计算**（【实测，逐个确认存在】）：

| dict 键 | 来源 |
|---|---|
| `active_voices` | `OverlayState::active_voices()`（`overlay_state.rs:137`）→ `&VoiceBook` |
| `closed_voices` | `OverlayState::closed_voices()`（同文件 `:142`；⚠️ `:468` 另有一个同名方法属 `VoiceExecBook`，返回 `&[ClosedVoiceExec]`，**别引错**） |
| `account_price_pnl` / `total_voice_pnl` / `reconcile_residual` | `OverlayRunResult` 同名字段（`backtest/runner.rs:401`/`:403`/`:405`），取值点 `runner.rs:588-589` |
| `level_nets` | `LevelLedgerMirror::nets()`（`level_ledger.rs:166`）→ `&BTreeMap<u32,i64>` |
| `lee_net_witness` | `LevelLedgerMirror::lee_net_witness()`（`:182`）→ `LeeNetWitness`（字段 `n_observations`/`max_abs_residual`/`max_abs_net`（`:101-107`）＋ `identity_witnessed()`（`:112`）） |
| `sep_legs` | `StepTrace.sep_legs`（`strategy/coverage/compose.rs:130`） |

**`ElementId` 怎么过 PyO3**：`SepLeg.id` / `VoiceBook.id` 是 `ElementId`（跨 bar 稳定的声部键）。
Python 侧只需要它**可哈希、可比较、跨 bar 一致**——
⟹ 出口投影为 `str`（`format!("{:?}", id)`）或 `(u32 level, ...)` 元组。
⚠️ **此处设计不完整**：本会话未打开 `ElementId` 的定义（它在 `strategy/coverage/element.rs`，
`coverage/mod.rs:101` 以 `pub(crate) use element::{... ElementView}` 形式半导出），
所以**不能断言它的字段构成**。实施时必须先打开它再定该投影，本文不代答。【设计不出来，见 §5.1】

### 2.3 ⚠️ 承重前提：出口 ② 依赖的 `sep_legs` 只在**回测链**上产生

【实测，两条链的调用图逐个打开】：

- `sep_legs` 的**唯一生产者**是 `coverage_step_from_buckets_sep*`，唯一非测试调用点在
  `strategy/coverage/compose.rs` 的 `pi_theta_step_traced_with_risk_seeds` 内部（如 `:315`）。
  检索式：`grep -rn "sep_legs" --include="*.rs" rust/src/theta_v0/`，覆盖 `rust/src/theta_v0/` 全子树，
  排除 `*_tests.rs`/`runner_tests.rs` 后剩 fill.rs（消费）/overlay_state.rs（消费）/level_ledger.rs（消费）/compose.rs（生产）四个文件。
- `pi_theta_step_traced*` 在 `coverage/mod.rs:143` 是 **`pub(crate) use`** ——crate 外不可见，crate 内可见。
- **`pub fn pi_theta_step`（`sizing.rs:616`）不返回 trace**，只返回 `(next_active, p_star, decision)`。
- ⟹ **出口 ① 单独用 `pi_theta_step` 就能做（`pub`，默认构建可达）；出口 ② 必须走 `pi_theta_step_traced`。**

**这对模块布局有硬约束**：新 FFI 必须放在 **crate 内**（`rust/src/theta_v0/ffi.rs`），
才能用 `pub(crate)` 的 `pi_theta_step_traced` / `ElementView` / `TwStepCtx`。
⟹ **不需要改任何可见性**。这是本设计选择「新模块进 crate」而非「独立 bridge crate」的决定性理由。

---

## 3. 九个消费者逐个判

### 3.0 「九个」的口径澄清

#808 §① 表格分四类：**生产 0 ／ 回测 3 文件（2 链）／ 一次性研究 9 文件 ／ 已死 1**。
D2 的「九个消费者」= **一次性研究那 9 个文件**。
本节把 **13 个单元全判**（3 回测 ＋ 9 研究 ＋ 1 已死），不留缺口。

13 个文件的存在性【实测，`ls` 逐个确认，本 HEAD 全部在场】：
`analysis/{capture_ratio_matrix, t_vs_v3_comparison, t3_exit_trigger_diag, tc_churn_diagnostic,
tc_churn_discriminant, tc_churn_exhaust, tc_churn_reason, tc_churn_safety_margin, tc_churn_stoploss_probe}.py`
＋ `trading_system/{backtest_t_fugue.py, backtest/rec_backtest.py, strategy/rec_t_strategy.py}`
＋ 导出 `run_t_fugue`（`lib.rs:2698`）。

FFI 直调点检索式：`grep -rn "RecTStream\|TFugueStream\|run_t_fugue\|run_recursive_t" --include="*.py" --include="*.ipynb" .`
（排除 `./rust/`、`node_modules`），覆盖仓根全目录。【实测】

### 3.1 #799 裁定六的前置划线

裁定六：**模块化管的是「生产判定路径」；对照臂／只读观测器／验证装置属「不适用」而非「例外」。**

先划管辖，再配处置：

- 【实测，与 #808 一致】`recursive_t` 的**生产消费者 ＝ 0**。
  `RecTStrategy` 只被 `trading_system/backtest/rec_backtest.py` 引用
  （检索式 `grep -rln "RecTStrategy" trading_system/`，命中 2 处：`backtest/rec_backtest.py` ＋ 自身定义文件；
  `trading_system/live/` 零命中）。
- ⟹ **13 个单元里，落在「生产判定路径」管辖内的是 0 个。**
  其余 13 个全部是**回测/研究/对照臂** ⟹ 按裁定六**「不适用」**。
- ⟹ **不需要为它们争「例外」，也不需要给它们配退场条款。**
  它们的处置判据只有一条：**这份数据/这个结论还要不要能复现。**

### 3.2 逐个处置

| # | 单元 | 类 | 处置 | 理由 |
|---|---|---|---|---|
| 1 | `trading_system/strategy/rec_t_strategy.py` | 回测策略 | **迁**（改写为 `ThetaStrategy`，接 `ThetaStream`） | 它是 D2 里**唯一**真调 `order_factory.market()`＋`submit_order()` 的（`:101`/`:106`）。新出口 `push_bar` 返回 `p_star`，`_rebalance` 的 `delta = target − cur` 逻辑（`:96-98`）**逐字不用改**——只改构造行 `:45` 与 `push_bar` 参数表（补 `p_t`/`nav`）。这是整批里迁移成本最低、收益最高的一个。 |
| 2 | `trading_system/backtest/rec_backtest.py` | 回测入口 | **迁**（跟 1 走） | 它只是 1 的 CLI 壳（`:8` 描述整条链）。1 迁完它跟着改一行 import。 |
| 3 | `trading_system/backtest_t_fugue.py` | 回测入口（第 2 链，走 `TFugueStream`） | **冻结**（不迁） | 它绑的是 `TFugueStream`（`:166`）＝ **另一个** PyO3 导出（`ffi.rs:219` 一带，`push_bar` 在 `:237` 返回 `Vec<Trade11>`——**不是** `f64` 敞口）。这是第三种范畴（逐笔 trade 流），`theta_v0` 无对应物。它托着 24 个 `t_fugue_*.json` 缓存（见 #4-#9 的 B3），迁 = 换命题。 |
| 4 | `analysis/capture_ratio_matrix.py` | 研究库 | **冻结** | B2 承重点：它整个建在 `finish_full()` 的九字段上（`:23` docstring 自陈数据源）。这九个字段在 `theta_v0` **作为导出字段名 8/9 零命中**（见下方 B2 订正），迁过去等于重写。它又被 6 个 `tc_churn_*` import ⟹ 冻结它 = 一次冻结 7 个。 |
| 5-10 | `analysis/tc_churn_{diagnostic,discriminant,exhaust,reason,safety_margin,stoploss_probe}.py` | 一次性研究 | **冻结**（随 4） | 6 个全部 `from capture_ratio_matrix import ...`（`:20`/`:22`/`:24`×4，逐个确认）。它们测的是 T 引擎自身的 churn 成因，命题里就带着「T 引擎」这个主语——换引擎 ≠ 重跑，是**换实验**。 |
| 11 | `analysis/t3_exit_trigger_diag.py` | 一次性研究 | **冻结** | 同 4：`:35` 建 `RecTStream`，`:4-5` 明写用 `leg_trades ＋ exit_trigger_log ＋ level_segments` 三字段分类每笔 r≤0 交易。三个字段在 `theta_v0` 全零命中。 |
| 12 | `analysis/t_vs_v3_comparison.py` | **对照臂** | **留作对照臂**（不迁不删） | B4：它的对照另一方是 `fugue_v3`（票面已裁）。**追加一条 #808 未记的事实**：它调的是 `run_recursive_t`（`:65`）而**不是** `RecTStream`——是**第三个**导出面（`lib.rs:2694`），吃 `orchestrator` 产的段序列、产全塔买卖点。⟹ 它连「目标敞口」这个量都不经过，B1 对它**根本不适用**。按 #799 裁定六，对照臂属「不适用」，无需处置。 |
| 13 | 导出 `run_t_fugue`（`lib.rs:2698`，定义 `ffi.rs:404` 一带） | 已死 | **删** | 全仓零 Python 调用（上方检索式命中的三处 `run_t_fugue` 全在 `backtest_t_fugue.py:38` 的 docstring 里，非调用）。#808 的附带订正（删导出不会让 standalone 子簇变孤儿，机制是 `rec_stream.rs:23` 的 `use super::iterate;`）**本会话未复核该行**，标【未验证，援引 #808】。 |

**汇总**：迁 2（#1/#2）／冻结 9（#3-#11）／留作对照臂 1（#12）／删 1（#13）。

### 3.3 「冻结」具体是什么（不能只写两个字）

按 #808 裁定三的冻结口径（**只修编译不修逻辑，且跨切面口径变更不同步**），
本票对 #3-#11 的冻结要落成**可检查的东西**，否则三个月后没人知道还要不要跟着改：

- `recursive_t` 的 4 个 PyO3 导出（`RecTStream` `ffi.rs:294` / `TFugueStream` `:219` /
  `run_t_fugue` `:404`附近 / `run_recursive_t` `lib.rs:2694`）里，**只删 `run_t_fugue`**（#13），
  其余 3 个**留在 pymodule 里不动**——冻结件的剩余价值就是复现它自己的历史结果（#808 净发现原文）。
- 9 个冻结脚本各加一行文件头声明：
  `# FROZEN(#951)：绑 recursive_t.finish_full 九字段/TFugueStream；口径变更不同步；重跑用 git checkout <本 commit>`
  ——这是**文档层**的东西，验收走 grep gate（§4）。

### 3.4 B2 的订正（票面断言「九字段零命中」，实测 8/9）

检索式：对九个字段名逐个 `grep -rn "<字段>" --include="*.rs" rust/src/theta_v0/`，覆盖 `rust/src/theta_v0/` 全子树。

- `final_nav` / `leg_trades` / `leg_close_reasons` / `leg_entry_stops` / `exit_trigger_log` /
  `pair_long_pnl` / `pair_short_pnl` / `level_segments` → **各 0 命中**。
- `level_centers` → **17 命中**，全在 `rust/src/theta_v0/classifier/mod.rs`（9 处）＋
  `classifier/recursive_tower.rs`（8 处）。逐个打开确认：
  `classifier/mod.rs:1727` 是**局部变量名**（`let level_centers = ... Rc::clone(&lc.centers)`），
  投影进 `LevelState.centers`——是**分类器塔的中枢**。
  而 `recursive_t` 的 `level_centers()`（`rec_stream.rs:735`）返回
  `Vec<(usize, i64, i64, f64, f64)>`＝「每级别 `apply_t` 产出的中枢（步骤 a `find_centers`）」。
  ⟹ **同名异物**，不构成可替代性。

**结论：B2 的实质（8 个研究脚本整体绑死、不可迁）成立；「九字段零命中」这个措辞应订正为
「九字段作为导出字段名 8/9 零命中，第 9 个（`level_centers`）是同名异物」。**
这属于「结论正确但论据要收紧」，不是改判。

### 3.5 B3 的复核（成立）

检索式 `grep -rn "Perfection\|parse_mode\|A0Source\|parse_a0" --include="*.rs" rust/src/theta_v0/` → **0 命中**（覆盖全子树）。
`recursive_t` 侧两个自变量的定义域逐行确认：
`parse_mode`（`ffi.rs:30-37`）: `structure|structural / and / or`；
`parse_a0`（`ffi.rs:41-47`）: `segment|seg / stroke|bi`。
`theta_v0` 无对应参数轴。
24 个缓存实测在场：`trading_system/data_cache/t_fugue_{BTC,ES,CL,BRN,GC,DX,QQQ,OKLO}_{structural,and,or}.json`
＝ 8 标的 × 3 模式 ＝ 24。【实测，`find . -name "t_fugue_*.json"` 计数 24】
⚠️ 注意：`"structural"` 这个**字符串**在 `theta_v0` 有 300 处命中，但是别的语境（不是 `PerfectionMode`）——
引「零命中」时必须指明是**类型/参数**零命中，不是字面量零命中。

---

## 4. 实施顺序与验收

### 4.0 前置订正：CI **已经跑** `cargo test`

#951 票面援引 #799 净发现「本仓 CI 根本不跑 `cargo test`」——**该结论在本 HEAD 已过期**。
`.github/workflows/ci.yml` `:183-187` 逐行读出【实测】：

```yaml
      - name: cargo test — default targets
        run: cargo test --all-targets
      - name: cargo test — backtest_bin feature（…）
        run: cargo test --all-targets --features backtest_bin
```

由 #810 于 2026-07-30 补入（`:165-182` 的注释块自陈）。触发面 `push`/`pull_request` → `main-rewritten`（`:4-7`）。

⚠️ **但有一个更麻烦的东西**：同注释块 `:169-177` 明写，落地时主线上
`theta_v0_classifier_parity` 有 **2 条断言为红**（教义分歧，已立 #811），
并明令「**不许修绿、不许加 `#[ignore]`、不许把它踢出本命令**」。
⟹ **`rust-check` job 现在是（有意的）红。**
⟹ **「CI 绿」不能作为本票任何一步的验收信号**，只能用「本票新增的那条测试单独 pass」＋「红的仍只有那 2 条」。
这条必须写进每一步的验收，否则会撞上「测试锁挂在一个恒红的 job 上 ≈ 无锁」。

另一条 CI 事实：`test` job 用 `pip install ./rust`（`:97`）＝ **default features**，
随后 `pytest -m "not slow"`（`:101`）。
⟹ **Python 侧测试是本票最可靠的锁**——只要新出口在 default features 下可编译，`tests/test_*.py` 就真被执行。

### 4.1 一个必须先裁的构建期问题

出口 ② 依赖的 `OverlayState`/`LevelLedgerMirror`/`pi_theta_step_traced` **都在默认构建里**
（`strategy` 模块无 feature 门控，`rust/src/theta_v0/mod.rs:75` `pub mod strategy;`）。
但驱动它们的**生产 π fill loop** 在 `backtest/`，门控是
`#[cfg(any(test, feature = "backtest_bin"))]`（`mod.rs:110-111` 逐行确认），
而 `backtest_bin = ["nautilus", "dep:sha2"]`（`Cargo.toml [features]`）会拉 5 个 `nautilus-*` crate。

⟹ **三个方案，本文推荐 C**：

- **A. 把 `backtest` 放进 default** —— 要连 `nautilus` 一起拉进 cdylib（`backtest_bin` 依赖 `nautilus`）。
  `pip install ./rust` 编译量从 28 个 crate 涨到含 nautilus 全家。**否决**（CI 预算 ＋ #297 注释明记 nautilus wheel 182.5MB 的教训）。
- **B. 新 feature `theta_pyo3 = ["dep:sha2"]`，把 `backtest` 门控改成
  `any(test, feature="backtest_bin", feature="theta_pyo3")`，并放进 `default`** ——
  `serde_json` 已是**非 optional** 依赖（`Cargo.toml [dependencies]` 逐行确认），
  只多一个 `sha2`。但会把 48552 行非测试 `backtest/` 代码编进 cdylib。
  【推断，未验证】`backtest/` 是否真能脱离 `nautilus` 编译——本会话**未跑** `cargo build --lib --features <该 feature>` 验证（feature 还不存在）。**这是 A/B/C 里唯一有编译风险的一支。**
- **C（推荐）. 抽 `ThetaPiStream` 到 `rust/src/theta_v0/stream.rs`（无门控），
  批量 fill loop 改为调用它** ——
  即 `recursive_t` 已经用过的形态：`ffi.rs:97-98` 逐字写「共享 `TFugueStreamCore` ⟹
  逐 bar 累积的 finish 与批量逐位等价（**bit-exact 由构造保证**）」。
  好处：① 不新增 feature、不改门控；② 不把 48K 行编进 cdylib；
  ③ **bit-exact 由构造保证**，不需要事后对拍去证（对拍仍要做，作回归锁）；
  ④ 顺带把 §1.4 的「两条订单链」收成一条。
  代价：这是**真重构**，`pi_theta_fill_loop_overlay`（`fill.rs:4369`，函数体到 `:6000+`）
  的单 bar 段要连同其跨 bar 状态一起抽出来（见 §5.2 的未决项）。

### 4.2 五步 ＋ 验收（每条判据声明档位，#799 裁定十一）

> 档位定义（#799）：**结构性质 → Lean 定理**；**行为不变式 → 测试锁**；**禁令 → grep gate**。

**S1. 抽 `ThetaPiStream`（纯重构，零行为改动）**

- 产出：`rust/src/theta_v0/stream.rs`，`ThetaPiStream::{new, push_bar(bar, p_t, nav) -> StepOut}`，
  内部持 `OwnedIncrementalClassifier`（`rust/src/theta_v0/classifier/streaming.rs:39`，`append_bar` 在 `:83`）
  ＋ `prev_active` ＋ `registry` ＋ `OverlayState` ＋ `LevelLedgerMirror` ＋ TW 账本。
  `pi_theta_fill_loop_overlay` 改为「构造 `ThetaPiStream` → 逐 bar 调 `push_bar` → 装配 `FillOutput`」。
- **验收 A1**【**行为不变式 → 测试锁**】：`run_theta_v0_pi_overlay` 在固定 BTC 窗上，
  重构前后 `net_result.equity_curve` / `n_orders` / `trade_pnls` **逐位相同**。
  落成 `rust/tests/` 下一个新集成测试，读**已 commit 的**期望值 fixture（不读 314MB 行情，见 §5.5）。
  **进 CI**：`cargo test --all-targets`（`ci.yml:184`）自动覆盖 `rust/tests/*`。
- **验收 A2**【**禁令 → grep gate**】：`! grep -rn "pi_theta_step_traced" rust/src/theta_v0/backtest/fill.rs`
  ——重构后 fill loop 不许再直调 traced 步（唯一调用点收敛到 `stream.rs`），防止两条链复活。
  加进 `ci.yml` 现有的 grep gate 段（`:36-37` 已有一条同形态的先例）。

**S2. 建 `theta_v0::ffi`，只出出口 ①**

- 产出：`rust/src/theta_v0/ffi.rs` 的 `PyThetaStream`（`push_bar` / `target_net_units` / `last_order` / `snapshot`），
  `lib.rs` 的 `#[pymodule]`（`:2682`）里 `m.add_class::<theta_v0::ffi::PyThetaStream>()?;`。
- **验收 B1**【**行为不变式 → 测试锁（Python）**】：新增 `tests/test_theta_pyo3_bridge.py`：
  构造一段合成 bar，逐 bar `push_bar(..., p_t=上一步返回值, nav=1e6)`，
  断言 ① 返回值是 `float`；② `abs(p_star) <= 名义上限`；③ `last_order()[0] ∈ 七构造子集合`；
  ④ **`p_t` 相同、bar 相同 ⟹ `p_star` 相同**（确定性）。
  **进 CI**：`test` job 的 `pytest -m "not slow"`（`ci.yml:101`）自动收；
  且该 job 的 `pip install ./rust` 走 **default features**（`:97`）⟹ 出口必须默认可编译，方案 C 满足。
- **验收 B2**【**结构性质 → Lean 定理（已有，不新证）**】：`Schedule_Θ 是全函数`（∀x ∃! O，spec §16 假设12）
  已由 `coverage/sizing.rs:508-522` 的 doc 锚到 Lean。本票**不新增 Lean 义务**——
  只要 S1 保住 bit-exact，该定理的有效域不动。**如实登记为「继承，非本票新证」。**

**S3. 出出口 ②（per-leg 账本）**

- 产出：`sep_legs()` / `finish_full()`（§2.2 的键集）。
- **验收 C1**【**行为不变式 → 测试锁**】：`finish_full()["reconcile_residual"] < 1e-6`
  （PDF §11 对账恒等 `Σ_v σ_v q_v ΔP = N ΔP`），Python 侧断言。
- **验收 C2**【**行为不变式 → 测试锁**】：`lee_net_witness()[3] is True`
  （`identity_witnessed`：`Σ_ℓ net_ℓ ≡ N` 残差恒 0 且曾见非零净敞口，`level_ledger.rs:112`）。
- **验收 C3**【**行为不变式 → 测试锁**】：`sum(σ_v · q_v for v in active_voices) == snapshot()[1] 取整`
  ——per-leg 与目标净敞口的加性一致（防出口 ①②各说各话）。
  ⚠️ 该断言的成立条件是「`p_star` 未被 `𝒦_Θ` cap 收窄」；cap binding 时 `p_star ≠ Σσ_v q_v`。
  ⟹ **判据要写成「cap 未 binding 时相等」**，并同时断言 binding 计数可读。
  【设计已确认此坑：`sizing.rs:481` 在 `pi_theta_position` 内调 `record_cap_binding_attribution`（定义 `:110`），
  读出口 `coverage::cap_binding_attribution_snapshot`（`coverage/mod.rs:91-92` 的 `pub use`）已是 `pub`】

**S4. 迁 #1/#2（`rec_t_strategy.py` → `theta_strategy.py`），做 D3 对拍**

- 产出：`trading_system/strategy/theta_rec_strategy.py`（新，不覆盖旧文件）＋ `backtest/theta_backtest_py.py`。
- **验收 D1**【**行为不变式 → 测试锁**】：D3 要求「一组对拍留档，**不要求 bit-exact**」。
  落成一份 `.chanlun/review-results/` 报告，含两引擎在同一窗上的
  （bar 数、下单次数、终值、多/空/平 bar 占比）四元组并列。
  ⚠️ **这条进不了 CI**（要 314MB 行情，见 §5.5）⟹ **按 #799「只编译不执行按无锁计」的精神，
  它是「报告级留档」不是「锁」，本文如实标注，不冒充测试锁。**
- **验收 D2**【**禁令 → grep gate**】：`! grep -rn "RecTStream" trading_system/`
  ——迁完后生产壳目录里不许再出现旧引擎（研究 `analysis/` 不在此 gate 覆盖内，因为它们是冻结件）。

**S5. 杀旧（#13 删 `run_t_fugue`）＋ 冻结声明落盘**

- 产出：`lib.rs:2698` 删一行 ＋ `ffi.rs` 对应 `#[pyfunction]` 删除；9 个脚本加 `# FROZEN(#951)` 头。
- **验收 E1**【**禁令 → grep gate**】：`! grep -rn "run_t_fugue" rust/src/lib.rs`。
- **验收 E2**【**禁令 → grep gate**】：9 个冻结脚本每个都有 `FROZEN(#951)` 头——
  写成一条脚本化检查（列表硬编码，缺一即红），进 `ci.yml` 的 grep gate 段。
- ⚠️ **D2 的「逐个有归宿」需要人显式表态**——#808 的「没人要了」全是【推断】，
  仓内无任何文档宣告这些脚本退役。**S5 的冻结声明就是那个显式表态，不代答。**

### 4.3 三条判死条件的收口状态（本设计走完后）

| | 条件 | 走完 S1-S5 后 |
|---|---|---|
| D1 | 继任者出口暴露目标净敞口 ＋ per-leg 账本 | **2/2**（S2 ＋ S3） |
| D2 | 九个消费者逐个有归宿 | **13/13 判完**（§3.2），其中 9 个的归宿是「冻结 ＋ 显式声明」（S5） |
| D3 | 一组对拍留档，不要求 bit-exact | S4 的 D1 验收 |

⟹ **本票走完，`recursive_t` 的三条判死条件全满足，可动刀。**
但注意：#808 裁定二「建新与杀旧不得分票分人」——S5 的杀旧只杀 `run_t_fugue`（唯一「没人要」的），
**`RecTStream`/`TFugueStream`/`run_recursive_t` 三个导出留着**（§3.3 已述理由：冻结件要能复现自己）。
⟹ **本票不是「删掉 `recursive_t`」，是「让它变成一个没有生产消费者、只服务历史复现的冻结件」。**
若 #808 的本意是整模块删除，**本设计与之冲突，须由人裁**（见 §5.6）。

---

## 5. 风险与未决（设计不出来的地方，照实）

### 5.1 `ElementId` 的 Python 投影——**设计不出来**

卡在：本会话**未打开** `ElementId` 的定义（在 `rust/src/theta_v0/strategy/coverage/element.rs`）。
它是 `SepLeg.id` / `VoiceBook.id` / `ClosedVoice.id` 的类型，也是 `LevelLedgerMirror` 的分桶键（`id.level`）。
不知道它的字段构成 ⟹ 不能定 `finish_full()` 里那几行的元组形状。
**实施第一步必须先打开它。** 本文不猜。

### 5.2 `ThetaPiStream` 要携带的跨 bar 状态清单——**只列出了一半**

`pi_theta_fill_loop_overlay` 的单 bar 段（`fill.rs:5142` 的调用点周边）实测要喂 14 个参数：
`step_work` / `step_gamma_trade` / `prev_active` / `p_t` / `exec_index` / `base_units` /
`config.risk` / `weights` / `gate` / `stop_risk_seeds` / `config.voice` / `registry` / `Some(&twc)` / `protocol_events`。
其中 `gate`（`KThetaRiskGate`）、`stop_risk_seeds`（admission 命中的逐腿 stop）、
`twc`（`TwStepCtx`，含 TW 账本态 ＋ `entry_v` 在飞映射 ＋ `eta_correction`）、
`protocol_events`、以及调用点上方的 `oscillation_campaign` 段（`fill.rs:5130-5139`）
**都是跨 bar 演化的状态，且它们的更新顺序散在 6575 行的 fill loop 里**。

⟹ **「哪些状态归 `ThetaPiStream`、哪些留在 fill loop、更新顺序怎么保」——本文给不出完整清单。**
卡在：需要通读 `fill.rs` 4369-6000 的循环体才能穷举，本会话没做（工作量 ≈ 一整程）。
**这是 S1 的真实成本，不要按「抽个函数」估。**

**缓解**：S1 的验收 A1（逐位相同）是**构造性**的——抽错了立刻红。
所以这个未决项不会静默通过，只会拖长 S1。

### 5.3 HEDGING 账户下的穿零——**不设计**

§1.2 的「穿零 = 单张跨零单」只在 NETTING 下成立。
`theta_v0` 的 `OverlayState` 是 **hedge-mode 逐声部簿**（`overlay_state.rs:97` 自称），
但它产的 `OverlayStep.order` 仍是 **ΔN 净额单**（`:124-127`）。
⟹ **theta_v0 自己就没做 HEDGING 真实执行**（`VoiceExecBook` 那条 `VOICE_EXEC=1` 支路是研究臂，`runner.rs:476` 一带的 doc 明写「env 未设 ⟹ 净额执行 …… bit-exact 回归锁」）。
本设计**不引入 HEDGING**，如实标为有效域边界。

### 5.4 拒单无限重试——**照抄现状，不假装补上**

`rec_t_strategy.py` 只有 `min_rebalance_units`（`:31`）挡微小 churn，无拒单退避。
新 `theta_strategy.py` 若照抄，同样会在被 venue 持续拒单时每 bar 重发。
**不在本票范围**（那是执行层的事，且本仓 `enable_orders` 至今硬默认 False、live 从未真跑过——
#949 报告 §6 已实测）。**如实登记为已知缺口，不设计。**

### 5.5 对拍数据不在仓里

D3 对拍要 `analysis/data_cache/btc_1m_full.json`（314MB，gitignore 内）。
#949 报告已记：该文件在主仓 worktree 有、在 `git worktree add` 出来的树里没有（gitignore 文件不随 worktree 复制）。
⟹ **S4 的对拍只能在主仓跑，进不了 CI。** 本文已在 §4.2 把它降级为「报告级留档」而非「锁」。

### 5.6 与 #808「杀旧」本意可能冲突——**须由人裁**

§4.3 已述：本设计的 S5 只删 `run_t_fugue` 一个导出，其余三个 PyO3 面留着服务冻结件复现。
理由是 #808 自己的净发现（「冻结件的唯一剩余价值是复现它自己的历史结果」）。
但 #808 裁定二说的是「**执刀**」，字面上可能指整模块删除。
**这两个读法差别很大（留 17409 行 vs 删 17409 行），本文不代裁。**
建议：按 #799 裁定六「先划管辖」——`recursive_t` 的生产管辖面 = 0，
既然它不在生产判定路径上，**留着不产生「同一判断两档口径」的风险**，
⟹ 倾向「冻结不删」。但这是**推荐，不是裁定**。

### 5.7 CI 现在是有意的红

`ci.yml:169-177` 明记 `theta_v0_classifier_parity` 有 2 条断言红（#811，教义分歧），且不许修绿。
⟹ 本票任何一步都**不能用「CI 绿」当验收信号**。
§4.2 的每条测试锁都要单独指名跑（`cargo test --test <name>` / `pytest tests/test_theta_pyo3_bridge.py`），
并在报告里写明「红的仍只有那 2 条」。
⚠️ **这本身是一个隐患**：一个长期红的 job，新增的红会被淹没。
本文如实登记，不在本票解决（属 #811）。

---

## 附：本文所有承重断言的检索式与覆盖目录

| 断言 | 检索式 | 覆盖目录 |
|---|---|---|
| `theta_v0` 零 PyO3 暴露 | `grep -rn "pyclass\|#\[pyfunction\]\|pymodule" rust/src/theta_v0/` | `rust/src/theta_v0/` 全子树（0 命中） |
| `sep_legs` 唯一生产者 | `grep -rn "sep_legs" --include="*.rs" rust/src/theta_v0/` | 同上；排除 `*_tests*.rs` 后剩 4 文件（compose 产 / fill、overlay_state、level_ledger 消费） |
| B2 九字段 | 逐字段 `grep -rn "<字段>" --include="*.rs" rust/src/theta_v0/` | 同上；8 字段 0 命中，`level_centers` 17 命中（同名异物） |
| B3 两自变量 | `grep -rn "Perfection\|parse_mode\|A0Source\|parse_a0" --include="*.rs" rust/src/theta_v0/` | 同上（0 命中） |
| Python 侧 FFI 直调点 | `grep -rn "RecTStream\|TFugueStream\|run_t_fugue\|run_recursive_t" --include="*.py" --include="*.ipynb" .` | 仓根全目录，排除 `./rust/`、`node_modules` |
| `RecTStrategy` 消费面 | `grep -rln "RecTStrategy" trading_system/` | `trading_system/` 全目录（2 命中，`live/` 零） |
| 24 个缓存 | `find . -name "t_fugue_*.json" -not -path "./.git/*"` | 仓根（24 个，全在 `trading_system/data_cache/`） |
| CI 跑 cargo test | `Read .github/workflows/ci.yml` `:1-195` | 单文件全读 |
