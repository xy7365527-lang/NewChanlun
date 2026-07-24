# V4 出场路径审计：完全平仓 vs 短差分流

> Issue: #137 (wayfinder:research, AFK)
> Parent: #135
> 前置: #136（两套出场系统辨识）
> worktree: `/tmp/kimi-nest-mainline` @ `640609071d`
> 纪律: 纯调研——只读分析源码，零代码改动、禁 cargo、禁 git mutation

## 结论速览（TL;DR）

| 维度 | 结论 |
|------|------|
| **V4 走哪套出场** | **有机赋格引擎（Theta π 闭环）**，即 `pi_theta_step_traced`（coverage.rs:2907）解释器 + TW 账本组合层。`closed_loop/sell.rs`（SellDecision CloseRoot/ReduceCore）**未被 V4 路径调用**——它是 Lean Origin port 的结构验证组件（L0/L1），存在于 `run_closed_loop` 旁路。 |
| **完全平仓路径** | ① 反向候选触发：`interpret_with_close_triggers` 规则2 → `StepTrace.closed` → `reverse_exit_type` 判 CloseRoot/ReduceCore。② P1 强平：`gate.force_flat` → `StepTrace.risk_exits` → `ExitType::RiskExit`。③ 静默结构失效（根腿）：`silent_drops` + `entry_v==Ambient` → `ExitType::CloseRoot`。 |
| **短差路径** | ① 反向关闭子声部：`closed` + `entry_v==ShortDiff` → `ExitType::CloseShortDiff`。② 静默连带剪（子声部）：`silent_drops` + `entry_v!=Ambient` → `ExitType::CloseShortDiff`。③ P2 overlay 关闭：`overlay_closes`（TW StageII）→ `ExitType::CloseShortDiff`。 |
| **级别感知** | **是**。`interpret_with_close_triggers` 规则2 按**同级别**反向关闭（`level_idx` HashMap 预索引，只匹配 `c.level == leg.level` 的腿）。排序 θ_key 也含 level 维（高 level 先）。V4 = 有机引擎 = 级别感知。 |
| **两套混用？** | **否**。V4 生产路径（`run_theta_v0_pi_overlay` → `pi_theta_fill_loop_overlay`）完全走有机引擎。`run_closed_loop` 只在 V4 末尾调一次产 `closed_loop_final` 结构证据（`Option<AssemblyState>`），其 `SellDecision` 逻辑**不参与**订单/出场决策。`SellDecision::CloseRoot/ReduceCore` 与 `ExitType::CloseRoot/ReduceCore` 同名但不同枚举、不同代码路径。 |
| **exit_type 来源** | 全部来自**有机引擎** `interp::ExitType`（interp.rs:224 五枚举）。`close_loop::SellDecision` 的 `CloseRoot/ReduceCore` 是同名独立枚举，不进 V4 trades。 |

---

## 一、V4 调用链概观

```
run_theta_v0_pi_overlay (runner.rs:640)
  └─ pi_theta_fill_loop_overlay (fill.rs:392)
       └─ 逐 bar 主循环 (fill.rs:562 `for i in 0..n`)
            ├─ ① 延迟成交 apply_order (fill.rs:110) —— 净额执行层
            ├─ ② 分类 classify_at(i) —— 前缀因果塔
            ├─ ③ pi_theta_step_traced (coverage.rs:2907) —— 有机引擎决策核 ★
            │    ├─ P1 force_flat → risk_exits
            │    ├─ P2 CloseOverlay → overlay_closes
            │    ├─ P3/P4 TW stage → tw_event
            │    └─ P5-P10 interpret_with_close_triggers → closed/silent_drops
            ├─ ③' 消费 StepTrace 四桶 → typed_ledger（exit_type 赋值）
            └─ ④ 挂单到 exec_index
  └─ run_closed_loop (runner.rs:836) —— 旁路结构验证，产 closed_loop_final
```

**关键事实**：`run_closed_loop` 与 `pi_theta_fill_loop_overlay` 是**两条独立路径**。前者只跑 `hybrid_step`（transition.rs）+ `recog_chanlun_sell`（sell.rs），产出 `AssemblyState` 终态作为诊断证据字段。后者才是生产路径——所有订单、trades、exit_type 都从这里出。

---

## 二、四个出场循环详解（fill.rs:1169-1293）

### 2.1 `closed`（fill.rs:1170-1197）—— 反向信号关闭

**来源**：`StepTrace.closed`（coverage.rs:3036）= `buckets.close` × `close_triggers`

**上游**：`interpret_with_close_triggers`（interp.rs:1165）规则2——候选 g 的 bits 反向关闭**同级别**活动腿。

**触发条件**：当一个反向买卖点候选出现（持多遇 sell1/sell2/sell3；持空遇 buy1/buy2/buy3），且与活动腿**同 level**，则关闭该腿。

**exit_type 赋值**（fill.rs:1180）：
```rust
exit_type: interp::reverse_exit_type(open.entry_v, trig.bsp_class)
```

`reverse_exit_type`（interp.rs:250）判据：
- `entry_v == ShortDiff` → **CloseShortDiff**（短差子声部关闭）
- `trigger_class == 3` → **ReduceCore**（三类反向=核心仓减仓）
- else → **CloseRoot**（一/二类反向=根清仓）

**完全平仓 vs 短差**：
- CloseRoot / ReduceCore = **完全平仓**（根腿/核心腿整体反向退出）
- CloseShortDiff = **短差**（子声部对冲腿关闭）

### 2.2 `silent_drops`（fill.rs:1201-1233）—— 静默结构失效

**来源**：`StepTrace.silent_drops`（coverage.rs:3041）= prev_active 中既未被 close 桶认领、也不在 next_active 的腿（AncOK 剪枝 / Stale prune）。

**触发条件**：§13 结构剪枝——父腿被关/失效后，子腿连带被剪掉（AncOK 连带剪）；或腿因塔结构演化变 Stale 被清除。**无独立反向信号触发。**

**exit_type 赋值**（fill.rs:1208-1212）：
```rust
let exit_type = if open.entry_v != Vertical::Ambient {
    ExitType::CloseShortDiff  // 子声部（ShortDiff/FollowParent）→ 短差
} else {
    ExitType::CloseRoot       // 根腿（Ambient）→ 完全平仓
};
```

**完全平仓 vs 短差**：
- 根腿（Ambient）结构失效 → **CloseRoot**（完全平仓语义）
- 子声部（ShortDiff / FollowParent）连带剪 → **CloseShortDiff**（短差语义）
- `via_structural_prune: true` 标记——μ 估计器可据此分离结构剪枝与信号平仓

### 2.3 `risk_exits`（fill.rs:1237-1262）—— 强平清空

**来源**：`StepTrace.risk_exits`（coverage.rs:2937）= `gate.force_flat` 时 prev_active 全部腿。

**触发条件**：P1 优先级（PDF §7 C_1 全互斥强平）——保证金不足 / Insolvent / Liquidation 风控态触发 `force_flat`，屏蔽所有其他出场决策（P2-P10），活动腿全部清仓。

**exit_type 赋值**（fill.rs:1247）：
```rust
exit_type: ExitType::RiskExit
```

**完全平仓**——整个仓位被风险门强制清零，是最彻底的完全平仓。

### 2.4 `overlay_closes`（fill.rs:1267-1292）—— TW StageII 重叠腿关闭

**来源**：`StepTrace.overlay_closes`（coverage.rs:2951）= TW `Stage::CapitalRecovered`（StageII）且 H>0（有 legacy ShortDiff 重叠腿）时，关闭这些重叠腿。

**触发条件**：P2 优先级（PDF §7 C_2）——TW 账本判定资本已回收（StageII），且仍有 legacy ShortDiff 腿在飞 ⟹ 关闭重叠腿（减少短差对冲暴露）。

**exit_type 赋值**（fill.rs:1277）：
```rust
exit_type: ExitType::CloseShortDiff
```

**短差**——关的是 legacy ShortDiff 重叠腿（短差对冲仓位），非根仓位。

---

## 三、完全平仓 vs 短差——分流总表

| 出场桶 | exit_type | 出场方式 | 触发条件 | 路径来源 |
|--------|-----------|----------|----------|----------|
| `closed` | CloseRoot | **完全平仓** | 一/二类反向候选关根腿 | interpret 规则2 + reverse_exit_type |
| `closed` | ReduceCore | **完全平仓**（减核） | 三类反向候选关核心腿 | interpret 规则2 + reverse_exit_type |
| `closed` | CloseShortDiff | **短差** | 反向候选关 ShortDiff 子声部 | interpret 规则2 + reverse_exit_type |
| `silent_drops` | CloseRoot | **完全平仓** | 根腿（Ambient）结构失效/连带剪 | §13 AncOK/Stale prune |
| `silent_drops` | CloseShortDiff | **短差** | 子声部（≠Ambient）结构连带剪 | §13 AncOK/Stale prune |
| `risk_exits` | RiskExit | **完全平仓** | force_flat 强平 | P1 风控门（gate.force_flat） |
| `overlay_closes` | CloseShortDiff | **短差** | TW StageII 关 legacy ShortDiff 腿 | P2 TW 账本谓词 |
| （窗口末尾） | Hold | 不出场 | 窗口内无出场信号 | censored 兑现（fill.rs:1495） |

### 净额执行层如何区分

`schedule_order`（coverage.rs:2692）根据 `p* - p_t` 的符号关系产生 `StrictAction`：
- **Close**（`flat_next`，p*→0）：完全平仓到空仓——对应 CloseRoot/RiskExit 的净额效果
- **Reduce**（同号减幅）：部分减仓——对应 ReduceCore/CloseShortDiff 的净额效果
- **Buy/Sell**（反号穿零）：先平后开翻转——完全平仓 + 反向开仓

净额层（`apply_order`/`apply_fill`）是**单一合并账本**——不区分腿级 exit_type，只看 net 持仓变化。exit_type 的区分纯粹在 **typed ledger 层**（腿级会计）做。

---

## 四、V4 trades 的 exit_type 名称来源

V4 trades 的 `exit_type` 字段全部来自 **`interp::ExitType`**（interp.rs:224），五枚举：

| ExitType | 语义 | 来源桶 | 填充位置 |
|----------|------|--------|----------|
| **CloseRoot** | P5 根清仓 | closed / silent_drops | reverse_exit_type / silent_drops Ambient 分支 |
| **ReduceCore** | P6 减核仓 | closed | reverse_exit_type（trigger_class==3） |
| **CloseShortDiff** | P7 短差关腿 | closed / silent_drops / overlay_closes | reverse_exit_type / silent_drops 非 Ambient / overlay_closes 固定 |
| **RiskExit** | P1 强平 | risk_exits | risk_exits 固定 |
| **Hold** | 窗口末尾未出场 | censored | fill.rs:1495 兑现 |

**名称不来自 `SellDecision`**。`closed_loop/sell.rs::SellDecision` 虽有 `CloseRoot`/`ReduceCore` 同名变体，但该枚举仅在 `run_closed_loop` → `hybrid_step` → `schedule_adapter` → `recog_chanlun_sell` 链条中流转，产出的是 `AssemblyState` 结构证据，不进 V4 trades。

源码注释（interp.rs:220-222）明确声明：
> `closed_loop/sell.rs::SellDecision` 已有 CloseRoot/ReduceCore 重叠（disjoint 路径，G4 把 μ 管线重接生产 π 后该路径废）——统一收敛到本枚举，届时删 SellDecision 侧（升级路径，非现在做）。

---

## 五、两套系统是否混用

**否。** V4 生产路径是纯有机引擎：

1. **决策路径**：`pi_theta_step_traced`（有机引擎）→ `interpret_with_close_triggers` → `StepTrace` 四桶 → typed_ledger。整个链条在 `strategy/coverage.rs` + `strategy/interp.rs` + `backtest/fill.rs` 内。

2. **`run_closed_loop` 是旁路**：在 V4 runner（runner.rs:692）末尾调一次，产出 `closed_loop_final: Option<AssemblyState>`，存入 `RunResult` 作结构验证证据。它的 `SellDecision` 逻辑**完全不参与** V4 的订单/出场/trades 产出。源码注释（runner.rs:834）：
   > 其输出仅作 `closed_loop_final` 结构证据，与生产 π 订单流 disjoint

3. **两套同名但不互通**：
   - `SellDecision::CloseRoot`（sell.rs:51）→ Lean port，ledger delta `(Π+1, A-1, 0)`，AssemblyState 结构验证
   - `ExitType::CloseRoot`（interp.rs:226）→ 生产 typed trade，腿级会计
   - 二者是不同枚举、不同模块、不同代码路径，只是名字一样（源码注释已标记为待收敛的 tech debt）

---

## 六、级别感知

**V4 路径（有机引擎）级别感知 = 是。** 具体体现：

1. **反向关闭按同级别匹配**（interp.rs:1201）：
   ```rust
   level_idx.get(&c.level).and_then(|idxs| {
       idxs.iter().copied().find(|&i| {
           let (leg, closed) = &working[i];
           !*closed && reverse_signal(leg.dir, &c.bits)
       })
   })
   ```
   候选只关**同 level** 的活动腿（`level_idx` HashMap 按 level 预索引）。

2. **θ_key 排序含 level 维**（interp.rs:1071-1079）：候选排序 `Reverse(c.level)`——高 level 先处理（spec:54 高 level 优先）。

3. **腿有 level 属性**（`ActiveLeg.level`）——入场时从 `CoverageElement.level` 继承，出场时按同 level 匹配。

4. **`Vertical` 枚举**（coverage.rs:927）区分 Ambient / FollowParent / ShortDiff——`GradeRel`（coverage.rs:950）进一步区分 SameLevel / SubLevel。

**对比 `closed_loop` 旁路**：`run_closed_loop` 的事件流是 `AssemblyEvent` = `(NewBar(rising), price)`，只有 rising/falling 布尔——**无级别信息**。`recog_chanlun_sell` 只消费 `SellEndpoint`（卖点端点），不涉及多级别塔。所以如果以 `closed_loop` 为参照系，它的级别感知 = 否；但它不是生产路径。

---

## 七、源码索引

| 文件 | 行号 | 内容 |
|------|------|------|
| `rust/src/theta_v0/backtest/runner.rs` | 640 | `run_theta_v0_pi_overlay` V4 入口 |
| `rust/src/theta_v0/backtest/runner.rs` | 836 | `run_closed_loop` 旁路结构验证 |
| `rust/src/theta_v0/backtest/fill.rs` | 392 | `pi_theta_fill_loop_overlay` 主循环 |
| `rust/src/theta_v0/backtest/fill.rs` | 110 | `apply_order` 净额执行 |
| `rust/src/theta_v0/backtest/fill.rs` | 1170 | closed 桶消费（反向关闭） |
| `rust/src/theta_v0/backtest/fill.rs` | 1201 | silent_drops 桶消费（静默离场） |
| `rust/src/theta_v0/backtest/fill.rs` | 1237 | risk_exits 桶消费（强平） |
| `rust/src/theta_v0/backtest/fill.rs` | 1267 | overlay_closes 桶消费（TW P2） |
| `rust/src/theta_v0/backtest/fill.rs` | 1495 | Hold exit_type（窗口末尾） |
| `rust/src/theta_v0/strategy/coverage.rs` | 2848 | `StepTrace` 结构体定义 |
| `rust/src/theta_v0/strategy/coverage.rs` | 2907 | `pi_theta_step_traced` 决策核 |
| `rust/src/theta_v0/strategy/coverage.rs` | 2930 | P1 force_flat 短路 |
| `rust/src/theta_v0/strategy/coverage.rs` | 2950 | P2 CloseOverlay |
| `rust/src/theta_v0/strategy/coverage.rs` | 3036 | closed/silent_drops 差分 |
| `rust/src/theta_v0/strategy/coverage.rs` | 2692 | `schedule_order` 净额订单 |
| `rust/src/theta_v0/strategy/interp.rs` | 224 | `ExitType` 枚举定义 |
| `rust/src/theta_v0/strategy/interp.rs` | 250 | `reverse_exit_type` 判据 |
| `rust/src/theta_v0/strategy/interp.rs` | 1165 | `interpret_with_close_triggers` fold |
| `rust/src/theta_v0/strategy/exec.rs` | 255 | `reverse_signal` 反向谓词 |
| `rust/src/theta_v0/closed_loop/sell.rs` | 50 | `SellDecision` 枚举（旁路） |

---

*审计人：GitHub issue #137 wayfinder:research（AFK）*
*日期：2026-07-22*
