# codex-margin 裁定：D2 真保证金模型设计（#103 实装前置）

- **工位**：ws-codexmargin（topo_address: swarm/ws-codexmargin，parent_callback: main）
- **日期**：2026-07-03
- **审计对象**：`.chanlun/review-results/margin-model-design-20260703.md`（D2 真保证金模型设计全文）
- **上游**：`.chanlun/review-results/codex-p2-design-ruling-20260702.md` §D2（fail，本次设计的直接否定源）
- **Codex 完整交互**：`.chanlun/review-results/codex-review-20260703-020619-4a5a.md`（自动持久化）
- **裁定结论**：**conditional-pass**——翻转条件成立，但设计文档本身存在 3 处致命自洽性缺口，不修复不能进实装。

---

## 1. 结论

### 总裁定：conditional-pass

「规则表 = 版本化 datum，非 Θ 参数」的存在论移动**成立**——Codex 确认这确实翻转了上轮 D2 fail
的核心否定（虚构 config 保证金率 → venue 规则表快照，是不同对象）。但设计文档本身在**推导到具体
公式/签名的层面**留下三处致命自洽性缺口，我逐条核实（读代码原文）后确认均**真实存在，非 Codex
误读**：

#### 致命缺口（3 项，逐条核实通过）

1. **历史快照策略缺失**（设计 §1.1 line 52 / §2.1 line 99）。设计承认「These values change over
   time」，但只写「拉一次落 snapshot_date」，未定义回测中 bar 的历史日期应对应哪个 snapshot——
   用最新快照跑历史区间是时间错配。**核实**：设计原文确实只给单快照机制，§4「接入后」章节也未提
   分段快照，缺口成立。
2. **单位错误：net_notional 二次乘 mark_price**（设计 §2.2 line 114 vs `risk.rs:437` `VoiceNotional`
   文档注释）。`VoiceNotional.notional_mag` 的文档明确写「单位美元」（`M_v·P_v·q_v` 已完成折算），
   `net_notional()`（`risk.rs:478`）对这些美元值求和取绝对值，**结果已是美元**。设计公式
   `N_t = |Σ_v signed_notional(v)| · mark_price` 对一个已经是美元的量再乘一次价格，量纲错误（美元
   → 美元·价格）。**核实**：读 `VoiceNotional` 结构体文档（`notional_mag: i64`，"单位美元"）与
   `net_notional` 实现，确认二次相乘是真实 bug，非 Codex 误读设计意图。
3. **`liquidation_flag` 签名自相矛盾**（设计 §2.4 line 146）。设计声明 v0 强平谓词是
   `liq_flag ⟺ E_t ≤ MM_t`，但给出的函数签名 `fn liquidation_flag(positions, mark, schedule) -> bool`
   **没有 equity 参数**——无法计算这个谓词所声称的判据。**核实**：直接对照设计原文两处（谓词声明
   与签名声明），两者不自洽是设计文档自身的矛盾，不是审查者的过度苛求。

#### 重要缺口（2 项，逐条核实通过）

4. **M2/M3 未被现有门消费**。`risk.rs:370` `global_risk_close()` 明确 `matches!(mode, Insolvent |
   Liquidation)`——只吸收 M0/M1。`runner.rs:489` `k_theta_risk_gate` 只把 `global_risk_close(mode)`
   接进 `KThetaRiskGate.force_flat`；`coverage.rs:1919` `KThetaRiskGate` 结构体只有
   `force_flat/stop_long/stop_short` 三个字段，**没有任何字段承载「限增仓/只平仓」**。**核实**：
   我直接读了 `KThetaRiskGate` 结构体定义和 `k_theta_risk_gate` 函数体，确认接入真实 MM 后
   Deleverage(M2)/CloseOnly(M3) 状态虽然在 `risk_mode()` 层面可达，但**没有下游消费者把它们转译
   成订单流约束**——设计 §2.6 表格声称的「M2 限增仓/M3 只平仓」行为在当前接线方案下不会发生，
   只是把枚举值算对了，订单流不变。这是设计文档遗漏的实装缺口，不是我方对设计的过度解读。
5. **输入验证协议缺失**。`risk_mode()`（`risk.rs:333`）对 NaN/负 buffer/未排序 bracket 无防线——
   NaN 参与比较会全部判假，最终落到 `else` 分支（Normal），即数据污染会静默退化为「一切正常」，
   这是风控函数最危险的静默失败模式。设计文档未提供 `MarginSchedule`/`RiskCushions` 的构造期校验。

### 三选择项裁定

| 项 | 裁定 | 理由 |
|----|------|------|
| (a) buffer1/2 归属 | **归 Θ_risk（接受设计立场），但强制敏感性网格** | 归类诚实（交易所不公布策略减仓垫），不需要另立 `EmpiricalDomain`——除非要对外声称 buffer 已经过经验校准。但 buffer 直接决定 M1/M2/M3 边界，必须补一组敏感性测试（buffer 取值扰动 → 触发时机/订单流变化的范围），否则「归 Θ_risk」只是把自由参数诚实地藏起来，没有诚实地暴露其影响 |
| (b) 账户模型默认 | **one-way/net** | 现有 `AccountState`（`strategy/mod.rs:119`）与 Nautilus adapter 均为净仓骨架，hedge 需要额外的双腿簿记结构；v0 默认 one-way，hedge 作显式 opt-in，与既有代码结构最小改动一致 |
| (c) CME 简化口径 | **条件接受，强制口径标签** | 百分比×名义简化可作 v0，但产出必须显式标注为「CME-simple」，不得在任何报告/日志中让人误认为这是 SPAN/portfolio margin/FCM 实盘保证金——避免声明膨胀（090号） |

### 影响声明完备性判定：needs_work（不完备）

设计 §4 只声明「MM=0 口径下的 alpha 冻结失效，须重跑重冻结」，但 Codex 指出更大范围的下游消费者
需要同步失效标注：`RunResult` 的 `trade_pnls_with_forced`、`daily_returns`、`equity_curve`、
`trades.forced_close`、`n_orders/is_l2` 字段，以及 `l3_fullwindow.rs` 等 L3 report 管线——这些都是
π（订单流）变化后必然联动变化的下游产物，设计文档目前只提了「alpha 冻结」一项，遗漏了其余具体
产物清单。此外 `closed_loop_final` 若仍固定走 Normal 分支，需要在文档中明确标注这是尚未接线的
残留路径，避免与已接线路径的口径混用。

---

## 2. 定义依据

- **formalization-validity-domain（231号）**：设计声称本次工作满足 L1（golden-vector 规则一致性，
  对照交易所公布数字），不声称 L2/L3——这个认识论等级标注本身合规；但 L1 验证要求「转录正确」，
  而致命缺口 2/3（单位错误、签名自洽性）恰恰是转录层面的错误，说明当前文档尚未达到自己声称的 L1
  标准，需先修复再声称已过 L1 门槛。
- **no-patch-mentality（090号）**：三个致命缺口都属于「设计推导链条不完整」而非「补丁思维」——
  修复方式是补全推导（如统一签名、去掉重复乘价），不是在错误公式上加特例判断，符合严格性要求的
  正确修复路径。
- **risk.rs 既有契约**（`RiskModeInput`/`risk_mode`/`global_risk_close`，Lean
  `risk_mode_complete_unique`）：机制本身穷尽互斥、已证正确，本次审计确认的缺口全部在**接线/
  派生函数层**，不涉及对已证机制的否定——与上轮 codex-p2 裁定的定性一致（D2 缺口是输入接线，
  非机制缺陷）。

## 3. 边界条件（裁定翻转条件）

- 若历史快照缺口按 Codex 建议修复为 `MarginScheduleBook`（`effective_from/effective_to` 分段），
  或团队明确接受「as-of forward simulation」的有效域降级声明（不声称历史 L2 精度）→ 致命缺口1
  解除。
- 若单位错误按二选一方案修复（改用原始 qty×contract_mult×mark 只乘一次，或直接用
  `net_notional(voices)` 不再乘 mark）→ 致命缺口2 解除。
- 若 `liquidation_flag` 签名补 `equity` 参数或改为由 `margin_inputs` 统一计算 MM/liq_flag（不再
  单独暴露不自洽的签名）→ 致命缺口3 解除。
- 若 `KThetaRiskGate`/`plan_orders` 未扩展出 `no_increase`/`close_only`/`deleverage_cap` 通道 →
  即便三个致命缺口修复，接入后 M2/M3 依然是「算出枚举值但不改变订单流」的空转状态，此时 conditional-
  pass 不能升级为 pass。

## 4. 下游推论

- 三个致命缺口必须在 P0-2 实装工位（#103 后续实装）动工前修复，否则实装出的代码会直接携带单位
  错误和签名不自洽——这不是「实装时顺手修」的量级，是设计层面必须先改的前置条件。
- M2/M3 接线缺口意味着仅接 `margin_inputs()` 不够，还需要扩展 `KThetaRiskGate` 或
  `plan_orders` 的可行集表达，这是实装范围的追加项，需要 team-lead 在派 #103 实装工位时把这一项
  显式纳入任务描述，否则实装工位会重复上一版「只算枚举值不改订单流」的半成品。
- 影响声明缺口意味着实装完成后，验收清单需要包含 `RunResult` 全字段 + L3 report 管线的重跑重
  冻结，不能只重跑 alpha 冻结这一项。

## 5. 谱系引用

- formalization-validity-domain（231号）：本裁定的核心判据（有效域 vs 定义域，L1 转录正确性）。
- no-patch-mentality（090号）：三个致命缺口的修复方式判据（补全推导，非打补丁）。
- `.chanlun/review-results/codex-p2-design-ruling-20260702.md` §D2：本次设计的直接上游否定，
  翻转条件在本裁定中被确认成立。
- `.chanlun/review-results/margin-model-design-20260703.md`：本次审计对象。

## 6. 影响声明

- 本工位纯只读审计 + Codex CLI 调用，**未修改任何生产代码**。
- 产出：本裁定文件 + Codex 完整交互记录
  `.chanlun/review-results/codex-review-20260703-020619-4a5a.md`。
- 下游消费：#103 实装工位（阻塞，需先修复 3 项致命缺口 + 补 M2/M3 接线扩展，方可动工）；
  team-lead（三选择项裁定已给出，可直接采纳进入实装任务描述）。

---

```yaml
---stance-declaration---
verdict: conditional-pass
review_target: D2 真保证金模型设计（margin-model-design-20260703.md）
stances:
  venue_schedule_as_datum_flip_condition: confirmed
  historical_snapshot_policy_missing: confirmed_fatal
  net_notional_mark_double_count_unit_bug: confirmed_fatal
  liquidation_flag_signature_missing_equity: confirmed_fatal
  m2_m3_not_consumed_by_current_gate: confirmed_important
  input_validation_missing: confirmed_important
  buffer_theta_risk_classification: accept_with_sensitivity_gate
  default_account_model: one_way_net
  cme_pct_simplification_v0: accept_with_scope_label
  impact_statement_completeness: needs_work
escalation_needed: []
concessions:
  - synthetic_config_margin_rate_rejection_flipped_by_real_venue_schedule
  - liq_flag_producer_name_no_longer_missing_at_design_level
---end-stance---
```
