# fugue_v3「回补信号存在但空头未被回补」根因诊断

**调研类型**：纯诊断，不改引擎代码
**数据通道**：serena `execute_shell_command`（Bash 工具读取产生幻影数据，见 `reference_bash_phantom_sandbox`；本报告所有数字经 serena 复核）
**认识论等级**：现象层 L2（真实数据直接证明）；精确逐 bar 拦截点 L3（需 instrument，本报告给方案不实施）
**诊断对象**：`rust/src/fugue_v3/{operate,cycle,accounting,morphology,observe,axis}.rs` + `trading_system/data_cache/fugue_v3_ES.json`

---

## 0. 执行摘要（结论）

用户的命题「回补信号 fire 了，但引擎没执行回补」在概念上需要拆成三个**不同的退出路径**，它们消费**不同的信号**、由**不同的代码步骤**驱动：

| 退出路径 | 代码步骤 | 触发信号（Long root / Short root） | 标的 | 在 ES 的状态 |
|---------|---------|----------------------------------|------|------------|
| **recover**（D） | operate.rs 240-257 | nf_buy(k) / nf_sell(k) | 机动子仓 @ k−1 | 强 Up 下被饿死 |
| **C 清仓**（C） | operate.rs 207-235 | buy_source≥core ∧ buy1 / sell_source≥core ∧ sell1 | 根仓 @ core_ladder | 高层永不触发 |
| **A 强平**（A） | operate.rs 171-191 | c≥2×basis（被动） | 非根子仓 | 唯一实际了结路径 |

**ES 是做多标普（BH +594%）却巨亏 -226%（final_nav = −126,100，资不抵债）。** 巨亏来自两类**无法平仓的逆势空头**：

- **机制 A（机动空头无法 recover）**：Long root 的 sink 在 seg(2) 开机动空头共 122 次（Σ=57.82 shares 累积成单一仓位），`recover@3` 平 Short = **0 次**，最终价格翻倍触发 A 强平，亏 **−118,410**。
- **机制 B（根空头无法 C 清仓）**：bar 870220 的 Short root 做空仓（σ-ascend 到 L5），`C-clear-short` 需 `buy_source≥5 ∧ buy1(5)`，强 Up regime 下 recL3 级别的买侧 source 极罕见 → 永不清仓，扛到 eod 亏 **−101,999**。

**共同根因**：两条「平空」路径（recover 的 nf_buy(父层)、C-clear 的 buy_source）都依赖**买侧底背驰**。`enable_macd_divergence=True` 下，单边上扬无 MACD 背驰 → 买侧 confirm 不 fire（mod.rs 原文：「单边上扬无背驰 ⟹ 机动仓自然休眠 = 结构性 regime 门」）。这个「regime 门」对**开空（sink，卖侧顶背驰频繁）单向打开、对平空（买侧底背驰稀缺）单向关闭** ⟹ 空头单边累积。

**这不是代码 bug，是引擎设计在强单边 regime 下的有效域边界**（与 `project_constitutive_throughput_falsified`「清仓频率是 regime 函数不可约」同构）。

---

## 1. recover 的完整条件链（用户问题 #1，代码精确）

`operate.rs` D 步骤（240-257）+ `cycle.rs::recover_chunk`（63-98）。recover@k 成功**必须同时满足**：

```
D 步骤前置门（operate.rs）：
  (D1) !cleared                      —— 本 bar C 未清仓（C 清仓短路掉 D/E/F）
  (D2) k ∈ [PENDING_LO=3, MAX_LADDER=11)
  (D3) layers[sub=k−1].units > 0     —— 子层有仓位
  (D4) !touched[k] ∧ !touched[sub]   —— 本 bar k 与 k−1 都未被 A/前序 D 动过
  (D5) signal = (core_polarity==Long ? nf_buy(k) : nf_sell(k)).is_some()

recover_chunk 内部门（cycle.rs）：
  (R1) m = mobile_quota(u_sub) = u_sub/3 > 0 ∧ finite ∧ ≤ u_sub
  (R2) d_sub == flip(core_polarity)  —— 子层方向必须是「父极性的反向」
                                        Long root → 子层须 Short；Short root → 子层须 Long
  (R3) 父层 k 若占用(units>0)，其 direction 必须 == core_polarity
                                        否则 return false（方向不相容拒绝）
```

**关键架构事实**：信号选择由 `parent_dir = self.core_polarity`（全局单一根极性）决定，**不是由子层方向决定**。

- **Long root**：D recover 消费 **nf_buy(k)**，平 Short 子仓（R2 要求子层=Short）。
- **Short root**：D recover 消费 **nf_sell(k)**，平 Long 子仓（R2 要求子层=Long）。

> **用户前提的第一处概念错配**：对 Short root，nf_buy **不是** recover 的触发信号——它是 **E sink** 的触发信号（operate.rs 270）。「nf_buy fire 但没 recover」对 Short root 是**设计使然**，不是 bug。

> **用户前提的第二处概念错配**：recover 的标的子仓在 **sub=k−1**，触发信号在**父层 k**。一个在 ladder 4 的机动空头，recover 它需要 **nf_buy(5)**（父层），不是 nf_buy(4)。「nf_buy[k] fire 但 ladder-k 空头没 recover」是**级别错位**——nf_buy[k] 对应的是 ladder k−1 的子仓。

---

## 2. 信号消费冲突与步骤顺序（用户问题 #2 + #4，代码精确）

### 2.1 每 bar 步骤执行顺序（`operate.rs::step`，171-356）

```
A   边界强平   (171-191)  遍历非核心层，c≥2×basis 强平空头子仓 / c≤basis/2 强平多头子仓
                          → 设 touched[k]，根层(k==core_ladder)永远跳过 ★
σ   σ-ascend   (193-202)  核心仓 relabel 升级（不交易，不设 touched，不读 nf）
C   顶背驰清   (204-235)  buy_source/sell_source≥core ∧ bsp1 → clear_to_cash → 设 cleared=true ★
D   recover    (237-257)  if !cleared：从 k−1 升回 k（见 §1）
E   sink       (259-292)  if !cleared ∧ has_position：从 k 下沉到 k−1（成本门 N4）
F   建仓       (294-356)  if !cleared ∧ !has_position：root_direction 决定方向，source 决定时机
```

### 2.2 「一个 nf_buy 能否同时触发 F 和 D」（用户问题 #2）

**不能**，三重互斥：

1. **F 与 D 持仓互斥**：D 要求 `has_position()`（隐含，layers[sub]>0），F 要求 `!has_position()`。空仓时只有 F，持仓时只有 D/E。
2. **C 短路**：C 清仓置 `cleared=true`，D/E/F 全部 `if !cleared` 跳过。**若 C 清仓与 nf_buy 同 bar，nf_buy 被丢弃**（observe 链被 engine.rs 81-87 reset）。
3. **touched 互斥**：D 成功后置 `touched[k]=touched[sub]=true`，同 bar E 对同一层对跳过。A 强平的层也 touched，D 跳过。

### 2.3 谁能在 recover 之前消费信号（用户问题 #2）

按顺序 A→σ→C→D。在 D 之前：

- **A 强平**：若 nf_buy 那个 bar，子层 k−1 或父层 k 已被 A 强平（touched），D 跳过该层对。
- **C 清仓**：若该 bar 触发 C（buy_source≥core ∧ buy1），`cleared=true`，**D 完全不执行**，且 clear_to_cash 已把所有层（含机动空头）清到现金。**此时机动空头是被 C「连带清掉」，不算 recover，会计上记为 `core_clear`/`core_clear_short` 而非 `recover`。**

---

## 3. ES 数据验证：僵尸空头解剖（用户问题 #3 + #5）

### 3.1 全局（serena 复核）

```
ES   bars=5,589,928  strat=-226.1%  BH=+594.3%  P1=FAIL  final_nav≈-126,100（资不抵债）
     n_trades=311 (多 237 / 空 74)  entries=16 core_clears=15 liq=1  sink=182 recover=107
     最后一次 C 清仓 @ bar 823,576 → 之后 476万 bar 无任何清仓（超长未清仓段）
```

### 3.2 eod 僵尸仓位（数据结尾仍存活）

```
L5(recL3) short  eb=870220   shares=22.4995  basis=2834.63→7368.00  pnl=-101,999  ← 机制B 根空头
L3(move ) long   eb=1014440  shares=0.0000   ...                    pnl=0         （空壳残留）
L4(recL2) short  eb=3395859  shares=0.0000   ...                    pnl=-0        （空壳残留）
L2(seg  ) short  eb=4049299  shares=0.0000   ...                    pnl=-0        （空壳残留）
```

### 3.3 P&L 分解（确认巨亏来源）

```
liq_short   Σpnl = -118,410   机制A：seg 机动空头被 2×basis 强平
eod         Σpnl = -101,999   机制B：L5 根空头 MtM 负债扛到结尾
reduce      Σpnl =  -42,849   逆势做空降成本的 sink 短差净亏
recover     Σpnl =  +23,504   recover 短差小赚（远不足抵消）
core_clear* Σpnl =  +13,628
```

### 3.4 机制 A：机动空头无法 recover（seg 层平衡表）

```
              Short注入(sink@3)   recover@3平Short   净累积   强平
seg(2)              122                0              120      2
```

**122 次 Long root sink@3 在 seg(2) 注入 Short（Σ=57.82 shares），累积成单一连续仓位
（entry_bar=1805937，basis 加权 2047.66），recover@3 平 Short = 0 次，
价格翻倍（2047×2=4095）@ bar 3763549 触发 A 强平，亏 -118,410。**

`recover` 的极性归属（决定性对照）：

```
ES  recover trades:  L2 long n=28, L3 long n=79   ← 全是 pol=long（Short root 平 Long 子仓）
                     L2 short = 0 ★               ← Long root 的 Short 子仓 recover 0 次
CL  recover trades:  L2 long n=107, L2 short n=51 ★, L3 long n=58, L4 long n=4
                                    ↑ Long root 的 Short 子仓被正常 recover 51 次
```

> nf_buy(3) = **4338 次** fire，但 Long root 周期的 recover@3（平 seg Short）= **0 次**。

### 3.5 机制 B：根空头无法 C 清仓（L5）

bar 870220 建立 Short root 做空仓（F-entry @ L4，σ-ascend 到 L5——所有 870220 的 trade
共享 entry_bar=870220，L4 short 被反复 sink 减仓 + L5 short 扛到 eod）。

退出唯一路径 = `C-clear-short`，条件 `buy_source≥core_ladder=5 ∧ buy1(5)`。
- `nf_buy[5] = 12` 次，`nf_buy[6] = 1` 次（裸信号已极稀）
- `buy_source≥5`（区间套定位贯通到 recL3）比裸 nf_buy 强得多 → 实测从未满足
- ES 一路涨（做空逆势），L5 Short 核心 basis 2834→7368 扛到 eod，亏 -101,999

### 3.6 nf fire 的 regime 不对称（核心根因证据）

```
        nf_sell(开空/sink源)   nf_buy(平空/recover源)   sell/buy
move(3)      5184                 4338                  1.2x
recL2(4)       98                  145                  0.7x
recL3(5)        2                   12                  0.2x
recL4(6)        0                    1                  0.0x
```

裸 fire 计数 nf_buy 不算少（4338 @ move）。但**关键不是全周期总量，而是「平空信号是否落在逆势空头存在的时间窗口内」**。在 823576→eod 的 476万-bar 强上涨段：

- 卖侧顶背驰（nf_sell → sink 开空）：每个上涨段末端周期性出现 → sink 持续开空
- 买侧底背驰（nf_buy → recover 平空 / buy_source → C 清仓）：强上涨段回调浅、力度弱，
  MACD 难形成标准底背驰 ⟹ 买侧 confirm 在该窗口系统性缺失 ⟹ 平空信号饿死

---

## 4. 为什么 CL 能正常 recover（用户问题 #3 对照）

```
CL   bars=5,528,156  strat=+29.4%  BH=+28.2%  P1=PASS（震荡 regime，接近 BH）
     recover L2 short = 51 次（机动空头正常 recover）
     eod 僵尸：basis 76→90 / 94→90，pnl ±数千（不是 ±十万量级）
```

CL 是震荡 regime：上涨段与下跌段交替，**买侧底背驰（nf_buy）与卖侧顶背驰（nf_sell）大致平衡**
（move 层 sell/buy = 5597/5298 ≈ 1.06）。机动空头开仓（sink）后，回调真实出现 → nf_buy(父层)
在子仓存在窗口内 fire → recover 正常工作（51 次）→ 空头不累积 → 无灾难性僵尸。

**判别量：`move 层 nf_sell/nf_buy 比` + `是否存在长期无 C 清仓段`。ES 强 Up = 平空信号饿死；CL 震荡 = 双向平衡。**

---

## 5. 精确逐 bar 拦截点（用户问题 #5，L3 待实证 + instrument 方案）

§3.4 已确凿证明 `recover@3 平 Short = 0`（数据层 L2）。但「每次 nf_buy(3) fire 的那个 bar，
D 步骤的条件检查具体被哪一道门拦住」是**逐 bar 事件**，缓存的聚合计数器无法分辨。

**结构推导（代码层）收窄到唯一一致解**：§1 的 D1-D5/R1-R3 中，在 Long root + seg 有 Short +
nf_buy(3) fire 的情况下，R1（配额>0）、R2（子层=Short ✓）、R3（父层 move 在 Long root 为
Long 或空）理论上都可满足。实测 0 次 ⟹ **唯一一致解释是 D5 的「nf_buy(3) fire」与「seg Short
存在」两个时间窗口系统性不重叠**——即 nf_buy(3) 的 4338 次 fire 几乎全部落在 seg Short 累积期
（1805937–3763549）**之外**（强 Up 段买侧背驰缺失，escalation `2026-06-15-confirm-arming-differance`
的 confirm 向心读法在此显形）。

**钉死方案（不入生产，诊断专用）**：在 `operate.rs` D 步骤每次 `signal.is_some()` 为真但
`recover_chunk` 返回 false（或 layers[sub]<=0 / touched 跳过）时，把
`(bar, k, core_polarity, layers[sub].direction, layers[sub].units, layers[k].direction, reason)`
推入一个 `Vec<DiagRow>`，finish 时落盘。重跑 ES 即可逐 bar 列出「nf_buy(k) fire 时 D 被拦在哪道门」。
预测结果：绝大多数 nf_buy(3) fire 时 `layers[2].units==0`（seg 当时无 Short）⟹ D3 跳过（时间错配假设）。

---

## 6. 结果包六要素

**结论**：ES「空头未回补」是两个独立机制叠加——(A) Long root 机动空头因强 Up regime 买侧底背驰
饿死而 recover@父层 失败，累积到 2×basis 强平（−118,410）；(B) Short root 根空头因
`buy_source≥core_ladder∧buy1` 在高 ladder 强 Up 下永不满足而 C 清仓失败，扛到 eod（−101,999）。
非代码 bug，是 sink/recover 降成本循环在强单边 regime 下的有效域边界。

**定义依据**：
- recover 条件链 = `operate.rs` 240-257 + `cycle.rs::recover_chunk` 63-98（§1，R2 子层方向 = flip(core_polarity)）
- C 清仓条件 = `operate.rs` 222-233（Short root：`buy_source≥core_ladder ∧ buy1(s)`）
- regime 门 = `mod.rs` 第 9 行「enable_macd_divergence=True ⟹ 单边上扬无背驰 ⟹ 机动仓自然休眠」
- A 强平根层豁免 = `operate.rs` 175（`k == self.core_ladder` 跳过）⟹ 根空头永不被 A 了结

**边界条件（结论翻转条件）**：
- 若 regime 转为震荡（买侧/卖侧背驰平衡，如 CL），recover 与 C 清仓恢复 → 无僵尸（CL 实证）。
- 若 nf_buy(3) fire 实际**落在** seg Short 累积窗口内而仍 recover=0，则时间错配假设被否证，
  根因转为 R3 父层方向冲突或 touched 互斥（需 §5 instrument 区分）。
- 若把 C-clear-short 的 `buy_source≥core_ladder` 放宽为裸 nf_buy(core_ladder)，机制 B 部分缓解
  （但会引入早平空 → 需 L3 回测验证，不在本诊断范围）。

**下游推论**：
- fugue_v3 在强单边趋势标的（ES/QQQ/BTC 牛市侧）做空方向结构性踏空——与 memory
  `project_unn_btc_spawn_throwback`（E spawn 强牛过度做空）、`project_t14_t5_root_flip_necessity`
  （根翻空在震荡正域否证）同构：**逆势空头的有效域 ⊂ 非强单边 regime**。
- 「平空信号 = 买侧底背驰」对 sink/C-clear 双路共用 ⟹ regime 门一关，两条退出同时失效 ⟹
  空头单边累积是**结构性单点失效**，不是两个独立 bug。

**谱系引用**：
- `2026-06-15-confirm-arming-differance.md`（confirm 向心读过去 vs 当下合取，矛盾 open）——§5 时间错配的理论来源
- `project_constitutive_throughput_falsified` / `project_nrf_v4_strict_accounting`：清仓频率是 regime 函数不可约
- `reference_bash_phantom_sandbox`：本诊断数据全程经 serena 复核（Bash 工具首次读取产生过幻影 4631/6182 数字）
- 不确定是否有「sink/recover 级别错配」专属谱系——`.chanlun/escalations/2026-06-16-spiral-pclose-l2-vs-l3.md` 可能相关，未深查

**影响声明**：
- 本报告**未改动任何引擎代码**。
- 新增诊断脚本 `analysis/_diagnose_es_zombie_short.py`（一次性，读缓存）。
- 揭示的有效域边界（逆势空头 ⊂ 非强单边 regime）若要修复，触及 sink 开仓的 regime 门控
  与 C-clear 的 source 阈值——属定义层调整，需走 escalate/回测，不在本诊断授权范围内。
