# #481 复审：#484 修复核销（commit `2ab93f32b2`）

评审日期：2026-07-27  
工作区：`/tmp/kimi-nest-mainline`  
HEAD：`2ab93f32b2 fix(rust): #484 修复 #481 打回——FeeAudit 求和结合序 1-ULP + VOICE_EXEC=1 BTC 基线断裂 + MED×3`  
固定对照：`4d0c3101c2`（`2ab93f32b2^`）  
复审面：4 文件，`+199/-51`

## 结论

**维持打回。**

首轮 `HIGH×2 + MED×3 + LOW×1` 的目标项均已核销；BTC/OKLO 的指定动态非回归也通过。但本次复审发现 **新增 MED×1**：`VOICE_EXEC=1` 下虽然不再做错误的跨域对账，净额影子 `FeeAudit` 也随之失去任何独立同域 oracle，只剩同一累加器的内部自洽断言；报告却仍把它作为“真实 fill 审计”展示，并漏掉承载 BTC venue 费用的 `unclassified_venue` 列。整笔净额 fill 漏记/重记时可静默全绿，当前报告的域标识和科目展示不足以支撑 090 声明。

- **Standards 轴：维持打回。** 新增 1 MED（090 声明与实际可验证能力不一致）；未见需单列的 Fowler smell。
- **Spec 轴：维持打回。** 首轮六项闭合，但追加的“voice_exec=Some 下 FeeAudit 静默错账通道 / BTC 审计行语义”未通过。

## 新发现

### MED — `VOICE_EXEC=1` 下净额 FeeAudit 无独立 oracle，且 BTC 审计表漏科目、混合域

**代码证据**

1. `rust/src/theta_v0/backtest/wverify_run.rs:1731-1745` 在 `voice_exec=Some` 时跳过：
   - `fee.n_fills == r.net_result.n_orders`；
   - `fee.total_fee == r.net_result.r_decomp.commission_slippage`。

   跳过跨域比较本身是正确的，因为此时 `net_result.n_orders/r_decomp` 已按既有契约切为声部执行投影（`runner.rs:645-660`），而 `FeeAudit` 仍来自净额影子 fill。

2. voice 分支只剩 `component_total() == total_fee`（`wverify_run.rs:1746-1752`）。但 `FeeAudit::record_executed` 在同一次调用中同步累加 `n_fills`、全部分项和 `total_fee`（`treasury.rs:217-240`）。若整笔净额 fill 没调用或重复调用 `record_executed`，分项与总额会一起少/多，component-total 仍通过。

3. `fee_audit_windows_checked += 1`（`:1753`）只证明该非空窗口执行过内部 component-total 检查；`assert_m8_audit_coverage`（`:1526-1530,1898`）因此是“非空窗口覆盖”，不是 FeeAudit 完整性覆盖。现名会放大断言能力。

4. `/tmp/rev481_armR_wf8.md:34-40` 的段落虽已明确“不同域、不作跨域伪对账”，但仍存在三处会误导读者：
   - 总述写“M5 净额执行 + overlay 旁路/账本”（`:3`），而本命令 `VOICE_EXEC=1` 的层2 `n_orders/R` 实为声部执行投影；
   - 同一表并列“声部投影trades”和未限定域的“真实fill”，后者实际是净额影子 `FeeAudit`；
   - `FeeAudit` 含 `unclassified_venue`（`treasury.rs:208,236,251`），报告表头/行构造（`wverify_run.rs:1759-1778,1939-1946`）未渲染该列。BTC 行因此显示佣金、pass-through、清算、SEC、TAF、滑点全部为 `0`，却显示 `Σ总费=1353002.745411`、venue 有效费率 `3e-4`；费用实际落在不可见的 `unclassified_venue`。

**影响**

当前复跑没有发现执行数值或 dump 漂移，本项不判 HIGH；但报告声称的是审计证据，整笔漏记/重记可静默通过，且当前 BTC 行无法由可见科目重算总费，违反 090“声明必须与实际能力一致”，故判 MED。

**核销要求**

- 在切换声部输出口径前保留净额影子的 `n_orders` 与 `cum_fee/commission_slippage`，使 voice 模式也能以同域 oracle 硬对账；或在未接通前隐藏/明确标为“未完成完整性对账”的净额影子 FeeAudit 表。
- 总述按 `voice_exec` 动态标明“层2=声部执行投影；FeeAudit/TW=净额影子域”。
- 表头将“真实fill”改为“净额影子fill”，并补 `Σ未分类venue`；否则 BTC 行不可解释。
- 将 `assert_m8_audit_coverage`/计数变量改成与真实能力相符的 window-coverage 命名，或提升它为真正的审计覆盖断言。

## 首轮发现逐项核销

| 首轮发现 | 复审证据 | 判定 |
|---|---|---|
| HIGH-1：`PerShareFeeBreakdown::total()` 求和结合序非逐位等价 | `venue_fee.rs:155-156` 已恢复 `(commission + passthru + clearing_cat) + (sec + taf)`；`:528-568` 用 `to_bits` 锁定 3⁵ 网格和编排者反例。release 定点测试通过。独立重算首轮 `qty=1, px=1, Sell`：旧/修复均 `0.010426`, bits `0x3f855a3a08398a65`；坏式为 `0x...66`。编排者反例：旧/修复 `4.0023337712048397`, bits `0x40100263c8bc0129`；坏式 `0x...12a`。OKLO 独立复跑通过；`rev481` 报告与 `/tmp/484_m8_oklo_treasury.md` 全文件相同，两个 dump 与 `/tmp/484_oklo_opsem`、`/tmp/419_oklo_opsem_reviewed` 均逐字节相同；两条 OKLO 数据行对 `/tmp/419_m8_oklo_treasury.md` 逐字节相同，差异仅为预期 header/措辞/审计声明。 | **PASS，核销** |
| HIGH-2：`assert!(r.voice_exec.is_none())` 断裂 BTC 臂 R | 断言已拆除；`wverify_run.rs:1731-1753` 分离条件跨域段与无条件 component-total 段。严格按指定命令独立跑 BTC wf8：`1 passed / 0 failed`。`rev481_opsem_wf8/{trades,tower_events}.jsonl` 对 `/tmp/m8_win_gate/wf8/` 两文件 `cmp` 均 SAME，也与 `/tmp/484_opsem_wf8` 相同。新报告对 `/tmp/423_armR_report_wf8.md` 的既有前 19 列逐字段完全相同；新增三列及 header/结算/审计段为 intended 渲染差异。 | **PASS，核销**；报告语义残差转入新增 MED |
| MED-1：component-total 断言缺口 | `runner.rs:1301-1313` 已形成 `n_fills == n_orders`、`total_fee == R`、`component_total == total_fee` 三段链；M8 `wverify_run.rs:1746-1752` 无条件执行 component-total。`run_theta_v0_pi_loop_produces_trades_nonempty` 与 `fee_audit_accumulates_executed_quote_at_ledger_caliber` 均通过。反例会咬人：任一分项漏记而 `total_fee` 保持原实扣值时，残差立即非零并超过 `1e-9` 容差。 | **PASS，核销**；“整笔同步漏/重记”是不同的新增 MED |
| MED-2：953/482 缺机制解释 | 正式报告 `oklo-treasury-three-stage-20260727.md:167-174` 已枚举六个 `TradeRecord` push 出口与五个 typed-ledger `write_trade` 出口；源码复核出口集合存在，且两边实体、粒度、窗口边界语义不同。 | **PASS，核销**。定性机制已经足够；要求 `953-482=471` 的定量构成会预设不存在的一一映射，反而制造伪对账。若未来定义映射 invariant，才需要定量分解。 |
| MED-3：认识论/执行措辞膨胀 | `m8_epistemology` 对 BTC=L2、OKLO=L1，其他品种 fail-loud；对应三个 release 测试通过。OKLO 报告已渲染 L1 与“净额执行 + overlay 旁路/账本”。 | **PASS，原问题核销**；该静态执行措辞在 `VOICE_EXEC=1` BTC 下失真，已转入新增 MED。 |
| LOW-1：新 hunk rustfmt | `git diff --check 2ab93f32b2^ 2ab93f32b2` exit 0。`rustfmt --edition 2021 --check` 整文件仍 exit 1，但输出落在大量既存上下文/旧债；本提交新增行（`venue_fee.rs:155-156,528-568`、`runner.rs:1301-1313`、本次 wverify 新增段）未出现新增格式 diff。 | **PASS，核销** |

## 复审追加项裁定

### 1. BTC 审计行域构成

**裁定：现有说明不足，需改注/补列；在未恢复同域完整性 oracle 前，倾向隐藏该表。**

“不作跨域伪对账”正确解释了为什么不能把主表 `260 orders / Comm+Slip 594120` 与净额影子 `455 fills / Σ总费 1353002.745411` 硬等同，但没有解决：

- 净额 FeeAudit 自身是否覆盖全部净额 fill 无机器证明；
- 表内未把 `455` 明标为“净额影子fill”；
- 未展示 `unclassified_venue=1353002.745411`，导致可见分项无法重算总额；
- 顶部又把整个 voice 跑批静态称为“净额执行”。

因此读者仍可能把同一报告内两表理解成同一执行账本，或误以为总费没有科目来源。仅加一句跨域免责声明不够。

### 2. 有无新引入问题

除上述新增 MED 外，未发现新的执行数值或 OKLO 行为漂移：

- `assert_m8_audit_coverage` 对过滤零匹配、登记窗为空、全为空数据窗能在报告落盘前 fail-loud；定点 panic 测试通过。但其名称/错误文案将“跑过非空窗”称为“审计覆盖”，能力表述过强。
- OKLO `voice_exec=None` 路径仍执行三段同域断言；独立报告、dump、数据行均逐字节保持。
- BTC `voice_exec=Some` 的执行投影与 opsem dump 保持 #423 基线；问题只在 FeeAudit 完整性验证与报告语义，不在本次实测的执行结果。

## 动态复核记录

1. BTC 臂 R wf8 指定命令：**PASS**，`1 passed / 0 failed`，32.57s。
   - `trades.jsonl` 对 `/tmp/m8_win_gate/wf8/trades.jsonl`：SAME。
   - `tower_events.jsonl` 对 `/tmp/m8_win_gate/wf8/tower_events.jsonl`：SAME。
   - 报告对 #423：旧 19 列值完全相同；只新增预期三列和渲染/审计段。
2. OKLO 隔离命令：**PASS**，`1 passed / 0 failed`，10.58s。
   - `/tmp/rev481_m8_oklo_treasury.md` 与 `/tmp/484_m8_oklo_treasury.md`：全文件 SAME。
   - 两个 opsem dump 对 `/tmp/484_oklo_opsem` 和 `/tmp/419_oklo_opsem_reviewed`：均 SAME。
   - 对 `/tmp/419_m8_oklo_treasury.md`：两条数据行 SAME；仅 header/措辞/审计声明变化。
3. 六条 release 定点测试：全部 PASS：
   - `per_share_breakdown_total_is_bit_exact_with_legacy_formula`
   - `run_theta_v0_pi_loop_produces_trades_nonempty`
   - `fee_audit_accumulates_executed_quote_at_ledger_caliber`
   - `m8_epistemology_headings_match_symbol`
   - `m8_zero_audited_windows_fails_loud`
   - `m8_registered_but_unsupported_symbol_fails_loud`
4. `python3 scripts/check_armR_trades_digest.py`：exit 0。
   - p3fold `0x6bf47daf0aa737cd`
   - wf7 `0x282c28ca8ca65e16`
   - wf8 `0xcafa7c4846c762cd`
5. `cargo test --lib`：exit 101，`1976 passed / 4 failed / 135 ignored`，与在案指纹一致：
   - `lee_m3_attribution_dimension_is_readable_and_not_residual_only`
   - `lee_m4_cap_on_sparsity_has_no_unexplained_violation`
   - `lee_m4_level_cap_narrows_position_when_enabled`
   - `extract_signals_bit_exact_digest_guard`
6. `git diff --check`：exit 0。
7. `rustfmt --edition 2021 --check`：整文件 exit 1（既存格式债）；新增 hunk 范围 clean。

## 未覆盖声明

1. 未跑 BTC p3fold/wf7、OKLO 之外其他品种的完整 ignored M8 跑批；本票指定的 BTC wf8 与 OKLO 已覆盖。
2. 未做 30 万组随机 fuzz；以源码同形求和树、3⁵ `to_bits` 测试、两个独立 1-ULP 反例和 OKLO 全路径逐字节对拍核销。
3. 未尝试构造仓内故障注入（只读边界禁止改代码）；“整笔漏/重记仍绿”由 `record_executed` 同步累加结构与 voice 分支断言集合逐行证明。
4. 未使用外网或 `gh`。
5. 未修改仓内任何文件；共享工位既有修改/未跟踪文件未触碰、未归因。本复审唯一正式产出为 `/tmp/shadow-review-481-recheck-20260727.md`。
