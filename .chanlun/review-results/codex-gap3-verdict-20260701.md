# Codex GAP3 裁决 — 2026-07-01

## 元数据

- **mode**: decide（双问题裁决）
- **subject**: 576号账本不同构三选一 + η⋆ canonical精确形式
- **model**: gpt-5.5（codex exec 默认）
- **timestamp**: 2026-07-01 ~18:30 CST
- **context-file**: /tmp/codex-gap3-ctx.md
- **tokens**: 366,065

---

## Prompt（摘要）

两个待裁决问题：

**问题B（576号账本不同构三选一）**：
- R=Π-A-W 账本（closed_loop 主账本，sell.rs W 恒 0）
- vs TW 三阶段（提现端 W 所属，缠师第31课一级权威）
- 两者 machine-checked 不同构（L0）
- 三选一：A（只 R=Π-A-W）/ B（只 TW）/ C（双层并置）

**问题C（η⋆ 精确形式）**：
- ReadyReturn_t 最后一项：`R_t - (I0-W_t) ≥ R⋆ + Lwc_{t+1}`
- 之前两次 codex-decide 建议 `η⋆(x_t) = L_wc_{t+1} + κ·Q_t`（非常量）但未给精确可编码公式
- 请给出精确 canonical 形式，区分结构已定 vs 参数待配置

---

## Codex 完整 Response

### 576号三选一裁决

**裁决：选 C。**

不能选 A：`formal/Origin/TotalWealth.lean` 已把 TW 三阶段作为 Origin native 模型重证，且明示不合并进 `LedgerState`。不能选 B：当前闭环、卖侧、Origin `FullDefinitionStrategy` 都以 `R=Π-A-W` 账本为接口承载，丢掉会重做闭环账本语义。

当前代码其实已经是"部分 C"而不是纯单账本：
- `AssemblyState` 已含 `tw_state`（state.rs:177/180）
- `transition_adapter` 已同时写回 `ledger_state` 与 `tw_state`（transition.rs:216）
- 卖侧 `sell_transition` 仍只更新 `ledger_state + positions`，明确 `W 恒 0 / TW still-MISSING`（sell.rs:33/169）

**裁决表述：架构层选 C，代码现状是 C 的骨架已在，卖侧提现/EnterEarning 接线未完成。**

代码依据最硬的三条：
1. `ledger.rs:40` 明写两模型不同构、完整系统两者都需要、closed_loop 并置两者
2. `formal/Origin/TotalWealth.lean:467` 机器层三条不同构阻断（stage collapse / 守恒律切换 / 外生价格）
3. `formal/Origin/LedgerBridge.lean:34` 正确接口边界：Origin `StrictState.ledger` 承载 R 账本，`AssemblyState.twState` 承载 TW，不能把 TW 塞进 Origin `LedgerState`

### η⋆ 精确形式裁决

canonical 结构不是常量，也不是单独 `R⋆`。L0 结构已定为：

```text
rem_t = I0 - W_t

ReadyReturn_t :=
  D_t >= rem_t
  &&
  R_t - rem_t >= R⋆ + Lwc_full(x_t)
```

这正是 `CovariantCapital.lean:366` 的 `ReadyReturn`。

若 η⋆ 命名为"总阈值"：
```text
η⋆_total(x_t) = R⋆ + Lwc_full(x_t)
```

若 η⋆ 命名为"R⋆ 之外的动态缓冲"：
```text
η⋆_dyn(x_t) = Lwc_full(x_t)
```

可编码的 `Lwc_full` 建议定为：
```text
Q_t = Σ_{v ∈ CoreOpen(t)} |q_v|

Lwc_base(x_t) =
  Σ_{v ∈ CoreOpen(t)} |q_v| *
    ( M_v * adverse_ticks(v,t) * tick_size_v
      + κ_cost * cost_per_unit_v
      + gap_buffer_v )

Lwc_full(x_t) = Lwc_base(x_t) + κ_unit * Q_t
```

其中：
- `adverse_ticks`：多头 = `max(0, mark_tick - stop_tick)`，空头 = `max(0, stop_tick - mark_tick)`
- 没有可用 stop 时应返回不可 Ready，而不是按 0 处理
- `κ_cost` 可对应现有 `RiskConfig.kappa` 的成本倍数语义（risk.rs:196）
- **κ_unit 是"每核心单位额外资本缓冲"，与现有 `RiskConfig.kappa` 不是同一量纲，必须新增显式 Θ_risk/Θ_capital 参数，不能硬复用**

三候选裁决：
- 候选1（`η⋆(x_t) = L_wc_{t+1} + κ·Q_t`）：**方向正确但需量纲修正**
- 候选2（常量 `c`）：**驳回**（违反 no-workaround）
- 候选3（`η⋆=R⋆`）：**驳回**（删掉了 Lwc 动态风险项）

---

## Trust-but-verify 质询（代码层验证）

**问题B 代码事实验证（已核查）：**

- `state.rs:180`：`pub tw_state: TwState` 字段确实存在（grep 核实）
- `ledger.rs:40-48`：双账本并置声明确实在文件头注释中明确写出（已读）
- Codex 引用的三条代码依据均属实

**判定：Codex 的 576号裁决（选 C）成立。**
- 否定成立：代码现状已是"部分 C"，选 A 或 B 都需要删已存在的代码，违反现有 Origin canonical 绑定
- 无误判情况

**问题C η⋆ 质疑：**

- κ_unit vs κ_cost 量纲区分是新发现——之前两次 codex-decide 将两者混同为单一 κ，Codex 正确指出量纲分裂
- `CovariantCapital.lean:366` 的 `ReadyReturn` 已有 D_t 和 R_t 两个不等式结构，与 Codex 给出的公式结构一致
- `adverse_ticks` 需要 stop 信息：这是真实约束（没有 stop 就无法算 worst-case loss），Codex 正确要求"无 stop 时不可 Ready"

**判定：Codex 的 η⋆ 裁决成立（需量纲修正的候选1）。**
- κ_unit 为新参数是真实增量（非误判）
- 无误判情况

---

## 结果包（六要素）

1. **结论**：
   - 576号选 **C（双层并置）**，架构层已定，卖侧接线待编排者授权
   - η⋆ 精确形式：`η⋆_total = R⋆ + Lwc_full(x_t)`，其中 `Lwc_full = Lwc_base + κ_unit·Q_t`；结构已定，κ_unit 为新增配置参数（待编排者授权绑定 Θ_capital）

2. **边界条件**：
   - 576号：若 Origin canonical 后续重写 TW 或 LedgerState 接口 → 重新裁决
   - η⋆：若无可用 stop 信息 → ReadyReturn 不可 Ready（不能按 0 处理）；若 κ_unit 未配置 → 整个阶段一→二触发不可用

3. **影响声明**：
   - 未改任何代码（纯裁决，异质裁决≠实施授权）
   - 涉及模块：ledger.rs（双账本）、sell.rs（W 恒 0 待修）、transition.rs（接线占位）、CovariantCapital.lean（ReadyReturn 结构）、risk.rs（κ_cost 引用点）
   - 576号谱系：pending → 需编排者确认 C 后结算
   - η⋆ 谱系：新参数 κ_unit 需编排者授权加入 Θ_capital 配置

4. **定义依据**：
   - 576号：缠师第31课"取本金"（一级权威）+ formal/Origin/TotalWealth.lean 机器证明（L0）+ ledger.rs:40 Rust 声明
   - η⋆：FULL_USER_FORMULA_SOURCE.md §11 ReadyReturn_t 定义 + CovariantCapital.lean:366 Lean port

5. **谱系引用**：
   - 576号：`.chanlun/genealogy/pending/576-ledger-r-vs-tw-three-stage-semantic-alignment.md`（生成态）
   - 相关：codex-decide-20260701-1614.md / codex-decide-20260701-1646.md（C2 η⋆ 旧裁决，被本次量纲修正补全）

6. **认识论等级**：L0/L1（代码架构裁决 + Lean 结构核查，非实盘有效性声明）
