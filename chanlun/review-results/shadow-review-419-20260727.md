# #419 影子评审（commit `4d0c3101c2`）

评审日期：2026-07-27  
工作区：`/tmp/nc-review-419`（detached HEAD=`4d0c3101c2ac5cd39a28c5035cb61ac87d7a8c66`）  
固定对照：`c0c1a74b1677a2857ea39579ced7fe80361f12bf`

## 结论

**打回。**

- **Standards：打回**。2 HIGH / 2 MED / 1 LOW。
- **Spec：打回**。2 HIGH / 1 MED；OKLO 既有产物的数值表、prereg 路由与 digest 门本身核对通过，但“逐位等价”和 BTC 非回归未兑现。

## Findings

### HIGH-1 — `PerShareFees::breakdown` 与旧 `fee_usd` 卖出分支不逐位等价

**轴：Standards + Spec。**

证据：

- 旧式在 `c0c1a74b16:rust/src/theta_v0/venue_fee.rs:178-195` 先计算
  `sell_reg = SEC + TAF`，再求
  `commission + passthru + clearing + sell_reg`。
- 新式在 `rust/src/theta_v0/venue_fee.rs:154-157,162-190` 把总额改成
  `commission + passthru + clearing_cat + sec + taf`。
- `fee_usd` 已改为消费这个新总和，见
  `rust/src/theta_v0/venue_fee.rs:221-232`；因此漂移进入生产
  `effective_rate`/现金流，不只是审计展示差异。
- 新测试只做容差比较：`rust/src/theta_v0/venue_fee.rs:842-855` 与
  `rust/src/theta_v0/backtest/fill.rs:4004-4016`，没有 `to_bits`/`assert_eq!`
  锁住旧求和树。

逐分支推导：

- Buy：旧式 `... + 0.0`，新式 `... + 0.0 + 0.0`；正有限输入下未找到差异。
- Sell：佣金三段 `max(min).min(cap)`、pass-through、清算、SEC、TAF
  各自公式不变，但旧式先舍入 `(SEC + TAF)`，新式改成逐项左结合，舍入树不同。
- 触达布尔本身一致：新式
  `min_hit = raw < min && min <= cap`、`cap_hit = cap < max(raw,min)`
  （`venue_fee.rs:186-189`），与 #388 手写口径
  （`fill.rs:3984-3991`）相同；当 `cap < min` 时只算 cap，min 不算，优先级正确。
  TAF 也同为严格 `raw > cap`（`venue_fee.rs:170-176`、`fill.rs:3995-4000`）。

独立复核：

```text
qty=1, px=1, side=Sell
old = 0.010426
new = 0.010426000000000001
old bits = 0x3f855a3a08398a65
new bits = 0x3f855a3a08398a66
差 1 ULP
```

复核使用 Python `struct.pack/unpack` 按源码运算顺序重算；输入全为正有限数，
与 Rust `f64::{max,min}` 语义一致。

建议：`breakdown` 可以保留分项，但 `total()` 必须保持旧求和树，例如明确写
`commission + passthru + clearing_cat + (sec + taf)`；补一条以旧内联公式为 oracle、
覆盖 Buy/Sell、min/cap 冲突边界与 TAF cap 的 `to_bits` 回归测试。

### HIGH-2 — FeeAudit 只审计净额 fill；M8 用硬拒 `VOICE_EXEC=1` 回避跨域，破坏指定 BTC 基线命令

**轴：Standards + Spec。**

证据：

- `fee_audit.record_executed` 只接在净额 `apply_order` 后，
  `rust/src/theta_v0/backtest/fill.rs:1037-1063`。
- 紧接着的声部执行 fill 只累计 `n_voice_fills`/`voice_trades`，没有记录
  `fee_audit`，见 `fill.rs:1067-1089`。
- `VOICE_EXEC=1` 时，R 分解的费用改取 `VoiceExecBook::cum_fee()`，
  `fill.rs:2120-2139`；输出的 `n_orders`/`trades` 也切到声部口径，
  但 `fee_audit` 仍原样返回净额影子累计，见 `fill.rs:2271-2280`。
- `RunResult.fee_audit` 却无条件声明为“与生产 treasury/R 分解同一批真实 fill”，
  `rust/src/theta_v0/backtest/runner.rs:176-184`。该声明在 `VOICE_EXEC=1`
  下为假。
- `run_theta_v0_pi_overlay` 明确把 `VOICE_EXEC=1` 解析成
  `voice_exec=Some(...)`，`runner.rs:705-725,805-824`。
- #419 在 M8 循环加入 `assert!(r.voice_exec.is_none())`，
  `rust/src/theta_v0/backtest/wverify_run.rs:1694-1699`。所以用户指定的
  `VOICE_EXEC=1` BTC wf8 命令即使数据齐备，也会在完成窗口计算后被该断言打红。
- 固定点前的 `/tmp/423_armR_wf8.out:443-445` 记录同一 M8 测试
  `1 passed / 0 failed`；本提交的报告复现命令刻意省略了 `VOICE_EXEC=1`，
  `chanlun/review-results/oklo-treasury-three-stage-20260727.md:122-126`，
  未核销原声部臂非回归。

独立复核：

1. 实跑
   `voice_exec_env_gate_off_bitexact_on_voice_readings`，release 结果
   `1 passed / 0 failed`；该测试在 `runner.rs:3242-3247` 机器确认 gate 开后
   `voice_exec=Some`。
2. 严格按票给的 BTC 命令复跑，release 首次编译成功，但本工位缺
   `analysis/data_cache/btc_1m_full.json`，在
   `wverify_run.rs:1587-1588` 先行失败，exit 101；未生成
   `/tmp/rev419_armR_wf8.md` 或 `/tmp/rev419_opsem_wf8`。因此没有把静态必达的后续
   `voice_exec.is_none()` panic 冒充成已实际跑到。

建议：二选一并明确裁定。

1. 把 FeeAudit 接到 `VoiceExecBook` 的开/平真实 fill，令其与声部 R 分解同域；或
2. 将审计结果类型化/可选化，明确 `NetAudit`，M8 在 BTC 声部臂保留既有行为且不渲染
   不同域的 treasury 表，OKLO 净额 overlay 臂单独验收。不能以通用 M8 panic 作为兼容方案。

### MED-1 — 指定集成测缺少“Σ科目 == total_fee”硬断言

**轴：Spec。**

证据：

- `run_theta_v0_pi_loop_produces_trades_nonempty` 只断言
  `n_fills == n_orders` 与 `total_fee == commission_slippage`，
  `rust/src/theta_v0/backtest/runner.rs:1271-1307`。
- 它没有调用 `fee_audit.component_total()`。因此构造“总费与 R 相同，但任一分项漏记”
  的反例，这个指定集成测仍会绿。
- 分项断言确实存在，但位置不同：合成单元测
  `treasury.rs:573-600`，以及数据跑批循环
  `wverify_run.rs:1701-1717`。这不能兑现“该生产集成测三段链全锁”的精确要求。

独立复核：逐行读取指定测试并搜索 `component_total`；该符号在
`runner.rs:1271-1307` 范围内零命中。8 条无数据依赖的 #419/prereg 核心 release
测试均独立通过，说明这里是覆盖缺口，不是现有用例已红。

建议：在指定集成测增加
`component_total ≈ total_fee ≈ r_decomp.commission_slippage` 同一容差链。

### MED-2 — `953` 与 dump `482` 是口径差异，不是真错；正式报告缺机制解释

**轴：Standards。**

结论：**不是 471 条漏写，不升 HIGH；应补报告说明并出订正小票。**

证据：

- 本票 OKLO 命令未开 `VOICE_EXEC`，M8 还硬断言其关闭。因此
  `RunResult.trades` 取净额 `trades`，见 `fill.rs:2271-2276`。
- 净额 `trades` 的主要生产者不是 dump：实际 fill 在 `fill.rs:1058-1062`
  调 `track_position_transition`；该 helper 对全平/翻转与部分减仓分别 push
  `metrics::TradeRecord`，见
  `rust/src/theta_v0/backtest/ledger.rs:206-260`。窗口末残仓另在
  `fill.rs:2050-2069` 产 forced 记录。
- `OPSEM_DUMP_DIR/trades.jsonl` 写的是另一类型 `TypedTrade`：
  reverse、silent drop、risk exit、overlay close、censored Hold 五条生命周期路径，
  分别在 `fill.rs:1748-1773,1777-1811,1814-1840,1843-1870,2075-2105`。
  它不是 `RunResult.trades` 的序列化，也不是其子集。
- 报告同时列出 953 与 482，却没有上述口径桥，见正式报告
  `:133-150`。

独立复核：

- `/tmp/419_oklo_opsem_reviewed/trades.jsonl` 为 482 行、663,115 bytes；
  `trade_id` 从 1 到 482 连续、482 个 distinct。
- dump 的 `exit_type` 构成为：
  `ReduceCore=234`、`CloseShortDiff=137`、`CloseRoot=110`、`Hold=1`，
  合计 482；无 RiskExit。
- 所以 `953−482=471` 没有可解释成“哪条 push 漏 write”的可加减语义；两边实体与粒度不同。

建议订正句：

> `RunResult.trades=953` 是净额持仓全平/翻转/部分减仓的
> `metrics::TradeRecord`；`OPSEM trades.jsonl=482` 是 typed 声部生命周期
> `TypedTrade`（含窗口末 Hold）。二者非同一集合，差值 471 不表示 dump 漏行。

### MED-3 — 报告模板仍有认识论/品种归因膨胀

**轴：Standards。**

证据：

1. OKLO 产物抬头正确标 L1（`/tmp/419_m8_oklo_treasury.md:5`），但同一产物
   `:29` 又写“层2/3/4（本跑批 L2 实测）”。源头是无条件静态字符串
   `rust/src/theta_v0/backtest/wverify_run.rs:1644-1657`。这把本票明确限定的
   L1 treasury/费用审计抬成 L2。
2. `resolve_m8_symbol` 接受所有同时在 `data::SYMBOLS` 与 `PREREG_WINDOWS`
   登记的品种（`wverify_run.rs:1499-1507`），但认识论分支只有
   `BTC` 与“其余一律 OKLO”（`:1602-1606`）。例如已登记的 ES 会生成
   “品种 ES；真实 OKLO 观察池”这种事实性错误，而不是 fail-loud。
3. OKLO 报告写“三系统同开：M5 overlay 声部执行臂”
   （`/tmp/419_m8_oklo_treasury.md:3`），但实际复现命令未设 `VOICE_EXEC`
   （正式报告 `:122-126`），且代码自己声明此票验收净额 overlay
   （`wverify_run.rs:1696-1699`）。应称“净额执行 + overlay 旁路/账本”，不能冒充声部独立执行臂。

独立复核：直接对拍产物 `:3,5,29` 与生成器分支；无需市场数据即可构造已登记非 BTC
symbol 的标签反例。

建议：heading 随 `epistemology` 动态渲染；把支持域限制为 BTC/OKLO 并对其他值 fail-loud，
或为每个已登记品种提供真实标签；同步修正“声部执行臂”措辞。

### LOW-1 — 新 Rust hunk 未保持 rustfmt 风格

**轴：Standards。**

证据：`rustfmt --edition 2021 --check` 明确要求重排本提交新增的
`venue_fee.rs:166-168,186-187` 等行。`git diff --check
c0c1a74b16..4d0c3101c2` 虽为 exit 0，但不等价于 rustfmt clean。

独立复核：`cargo fmt --all -- --check` / 针对变更文件的 `rustfmt --check`
均 exit 1。仓库还有大量与本票无关的既存格式漂移，本项只登记 rustfmt 输出中明确落在新增
hunk 的位置，不把全仓旧债归给 #419。

## 重点核查逐项结算

1. **FeeAudit 算术逐位等价：FAIL / HIGH。** 分项公式相同，卖出总和求值树不同，存在 1 ULP 反例。
   min/cap/TAF 触达语义与 #388 一致；cap 与 min 冲突时 cap 优先。
2. **机器对账声称：PARTIAL / MED。** M8 数据循环有三段硬断言；指定
   `run_theta_v0_pi_loop_produces_trades_nonempty` 缺 component-total 一段。
3. **953 vs 482：口径差异 / MED 文档缺口。** 非 trades 真错；差值无同口径分解意义。
4. **BTC 非回归：未能完成机器 diff/cmp；另有 HIGH 静态阻断。** 精确命令因缺 BTC 数据
   exit 101；即使数据齐备，`VOICE_EXEC=1` 与新增 `voice_exec.is_none()` 断言冲突。
5. **prereg：OKLO/BTC 目标路径 PASS。** `PREREG_WINDOWS`
   明载 OKLO Observation、`2025-01-01..2026-06-24`、IS/Holdout=None、WF 两表空，
   见 `prereg_windows.rs:336-345`；机器断言见 `:414-423`。M8 默认 BTC、OKLO 单 OOS、
   未知 symbol panic 测试均存在并独立通过。扩展到其他“已登记 symbol”时标签有 MED-3。
6. **读数对拍：PASS。** 正式报告 §2.4 与
   `/tmp/419_m8_oklo_treasury.md:52-54` 逐项一致。独立 Decimal 重算：
   - venue = `1399.243300 + 1.035440 + 73.772230 + 232.577754 + 35.468940`
     = `1742.097664`；
   - 总费 = `1742.097664 + 4501.233978 = 6243.331642`；
   - 有效费率 =
     `1742.097664 / 22506169.890000015`
     = `7.740533695936e-5`，按产物精度为 `7.740534e-5`；
   - 触达 `427/1313`、`17/1313`、`0/1313` 与产物一致；
   - #388 三组表与 `/tmp/388_oklo_per_share_readings.md:7-9` 全部一致；
     相对 100/500 股线性档高 `1.125459%`，报告写“约 1.1255%”正确。
   - `/tmp/419_m8_oklo_treasury.md` 实际 sha256 =
     `cbc4c0ba6bb283802f0d31c01e5ca937742ce4274f39db0768a2458d149ad87f`，
     与正式报告一致。
7. **回归门：PASS。** `python3 scripts/check_armR_trades_digest.py` exit 0：
   p3fold=`0x6bf47daf0aa737cd`、wf7=`0x282c28ca8ca65e16`、
   wf8=`0xcafa7c4846c762cd`。
8. **v3/090：PARTIAL / MED。** 没有发现把 OKLO 数值用作 alpha/择优的正向声明；
   “禁作 alpha/择优”护栏存在。OKLO 的无条件 “L2 实测” heading 与声部臂措辞不诚实，
   见 MED-3。

## 独立测试记录

- `git diff --check c0c1a74b16..4d0c3101c2`：exit 0。
- 8 条无数据依赖核心 release 测试：全部通过：
  `production_quote_exposes_oklo_treasury_components_and_triggers`、
  `fee_audit_accumulates_executed_quote_at_ledger_caliber`、
  `run_theta_v0_pi_loop_produces_trades_nonempty`、
  `m8_symbol_routes_oklo_to_preregistered_single_oos`、
  `m8_symbol_unknown_fails_loud`、
  `m8_report_path_default_and_override`、
  `m8_oklo_layer23_settlement_tracks_actual_positive_stage_ii`、
  `prereg_okla_special_case`。
- `voice_exec_env_gate_off_bitexact_on_voice_readings`：release 通过，确认
  voice gate 开后 `voice_exec=Some`。
- `python3 scripts/check_armR_trades_digest.py`：exit 0。
- 指定 BTC wf8 跑批：exit 101，原因是本工位缺 BTC 数据文件；release 构建成功。

## 未覆盖声明

1. 未完成 `/tmp/rev419_armR_wf8.md` 与 `/tmp/423_armR_report_wf8.md` 的内容 diff；
   新报告未生成。
2. 未完成 `/tmp/rev419_opsem_wf8` 与 `/tmp/m8_win_gate/wf8/` 的 bytewise cmp；
   新 dump 未生成。
3. 未重新跑 OKLO 268,411-bar M8；本工位同时缺
   `analysis/data_cache/oklo_1m.json`。对 OKLO 的结论仅使用只读既有产物，并独立重算其
   表格、行数、哈希与 dump 构成。
4. 未重新执行 datum sidecar `shasum -c`；对应 datum/sidecar 文件不在本工位。
5. 未独立复跑全量 `cargo test --lib`，因此正式报告所述
   `1972 passed / 4 failed / 135 ignored` 只登记为未覆盖，不背书。
6. `cargo fmt --all -- --check` 的全仓失败包含大量本票外既存格式债；本报告只把明确落在
   #419 新增 hunk 的格式差异列为 LOW。
7. 未使用外网或 `gh`；未修改仓内文件，唯一评审产出为本 `/tmp` 报告。
