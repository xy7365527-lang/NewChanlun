# #419 OKLO 真实窗接入 treasury 三阶段层（2026-07-27）

## 1. 设计

### 1.1 预注册窗口口径

选择：不改冻结的 `PREREG_WINDOWS`，也不为 OKLO 伪造 walk-forward 窗；`m8_e2e_all_systems_oos`
在 `M8_SYMBOL=OKLO` 时，从既有 OKLO 登记读取唯一单段 OOS，并标为 `oklo_oos`。

理由：

- `PREREG_WINDOWS` 已按 §2.4 登记 OKLO 为观察池，`oos =
  ("2025-01-01", "2026-06-24")`，同时机器断言它无 IS、无 walk-forward、无独立 Holdout。
- 本票需要真实窗的 treasury 层读数，不需要扩大 OKLO 的统计主判据。直接消费该唯一真相源，
  与 prereg 纪律一致；新增 WF 窗反而会改写冻结协议。
- OKLO 读数只作费用算术、触达计数与三阶段账本路径核验，遵守 v3 硬禁令，不作 alpha
  论据、不作策略择优输入。

### 1.2 symbol 参数化

选择：仿 `M8_WIN_FILTER` / `M8_FEE_DATUM` 先例，在既有 m8 跑批 seam 增加
`M8_SYMBOL`；未设置时保持 `BTC` 默认，设置后必须命中 `data::SYMBOLS` 与
`PREREG_WINDOWS`，否则 fail-loud。没有另造一条只在测试中存在的新缝。

理由：

- m8 的 overlay、cost model、TW 三阶段账本和报告层已经是本票要验收的生产组合路径；
  复用它才能证明 OKLO 穿过 overlay 臂，而非再次停留在 `simulate_fills` 费用面。
- env 未设时 BTC 数据集与窗口清单不变，保留臂 R 的既有行为；设置 OKLO 时，
  `M8_FEE_DATUM` 的品种绑定校验仍在 `data::load_by_symbol` 单点 fail-loud。
- 复现命令可显式列出 symbol、datum、窗口、dump 目录与报告路径，避免隐含测试状态。

### 1.3 产物隔离

选择：增加 `M8_REPORT_PATH`，未设置时仍写既有
`/tmp/m8_e2e_all_systems_oos.md`；本票实跑设置为
`/tmp/419_m8_oklo_treasury.md`，并把 `OPSEM_DUMP_DIR` 指向 `/tmp/419_*` 目录。

理由：默认路径保持既有调用兼容；显式路径兑现 #419 证据保护，不覆盖 #423 或其他票产物。
任何 m8 实跑前先把现存 `/tmp/m8_e2e_all_systems_oos.md` 备份为
`/tmp/419_backup_m8_e2e_all_systems_oos.md`。

### 1.4 treasury 费率科目审计

选择：不从 `Commission+Slippage` 总额反推科目。由生产唯一费率入口 `FeeQuoter` 对每个
真实 fill 生成逐科目 quote（佣金、pass-through、清算+CAT、卖出 SEC、卖出 TAF、未标定
滑点 addon），fill 层按实际成交比例累计，并随 `RunResult` 交给 m8 报告。

理由：

- per-share 档含最低佣金、1% 名义额上限与仅卖出监管费，不能用单标量费率恢复。
- 逐科目和必须与 treasury/R 分解实际扣取的 `commission_slippage` 对账；触达计数必须来自
  同一报价，而不是事后猜测。
- #423 对按股档的层4裁定保持不变：`scalar_cost_rate_opt=None`，LCB/三态/随机对照继续标
  “不可用”；本票只交付层1-3、trades、费率科目与触达读数。

### 1.5 TDD seam

预先约定的公开 seam：

1. `FeeQuoter` 的生产报价接口：给定真实 `(qty, px, side)`，逐科目和等于 venue 总费，
   且最低佣金/1% 上限/TAF 上限触达可判别。
2. `RunResult`/m8 env 跑批接口：`M8_SYMBOL=OKLO` 只消费预注册 `oklo_oos`，
   报告带真实 L2 datum 标签并落 treasury 科目审计表。

按单个行为逐条执行红 → 绿；不为私有 helper 写实现耦合测试。

## 2. 实现与验证

### 2.1 改动清单

| 路径 | 改动 | 原因 |
|---|---|---|
| `rust/src/theta_v0/venue_fee.rs` | 增加生产 `FeeQuote`；把 per-share 佣金、pass-through、清算+CAT、SEC、TAF 及三类触达收口到 `PerShareFees::breakdown` 单一算术真源 | 让账本取得与既有费率同源的原生科目，避免总费与分项重复公式漂移 |
| `rust/src/theta_v0/backtest/treasury.rs` | 增加 `FeeAudit`，只累计真实 `executed_qty>0` fill；分项和、venue 总费和触达计数可审计 | 在 treasury 账本口径保留实际扣费分解 |
| `rust/src/theta_v0/backtest/fill.rs` | 生产 π 净额 fill 同时取得 fee rate 与 quote，成交后把 quote 记入 `FeeAudit` | 审计与现金流消费同一个真实 fill，不从汇总额反推 |
| `rust/src/theta_v0/backtest/runner.rs` | `RunResult` 透传 `FeeAudit`；既有生产 π 集成测增加与 R 分解对账断言 | 让 overlay/m8 报告消费生产账本证据 |
| `rust/src/theta_v0/backtest/wverify_run.rs` | 增加 `M8_SYMBOL`、预注册窗路由、`M8_REPORT_PATH`、treasury 科目表及动态层2/3结算 | OKLO 走既有三系统同开路径，产物隔离且报告不复用 BTC 静态结算 |
| `chanlun/review-results/oklo-treasury-three-stage-20260727.md` | 本设计、复现、读数、对照、回归与 review 记录 | 票 #419 正式交付 |

未修改 #446 的 `lee_m3`/`lee_m4` 红测或用户列出的共享工作树他人文件。

### 2.2 TDD 红 → 绿

新增 6 条测试，均先观察到目标红态后补实现：

1. `production_quote_exposes_oklo_treasury_components_and_triggers`：
   RED=`FeeQuoter` 无逐科目生产报价方法；GREEN=OKLO per-share 五类 venue 科目、滑点及三类触达全过。
2. `fee_audit_accumulates_executed_quote_at_ledger_caliber`：
   RED=`FeeAudit` 未定义；GREEN=真实成交累计、比例缩放、分项和对账全过。
3. `m8_symbol_routes_oklo_to_preregistered_single_oos`：
   RED=无 symbol/窗口路由；GREEN=OKLO 只消费既有 `2025-01-01..2026-06-24`，
   明确不伪造 walk-forward。
4. `m8_symbol_unknown_fails_loud`：
   RED=无 symbol 校验；GREEN=未知品种不静默回退 BTC。
5. `m8_report_path_default_and_override`：
   RED=无隔离路径 seam；GREEN=默认路径兼容、`/tmp/419_*` 可显式指定。
6. `m8_oklo_layer23_settlement_tracks_actual_positive_stage_ii`：
   RED=`m8_layer23_settlement` 不存在；GREEN=层2/3结算直接消费逐窗 `net_r`/TW Stage，
   不再把 OKLO 正值写成“极负”、Stage II 写成 CostReduction。

另在既有 `run_theta_v0_pi_loop_produces_trades_nonempty` 增加生产接线断言：
`fee_audit.n_fills == n_orders` 且 `fee_audit.total_fee == RDecomposition.commission_slippage`。

两轴 review 的 Standards 首轮发现两项并已修：

- 旧注释仍把 `production_rate_or_fallback` 声明成唯一入口；已改为
  `production_rate_or_fallback` / `production_quote_or_fallback` 两个无角色门面，声明与能力一致。
- 新 quote 曾重复写 per-share 公式；已抽为 `PerShareFees::breakdown`，绝对费用与 treasury
  科目共同消费。

### 2.3 真实窗复现

工作目录：`/tmp/kimi-nest-mainline/rust`。

任何 m8 实跑前已执行证据保护：

```sh
cp /tmp/m8_e2e_all_systems_oos.md /tmp/419_backup_m8_e2e_all_systems_oos.md
```

最终（review 修复后）跑批命令：

```sh
M8_SYMBOL=OKLO M8_WIN_FILTER=oklo_oos M8_FEE_DATUM=venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES M8_REPORT_PATH=/tmp/419_m8_oklo_treasury.md OPSEM_DUMP_DIR=/tmp/419_oklo_opsem_reviewed cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

结果：`1 passed / 0 failed`，真实 OKLO 预注册窗 `2025-01-01..2026-06-24`
（268,411 bars）穿过 `run_theta_v0_pi_overlay`。datum sidecar 在
`analysis/data_cache/` 执行 `shasum -a 256 -c venue_fee_ibkr_pro_20260726.json.sha256`
为 `OK`；报告标签为 **`[L2费率标定: datum cc0d3fa3695d]`**。

最终产物：

- `/tmp/419_m8_oklo_treasury.md`，sha256
  `cbc4c0ba6bb283802f0d31c01e5ca937742ce4274f39db0768a2458d149ad87f`。
- `/tmp/419_oklo_opsem_reviewed/trades.jsonl`：482 行 / 663,115 bytes。
- `/tmp/419_oklo_opsem_reviewed/tower_events.jsonl`：1,850 行 / 196,788 bytes。
- `/tmp/419_backup_m8_e2e_all_systems_oos.md`，sha256
  `0efa787996595d244ee1f1bfcabe489369ee73d4ed28daeb596884262ab94969`；
  与未被本票覆盖的 `/tmp/m8_e2e_all_systems_oos.md` 哈希相同。

### 2.4 OKLO overlay/treasury 读数

以下全部读数只用于路径、费用算术、触达与账本对账，**不作 alpha 论据，不作策略择优输入**。

| 项 | treasury 真实窗读数 |
|---|---:|
| `RunResult.trades` | 953 |
| 真实 fills / orders | 1,313 / 1,313 |
| Σ名义额 | 22,506,169.890000015 |
| Σ佣金 | 1,399.243300 |
| Σpass-through | 1.035440 |
| Σ清算+CAT | 73.772230 |
| Σ卖出SEC | 232.577754 |
| Σ卖出TAF | 35.468940 |
| Σvenue 费用（排除滑点） | 1,742.097664 |
| Σ未标定滑点 addon | 4,501.233978 |
| Σ总费 / R 分解 Comm+Slip | 6,243.331642 / 6,243.331642 |
| 最低佣金触达 | 427/1,313（32.5209%） |
| 1% 上限触达 | 17/1,313（1.2947%） |
| TAF 上限触达 | 0/1,313 |
| venue 有效费率 | `7.740534e-5` |
| execution `net_r` | `+46,635` |
| TW 终态 | `II(已回本)` |

逐窗硬断言已经验证：

- `fee_audit.n_fills == n_orders`；
- treasury 分项和 `== fee_audit.total_fee`；
- `fee_audit.total_fee == RDecomposition.commission_slippage`。

层4按 #423 设计保持
`不可用(单标量成本口径无良定义 #374/#423)`；本票没有以近似费率“修复”或绕开该降级。

### 2.5 与 #388 `simulate_fills` 费用面对照

#388 基准：`/tmp/388_oklo_per_share_readings.md`，同一 datum/hash，真实价格序列但订单流为末
2,000 根可交易 bar 上每 100 bar 买、其后 50 bar 卖的确定性 40 腿夹具。

| 科目/触达 | #419 overlay/treasury（1,313 fills） | #388 50股（40腿） | #388 100股（40腿） | #388 500股（40腿） |
|---|---:|---:|---:|---:|
| Σ佣金 | 1,399.243300 | 14.0000 | 14.0000 | 70.0000 |
| Σpass-through | 1.035440 | 0.010360 | 0.010360 | 0.051800 |
| Σ清算+CAT | 73.772230 | 0.4060 | 0.8120 | 4.0600 |
| Σ卖出SEC | 232.577754 | 1.1803 | 2.3605 | 11.8027 |
| Σ卖出TAF | 35.468940 | 0.1950 | 0.3900 | 1.9500 |
| Σvenue 费用 | 1,742.097664 | 15.7916 | 17.5729 | 87.8645 |
| 最低佣金触达 | 427/1,313 | 40/40 | 0/40 | 0/40 |
| 1% 上限触达 | 17/1,313 | 0/40 | 0/40 | 0/40 |
| TAF 上限触达 | 0/1,313 | 0/40 | 0/40 | 0/40 |
| venue 有效费率 | `7.740534e-5` | `1.375701e-4` | `7.654387e-5` | `7.654387e-5` |

机制解释：

- 绝对金额不应相等：#388 是固定 40 腿、固定每腿 50/100/500 股的费用面夹具；#419 是
  268,411-bar 生产 overlay 路径生成的 1,313 个真实 fills，数量、价格、买卖侧与腿生命周期均不同。
- #388 的 50 股档每腿都被 `$0.35` 最低佣金托底；100/500 股落线性段。#419 混合了真实尺寸，
  因而同时出现 427 次最低佣金与 17 次 1% 名义额上限触达；再叠加真实价位及买卖侧构成，
  共同形成有效费率相对 #388 100/500 股线性夹具高约 1.1255% 的机制差异，而不是两层公式不一致。
- #388 表中“总费用/有效费率”按声明排除未标定 2 bp/腿滑点；#419 也用排除滑点的
  `Σvenue费用/Σ名义` 做同口径有效费率对照，同时另列 4,501.233978 滑点并把它纳入 treasury
  实扣总费。若直接拿 #419 的 6,243.331642 与 #388 venue 总费比，会把滑点口径混入。
- 两层共享 `PerShareFees::breakdown` 的同一费则真源，差异来自订单流/成交路径及尺寸分布；
  production treasury 分项和与 R 分解总扣费已机器对账，未发现算术泄漏。

### 2.6 回归与基线

- 开工基线（编排者在案）：`1966 passed / 4 failed / 135 ignored`。
- 收尾 debug `cargo test --lib`：exit 101，`1972 passed / 4 failed / 135 ignored`；新增 6 passed，
  failed 集未扩大：
  - `lee_m3_attribution_dimension_is_readable_and_not_residual_only`（#446）；
  - `lee_m4_cap_on_sparsity_has_no_unexplained_violation`（#446）；
  - `lee_m4_level_cap_narrows_position_when_enabled`（#446）；
  - `extract_signals_bit_exact_digest_guard`（#115 线）。
- `python3 scripts/check_armR_trades_digest.py`：exit 0；
  p3fold=`0x6bf47daf0aa737cd`、wf7=`0x282c28ca8ca65e16`、
  wf8=`0xcafa7c4846c762cd`，三份均逐位无漂移。

### 2.7 两轴 code-review

- **Standards**：首轮 2 项（能力声明过时、per-share 公式重复），修复后复审 residual 1 项
  （`fill.rs` 局部旧声明），再次修正为 rate/quote 两个无角色入口；最终无已知 residual。
- **Spec**：首轮 2 项均是正式报告尚未补齐（复现/验证、#388 对照）；代码与真实产物未发现
  scope creep 或实现错误。本节补齐后，票体要求已逐项核销，无已知 residual。

### 2.8 环境差异照实登记

票体指定的 `.claude/skills/implement/SKILL.md` 与 `.claude/skills/tdd/SKILL.md` 在本 worktree
不存在。实际只读使用主仓同名镜像：

- `/Users/silencehan/Projects/NewChanlun/.agents/skills/implement/SKILL.md`
- `/Users/silencehan/Projects/NewChanlun/.agents/skills/tdd/SKILL.md`

并读取当前仓 `.claude/skills/test-driven-development/SKILL.md`。`implement` skill 的 commit
步骤被本票“禁一切 git mutation”硬禁令覆盖；全部改动按要求留在共享工作区未提交。
