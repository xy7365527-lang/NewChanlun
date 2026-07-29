# #481 二轮复审：#490 修复核销（commit `d772ea8eff`）

评审日期：2026-07-27  
工作区：`/tmp/kimi-nest-mainline`  
HEAD：`d772ea8eff fix(rust): #490 修复 #481 复审新 MED——FeeAudit 净额同域 oracle + unclassified_venue 展示 + header 动态`  
固定对照：`2ab93f32b27720746ba87ddef7c0307165081386`（`d772ea8eff^`）  
复审面：2 文件，`+133/-39`

## 结论

**维持打回。**

#490 的三个核心机制均已通过静态与动态核销：同域 oracle 成立、BTC/OKLO 的
`unclassified_venue` 展示正确、header 随 `VOICE_EXEC` 动态且与实际返回域 fail-loud
对拍；未发现执行数值、dump、臂 R digest 或既有主表列的新漂移。

但前轮 MED 的一项明确核销要求仍未落地：`VOICE_EXEC=1` 报告在同一表并列
“声部投影trades”与未限定域的“真实fill”，后者实际是净额影子 `FeeAudit`。
前轮要求逐字改成“净额影子fill”；本提交只在顶部 header 和表前说明中补了域解释，
字段本身仍保留模糊地带。按本仓 090“声明必须与实际能力一致、禁模糊地带”的严格纪律，
登记 **MED×1（未核销残项，不是新的执行回归）**。

- **Standards 轴：维持打回。** MED×1，报告字段仍未自带正确域标签。
- **Spec 轴：核心三项 PASS；完整核销 PARTIAL。** 三项机制均实现，但前轮明确的表头改名要求漏实现。

## 三项核销

| 核销项 | 独立证据 | 判定 |
|---|---|---|
| 1. 净额同域 oracle | `fill.rs:959,1021-1022` 分别持有净额 `n_orders_executed`、`cum_fee` 与影子 `FeeAudit`；唯一生产 π 净额成交出口在 `:1060-1086`，主累计器先以 `cum_fee += fill.fee` 累计，影子路径再以 `record_executed(fee_quote, fill.fee)` 累计，执行计数另增。`treasury.rs:217-237` 独立更新 `n_fills/total_fee`。`fill.rs:441-450` 是普通 `assert_eq!`，`:2132-2134` 在 `VOICE_EXEC` 输出域切换（`:2150-2163`）前无条件调用。 | **PASS** |
| 2. `unclassified_venue` 展示 | `wverify_run.rs:1570-1602` 的单一 formatter 在滑点列前渲染 `fee.unclassified_venue`；`:1968-1970` 表头同序加 `Σ未分类venue`；`:2171-2188` 定点测试锁列位。独立 BTC 跑批行显示 `1353002.745411`，佣金/pass-through/清算/SEC/TAF/滑点均为 `0`；OKLO 同列为 `0.000000`。 | **PASS** |
| 3. header 动态 | `m8_execution_projection_label`（`wverify_run.rs:1561-1568`）两分支分别为“声部执行投影 + 净额影子账本”与“净额执行 + overlay 旁路/账本”；`:1675-1677` 按 gate 取值，`:1768-1772` 逐窗与实际 `r.voice_exec.is_some()` 硬对拍；`:2158-2168` 双分支测试。BTC/OKLO 实产物分别命中对应字符串。 | **PASS** |

### oracle 独立性与 exact-equality 裁定

1. 两边是**独立状态/累计器**，但并非重新计算两套费率公式：两边共同消费同一
   `FillOutcome.fee`。这正好能检测 `FeeAudit.record_executed` 的整笔漏记/重记，而不制造第二套
   费用裁决源。
2. `cum_fee` 与 `FeeAudit.total_fee` 都从 `+0.0` 开始，按同一成交顺序对同一个有限非负
   `fill.fee` 执行同形 `+=`，所以相等是构造性结果，不是巧合级近似。源码用
   `assert_eq!`，没有容差；虽未显式比较 `to_bits()`，可达费用域排除了 NaN、负零等例外，
   因而在本域等价于逐位同结果。
3. 拒单/noop 的 `FillOutcome.fee=0`，只令主累计器加 `+0.0`，两边状态不变；部分成交时
   `record_executed` 按同一 `fill.fee` 入账。窗口终点 forced close 是虚拟兑现，不改现金、
   不计真实 fill；censored 只结算 typed ledger，均不属于 `FeeAudit` 集。
4. `plan_and_fill_mtm` 与 dual/legacy 路径在 `fill.rs:2656,3130` 明确返回空
   `FeeAudit`，不消费 #419 审计；M8 消费的生产 overlay 路径全部经过新 oracle。
5. `net_fee_audit_completeness_rejects_an_omitted_fill`
   （`fill.rs:3602-3608`）以空 audit 对 `1 fill / 12.5` 调 oracle，确实首先命中
   `n_fills` panic，使整笔漏记变红。它是 helper 级故障反例，不是仓内生产接线篡改；
   生产接线完整性由上述唯一成交出口静态核查与两次真实跑批共同覆盖。

## 未核销残项

### MED — Treasury 表仍把净额影子 fill 写成未限定域的“真实fill”

前轮报告 `/tmp/shadow-review-481-recheck-20260727.md:35-48,67-74` 已明确：

- 同一表里的 `455` 是净额影子 `FeeAudit`，不是 `VOICE_EXEC=1` 的声部执行 fill；
- 表头须将“真实fill”改为“净额影子fill”；
- 仅加跨域免责声明不够。

本提交的 `wverify_run.rs:1963-1970` 仍生成：

> `Treasury 真实 fill` / `声部投影trades` / `真实fill`

独立产物 `/tmp/rev490_armR_wf8.md:34-40` 也仍是该组合。顶部动态 header 与
`:1953-1956` 的表前说明已大幅降低误读风险，但字段被单独摘录、机器消费或横向比较时，
仍可能把 `455` 读作声部实际执行 fill。该项是前轮 MED 的明确核销条件，不能降格为建议项。

最小核销面：章节标题与列名改为“净额影子 fill / 净额影子fill”，并为该列名加定点测试。

## 新引入排查

除上述**未核销残项**外，未发现新引入的 HIGH/MED/LOW 执行或算术问题：

- oracle 是 release 可见普通 `assert_eq!`，没有 `#[cfg(test)]`、`debug_assert` 或
  `voice_exec=Some` 绕过；
- 新 formatter 的字段顺序与 16 列表头一致，BTC/OKLO 都经真实产物验证；
- header 不是只测 helper：逐窗还对实际 `OverlayRunResult.voice_exec` 硬断言；
- 新测试确实咬住对应反例；未见 scope creep 或 Fowler baseline 新 smell；
- `git diff --check 2ab93f32b2..d772ea8eff`：exit 0。

## 动态复核记录

### BTC 臂 R wf8

严格按指定隔离命令运行：

- cargo test：exit 0，`1 passed / 0 failed`，测试体 30.95s；
- `/tmp/rev490_opsem_wf8/tower_events.jsonl` 对
  `/tmp/m8_win_gate/wf8/tower_events.jsonl`：`cmp` exit 0；
- `/tmp/rev490_opsem_wf8/trades.jsonl` 对
  `/tmp/m8_win_gate/wf8/trades.jsonl`：`cmp` exit 0；
- 主表旧 19 列对 `/tmp/423_armR_report_wf8.md`：逐字段 SAME；
- header：`声部执行投影 + 净额影子账本`；
- 审计行：声部投影 trades `133`，净额影子 fill `455`，
  `unclassified_venue=1353002.745411`，`total_fee=1353002.745411`。

### OKLO

严格按指定隔离命令运行：

- cargo test：exit 0，`1 passed / 0 failed`，9.89s；
- 主表数据行对 `/tmp/484_m8_oklo_treasury.md`：SAME；
- 审计行除插入 `unclassified_venue=0.000000` 一列外，其余字段对 #484：SAME；
- 两份 dump 对 `/tmp/484_oklo_opsem` 与
  `/tmp/419_oklo_opsem_reviewed`：均 `cmp` exit 0；
- header：`净额执行 + overlay 旁路/账本`。

### 全量与回归门

- `cargo test --lib --quiet`：**exit 101**，
  `1979 passed / 4 failed / 135 ignored / 0 measured / 0 filtered out`，6.07s。
- 四个失败与前轮指纹一致：
  - `lee_m3_attribution_dimension_is_readable_and_not_residual_only`
  - `lee_m4_cap_on_sparsity_has_no_unexplained_violation`
  - `lee_m4_level_cap_narrows_position_when_enabled`
  - `extract_signals_bit_exact_digest_guard`
- `python3 scripts/check_armR_trades_digest.py`：**exit 0**：
  - p3fold `0x6bf47daf0aa737cd`
  - wf7 `0x282c28ca8ca65e16`
  - wf8 `0xcafa7c4846c762cd`

## 未覆盖声明

1. 未复跑 BTC p3fold/wf7、OKLO 之外其他品种的 ignored M8；本票指定的 BTC wf8 与 OKLO 已覆盖。
2. 只读边界下未篡改生产源码做端到端故障注入；漏记红测为提交内 helper 级 `should_panic`，
   并结合唯一生产成交出口做静态闭包核查。
3. 未把 `assert_eq!` 改写为 `to_bits()` 试验；本报告按有限非负费用域与同序同值累计证明
   exact-equality。
4. 未使用外网或 `gh`；共享工位既有修改/未跟踪文件未触碰、未归因。
5. 未修改任何仓内文件；唯一正式评审产出为
   `/tmp/shadow-review-481-recheck2-20260727.md`。
