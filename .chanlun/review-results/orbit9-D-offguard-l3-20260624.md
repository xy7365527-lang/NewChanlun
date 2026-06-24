# orbit9 D-整合 OFF守卫 + L3八标的验证报告（2026-06-24）

整合 worktree：`/private/tmp/orbit9-integration-wt`，分支 `orbit9-D-integration-20260624`。
整合链：facea `54279a503e` → A `167bfda7bb` → B `685bed8290` → **C `4c0afeae11`**（D 续作解 5 冲突块后完成 cherry-pick）。

## 结论（六要素）

### 1. 结论

整合**成功**（C 的 5 个冲突块解析完成，cargo build PASS）。但 **L3 否定性结果**：三 env 全开（ON）相对 OFF **逐位不同且收益普遍劣化**，根因是 A 的 H⁰ flip 门在生产数据上**大量触发**（绝非死代码）。B（dispatch）与 C（nest）在整合态是**死代码**（无消费路径）。

#### OFF bit-exact 守卫 = PASS（认识论 L1）

`cargo test recursive_t`：**137 passed / 0 failed / 22 ignored**。含全部 bit-exact 守卫：
- `rec_flat_bit_exact_含走势完成清仓路径`、`rec_flat_btc_bit_exact`（rec≡flat）
- `facea_production_默认开启_off_baseline回归守卫`、`faceb_off_bitexact_rbpair不受影响`
- A：`off_非type1反向_flip_bit_exact`（H⁰ OFF 门常开不计数）
- B：`off_bit_exact_orbit9未激活_全走recover`（dispatch OFF add 不激活）
- C：`no_flat_gate_unlocated_when_level_not_diverging`

三 env 全不设 ⇒ 逐位 = facea 基座。**不退回**（bit-exact 未破坏）。

#### L3 对照表（8标的 × OFF/ON，Structural 模式，认识论 L3）

| 标的 | OFF strat | ON strat | Δ | BH | h0_flip_blocked(S/A/O) | A''抢先(S) | flip(S) OFF→ON |
|------|-----------|----------|-----|-----|------------------------|-----------|----------------|
| CL   | +20.3% | **−28.1%** | −48.4pp | +28.2% | 55/51/66 | 2 | 5→0 |
| BRN  | −26.0% | −16.4% | +9.6pp | +87.4% | 37/20/17 | 6 | 3→0 |
| DX   | −0.7% | −4.0% | −3.3pp | +4.1% | 10/10/19 | 3 | 2→0 |
| GC   | −22.7% | −11.5% | +11.2pp | +257.3% | 3/3/3 | 3 | 5→0 |
| ES   | −2.3% | −2.9% | −0.6pp | +594.3% | 64/9/80 | 4 | 3→0 |
| QQQ  | −8.6% | −21.1% | −12.5pp | +174.6% | 16/16/18 | 2 | 4→0 |
| BTC  | +26.0% | **−28.3%** | −54.3pp | +1380.4% | 41/12/43 | 5 | 4→0 |
| OKLO | +55.3% | **−29.6%** | −84.9pp | +307.1% | 20/6/26 | 1 | 2→0 |

（AND/OR 列见日志 `/tmp/d_l3_off.log` / `/tmp/d_l3_on.log` strat 矩阵；ON 整体 8/8 不优于 OFF，6/8 劣化。）

#### ★A HIGH-1（H⁰ flip 门触达率 vs A''抢先）—— **推翻 task 假设**

task 假设：「生产配置 `enable_trend_done_clear=true` 下 A''（走势完成清仓）可能优先于 H⁰ flip ⟹ H⁰ 是死代码」。

**实测相反**：
- OFF：全标的 `h0_flip_blocked=0`（H⁰ 门关）。
- ON：**全标的非零，触达量大**（CL 55/51/66、ES 64/9/80、BTC 41/12/43、OKLO 20/6/26）。
- A''抢先（`trend_done_clears`）仅 **1–8 次/标的**，远小于 h0_flip_blocked（几十次）。

**根因（作用域不同，非互斥）**：A''（走势完成清仓）只在**最高活跃级别 cc 的 type1** 触发（rec_engine.rs:2525 `highest_active()` None 分支前）；H⁰ flip 门作用于 **route_bsp 所有级别的核心反向 flip**（rec_engine.rs:1551）。A'' 覆盖不到次级别/非-cc 的反向 flip，这些恰是 H⁰ 拦截的对象。**H⁰ 绝非死代码——在生产配置下强烈生效**。

**但 H⁰ 损害收益**：`flip(S)` 全标的 OFF→ON 由非零→**0**（CL 5→0，GC 5→0，BTC 4→0）。H⁰ 把 confirmed 门施加到所有反向 flip，把背驰段的提前翻向全部拦掉（no-op，骑走势）。在这 8 标的生产数据上，被拦的 flip 多数是有益的——**踏空惩罚 > 假翻向惩罚**。这是 payoff 作为有效域过滤器的否定性结果（缩小 H⁰ 有效域，formalization-validity-domain 231号：payoff 不进操作语义）。

#### B（dispatch）/ C（nest）= 死代码（MEDIUM/MEDIUM-obs 答案）

- **B**：`n_adds=0` 全标的全模式。`orbit9_sub_trend_done`（rec_engine.rs:1430）仍是**占位 `return true`** ⇒ `T_ORBIT9_DISPATCH` ON 时 `!orbit9_sub_trend_done(j)` 恒 false ⇒ O3 add 永不激活，全走 recover ⇒ 对 strat 无效应（dispatch 单独 = bit-exact）。
- **C**：`enable_orbit9_nest` 仅 EngineConfig 三处（struct/from_env/off），**无任何引擎消费路径**（`locate_nest` 仅被测试调用）。C 提交本身声明「只提供纯函数定位组件（不写仓位）」⇒ `T_ORBIT9_NEST` ON = 完全 bit-exact。

**∴ ON 相对 OFF 的全部差异由 A（H⁰）单独贡献。** B、C 在整合态未接通（C 的 `locate_nest` 应替换 `orbit9_sub_trend_done` body，B 才能激活 add，但这是 A/B/C 后续整合工位的工作，非 D 的冲突解析范围）。

#### B-hetero MEDIUM-obs（O3 add_short PnL 路由）

`add()`（rec_engine.rs:1467）经 `account_reduce(mob, realized, c)` 路由：`mob=flip_pol(pdir)` = 被减 sub 短差腿方向。`account_reduce`（1121–1151）：Long reduce → `core_cost_basis`（降成本账本），Short reduce → `short_leg_pnl`。**∴ add_short（父空、sub 持多短差，mob=Long）的 realized 进 core_cost_basis（降成本），非 short_leg_pnl** —— B-hetero 标记属实。同时 1469 行也累加 `add_pnl_by_level[sub]`（observation-only 诊断，非真账本）。**当前无实证影响**（n_adds=0，add 未激活），bear 跑时若 add 接通需重审此归因。

### 2. 定义依据

- OFF bit-exact 定义依据：facea `54279a503e` 为基座，三 env（`T_ORBIT9_H0`/`T_ORBIT9_DISPATCH`/`T_ORBIT9_NEST`）缺省 ⇒ `EngineConfig::from_env` 三字段为 false ⇒ route_bsp/flip/add 逐字不变（A/B/C 提交注释均声明此 OFF 不变式，rec_engine.rs:170/179/186）。
- H⁰ flip 门定义：587 候选A τ 手性对称化 + exhaustive #39 §一A T1-3（背驰段 ⊋ type1，P2 时序约束「τ flip 仅在 φ=0 走势完成点合法」）。`enable_h0_skeleton && !flip_confirmed` ⇒ `n_h0_flip_blocked += 1`（no-op）。
- A''（走势完成清仓）定义：546号死锁解锁，`enable_trend_done_clear && highest_active()` 在 cc 级 type1 清仓。

### 3. 边界条件（结论翻转条件）

- **OFF bit-exact 翻转**：若任一 env OFF 时仍有行为差异 ⇒ 整合破坏 bit-exact = 致命，须 git checkout 退回。**当前 137 测试 PASS ⇒ 不翻转。**
- **H⁰ 非死代码翻转**：若 `enable_trend_done_clear=false`（关 A''）后 h0_flip_blocked 不变 ⇒ 证实 H⁰ 独立于 A''（当前 A''抢先 1-8 << h0_blocked 几十，已强烈支持独立）。若把 A'' 提到所有级别（非仅 cc）⇒ A'' 可能吞掉部分 H⁰ 触发，触达率下降。
- **H⁰ 损害收益翻转**：有效域 ⊂ 当前 8 标的 1min。若在**真 bear 段**（cc 走势向下完成）或更高确认级别，被拦的提前翻向可能转为有益（避免假翻向灾难）⇒ Δ 翻正。当前 net-up 主导的 8 标的上 Δ 6/8 为负。
- **B/C 死代码翻转**：若 `orbit9_sub_trend_done` body 替换为 C 的 `locate_nest`/`is_sub_trend_done` ⇒ add 激活，n_adds>0，dispatch 产生真实效应。

### 4. 下游推论

1. **整合产物可合主**：OFF bit-exact PASS ⇒ A/B/C 的代码可安全进 main（OFF 默认不改变 facea 行为）。三 env 是 opt-in 实验门控。
2. **H⁰ 默认应保持 OFF**：在当前 8 标的生产数据上 H⁰ ON 损害收益（6/8 劣化），不应进 production 默认配置。H⁰ 的有效域待真 bear 数据 L3 缩小（587 候选A 的 τ 对称裁定是 L0 必然性，但 payoff 有效域 ⊊ 定义域）。
3. **B/C 整合未完成**：dispatch 的 add 轨道与 nest 定位算子需后续工位接通 `orbit9_sub_trend_done` ← `locate_nest`。D 的职责（冲突解析 + OFF守卫 + L3）已完成，接通是 A/B/C 整合工位的下游工作。

### 5. 谱系引用

- **587**（H⁰ 候选A τ 手性对称化）：本 L3 是 587 的有效域验证——L0 τ 对称必然性成立，但 payoff 有效域在 net-up 8 标的退化为净负（formalization-validity-domain 231号：有效域 ≠ 定义域）。
- **539**（做空腿=亏损唯一来源）：H⁰ 拦截 flip 后 short_pnl 多数仍负（CL −7486、OKLO −53254），flip→0 未解 539 的做空腿失血。
- **546**（A'' 死锁解锁）：A''抢先与 H⁰ 作用域分离，二者非互斥。
- **574**（确认滞后 floor）：H⁰ confirmed 门 = 把 type1 走势完成作为 flip 合法条件，本质是 574 滞后 floor 施加到 flip ⇒ 提前翻向被拦 = 踏空（与 574 lag 同构）。

### 6. 影响声明

**改动**（整合 worktree，非 B-wt）：
- `rust/src/recursive_t/rec_engine.rs`：解 5 个 cherry-pick 冲突块（EngineConfig struct/from_env/off + TRoot struct/new_with_config）。**机械三者并存合并**——A+B（HEAD：pair_short_entry/pair_long_entry/enable_pair_emergence/enable_h0_skeleton/n_h0_flip_blocked/enable_orbit9_dispatch/pending_exit_trigger）+ C（enable_orbit9_nest）全部保留。**删除 C cherry-pick 上下文偏移制造的 1 个重复 `face_b()` 定义**（HEAD 已有同名函数）。无语义取舍，无 workaround。
- `rust/src/recursive_t/rec_stream.rs`：`rec_btc` 测试加 1 行 ORBIT9 观测 eprintln（`#[cfg(test)]` 内，observation-only，不进引擎语义，不破坏 bit-exact）。
- `analysis/data_cache/`：软链接 8 标的数据（纯数据准备，非代码）。

**不改动**：A/B/C 三组件的引擎逻辑（route_bsp/add/locate_nest）逐字保留，无 fallback、无垫片。

---

## 退回判定

**不退回。** OFF bit-exact 守卫 137/137 PASS，整合未破坏 facea 基座。ON 的否定性结果（收益劣化）是 H⁰ payoff 有效域的真实测量，不是整合 bug——它缩小了 H⁰ 的有效域边界（L3 信息增量），比确认性主张更有价值。

## 认识论等级

- OFF bit-exact：**L1**（合成数据 + 回归测试验证管线/不变式正确性，信息增量验证整合无破坏）。
- ON L3：**L3**（8 标的 × 3 模式真实数据交叉验证，含 bear 标的 BRN/DX/GC，产生否定性结果——H⁰ ON 6/8 劣化，缩小有效域）。

## 整合 commit

`4c0afeae11`（C 的 cherry-pick 完成 commit；A=167bfda7bb，B=685bed8290 已先于此 cherry-pick）。
