# issue #838 乙侧：本仓做错了什么——跨级反向持仓的机制归因

- 票：[#838](https://github.com/xy7365527-lang/NewChanlun/issues/838)（`wayfinder:research`，父图 #787，喂 #834）
- 事实基座：[#837](https://github.com/xy7365527-lang/NewChanlun/issues/837) resolution + `.chanlun/review-results/issue837-cross-level-opposing-actions-20260731.md`
- 分支 `issue838-b`（基于 `main`），**只读，零生产改动**
- 编排者中途纠正已并入：**本仓做的不是单向市场**（BTC 永续，双向可开、无 T+1）。第 5 问按新口径重写，候选 ③ 升为主线。

---

## 一句话结论

**决策变量本身是一个标量净持仓。** `net_target_units`（`leg.rs:300`）把所有级别的腿塌缩成一个 `f64` 之后，
下游整条链——可行集 𝒦_Θ、目标函数 J_x、保证金、风控模式——**全部只认这个标量**。
「两个级别方向相反」在这套坐标里**不是一个可以为真的命题**，因此它既不能被禁止，也不能被计价，
也不需要有人为它负责。62.6%–72.0% 不是一个漏掉的检查，是**一个不存在的维度**。

唯一一条「什么时候不该反向」的条款（639(c)：未持父则剔除 `ReverseOpen` 子腿）
只作用于**同一条操作分支内部**，且对 `Ambient` 根声部完全不适用——
而 #837 最长的几例对冲恰恰**两侧都是 `Ambient` 根**。

---

## 第 1 问：反向持仓是怎么产生的——最小路径 + 「本该有检查但没有」的点名

### 1.1 最小路径（五步，全部点名到行）

**步①　候选独立生成，逐级别，互不相看。**
`interp.rs:646` `assemble_gamma_with_tower(classification, tower)` 从 `classification.levels[ℓ].bsp`
逐级别产候选。角色 R(g)=(H,V,δ,G) 的三条轴全部相对**本元素在因果塔里的结构父**定义
（`role.rs:20-64`）：
- `Horizontal`：相对**同一父容器下的前一同级别兄弟**（`role.rs:19-32`）
- `Vertical`：相对**父容器方向 σ_p**（`role.rs:35-64`）
- `GradeRel`：ℓ_g vs ℓ_{p(g)}（`role.rs:70-88`）

**角色空间里没有任何一条轴指向「别的级别当前持有什么」。** 这不是遗漏，是 spec §7-§8
的完全分类本身就只有这三条轴（`role.rs:90-110`，3×3×2=18 类 canonical）。所以候选一旦生成，
「对侧」这个概念在类型层面就已经无处安放。

**步②　`Ambient` = σ_p = 0，去根化。**
`role.rs:37-40`：

> `Ambient`：σ_{p(g)}=0（父容器无方向/胚元/空）——spec P7 显式：**Ambient 不是根规则**，
> 而是任意父容器处于无方向状态时的普通情形（去根化核心）。

顶层元素（塔顶／冷启动 `tower.len()<2`，`runner.rs:210` 边界条件②）父 = 边界胚元 ∂ ⟹
`parent_id = None` ⟹ `element_as_leg`（`held.rs:125`）置 `is_boundary_root = true`。

**步③　三桶合流，全级别同时进入唯一一个活动集推进函数。**
`step.rs:100` `coverage_step_from_buckets_sep_with_risk_seeds` 接收的 `work: ElementView`
是**整棵多级别树**，`prev_active` 是**全部级别的在场腿**。这里是全仓唯一一个
「所有级别的持仓与所有级别的新候选同时在场」的位置——也就是说，**跨级冲突在这里
是可见的**（数据都在手上），只是没有人看。

**步④　★「本该有检查但没有」的点名：`exit.rs:355-373`。**
`step.rs:490` 调用 `exit::step_active_set_with_subtree_close(&a_t_legs, &close_and_flip_seeds, &b_x_legs)`
——这是 #183 归一后**生产唯一**的活动集准入裁决。它的全部判据是（`exit.rs:355-373`）：

```rust
raw.into_iter()
    .filter(|leg| {
        let mut cur = *leg;
        let mut steps = 0usize;
        loop {
            if cur.is_boundary_root {
                return true;          // ← 就是这一行
            }
            steps += 1;
            if steps > bound {
                return false;
            }
            match cur.parent_id.and_then(|p| by_id.get(&p)) {
                Some(parent) => cur = *parent,
                None => return false,
            }
        }
    })
    .collect()
```

三件事：

1. **`ActiveLeg` 携带 `dir` 字段**（`held.rs:120` `dir: e.eps`，字段定义 `interp.rs:150` 邻近），
   而这个 filter **从头到尾没有读过 `leg.dir` 一次**，也没有读过任何**其他**腿的 dir。
   判据只有三样：`is_boundary_root`、`parent_id`、以及「该 id 是否在本集合内」。
2. **`exit.rs:360-362` 是一条对根腿的无条件放行**：任何 `is_boundary_root` 的腿，
   不论级别、不论方向、不论集合里已经有几条反向的根腿，循环第一轮就 `return true`。
   两条 `Ambient` 根（L3/Long 与 L4/Short）互相**不是对方的祖先**，各自 `is_boundary_root=true`,
   于是**两条都在第一次迭代就被放行，彼此从未被对方看见**。
3. 这正是 #837 「最长的几例两侧都是 `Ambient` 根声部」的机制解释：
   它们走的是这条**最短的准入路径**，连祖先链都不用爬。

**步⑤　塌缩成标量，冲突消失。**
`leg.rs:300` `net_target_units(legs) = Σ ε_e·s_e`（Long 计 +units，Short 计 −units）
⟹ `p̃: f64`。从这一行往下，「两条反向的腿」和「零仓」在数值上不可区分。

### 1.2 这条路径上**存在但都不管方向对撞**的其他门（否证「其实有别的地方在管」）

| 门 | 位置 | 它实际检查什么 | 为什么管不到跨级反向 |
|---|---|---|---|
| §13 AncOK（639(c)） | `exit.rs:355`；语义 doc `held.rs:151`、`runner.rs:193-195` | ReverseOpen 子腿的**操作父是否在场** | 只作用于**同一分支内部**；`Ambient` 无父方向 ⟹ 不适用 |
| I3 父 carrier 身份 | `interp.rs:2871`（测试 `position_node_identity_is_carrier_not_leaf_ordinal`） | 候选 `parent_id == par_C(carrier)` | 是**结构性祖先身份**，与持仓、与对侧无关（#837 已订正此点，#639 裁定 σ_p 与持仓正交） |
| 全互斥解释器 I_Θ（P1..P10） | `mutex.rs:5-19` | 风控/TW/close typed/open root/open reverse_open | **十条谓词里没有一条以「对侧持仓」为条件**；P8=Open Root 的条件是「slot 空」，P9=Open ReverseOpen 的条件是「父声部 active」 |
| 𝒦_Θ 风控门 | `sizing.rs:318-328` | `force_flat` / `stop_long` / `stop_short` / `no_increase_cap` | 四个字段全部作用在**标量净持仓区间** `[−lo,+hi]` 上（`sizing.rs:341-359`）；反向对已使净≈0，四个门全部不 binding |
| G7 毛头寸约束 | `leg.rs:~340` `apply_gross_cap` | 毛敞口 G=Σ\|s_e\| 超 Ḡ 时**按根子树缩放** | ① 默认关（见 §2.3）；② 即使开也只是**幅度缩放**，不是冲突拒绝；③ `leg.rs` doc 明写「跨根无 spec 比率绑定 ⟹ 逐根独立」——两条反向根各自缩放，永不交互 |
| 毛杠杆约束 `leverage_ok` | `risk.rs:552` | 同时查 L^G ≤ L̄^G 与 L^N ≤ L̄^N | **零生产调用点**（见 §2.4） |

---

## 第 2 问：哪个设计选择使它成为常态

### 2.1 候选 ①（每级独立账户 `Core{level}`，跨级只读不写）——**成立，但只是次要成因**

`account.rs:36-48` 三个账户身份：

```rust
pub enum AccountIdentity {
    Core { level: u32 },          // 本仓账（按级别分层）
    ReverseOpen { level: u32 },   // 首开反向账（每级一本，修4）
    Short,                        // 空仓账 = 无父反向声部（ambient 空根）
}
```

`Core{level}` / `ReverseOpen{level}` 都带 `level` 维度 ⟹ **每级别一套账**。
`rec_engine.rs:2156` `prove_pair_isolation` 是 `recursive_t` 侧的同族结构守卫
（「g_pair(k) 只写 leg_pairs[k]」，L0 结构 panic），调用点 `rec_engine.rs:2193/2215/2260/2310`。

**但**：#837 票面猜测「隔离本身是有意为之，问题是隔离之后没人做净额层的准入」——
**这个描述需要订正**。净额层确实**有**准入（𝒦_Θ + LexArgmin，`sizing.rs`），
问题不是「没人做」，而是**净额层的准入对象是标量**，反向对在它眼里就是 0，
所以它做了准入，只是准入的东西里不含这条信息。**根因在坐标，不在缺门。**

### 2.2 候选 ②（双向可开 / `ReverseOpen` 声部）——**成立，但不是「错误」**（见第 5 问）

`account.rs:40-47`：`ReverseOpen{level}`（父声部 active 期间的次级别反向对冲声部，父仓不动）
+ `Short`（无父反向声部，ambient 空根）。二者「互斥且完备：任一反向声部，有父即首开反向、无父即反向根」。

**这就是「双向可开」在类型层面的落点。** 但在 BTC 永续上双向本身合法（编排者纠正），
所以它是**使反向持仓可表示**的条件，不是使它**失控**的原因。失控的原因是 ③。

### 2.3 ★候选 ③（`net_target_units` 在下单前塌缩净额）——**成立，且是主因；升为主线**

`leg.rs:294-306`（doc 逐字）：

> ★分账本（毛）→ 净账本的语义降维（M29 §7 诚实声明）：分账本目标头寸 q̄_Θ 是**多空独立坐标**
> （C25，毛收益级，G_net=0 双开净额退化）；Nautilus 净额账户只持**单一净持仓**——本函数把毛腿
> 净额化（Σ ε_e·s_e）。净额化**丢失**双开的毛敞口信息（净杠杆 ≤ 毛杠杆，risk.rs `net_le_gross`）
> ——这是 Nautilus 净额兼容的必然降维，**非** bug。完整毛分账本执行须 hedging 账户（v0 净额）。

**这段自陈是本票最重的一份证据，但它的定性需要收窄**：
「非 bug」这个判断只对**执行层**成立。真正的代价发生在**它上游**——

**塌缩不止发生在执行层，决策变量本身就是标量：**

- `sizing.rs:318` `KThetaRiskGate` 的四个字段全部是标量净持仓上的约束。
- `sizing.rs:373-377` `position_bounds(cap) -> (lo, hi)`：可行集是**一维区间**。
- `sizing.rs:~410` `feasible_candidates(p_tilde, p_t, lo_cap, hi_cap, lot)`：
  𝒦_Θ 的候选点集是 `f64` 网格上的六个点。
- 目标函数 `J_x(p)` 主键 = `w·(p−p̃)²`（`sizing.rs:~437` doc），
  次键 = `λ·|p−p_t|`——**两项都是标量函数**。

⟹ **「L0 持多 + L3 持空」与「空仓」在 𝒦_Θ 和 J_x 上是同一个点。**
一个在可行集里不可区分的状态，既不能被约束排除，也不能被目标函数惩罚。

**这条解释的正是「为什么这个缺陷能长期不被发现」**——#837 反事实实测
（剔除全部 ReverseOpen 腿 Δnet ≤0.63%、符号跨窗翻转）不是巧合，
是**必然**：净额层看不见的东西，净额指标当然也测不出来。

### 2.4 ★候选 ④（本报告新增，且是 ③ 的真正后果）：**冲突不产生任何账面代价——两条反向腿在对方账上完全不可见**

这是编排者升权的那一问：**它发生了，谁在为它的代价负责？** 逐层查完，答案是**没有人**。

**(a) 保证金：只按净名义计，反向对 ⟹ MM ≈ 0。**

`admission.rs:1236-1240`：

```rust
// p_t 净 lot × mark = 净名义（美元，margin-design §2.2：net_notional 已折算勿再乘价）。
let net_notional_usd = p_t.abs() * px;
risk_mode(&margin_inputs(net_notional_usd, equity, sched, &m.cushions))
```

`p_t` 是**标量净持仓**。`risk.rs:658` `maint_margin(&self, net_notional_usd: f64)`
第一行 `let n = net_notional_usd.abs();`——毛敞口从未进入过这个函数的签名。

⟹ L0 持多 100 + L3 持空 100：`p_t ≈ 0` ⟹ `net_notional_usd ≈ 0` ⟹ `MM ≈ 0`
⟹ `risk_mode` 返回最安全档 ⟹ `global_risk_close` 不触发（`admission.rs:1250`）。
**一个 60 天、双边满仓的对冲，在保证金账上是免费的。**

**(b) 这一点还被一条测试锁死了。**
`risk.rs:2146-2148`（doc）：

> 断言 `margin_inputs`/`risk_mode` 判据输出只由 (net_notional, equity, schedule,
> cushions) 决定……**任何未来把 cost_basis 喂进强平链的改动（margin_inputs 签名加参 /
> risk_mode 读账本态）被本测试的期望值击杀。**

该测试 `a10_tn5_negative_cost_basis_never_enters_liquidation_criteria`（`risk.rs:2152`）
的直接意图是防负成本基进强平链（T-N5 / 裁定 C4），**但它顺带把 `margin_inputs` 的四参签名
冻结成了回归锁**——想把毛敞口喂进强平判据，同样会被它击杀。
（090：这是我读出的**副作用**，未见任何裁定文件明文以此为目的；**未查证**该裁定是否讨论过毛敞口。）

**(c) 毛敞口的度量在仓里，且形式链证明了必须约束它——但生产不调。**

`risk.rs:544-553`（doc 逐字）：

> ★必须同时约束（strict §11 line 359 / Lean `net_ok_not_imply_gross_ok`）：净约束**不能**替代
> 毛约束——双开 long k + short k 使 N=0（净约束平凡满足）但 G=2k（毛约束可违反）。故本函数
> 查两者合取，不是只查净（只查净会漏掉双开抬高的毛敞口风险）。

`risk.rs:552` `pub fn leverage_ok(metrics, caps) -> bool`。
**全仓 `leverage_ok` / `gross_notional` / `leverage_metrics` 的非本文件引用只有两处，且都在 doc 注释里**
（`sizing.rs:226`、`sizing.rs:228`）——**零生产调用点**。`risk.rs:565` 自陈：

> `gross_notional`/`leverage_metrics`/`leverage_ok` 保留为美元空间 **Lean 镜像**

形式链侧确有定理：`formal/Origin/LeverageCapital.lean:131` `net_le_gross`、
`:263` `net_ok_not_imply_gross_ok`；`formal/Origin/SeparateLedger` 的
`hedged_leg_nonzero_but_net_zero`（引用见 `SeparateFinalTheorem.lean:29/124/281`、
`SeparateEat.lean:25/66/254`）。

⟹ **形式链已经证明「净额约束不蕴含毛敞口约束」，生产只约束净。**
这是本票最尖锐的一条：不是没想到，是想到了、证明了、写进 Lean 了，然后没接线。

**(d) 唯一接了线的毛约束 G7，默认关。**
`config.rs:199` `pub enforce_gross_cap: bool`；`config.rs:227` `enforce_gross_cap: false`（default）；
`config.rs:456` `assert!(!c.risk.enforce_gross_cap); // frozen：毛头寸约束默认不激活（G7 约束未配置=不激活）`
——**默认关这件事本身被一条测试钉成了 frozen 契约。**
生产打开它的唯一路径是 env（`wverify_run.rs:218-220` `apply_enforce_gross_cap_from_env`）。

**(e) 账本层：不可见是被一条运行时守卫主动维护的。**
`fill.rs:398-402`（doc）+ `fill.rs:412-440`（实现）：

> ★#198 断言4：`fill.account == ReverseOpen ⟹ Δqty(Core{*})==0 且 Δqty(Short{*})==0`
> ——首开反向过账不得碰本仓/空仓任何分实例

即：**「反向腿的成交不得在核心仓账上留下任何痕迹」不是疏忽，是一条被断言保护的不变量。**
（注：该函数是 `#[cfg(debug_assertions)]`，`fill.rs:411`——release 构建里连这条断言都被编译消除。
注释里的 `#198`/`#185`/`#199` 属 2026-06 期编号，**可能是当年 kimi 任务号，未按 GitHub issue 核实**。）

**(f) 毛的逐声部账本存在，但在决策链外。**
`ledger/separate.rs`（P^sep 分账本，C25/C26/C29，contract anchor `formal/Origin/SeparateLedger.lean`）
与 `leg.rs:310` `gross_target_units` 都能算出毛。
`leg.rs:310-315` doc 明写：「用于**诊断**净额降维丢失的毛敞口」——**定位就是诊断，不是约束**。
`step.rs:28-33` 的 `sep_legs` 出口同样自陈「**纯只读，不进决策路径**」。
声部独立执行臂 `VOICE_EXEC=1`（`admission.rs:31-45`、`env_registry.rs:76`）默认关，
且 `theta_overlay.rs:78-84` 明写：开了之后**决策层仍与净额臂逐字节一致**。

**⟹ 第 2 问的答案（收敛）**：使跨级反向成为常态的设计选择，不是「隔离」也不是「双向」，
而是 **「决策变量 = 标量净持仓」**。毛的坐标、毛的度量、毛的定理全都在仓里，
但从 `leg.rs:300` 往下的整条决策—风控—保证金链**一个都不消费**。

---

## 第 3 问：H4（单向市场前提）成立时，本仓失去了哪些保护

**前置声明（编排者纠正已并入）**：本仓做的**不是**单向市场——BTC 永续，双向可开、无 T+1。
所以本节读作「原文系统靠市场机制免费获得、而我们必须自己实现却没实现的保护」，
**不是**「所以本仓应当禁止反向」。甲侧 H4 是否成立由甲侧裁，本节按「若成立」推演。

**丢失的保护（逐条点名）：**

1. **「同时持多与持空」在 A 股物理上不可表示 ⟹ 净额账本天然等于毛账本。**
   本仓 `account.rs:40-47` 用 `ReverseOpen{level}` + `Short` 两个身份把这个状态显式造了出来，
   于是**净≠毛**第一次成为可能——而下游没有任何一处消费这个差（§2.4）。

2. **「一个资金池一个操作级别」在 `theta_v0` 完全不存在。**
   全仓 `操作级别` 一词在 `rust/src/theta_v0/` 下**零命中**（grep 结果只落在
   `recursive_t/rec_engine.rs:76`、`fugue_v3/operate.rs:41`、`spiral/`、`trading/`、
   `classifier/mod.rs:3693`）。`theta_v0` 的 π loop 每 bar 对**全部级别同时**产候选、
   同时准入、同时下单。**H1「单一操作级别」这条结构性保护在本引擎里没有对应物。**

3. **★但本仓另一根支柱里有这条保护——只是没用在这里。**
   `rust/src/trading/level_operating_unit.rs`（GUARD-ROLE 标记 `现役`，`:1` #763 C7-E4 核定）
   是 38 课操盘程式 FSM，每个 voice 有一个 **home 级别**，反向段以 `RevLeg.tranches`
   （`:60` 起）挂在同一个 voice 内部，并且**有三条枚举化的「什么时候不该反向」守卫**：
   `RevOpenVerdict`（`:825-833`）——
   - `RejectedSubAnchor`（G1：次级别锚定不成立）
   - `RejectedGate`（G2：41 课门关）
   - `RejectedFrozen`（G3a：本级别冻结）

   **这是 #787「答案在仓里」的又一个标本**：一套「反向必须先过守卫」的设计已经实装、
   现役、拒因分计数，但它在 `trading/` 里，`theta_v0/` 的 π loop 不经过它。
   **090**：我**未核实**两套引擎的接线关系与 `trading/` 是否被 #837 的实测路径覆盖
   （#837 跑的是 `theta_v0` 侧 `ThetaConfig::default()`）。

4. **`Short` 账（无父反向根）本身就是「单向市场里不存在的东西」。**
   `account.rs:45-47` + `#200` 二类开空通道（`account.rs:80-87` `ActionReason::OpenShort`）。
   在无做空机制的市场里，「二类点开空」这条通道不存在，因此不需要为它配任何跨级仲裁；
   本仓造了这条通道，**未见**任何配套的跨级准入条款（090：确认没有，grep 全仓
   `naked`/`裸空`/`逆势仓` 命中的 16 处全部是 639(c) 的父仓在场判据及其测试）。

---

## 第 4 问：`ReverseOpen` 的来历

**一句话：它不是本仓任何一次裁定引入的——是 2026-06-28 照一份 PDF spec 抄进来的；
它唯一的原文依据在 2026-07-25 的 #272 裁定里被判定「原文零命中」并**废止**；
但代码路径没有跟着退役，只在 #281/#283 被改了个名字。**

### 4.1 最早引入：commit `89c23de3ac`（2026-06-28 07:32 -0400），无对应 issue

commit message：
`feat(theta_v0): 严格全互斥定义策略七链rust实装——买卖点Γ入场(GAP-5闭合)→R_Θ三桶→活动集AncOK→π_Θ LexArgmin→唯一订单`

该 commit 新建 `rust/src/theta_v0/strategy/coverage.rs`（+1649 行），
diff 内 `+371` 行首次出现 `ShortDiff,`（即 `enum Vertical { Ambient, FollowParent, ShortDiff }`），
同 commit 注释已给定义：

> `/// - ShortDiff：σ_{p(g)}≠0 ∧ δ_g = −σ_{p(g)}（短差 = 父级方向的反向子操作，spec §9 / P8）`

当天更早的三个提交（`a0d5a7ca66` 06:45 `docs(spec): 递归完全分类买卖点20页完整提取——严格全互斥定义策略权威版`、
`360e2ce2ea`、`83377236ec`）是 PDF 提取。

**⟹ 引入路径是「PDF spec 提取 → 当天照抄实装」，不是一次辩论后的裁定。**
**090**：引入它的 GitHub issue = **查不到**（不是「不存在」）——
2026-06-28 那批提交处于无 issue tracker 的蜂群期，
`gh issue list` 里最早的相关票已是 7 月的 #135/#136/#137。

当前定义位点：`role.rs:55-64`（`Vertical::ReverseOpen` 在 `role.rs:63`）。

### 4.2 引入理由（逐字，全部落在 PDF 提取件里）

`.chanlun/specs/2026-06-28-recursive-complete-classification-bsp-pdf-extract.md:456,460,483-486`：

> `ShortDiff(g) ⟺ V(g) = ShortDiff.`
> `ShortDiff(g) ⟺ σ_{p(g)} ≠ 0 ∧ δ_g = -σ_{p(g)}.`
> **结论**：短差严格等价于垂直关系取 ShortDiff……父多头(σ=+1)时短差做空(δ=-1)，
> 父空头(σ=-1)时短差做多(δ=+1)。短差是父级方向的反向子操作，非绝对做空。

`.chanlun/specs/2026-06-28-buy-sell-point-operational-semantics-pdf-extract.md:26`（P12 §7.5）：

> **§7.5 ShortDiff** ℓ_g<ℓ_α ∧ δ_g=−σ_α 父仓保持建立独立反向子声部……
> `p_after = s_α e^{σ_α}_α + s_g e^{−σ_α}_{ν(g)}`；父仓保持 `Δs^child_α=0`；
> 同股数短差要求 `s_g=s_α`

教义侧落点 = ADR 0001 的 **S6**，`docs/adr/0001-graded-exit-and-shortdiff-doctrine.md:40-43`：

> ### S6 短差基线表示 = 子级反向对冲声部
> 父级（ℓ+1）持仓期间：ℓ 级卖点 ⇒ 开 ℓ 级**反向**声部（`σ_u = −σ_{p(u)}`，父仓不动）；
> ℓ 级买点 ⇒ 平该反向声部。
> **出处：缠论的全互斥定义策略.pdf §三.3（p.17/27）「父级多头时，卖点开短差空头，买点平短差空头……
> 这就是缠论买卖点在多空双开、多级别多重赋格里的操作语义」；推导完全互斥分类.pdf §5 分账本头寸空间。**
> **不采用**"同向声部清仓再入场"表示：二者净头寸轨迹不等价……

**注意这段「出处」的性质**：`docs/formal-chain/*.pdf` 是 GPT 与用户对话产出的形式化文档，
**不是缠师博文**。

### 4.3 ★原文依据：**没有，而且本仓自己已经正式判过「原文零命中」**

**issue #272 裁定**（2026-07-25，用户逐题终审）第 1 条逐字：

> **1. 短差 = 减仓回补（唯一定义）**。本级仓先减后补，不开空腿。
> 开空腿形态**原文零命中**（172 处检索；唯一命名程式 014:42 = 次级别一类卖点减仓、一类买点回补）。
> S6 原短差表示（父仓不动 (+Q,0)→(+Q,−Q)）**废止**；
> 该机制（SplitLegLedger/P9，休眠中）夺「短差」名，移交多空双开线处置。

**该考据经复核属实**（探针逐字核验）：
- `grep -o "短差" docs/chanlun/text/blog/*.md | wc -l` = **172**，与裁定数字逐位吻合。
- 唯一命名程式 `docs/chanlun/text/blog/014-第14课.md:42`，属**【正文】**（第14课讲义主体，
  茅台周线举例段落中间，无时间戳、无提问者署名；对比 `038-第38课.md:92`、`025-第25课.md:254`
  等含 `2007-03-21 15:41:09` 时间戳的行才是【答疑】）：

  > 缠中说禅短差程序就是：大级别买点介入的，在次级别第一类卖点出现时，可以先减仓，
  > 其后在次级别第一类买点出现时回补。对于周线买点介入的，就应该利用日线的第一类卖点减仓，
  > 其后在第一类买点回补。

**原文这句是「同一仓位先减后补」，没有「父仓不动、另开反向腿」的意思。**
⟹ `ReverseOpen`（父仓保持 + 反向子腿）这个机制**在缠师原文里没有依据**，
且这一点已被本仓 #272 裁定在案。

### 4.4 #278 / #280 / #281 / #282 / #283 各自做了什么

| 票 | 类型 | 做了什么 |
|---|---|---|
| **#278** | 🗺️ map `多空双开与反向操作对齐`（已关） | 总图，收三件事：散装域处置 / 对冲腿名分 / 狭义短差 spec。关图决议：「**散装域处置** ✔——#281 裁定 → #283：ReverseOpen 词汇对齐，生产反向操作全部归位「次级别首开反向」。」 |
| **#280** | grilling `对冲腿机制名分终裁`（已关） | 裁「留拆」：删 S6 开空腿的**账面形态**全套（`SplitLegLedger`（`ledger.rs:821`）、`channel.rs` P5 槽、OscillationBook 账面形态），保留其**触发链**给 #274 狭义短差。实装票 #282 |
| **#281** | grilling `散装域处置路线`（已关） | **只改名不改行为**。逐字：「**处置路线 = 词汇对齐**：191 笔全部口径合法（A+B 合流），**生产行为一行不动**……对齐内容：ShortDiff 角色/枚举名重读为修4「次级别首开反向」、AccountIdentity 补级别参数、注释/报告口径对齐」。落 ADR 0001 **补充一**（`docs/adr/0001-...md:133-137`） |
| **#283** | 实施票，commit **`34fed43877`**（2026-07-25 23:52） | 机械改名，行为零改动：`Vertical::ShortDiff→ReverseOpen`、`ExitType::CloseShortDiff→CloseReverseOpen`、`AccountIdentity::ShortDiff→ReverseOpen{level:u32}`、mutex P7/P9 谓词、25 个测试函数。`TwEvent::ShortDiff` **明文保留原名**（属修1 合法减补链）。尾巴修正 `a83b5e0364` |

**★ 这里有一处必须点名的分叉：**
#272 判「S6 废止、原文零命中」，#280/#282 据此删掉了 **#149 域**的休眠开空腿账面形态；
但 **#281/#283 保留了散装域那条真在开火的 `classify_vertical` 反父证书路径，只给它换了个名字。**
语义 `δ_g = −σ_p` 从 2026-06-28 至今**一行未动**。

⟹ **被废止的是教义，被保留的是代码。** #837 实测的 62.6%–72.0% 正是这条被保留的路径的产物。

### 4.5 相关 PDF 在仓情况

- `买卖点alpha2.pdf`：**在仓**（`docs/formal-chain/买卖点alpha2.pdf`，38 页）。
  引用点 `backtest/selector.rs:1`、`backtest/l3_delta_r_alpha.rs:9`、`backtest/mu_estimator.rs:1`、
  `strategy/mutex.rs:2`。它承 χ 选择器／μ 估计／互斥化定理，**不是 ReverseOpen 的定义源**。
- ReverseOpen 真正的定义源 PDF：`递归完全分类买卖点.pdf` §9、`缠论的全互斥定义策略.pdf` p.18/27、
  `推导完全互斥分类.pdf` §7.4/§16/§17、`买卖点.pdf` §4-§5（清单 `.chanlun/implementation-index-20260723.md:142`）
  ——**四份都在仓**。
- **`命名冲突.pdf` 查不到**：`role.rs:43` 的「★GPT 权威裁决（命名冲突.pdf §二-§四，2026-07-05）」
  是 V 轴回滚三分类（撤 `67d20d292f` 四分类）+ 新增 G 轴 `GradeRel` 的**唯一援引权威**，
  而 `find -iname "*命名冲突*"` 全仓零命中，`.chanlun` 下也无对应落盘件。
  **090：这是「查不到」，不是「不存在」**——可能在 Downloads 或未入仓。

---

## 第 5 问（按编排者新口径重写）：「双向可开」是被有意设计的，还是顺手就有的？

### 5.1 提问方式的更正

原票面问的是「若禁止跨级反向持仓会撞坏哪些裁定」。编排者当场纠正：
本仓是 BTC 永续，**双向合法**；把缠师那句话的**前提**（A 股无做空机制）当成**结论**
（所以不该反向）是误读。因此本节改问：

> **「双向可开」这个我们独有的能力，在架构里是被论证过的，还是顺手就有的？**

### 5.2 证据面（代码侧；裁定侧见第 4 问）

**(a) 双向能力在类型层面是被显式设计的，且设计得很细。**
- `account.rs:40-47`：`ReverseOpen{level}`（有父反向）与 `Short`（无父反向）
  被明文声明为「**互斥且完备**：任一反向声部，有父即首开反向、无父即反向根（CONTEXT.md:93）」。
  一个「互斥且完备」的二分法不是顺手就有的，是有人坐下来把反向声部的全域切了一刀。
- `role.rs:35-64`：V 轴三分类 `Ambient / FollowParent / ReverseOpen`，
  并附 GPT 命名冲突裁决（2026-07-05）的商映射论证 + 定理 1（G 轴细化的投影保持原像划分）。
- `mutex.rs:16-18`：P7 `Close ReverseOpen` / P9 `Open ReverseOpen` 在
  全互斥解释器 P1..P10 里各占一格 —— 反向开／关是**一等公民谓词**。
- 形式链：`formal/Origin/SeparateLedger.lean` 的 `legHedged` / `hedged_leg_nonzero_but_net_zero`
  ——**双开状态在 Lean 里被证明为 P^sep 的非零元**。

⟹ **「双向可开」在类型／记账／形式化层面被精心设计过。** 但见 (a′)。

**(a′) ★但设计的**来源**是 PDF 转录，不是本仓的必要性论证——而且那份论证的原文根据已被本仓自己判掉。**

第 4 问查实：
- 引入 = commit `89c23de3ac`（2026-06-28）照 PDF spec 实装，**查不到对应 issue**，无辩论痕迹。
- 论证 = PDF 提取件 + ADR 0001 S6，其「出处」是 `docs/formal-chain/*.pdf`（GPT 产出），**不是缠师博文**。
- **#272 裁定（2026-07-25）已判 S6「开空腿形态原文零命中（172 处检索）」、S6 表示废止。**
- **#281 裁定明文「生产行为一行不动」**，#283 只做机械改名。

⟹ 严格说：**「双向可开」是被设计的，「跨级同时持相反方向」是顺手就有的。**
前者有 PDF 依据（虽非原文），后者**从未被任何文件论证过**——
所有论证都停在「父仓不动 + 反向子腿」这个**父子局部**场景（见下 (b)）。

**(b) 但被论证的是「反向声部作为一种角色如何分类／记账／证明」，
不是「本策略为什么需要在两个级别上同时持相反方向」。**

我读到的全部论证都停在**局部场景**：
- `leg.rs:298`：「对齐 **M11 短差**『父声部不动建反向子声部』的净额体现」——
  场景是**父子**（同一分支），不是**跨级独立根**。
- `account.rs:40`：「**父声部 active 期间**的次级别反向对冲声部（σ_u = −σ_p，父仓不动）」——
  同样以「父在场」为前提。
- `runner.rs:193-195`：639(c) 的兑现语是「**不开 naked 逆势仓**」——
  即**反向必须有父**，这是全仓唯一一条「什么时候不该反向」的条款。

**⟹ 全仓对「反向」的正当性论证，一律以「有父、父在场、父仓不动」为前提。
而 #837 实测的主战场（`Ambient` × `Ambient`，隔 2–3 级）恰恰是这个前提之外的区域——
那里没有父，因此 639(c) 不适用，因此也从来没有被任何论证覆盖过。**

**(c) 「什么时候不该反向」的条款盘点（穷举）**

| 条款 | 位置 | 适用域 | 覆盖 #837 主战场？ |
|---|---|---|---|
| 639(c)：未持父则剔除 ReverseOpen 子腿 | `exit.rs:355`（判据）、`held.rs:151`、`runner.rs:193-195`、测试 `ancok_tests.rs:187/239`、`sizing_tests_2.rs:395/444/470`、`step_tests.rs:435` | V=`ReverseOpen` 且有操作父 | **否**（Ambient 根无父） |
| `RevOpenVerdict` G1/G2/G3a | `trading/level_operating_unit.rs:825-833` | `trading/` 引擎的 voice 内 REV 腿 | **否**（不在 `theta_v0` π loop 上） |
| G7 毛头寸约束 | `leg.rs:~340`、`config.rs:199/227/456` | 毛敞口幅度 | **否**（默认关；且是缩放不是拒绝；逐根独立） |
| `KThetaRiskGate.stop_long/stop_short` | `sizing.rs:322-326` | 标量净持仓单侧禁开 | **否**（反向对使净≈0，门不 binding） |

**除此之外，全仓查不到任何以「对侧级别当前持有什么」为条件的条款。**
（090：这是「确认没有」——grep 覆盖 `rust/src` 全部 `.rs`，检索词
`naked`/`裸腿`/`裸空`/`逆势仓`/`ReverseOpen`/`opposing`/`对冲` 及 `exit.rs`/`step.rs`/`mutex.rs`/
`sizing.rs`/`admission.rs` 逐函数通读。）

### 5.3 边界探测（降权保留）：若真的禁止跨级反向，会撞到哪些裁定

**声明：本节是边界探测——用来量这条约束有多深，不是候选方案。**

| # | 会撞坏的裁定／不变量 | 位置 | 撞法 |
|---|---|---|---|
| 1 | **C25 分账本多空独立坐标** + Lean `hedged_leg_nonzero_but_net_zero` | `ledger/separate.rs:1-84`；`formal/Origin/SeparateLedger.lean`、`SeparateFinalTheorem.lean:279-289` | 双开腿 (Q,Q) 是 P^sep 的合法非零元；禁反向 ⟹ 该元素类被排除，形式链定理的有效域被砍 |
| 2 | **639(c) 的正交性裁定** | `runner.rs:193-195` | 639 明文裁定 σ_p 来源（因果塔）与持仓准入是**两条正交机制**；加入「跨级对侧持仓」判据等于把持仓状态重新塞回准入，与 639 删掉的「活动父腿错口径」（`interp.rs:1283`、`interp.rs:2949`）同型 |
| 3 | **V 轴三分类 living authority** | `role.rs:35-64` | `ReverseOpen` 是 3×3×2 完全分类的 1/3；禁掉它 ⟹ 完全分类的 Σ=1 结构定理需重证 |
| 4 | **I_Θ 全互斥全定义解释器 P1..P10** | `mutex.rs:5-19`、定理 1 `mutex.rs:31-34` | P7/P9 两条谓词失去触发面；`Σ_{j=0}^m 1[C_j]=1` 的构造不受影响，但谓词集的经验有效域塌一块 |
| 5 | **#198 断言4 首开反向隔离** | `fill.rs:398-440` | 该断言的前提是 ReverseOpen 账存在且会有 fill；禁反向后成空真断言 |
| 6 | **#199/#200 二类开空通道** | `account.rs:59-63`（`ReverseType2`）、`account.rs:80-87`（`OpenShort`） | ID-3「二类点合法卖出仅短差、开空」——禁跨级反向会连带切掉二类开空的一部分触发面 |
| 7 | **TW StageII 重叠腿 / P2 CloseOverlay** | `mutex.rs:9`、`backtest/ledger.rs:28`、`backtest/ledger.rs:362` | legacy ReverseOpen 腿是 TW 三阶段 H 分量的来源 |
| 8 | **alpha2 §6–§7 `N = Q − H` 抵消** | #837 实例 3（w2021bull，L2/Long × L3/Short，+1,092,722 / −883,233）为其教科书级实例 | 该公式的存在本身预设了 H（对冲腿）可以非零 |
| 9 | **M5 声部执行层 sep 出口 / hedge-mode** | `step.rs:26-33`、`overlay_state.rs`（hedge-mode P^sep 账本 + ΔN 订单） | 逐声部毛账本的用途主要就是承载双开 |

**深度读数**：这条约束至少触及 **9 处**已落定的裁定／不变量，其中 2 处（#1、#8）**在形式链里**。
⟹ 「禁止反向」不是一个配置项，是一次形式链层面的重定义。**这正是它不该被当成候选方案的理由**——
而在双向市场上它也**不必**被当成候选方案。

### 5.4 本票真正的产出口

按编排者定位，问题不是「它为什么发生」，是「**它发生了，谁在为它的代价负责**」。
§2.4 的答案是：**没有人**。因此可交给 #834／#836 的，是三个**已点名到行**的缺口：

- **G1（保证金）**：`admission.rs:1238` 的 `p_t.abs()*px` 是净名义 ⟹ 反向对的维持保证金为 0。
  真实交易所（Binance 永续单向持仓模式）不会给你这个免费午餐——**这是回测与生产的口径分叉**，
  不只是「保守不保守」的问题。
- **G2（毛敞口不可观测）**：`leverage_ok`（`risk.rs:552`）零生产调用；G7 默认关（`config.rs:227/456`）。
  形式链已证 `net_ok_not_imply_gross_ok`（`LeverageCapital.lean:263`）。
- **G3（无「无意撞车」默认规则）**：全仓唯一的反向条款 639(c) 以「有父」为前提，
  #837 主战场（`Ambient`×`Ambient`）在其域外。**这是 #834 该仲裁的那块空地。**

---

### 5.5 ★补一条：这条能力的教义已经退役，代码没跟着退役

第 4 问查实的分叉值得单列，因为它是**这个缺陷长期不被发现的第二个原因**（第一个是 §2.3 的净额塌缩）：

- **#272**（2026-07-25）判：S6「父仓不动、另开反向腿」的短差表示 **原文零命中、废止**。
- **#280/#282** 据此删掉了 **#149 域**的休眠机制（`SplitLegLedger` 等账面形态）。
- **#281/#283** 对**散装域**（真在生产开火的 `classify_vertical` 反父证书路径）
  的处置是 **「生产行为一行不动」+ 机械改名**。

⟹ 从 2026-07-25 起，本仓处于这样一个状态：
**教义层已经承认「父仓不动开反向腿」没有原文依据，执行层仍在每 bar 这么做，
而且因为改了名字（`ShortDiff`→`ReverseOpen`），事后按旧名检索的人查不到它还活着。**

**这不是说它应该被删**（双向市场上它合法，见 5.1）。
这是说：**它现在没有任何一层为它背书**——原文不背书（#272），教义不背书（S6 废止），
决策层看不见它（§2.3），风控不为它计价（§2.4）。**它是一个孤儿机制。**

---

## 090 照实（本报告未做／未核实项）

1. **第 4 问由并行探针供料**，本体未独立复跑 `git log -S` 全历史；
   探针自陈已用 `gh issue view` 逐一核对 #272/#278/#279/#280/#281/#282/#283 为真 GitHub issue，
   并逐字复核了 172 处 `短差` 计数与 `014-第14课.md:42` 的【正文】属性。
   **本体未二次复跑这两项核验。**
   探针同时登记两处「查不到」：① 引入 `ShortDiff` 的 GitHub issue（蜂群期无 tracker）；
   ② `命名冲突.pdf`（全仓零命中）。**两项均为「查不到」，非「不存在」。**
2. **`trading/` 与 `theta_v0/` 的接线关系未核实**——我只读出 `trading/level_operating_unit.rs`
   存在 `RevOpenVerdict` 三守卫且模块标 `现役`；**未核实**它是否在 #837 实测路径上、
   是否与 `theta_v0` π loop 共用同一资金池。这是「我没查到」，不是「确认无关」。
3. **`fill.rs` 注释里的 `#198`/`#185`/`#199`/`#124`、`account.rs` 的 `#281`/`#283`、
   `step.rs` 的 `#216`/`#226`/`#233` 等 2026-06/07 期编号未逐一按 GitHub issue 核实**
   （#807 判例：kimi 任务号与 issue 号段碰撞）。本报告引用它们时只作**代码内部谱系标签**，
   不作 GitHub issue 解。
4. **未运行任何代码**——本票零改动、零执行，全部结论来自静态阅读。
   §2.4(a) 「反向对 ⟹ MM≈0」是从 `admission.rs:1238` + `risk.rs:658` 的签名与实现推出的
   **结构性结论**，**未做数值实验证实**（#837 的五窗跑批未采集 MM 读数）。
5. **`overlay_state.rs` hedge-mode 的完整决策旁路性未逐行核实**——
   我依据 `step.rs:28-33` 与 `theta_overlay.rs:78-84` 的自陈（「纯只读，不进决策路径」/
   「决策层仍与净额臂逐字节一致」），**未独立追它的全部调用点**。
6. **甲侧 H1–H5 的裁定未在场**——第 3 问按「若 H4 成立」推演，H4 是否成立不由本侧裁。
7. **`prove_pair_isolation`（`rec_engine.rs:2156`）属 `recursive_t` 侧**，
   与 `theta_v0` 的账户隔离是**同族但不同实现**；#837 实测跑的是 `theta_v0`，
   票面把它列为候选 ① 的证据需按此收窄。

---

## 交付

- 报告：本文件
- 分支 `issue838-b`，**零生产改动**（`rust/` 下无任何文件被修改）
- 第 4 问由并行探针供料，结论已并入 §4，核验标注见 090 第 1 条
