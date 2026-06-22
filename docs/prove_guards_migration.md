# recursive_t prove 守卫移植报告

**日期**：2026-06-21
**改动文件**：`rust/src/recursive_t/prove_guards.rs`（新增 6 守卫 + 测试）、`t_engine.rs` / `rec_engine.rs`（对称接入）
**认识论标注**：移植判定 = L0（架构定义推导）；BTC 零 panic 验收 = L2（真实单标的）。

---

## 1. 背景与原则

session4/5 在旧引擎（`spiral` / `fugue_v3`）积累了几十个 prove 守卫。prove **不是回测指标，是必然性的可执行形式**（L0 结构 panic / L2 观测计数）——按项目纲领，必然性累积不该随引擎更替丢弃。递归 T 引擎（`recursive_t`）作为当前引擎，迁移前只有 5 个守卫（`prove_tw_neutral` + session 新加的 4 个）。

**核心约束（三条规则联合）**：
- `formalization-validity-domain`：旧守卫的**有效域可能严格小于其定义域**。T 引擎与 spiral/fugue 架构不同 → 不可假设旧守卫全部经验有效。
- `no-patch-mentality`：诚实声明代码能做什么——不可把不成立的守卫硬塞进来"凑数"。
- `no-workaround`：遇到架构矛盾（如同股数 vs 同资本）停下来精确描述，不模糊化。

**移植判据 = 耦合度**：只依赖 `usize`/`f64`/`Polarity` 且不变量在 T 引擎架构下**真实成立**的 → 移植；依赖旧引擎特有数据结构（`GroupElement`/`SpiralVoice`）或前提被 T 架构否定的 → 跳过并记录原因。

---

## 2. T 引擎 vs 旧引擎的三个架构差异（决定有效域）

| 维度 | spiral / fugue_v3 | recursive_t（flat + rec） | 后果 |
|------|-------------------|--------------------------|------|
| 级别转移 | 逐级 ladder（每级活跃，`Δr=−1`） | **稀疏 ladder 区间套**（`nearest_active_parent` 跨 idle 间隙，`Δr` 不恒 −1） | `cross_level_closure` 弱化为 `sub<parent` |
| 仓位转移 | 同股数（`Σ\|units\|=n_base` Casimir 守恒） | **同资本**（`short_u=m·pb/c`，价变则 units 不等） | `conservation` 不可移植，守恒律升级为 TW 中性 |
| 会计阶段 | 单阶段恒仓 | **三阶段**（CostReduction/CapitalRecovered/**EarningShares 增股数**） | 股数守恒被增股数破坏 |
| 群表示 | D∞ 群代数（`GroupElement{a,t}`/`act_helix`/`compose`/`power`） | 无群代数（`Direction`/`Polarity` 二值枚举，递归=`Unit` 封装迭代） | 群关系守卫（h²³=σ 等）无对应物 |

---

## 3. 移植清单（总表）

### ✅ 已移植（6）

| 新守卫 | 模式 | 来源 | T 引擎依据 | 接入点 |
|--------|------|------|-----------|--------|
| `prove_sink_descends(parent,sub,bar)` | panic L0 | spiral/fugue `cross_level_closure`（Δr=−1）**弱化** | `nearest_active_parent(j)` 返回 >j ⟹ sink/recover 恒 `sub<parent` 向心下沉 | sink / recover |
| `prove_sigma_quota(m,units,level,bar)` | panic L0(形式) | spiral `theta_sigma_invariant` + fugue `sigma_quota`（542号缺瓦） | sink/drain 配额 `m=quota(u)=u×MOBILE_FRAC` 级别无关 | sink / drain |
| `prove_relabel_invariant(lu,su,lu',su')` | panic L0 | spiral `a5_relabel` | ascend = 把 layer 从 `from` 挪到 `to`（同 units/dir/basis）⟹ 敞口不变 | ascend |
| `assert_polarity_involution(f)` | panic L0(元) | spiral `tau_involution`（τ²=e）**弱化** | `Polarity` 是 Z₂，`flip∘flip=id`；sink 下沉 `flip(d_P)` / recover 升回 `d_P` 往返闭合 | 测试（编译期代数事实） |
| `count_adjacent_same_dir(occupied)` | 观测 L2 | fugue `count_chiral_violations` + spiral `t57_chirality_mirror` | 几何塔应手性交替，emergence 间隙两侧可同向（已知违反） | 共享函数（引擎已内联 `max_chiral_same_dir`） |
| `count_radial_scaling_violations(per_level)` | 观测 L2 | spiral `t50_radial_scaling` | sink sizing=1/3 ⟹ 频率应随级别递减 f∝λ⁻ᵏ；强趋势可局部违反 | 共享函数（消费 `sink_by_level` 等） |

### ⏭️ 跳过（带原因，不强行移植）

| 旧守卫 | 跳过原因（架构矛盾） |
|--------|---------------------|
| `prove_conservation`（Σ\|units\|=n_base，T48 Casimir） | T 引擎**同资本转移**（`short_u=m·pb/c`）非同股数 + EarningShares **增股数** ⟹ 股数本就不守恒，守恒律已升级为 **TW 中性**（`prove_nav_neutral`/`prove_tw_neutral` 守，已有） |
| `prove_h23_eq_sigma`（T56 角径全纯 h²³=σ） | T 引擎无 D∞ 群代数表示，无 `GroupElement`/`act_helix`，无角向圈/径向跃迁的群元素 |
| `prove_conjugation`（T57 τhτ⁻¹=h⁻¹） | 同上（无群代数） |
| `assert_action_matches_element`（GroupAction↔Element） | 同上（无 `GroupAction`） |
| `prove_chirality_seam`（NR-3 τ@φ=0） | T 引擎无角向相位 `phi`（角向奇点）概念 |
| `prove_n1_forest`（单根/孤儿/children 树链） | T 引擎用**扁平 ladder 数组** `layers[k]`，父子关系由 `nearest_active_parent` **动态计算**，无存储的 voice 森林树 |
| `prove_n2_per_voice` / `prove_no_double_act`（per-bar 层互斥） | **区间套下父级可同 bar 被多个子级 BSP 减仓**（多次 `reduce_at`）⟹ per-layer 互斥不成立 |
| `prove_n3_type2`（type2 全消费） | T 引擎 view **纯 BSP 驱动，抹除 type1/2/3 区分**（`buy[ladder]`/`sell[ladder]` 不分类型） |
| `prove_n4_cost_gate`（floor_stop=0） | T 引擎递归由**涌现上界 r***（`iterate` 数据涌现）终止，无固定 floor 成本门 |
| `prove_n7_spawn_self_level`（触发层=voice层） | spiral E spawn 自层触发语义；T sink 触发层=sub（BSP 来层）、被减层=parent，语义不同构 |
| `prove_t14_root_flip`（根 in-place M=N 翻转） | T 引擎 flip = `clear_all + enter`（清仓重建，用全部 free），**非 in-place 翻转**，units 不守恒 |
| `prove_t1_no_voluntary_exit`（不主动清仓恒仓） | T 引擎设计上**主动 clear**（flip / eod），无恒仓约束 |
| `prove_epsilon_symmetry`（panic：core_pol=f(root_dir)） | T 引擎 BSP 驱动方向，强牛中偏离涌现方向（已知失配）⟹ panic 版会 fire；**已降级为 `check_direction` 观测**（`prove_direction_matches_trend`，已有） |
| `self_level_counter_fire` / `sub_level_counter_fire` 及其 `prove_*_symmetric` | 依赖 spiral `nf_sell`/`nf_buy` 数组（向心 confirm 自层/次级别）；T 引擎用 `view.buy/sell[ladder]`，无 nf 数组 |
| `prove_t56/t58/t59`（角向圈/基本域/自相似） | spiral 角向 fire vs 径向 fire 区分；T 引擎无此区分 |
| `prove_t57_chirality_mirror`（买卖侧镜像） | 部分语义并入 `count_adjacent_same_dir`（手性观测）；纯 spiral 角向行程部分跳过 |

### 🔁 已有（不重复移植）

| 既有守卫 | 等价旧守卫 |
|----------|-----------|
| `prove_nav_neutral`(tw_pre,tw_post)（flat）/ `prove_tw_neutral`（rec） | fugue `prove_nav_neutral` + spiral `prove_n8`（NAV 部分），且**升级**为 TW 中性 |
| `prove_bsp_triggers_operation`（note_op） | spiral N 类操作触发归因（T 版） |
| `prove_direction_matches_trend`（check_direction） | fugue `prove_epsilon_symmetry` 的观测降级版 |
| `prove_sink_recover_balance` / `prove_per_level_pnl`（campaign） | 走势终完美 ⟹ sink↔recover 配对（T 引擎特有 campaign 边界） |

---

## 4. 验收证据

| 检查 | 命令 | 结果 |
|------|------|------|
| 编译 | `cargo build` | ✅ Finished（仅预存 dead-code 警告，与改动无关） |
| 单元测试 | `cargo test recursive_t::` | ✅ **89 passed; 0 failed**（含 6 守卫的正向 + 反证测试） |
| 非重言性 | 反证测试 | ✅ 4 个 `should_panic ... ok`（错误输入必 fire，非 `assert(true)`） |
| flat BTC 全量 | `BT_SYMBOLS=BTC … t_engine_8x3 --ignored` | ✅ **4,625,119 bar × 3 模式零 panic**（`no_trigger=0`，179s） |
| rec BTC 全量 | `BT_SYMBOLS=BTC … rec_stream::tests::rec_btc --ignored` | ✅ **2 passed; 0 failed**（205s） |

三个 panic 守卫（`sink_descends` / `sigma_quota` / `relabel_invariant`）在 460 万 bar 真实数据上**零 fire** = L0 不变量在 BTC 有效域内成立。

**bit-exact 保证（构造性）**：所有接入点只读——`prove_*` 是 `assert!`（不改状态）、`exposure()` 是 `&self` 只读求和存入局部变量，仅传给 assert。引擎状态演化逐字不变，回测数字不受影响（无需前后对比，代码层可验证）。

---

## 5. 结果包六要素

1. **结论**：从 spiral/fugue_v3 移植 6 个 prove 守卫到 `recursive_t::prove_guards`（4 panic + 2 观测），对称接入 flat/rec 两引擎；系统记录 16 类跳过守卫的架构原因。

2. **定义依据**：
   - `prove_sink_descends`：缠论区间套（第27/65课 `Move(k)≡Level-(k+1)笔`）→ sink 向心下沉到次级别，`nearest_active_parent` 保证 sub<parent。
   - `prove_sigma_quota`：026:80「用其中的 1/3」→ MOBILE_FRAC=1/λ 级别无关（542号 σ-不变缺瓦补全）。
   - `prove_relabel_invariant`：T30 根级别涌现=会计重组（A3 禁加仓）→ ascend relabel 敞口中性。
   - `assert_polarity_involution`：D∞ 关系 τ²=e（`orbit_enumeration_completeness`）在 T 引擎退化为 Z₂ 极性对合。

3. **边界条件（结论何时翻转）**：
   - 若 T 引擎改回**逐级 ladder**（每级强制活跃）→ `sink_descends` 应升级回 `Δr=−1`（panic 收紧）。
   - 若 sink 配额改为**级别依赖**（非 `u×MOBILE_FRAC`）→ `sigma_quota` 会 fire（这正是它要守的）。
   - 若未来数据使某 panic 守卫在其他标的 fire → 该不变量有效域 ⊊ 定义域，须按 `count_chiral_violations` 先例**从 panic 降级为观测计数**（非删除）。
   - 若 T 引擎引入群代数表示 → 当前跳过的群关系守卫（h²³=σ 等）可重新评估移植。

4. **下游推论**：
   - 必然性累积在 recursive_t 恢复（5→11 守卫），后续引擎改动若违反这些 L0 不变量会在热路径 panic 暴露。
   - 跳过清单 = T 引擎与 spiral/fugue 的**架构差异图谱**，可指导未来"哪些旧分析可迁移、哪些需重做"。
   - `count_adjacent_same_dir` / `count_radial_scaling_violations` 与引擎内联 `max_chiral_same_dir` 重复——可在未来统一（本次不改操作逻辑，保留内联）。

5. **谱系引用**：
   - `count_chiral_violations`（fugue）：panic→观测降级先例（"永远交替"假设被 4/8 标的否定）。
   - 542号：配额 σ-不变缺瓦（N8/A4 守恒不覆盖级别无关性）。
   - 项目记忆 `project_t_cross_level_coupling_falsified`（深层级别短差失血）、`project_t_short_leg_regime_function`（空头腿=regime 函数）：解释为何 `sink_recover_balance`/`per_level_pnl` 必须是观测而非 panic（BTC 已知违反）。
   - 不确定是否存在"同资本 vs 同股数转移"的独立谱系记录——若无，建议结晶（这是 conservation 不可移植的根因）。

6. **影响声明**：
   - 改动 `prove_guards.rs`（+6 守卫 +10 测试）、`t_engine.rs`（4 接入点）、`rec_engine.rs`（4 接入点）。
   - **不改操作逻辑**（接入点纯只读），bit-exact 保持，回测数字不变。
   - 影响模块：recursive_t 验收层；不影响信号层（H⁰ 结构识别）、会计层（fugue_v3::accounting 复用）。
