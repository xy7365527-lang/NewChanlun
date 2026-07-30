# #803 三阶段是单例还是自相似——`aa45a76d1f` 复制的是流程还是钱包

- 日期：2026-07-30
- 票据：[#803](https://github.com/xy7365527-lang/NewChanlun/issues/803)（parent #787，blocking #799）
- 性质：**只查不改**。零代码改动，未实装、未重构、未给实现方案。
- 分支：`probe/i803-three-stage-singleton`（自 `docs/grill-with-docs-entry` 切；`origin/main` 是另一条谱系，不含 `rust/`）
- 标注约定：**【事实】**＝代码/commit/文档可直接核对；**【推断】**＝由事实推出但无直接留痕；**【未能判定】**＝证据不足。

---

## 0. 一页速查

| 问 | 结论 |
|---|---|
| 1 `aa45a76d1f` 复制了什么 | **【事实】复制的是流程，不是钱包。** 复制对象 = 一个函数 `act(k, want)` 的调用（每级别逐字相同）＋ 每级别一个 5 字段仓位槽 `Layer{ladder,direction,units,basis,entry_bar}`。**现金池 `free` 是单例**（源码自述「统一现金池」）。**且该 commit 时点仓库里根本还没有三阶段**——三阶段两天后才落地 |
| 2 `sink=0` 机制链 | **【事实】与资金分配零关系。** 链条全在信号/确认拓扑：`reconcile_chain` 从 root 下钻 → core 无子 → 只能 `sink@core` 建子 → `sink@core` 需 core 级别 candidate 确认 → C1（核心持多骑牛永不见顶）⟹ `candidate@core=0` → `break` → 永不建子。`sink()` 函数体内**没有任何现金/预算门**，配额取自父仓位股数 `quota(u_p)=u_p/3` |
| 3 当前形态 | **【事实】已经是分开的。** 资金池 `free` 单例在 `TRoot:945`；三阶段五字段（`stage`/`notional_in`/`core_cost_basis`/`withdrawn`/`earning_cash`）也在 `TRoot:949-954`；`TInstance`（`:701-715`）只有 `{node,level,direction,units,basis,anchor}`——**无现金、无本金、无退本金份额**。三阶段与资金池**都是单例**，且两者在 `TRoot` 内通过 `try_withdraw_capital`（`free→withdrawn`）耦合 |
| 4 拆开后 `sink=0` 还会不会发生 | **【事实+推断】不会因资金原因发生，但也不因拆开而消失。** `sink=0` 的四步因果链一步都不涉及资金；拆开资金池不触碰其中任何一步。**且拆开这件事已经部分做过了**——当前 `leg_pairs` 路径已有「单一 `free` 池 ＋ 每级复用的开平腿逻辑 ＋ 显式分配规则」（`geom_tower_quota:1975` / `uniform_base_units:1993`），该路径下每级别独立开腿、不依赖父级下沉 |
| 5 #800「本质单例」是否订正 | **部分订正（术语层），结论层维持。** 见 §5 |

**一句话直答第 1 问**：**复制的是流程（`act` 函数 ＋ 每级一个仓位槽），不是钱包。现金池从第一天起就是单例，而且「每实例独立资金池」这个选项在 2026-06-20 被编排者显式否决过（`docs/recursive_t_architecture_v2.md:432`「已结算——不走每实例独立池」），从未落码。**

---

## 1. Q1：`aa45a76d1f` 复制的对象逐项列出

### 1.1 时间序（决定性事实）

**【事实】** commit 时刻表（`git log -1 --date=format:"%Y-%m-%d %H:%M"`）：

| commit | 时刻 | 事件 |
|---|---|---|
| `aa45a76d1f` | **2026-06-18 23:33** | 「每级别独立自我复制（删 root 方向）」= 本票追问的那次 |
| `5a0ad774f8` | **2026-06-20 14:16** | 三阶段会计（降成本/退本金/增股数）**首次落地**，且落在 flat `t_engine.rs` |
| `782be90dca` | 2026-06-20 18:30 | 架构 v2 设计文档 |
| `c6d7e70500` | 2026-06-20 18:38 | `rec_engine` 核心落地，**per-instance 三阶段**在此出现 |
| `63b9ae8b2a` | 2026-06-20 19:04 | `rec_stream` ＋ BTC L3（否定性结果，`sink=0`） |
| `75ecfc9ee0` | 2026-06-20 22:05 | 编排者裁决：三阶段 per-instance → 上移 TRoot（**docs-only，未改代码**） |
| `953d8dfdb3` | 2026-06-21 14:15 | 「flat 逻辑递归化完成」——per-instance 三阶段**实际删除**在此 |

**⟹ `aa45a76d1f` 时点，仓库里不存在三阶段。** 铁证：`git grep -l "earning_cash\|core_cost_basis\|withdrawn" aa45a76d1f -- rust/src` **零命中**；`git grep -n "enum.*Stage\|stage:" aa45a76d1f -- rust/src` **零命中**。所以 `aa45a76d1f` 在逻辑上**不可能**复制三阶段，也不可能连同三阶段复制资金池。

### 1.2 它复制的到底是哪几样（逐项）

**【事实】** 该 commit 新建 `rust/src/recursive_t/t_engine.rs`（455 行）。被「每级别复制」的对象**只有两样**：

**(1) 一个函数的调用——纯流程逻辑，无状态。** `t_engine.rs` `fn act(&mut self, k, want, bar, c)`，源码注释逐字：「**一个级别的操作**（每级别逐字相同——T 操作层自我复制的代码体现）」。函数体两步：① 平本级别反向仓得下沉量 `m_down`；② `if k > BASE_LADDER { self.direct_to(k-1, want, m_down, bar, c) }`。调用点是 `step()` 里的 `for k in (0..MAX_LADDER).rev() { if view.sell[k] {act(k,Short,..)} if view.buy[k] {act(k,Long,..)} }`——**同一函数体、无 `match k`**。

**(2) 每级别一个 `Layer` 槽——仓位槽，不含钱。** `layers: Vec<Layer>`。`Layer` 定义在 `rust/src/fugue_v3/layer.rs`，**五字段**：`ladder: usize` / `direction: Polarity` / `units: f64` / `basis: f64` / `entry_bar: i64`。**没有 cash、没有 principal、没有 withdrawn、没有 cost_basis、没有 stage。** `units` 是股数、`basis` 是加权入场价——两者都是仓位属性，不是钱包。

**(3) 明确**没有**被复制的：现金。** `TPositionEngine` 字段表（`t_engine.rs`，`aa45a76d1f` 版）：

```rust
pub struct TPositionEngine {
    res: FugueResult,
    layers: Vec<Layer>,   // 每级别一个仓位槽
    free: f64,            // ← 单数。文档注释逐字：「统一现金池。」
    n_base: f64,          // ← 单数。全局 Σ|units| 会计锚
    last_close: f64,
    max_concurrent_seen: usize,
}
```

commit message 同段逐字：「**统一现金池 `free` + 每级别独立 [`Layer`]**（复用 fugue_v3 会计原语，DRY）」。模块头文档同样逐字：「统一现金池 `free` + 每级别独立 `Layer`」。

**(4) 唯一与「钱」相关的级别差异，不是钱包而是入场闸门。** `direct_to()` 里开仓量的三分支：

```rust
let m = if m_down > 1e-12 { m_down }                       // 守恒下沉（父级平仓量接力）
        else if self.global_flat() && self.free > 0.0 { self.free / c }  // 全局空仓才用 free 全压
        else { 0.0 };                                       // 否则不凭空加仓（杠杆防护）
```

**【事实】** 这是**一个共享池的支取规则**，不是每级一份钱：`free` 只有一份，`global_flat()` 判的是全局 `n_base≈0`。**【推断】** 这条规则确实构成一种「底层拿不到独立资金」的形态——非最高级别拿不到 `free`，只能靠父级 `m_down` 接力——但它的机制是**共享池的支取顺序**，与「每级配一个钱包」是相反的病：不是钱被切碎，是钱只有一个入口。这一点是本票反假说里**唯一沾边**的证据，但它与 `sink=0` 无因果关系（见 §2，`sink=0` 发生在另一条代码路径 `rec_engine`/`rec_driver` 上，且那条路径的断点在 candidate 门）。

### 1.3 那「含资金字段的结构被 per-level 复制」这件事到底发生过没有

**【事实】发生过一次，但不在 `aa45a76d1f`，而在 `c6d7e70500`（2026-06-20 18:38，晚 43 小时），而且复制的仍不是物理现金。**

`c6d7e70500` 的 `TInstance` 有 13 字段，其中 5 个是三阶段字段：

```rust
pub cost_basis: f64,   // 三阶段有效持仓成本（可<0；穿0→退本金）
pub phase: RecStage,
pub notional_in: f64,  // 本 campaign 投入本金 K_k
pub withdrawn: f64,    // 本实例已退本金份额（物理在 root.withdrawn_total）
pub earning: f64,      // 本实例③阶段弹药份额（物理在 root.free）
```

**注意注释里的两处「物理在 root.*」——这是决定性的。** 同 commit 的 `TRoot` 字段：`free: f64`（注释逐字「**单一现金池**（fungible 交换媒介，§8.7）」）＋ `withdrawn_total: f64`（「全树安全池」）。也就是说：**per-instance 的 `withdrawn`/`earning` 是归属份额（会计标签），物理钱仍在根的两个单例池里。**

**【事实】** 而且「每实例独立池」这个选项**被显式否决过、从未落码**。`docs/recursive_t_architecture_v2.md:432`（§8.7，编排者 2026-06-20 裁决，「已结算」）逐字：

> **一个根账本 `free`（现金是级别间交换媒介，fungible，非级别专属资源），每个 T 实例独立追踪 units/cost_basis/phase（仓位隔离）。** …… **已结算——不走每实例独立池（那会破坏现金 fungibility 与逐 bar TW 守恒）。**

同文档 `:591` 边界条件 (a) 复述：「TW 中性仅在同 bar 同价 c ＋ **单根账本**下成立——异步/**每实例池即破**」。

**⟹ Q1 定论（事实层）**：`aa45a76d1f` 复制的是**流程逻辑 ＋ 仓位状态槽**；资金池自始至终是单例；「每级一个钱包」这个形态**在本仓从未存在过**，它在落码前 22 小时就被裁掉了，理由是会破坏 TW 逐 bar 守恒。

---

## 2. Q2：`sink=0` 的机制链

### 2.1 链条（四步，全部在信号/确认侧）

**【事实】** 依据 `docs/recursive_t_deadlock_escalation.md` §2 / §8 / §9，以及 `63b9ae8b2a` 的 `rec_driver.rs`：

1. **`reconcile_chain` 从 root 下钻**（`rec_driver.rs:99-141`）。循环体：(a) 已有子 → 判 keep/recover；(b) 无子 ∧ 视图有新回调 → `sink`；(c) 有子则下降一层，**`None => break`**。
2. **初始链上只有 root**（core），无子 ⟹ 唯一能建子的路径是 `sink@core`。
3. **`sink@core` 需 core 级别的确认信号**（candidate@core，方案 D 下再加次级别 type1 级联）。
4. **C1（核心持多骑牛永不翻空）⟹ core = 最高涌现 Up 走势 ⟹ `candidate@core = 0`。** 于是 (b) 不触发、(c) `break`，**永远建不出第一个子** ⟹ 永远下钻不到 candidate 频繁的低级别（L0/L1/L2）。

**铁证读数**（同文档 §8:74，BTC/CL/DX 三标的）：

```
BTC:  cand_sell=[256, 486, 390,  0,  0, 0]   t1sell=[1692, 792, 271, 142, 30, 0]
CL:   cand_sell=[392, 302, 726,  0,  0, 0]   t1sell=[2007, 994, 322,  50,  0, 0]
DX:   cand_sell=[158, 177,  60,  0,  0, 0]   t1sell=[ 861, 398, 148,  25,  0, 0]
            L0    L1    L2   L3  L4 L5
```

`candidate` 在高级别（L3/L4/L5）恒 0，在低级别（256~726）频繁——**断点在高级别，货源在断点下方**。

**穷尽验证**（§9:103）：4 角度（candidate 重定义 / 级联重定义 / core 级别管理 / 反证）**全部 `infeasible`，`has_rec_internal_solution=false`**。原文结论逐字：

> **不是 candidate 定义问题（AND/MACD/Structural 都不解），是 C1（core 持多骑牛升最高涌现永不见顶）与「sink 必须 core 级别确认」的内在矛盾。**

### 2.2 资金侧的负证据（本票要点）

**【事实】三条独立证据，`sink=0` 与资金分配无关**：

1. **`sink()` 函数体内零现金门。** `c6d7e70500` 的 `TRoot::sink(parent_slot, callback_node, c, bar)` 的全部前置条件是：`c > 0` ∧ 父实例 `is_active()` ∧ **回调 node 方向必反父向** ∧ `m = quota(u_p) = u_p/3` 有限且 `≤ u_p`。**没有一条读 `free`、读预算、读级别配额。** 配额分母是**父级持仓股数**，不是钱。
2. **`sink` 的资金语义是守恒转移，不是新拨款。** 父级 `rec_reduce` 出 `m` 股 → 现金进 `free` → 子级 `rec_add` 同股数 `m` 反向开 → 现金出 `free`。同 bar 同价 `c` ⟹ TW 中性（`prove_tw_neutral(tw_pre, c)` 每次 `sink` 尾部断言）。单测 `sink_父持多_回调node上spawn子T持空_同股数` 逐字验收「同股数：子 T 短 = 父减出的 1/3」＋「sink TW 中性」。**即使给底层配一个独立钱包，也不改变 `sink` 一次都没被调用这个事实。**
3. **`reconcile_chain` 的门是 `view.nodes`**（信号视图链），不是账本。`let want = view.nodes.get(depth + 1).copied();` —— 判 `!w.completed && dir_to_polarity(w.node.direction) == flip_pol(cur_dir)`。全是结构谓词。

**⟹ Q2 定论**：`sink=0` 是**确认拓扑死锁**（连续下钻 ∧ 必须穿过高级别 candidate 断点 ∧ C1 保证该断点恒关），**不是资金分配不到**。编排者 2026-06-21 第五方案（§10:123「对照 flat 引擎把它的逻辑递归化，**不要自创约束**」）删掉 C1/candidate gate/连续 level=父−1 后，`sink` 从 0 升到千次级、`short_pnl` 非 0，且与 flat bit-exact——**这次修复一分钱都没动**，进一步反证资金无关。

---

## 3. Q3：当前回退后的形态——资金池与三阶段是分开的还是缠在一起的

**【事实】结构上完全分开，语义上都是单例，通过两个函数耦合。**

| 对象 | 落点 | 字段 |
|---|---|---|
| 资金池 | `rec_engine.rs:945` `TRoot.free` | 单例，注释「单一共享现金池（NAV = free + Σ sign(d)·u·c）」 |
| 三阶段 | `rec_engine.rs:949-956` `TRoot` | `stage: RecStage` / `notional_in` / `core_cost_basis` / `campaign_entry_cost` / `withdrawn` / `earning_cash`——**全部单数标量**，注释头「── 持仓三阶段（单 campaign，= flat）──」 |
| 每级仓位 | `rec_engine.rs:701-715` `TInstance` | `{node, level, direction, units, basis, anchor}` ——**六字段，零资金字段** |
| 每级腿（读法B） | `rec_engine.rs:739-746` `Leg` / `LegPair` | `{level, direction, units, basis, riding_node, active}` ——**同样零资金字段** |

**耦合点（两处，都在 `TRoot` 内部）**：

- `try_withdraw_capital()`（`rec_engine.rs:1415-1430`）：`self.withdrawn += w; self.free -= w;` —— 退本金阶段把钱从 `free` 移到 `withdrawn`（移出在险）。
- `account_reduce()`（`:1378-1412`）：按被减腿方向分派——`Long` 走三阶段状态机（降成本 → `core_cost_basis ≤ 0` 且 `enable_three_stage` ⟹ 转 `CapitalRecovered` → 退完转 `EarningShares` → `earning_cash` 累加，且 `.min(self.free.max(0.0))` 钳在池内）；`Short` 只累加 `short_leg_pnl`。

**【事实】** `rec_engine.rs:41` 的留痕逐字（本票锚点，实际在文件头 §「与 flat 的映射」块）：

> flat 单核心 campaign 三阶段（`stage`/`core_cost_basis`/`withdrawn`/`earning_cash`）→ 在 `TRoot`（**非 per-instance，per-instance 三阶段是递归自创，已删**）。

**【推断】** 所以当前形态的准确描述**不是**「资金池和三阶段缠在一起」，而是「**三阶段本身就是资金池的状态机**」——三阶段的三个状态刻画的是同一笔钱在同一个 campaign 中的处境（成本是否穿 0、本金是否已移出在险、盈余现金是否在买回股数）。它们分不开，不是因为耦合没解，而是因为二者描述同一对象。

**一处必须照实标注的对照事实**：**【事实】** 每级别**确实**已经有一套「独立仓位生命周期」，只是它不叫三阶段——`leg_pairs: Vec<LegPair>`（`:1026`）每级别一对多空腿，各自 `units`/`basis`/否定线止损/`pair_long_pnl[k]`/`pair_short_pnl[k]`。**开平腿是每级复用的同一段代码，取仓量走显式分配规则**（`leg_open_units(k, top, c)`，`:2002`，二选一：`uniform_base_units` = `initial_capital × 1/3 / c`，级别无关；或 `geom_tower_quota` = `free × 2/3 × (1/3)^(top−k) / c`，按深度衰减）。**这正是本票 Q4 假设的那个形态，且它已经在仓里了。**

---

## 4. Q4：若拆开（单例资金池 ＋ 每级复用三阶段 ＋ 显式分配规则），`sink=0` 还会发生吗

**【事实】不会因资金原因发生。** `sink=0` 的四步链（§2.1）逐条对照：

| `sink=0` 因果步 | 拆开资金池是否触碰 |
|---|---|
| ① reconcile 从 root 下钻、无子只能 sink@core | ❌ 不触碰（是驱动器遍历策略） |
| ② sink@core 需 core 级别 candidate 确认 | ❌ 不触碰（是信号门） |
| ③ C1 ⟹ candidate@core 恒 0 | ❌ 不触碰（是自创约束，已删） |
| ④ 断点在 L3/L4，货源在 L0-L2，连续架构不能跳级 | ❌ 不触碰（是级别拓扑） |

**【事实】而且这个形态已经被部分实测过**：当前 `leg_pairs` 路径（`enable_reading_b_pair`，任务57=53.1）就是「单一 `free` ＋ 每级复用开平腿逻辑 ＋ 显式分配规则 `leg_open_units`」，该路径下每级别独立开腿、**不依赖父级下沉**，因此结构上不存在「底层攒不够钱」的失效模式——它的失效模式是别的（做空腿捕获率 34-43%、T1 方向错位 58.7%，见 `ShortEntry`/`LongEntry` 两个 enum 的文档块）。

**【推断】三条需要下票（#799/spec）承接的判断**：

1. **不要指望「拆资金池」能解任何东西**——它已经是拆开的，`sink=0` 的病根在信号确认拓扑，不在会计。
2. **「每级复用三阶段」这个提议要先答一个前置问题：per-level 的三阶段状态机，各自的「本金 `notional_in`」是什么。** 当前 `notional_in` 是单例（一次 campaign 投入的本金 K）。若每级一份，则必须有一条把总本金切给各级的规则——**而这条规则一旦写死为「每级一份预算」，就正好复现本票反假说担心的那个病**（钱被切碎），且会破坏 §8.7 已结算的现金 fungibility 与 TW 逐 bar 守恒。**【未能判定】** 是否存在既 per-level 又保 TW 守恒的三阶段形式化——本票范围内无证据可判；要判需要先给出 per-level `notional_in` 的定义并证 `TW = free + Σ持仓 + Σwithdrawn_k` 逐 bar 中性。
3. **原文侧有一条反向证据必须一并上桌**（见 §5）。

---

## 5. Q5：#800「本质单例」结论——**部分订正（术语层），结论层维持**

### 5.1 维持的部分

**【事实】** #800 那句话的字面是：「**仓位三阶段战役（降成本/退本金/增股数）本质是单例的**，per-level 复制它被裁决删除（`rec_engine.rs:41` 注释在案）」。这句话的**每一个可核验部分都成立**：

1. per-level 三阶段确实存在过（`c6d7e70500` 的 `TInstance` 5 字段）；
2. 它确实被裁决删除（`75ecfc9ee0` 裁决 → `953d8dfdb3` 落码）；
3. `rec_engine.rs` 头部注释确实写着「per-instance 三阶段是递归自创，已删」。

**而且删除理由不是工程妥协，是原文裁定。** `75ecfc9ee0` commit message 逐字给了四条原文溯源：

> 原文强力确证：
> - 第31课：24/169（**三阶段总体**＋成本穿 0 **全局门**＋成本 0 前同股数不增仓）
> - chan99/0033:11（**级别只决定量**）/:23（增股数两层）
> - 第15课：976/956（认沽立于不败＋权利金损失边界）
> - flat `t_engine.rs:105` `TStage` **已是 GLOBAL = 原始设计**，rec_engine per-instance 是**偏离**

「级别只决定量」这一条，正是对本票反假说的**正面反驳**：级别在原文里决定的是仓位规模，不是每级各有一套本金账。

### 5.2 需要订正的部分（术语层，两处）

**订正一：「本质单例」这个说法把两件事压成了一件，编排者的拆分是对的。** 精确表述应是：

| 对象 | 性质 | 依据 |
|---|---|---|
| **资金池 `free` / 本金 `notional_in` / 退本金 `withdrawn`** | **单例**（fungible，非级别专属资源） | `architecture_v2.md:432` §8.7 已结算 |
| **三阶段状态机的代码** | **一份，被复用**——`account_reduce` 是一个函数，任何级别、任何腿的减仓 realized 都进它 | `rec_engine.rs:1378` |
| **三阶段的状态变量 `stage`** | **单例**——因为它刻画的是「那**一笔**本金此刻在哪个处境」，不是「某个仓位的生命周期阶段」 | `rec_engine.rs:949`，原文第31课「三阶段总体」 |
| **每级别的仓位生命周期** | **确实每级一份、确实自相似**——但它的载体是 `LegPair`（开腿/否定线止损/平腿/per-level pnl），**不是三阶段** | `rec_engine.rs:1026-1031` |

⟹ 编排者的反驳「账户是单例不等于三阶段是单例」在**逻辑上成立**（这是两个命题），但在**本仓的事实上不成立**：三阶段恰好也是单例，而且是独立于账户单例的另一条理由（原文第31课「三阶段总体」＋「成本穿 0 全局门」）在支撑它。**「每个仓位都要走一遍的那个生命周期」在本仓是真实存在的，但它是 `LegPair` 那一套，不是三阶段。** 把二者当成同一个东西，是这次反假说的实际偷换点。

**订正二：#800 把 `sink=0` 与「三阶段单例」列在同一段里，容易被读成因果，需切断。** 本票查实二者**无因果关系**：`sink=0` 早于三阶段上移裁决（`63b9ae8b2a` 19:04 vs `75ecfc9ee0` 22:05），且 `sink=0` 发生时 per-instance 三阶段**正在生效**——也就是说，**`sink=0` 恰恰是在「每级各有一套三阶段字段」的世界里测出来的**。这条时序本身就否证了「per-level 三阶段/钱包导致 sink=0」。#800 §6b-2 的行文未点破这一点，建议按本节切开。

### 5.3 对 #799 的净影响

**【推断】** 结论不变但理由更硬了：操作层「这一层若要重来，需要先回答的不是『能不能自相似』，而是『三阶段战役是不是本来就只有一个』」——本票把这句话的答案给到 **是，只有一个，且有原文（第31课「三阶段总体」/「成本穿 0 全局门」）与守恒律（TW 中性要求单根账本）两条独立支撑**。同时新增一条 #800 没说的：**每级复用的那套东西已经在仓里了（`LegPair` ＋ `leg_open_units` 显式分配规则），#799 要裁的不是「要不要拆」，而是「已经拆出来的这套为什么没人当它是答案」**。

---

## 6. 未能判定 / 缺什么才能判

1. **【未能判定】** 是否存在「per-level 三阶段 ∧ TW 逐 bar 守恒」的一致形式化。要判需先定义 per-level `notional_in`（总本金如何切分）并证 `TW = free + Σ持仓价值 + Σ_k withdrawn_k` 同 bar 同价中性。`architecture_v2.md:591` 边界 (a) 断言「异步/每实例池即破」，但那是对**物理独立池**说的，对**归属份额式**的 per-level 三阶段未给证明也未给反例。
2. **【未能判定】** `LegPair` 路径（`enable_reading_b_pair` ON）是否曾在与 `sink=0` 同口径的 L3 上跑过。本票只读代码与文档，未编译未回测。要判需要该 flag ON 的 L3 报告。
3. **【事实但需下票承接】** `aa45a76d1f` 的 `direct_to` 里「非全局空仓 ⟹ 底层只能靠父级 `m_down` 接力拿到仓位」这条规则，是本仓里最接近「底层拿不到资源」的形态。它没有导致 `sink=0`（不同代码路径），但它是否在 flat 支造成过别的病，本票未查。

---

## 7. 结果包六要素

1. **结论**：见 §0。核心两条——(a) **`aa45a76d1f` 复制的是流程（`act` 函数 ＋ 五字段仓位槽 `Layer`），不是钱包；现金池 `free` 自始至终单例，且该 commit 时点三阶段尚不存在**；(b) **#800「三阶段本质单例」结论维持，术语层部分订正**——单例的是「资金池／本金／`stage` 状态变量」，自相似复用的是「`account_reduce` 代码」与「`LegPair` 每级仓位生命周期」；`sink=0` 与三阶段/资金池**无因果**，它是确认拓扑死锁（C1 ∧ 必须穿高级别 candidate 断点）。
2. **定义依据**：commit 锚 `aa45a76d1f`（2026-06-18 23:33）/ `5a0ad774f8`（06-20 14:16）/ `c6d7e70500`（06-20 18:38）/ `63b9ae8b2a`（06-20 19:04）/ `75ecfc9ee0`（06-20 22:05，docs-only）/ `953d8dfdb3`（06-21 14:15）/ `4c8458f20c`（#800 报告）。代码锚 `rust/src/recursive_t/rec_engine.rs:{41(头部映射块), 653(quota), 701-715(TInstance), 739-746(Leg), 941-1031(TRoot), 1378-1412(account_reduce), 1415-1430(try_withdraw_capital), 1975(geom_tower_quota), 1993(uniform_base_units), 2002(leg_open_units)}`；`rust/src/recursive_t/t_engine.rs`@`aa45a76d1f`（`TPositionEngine` / `act` / `direct_to` / `step`）；`rust/src/fugue_v3/layer.rs`（`Layer` 五字段）；`rust/src/recursive_t/rec_driver.rs:99-141`@`63b9ae8b2a`（`reconcile_chain`）；`rust/src/recursive_t/rec_engine.rs:432-477`@`c6d7e70500`（`sink`）。文档锚 `docs/recursive_t_deadlock_escalation.md:{2节, 74, 103, 123, 134-139}`；`docs/recursive_t_architecture_v2.md:{432(§8.7), 591}`。
3. **边界条件**：只读 `docs/grill-with-docs-entry` 线（`5984ab0d8e` 起）＋ 历史 commit 快照；未编译、未回测、未跑任何 flag。§6 三条为未判定项。
4. **下游推论**：#799 的操作层裁决可直接采信「三阶段单例」，并把问题重心从「拆不拆资金池」移到「`LegPair` 这套已在仓的每级复用形态为何未被承接」；#800 报告 §6b-2 建议按本票 §5.2 切开 `sink=0` 与三阶段单例的行文邻接。
5. **谱系引用**：`project_recursive_t_architecture_v2`、`project_unified_recursive_operator_T`、`project_t_engine_btc_baseline`、`project_unified_architecture_map_787`、`project_gap3_l0_earning_unreachable`、`project_gap3_l2_unreachable_architecture`；票据 #787 / #799 / #800 / #803。
6. **影响声明**：**零代码改动**。新增本报告一份文件。未动生产代码、未重构、未给实现方案。
