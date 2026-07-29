# 影子评审：#423 层4恢复 scalar 三分叉（`c0c1a74b16`）

## 结论

**总体：打回。最高等级：HIGH。**

- **Standards：打回 / HIGH**。最终 commit 内存在声明与能力相反、机器测试计数失实、源码行号失准，以及“自动纳入”能力虚构。
- **Spec：打回 / HIGH**。三分叉、结构守卫、按股/非对称 fail-loud、臂R bit-exact、D/C 层4 LCB/三态数值均正确；但最终 22 列输出形态没有补跑落盘，验收②的最终交付物只部分兑现。

审计面固定为 `8b29b13aa6..c0c1a74b16`。源码行号均来自 `git show c0c1a74b16:<path> | nl -ba`，未使用当前 HEAD 后续票内容归因。

## Standards

### HIGH-1：最终 commit 的补遗声明与实际输出能力相反，且 `/tmp/423_*` 不是最终输出面产物

证据：

- `rust/src/theta_v0/backtest/wverify_run.rs:358-379` 明确新增 `layer4_random_control_cells`，把 `theta_beats_random / shift_pvalue / indep_pvalue` 变成三格。
- 同文件 `:1588-1592` 把 `Θ>随机 / p_shift / p_indep` 加进 m8 表头；`:1665-1694` 保留完整 `Significance`、渲染并落盘三值。
- 但 `chanlun/review-results/treasury-reverify-20260727.md:892-898` 与
  `chanlun/review-results/treasury-reverify-t2-armD-20260727.md:562-567` 仍声明三值“未产出”“m8 不渲染”“无落盘出口”，还要求后续加列。
- 独立检查九份 `/tmp/423_arm{R,D,C}_report_{p3fold,wf7,wf8}.md`：表头均为旧 19 列，`Θ>随机` 零命中；最终 commit 的生成器是 22 列。因此这些产物可支持既有 LCB/三态数值，但不能冒充最终 commit 输出面的完整产物。

独立复核：

```bash
git grep -n -E 'm8 四层报告不渲染|layer4_random_control_cells|Θ>随机|p_shift|p_indep' \
  c0c1a74b16 -- 'chanlun/review-results/treasury-reverify*.md' \
  rust/src/theta_v0/backtest/wverify_run.rs
for f in /tmp/423_arm{R,D,C}_report_{p3fold,wf7,wf8}.md; do
  grep -n 'Θ>随机' "$f" || true
done
```

判定：090 声明纪律的直接违反；文档说能力不存在，代码却已实现，现存跑批产物又停在加列前。

### HIGH-2：新增内容中的多组“当前行号”在同一固定 commit 已失准

证据（均以 `c0c1a74b16` 为准）：

- `scripts/fee_account_decomposition.py:60-62,116-119,185-199` 多次指向
  `treasury.rs:124/128`；实际 `fee_quoter` 在 `treasury.rs:174`，`assert!` 在 `:178`。
- `rust/src/theta_v0/backtest/treasury.rs:154-157` 自述 assert 在 `treasury.rs:128`，实际同上。
- T3 `treasury-reverify-20260727.md:865-866` 指 `wverify_run.rs:1601-1606` 的同真值断言；实际在 `wverify_run.rs:1659-1663`。
- T3 `:887-890` 与 T2 `treasury-reverify-t2-armD-20260727.md:556-560` 指 `wverify_run.rs:1862` 的 unavailable 断言；实际在 `:1926`。
- T3 `treasury-reverify-20260727.md:198-203` 的“行号订正”称默认三常数在 `config.rs:302-304`；最终 blob 实际在 `config.rs:318-320`。
- T2/T3 未产出表指 `wverify_run.rs:1608-1617` 只消费 `boot_ci95_lo`；最终 blob 的真实调用/消费在 `:1667-1690`，且已消费三项随机对照值。

独立复核：

```bash
git show c0c1a74b16:rust/src/theta_v0/backtest/treasury.rs | nl -ba | sed -n '150,190p'
git show c0c1a74b16:rust/src/theta_v0/backtest/wverify_run.rs | nl -ba | sed -n '1584,1700p;1900,1940p'
git show c0c1a74b16:rust/src/theta_v0/config.rs | nl -ba | sed -n '300,325p'
```

判定：用户打回标准明定“数字/行号事实错误 = HIGH”；这里不是后续 HEAD 漂移，而是在被审 commit 自身失准。

### HIGH-3：费率分解脚本虚构“上游改变后自动纳入”的能力

证据：

- `scripts/fee_account_decomposition.py:60-64` 把 D/C tax 硬编码为
  `ARM_CALIBRATED_TAX_BPS = 0.0`。
- `:148-151` 主循环只把这个 Python 常量传给 D/C 分解。
- `:196-199` 却声称“若上游改变该约束或该常量非零，本表逐笔重算会自动纳入”。
- Rust 的 `fee_quoter` assert 即使放宽，也不会修改 Python 常量；脚本没有读取 `ExecConfig` 或该 assert。

独立复核：

```bash
git show c0c1a74b16:scripts/fee_account_decomposition.py | nl -ba | \
  sed -n '50,70p;140,165p;175,205p'
```

判定：能力声明不实，违反 090；同时削弱尾扫第 6 项“声明处/强制处归属”的诚实度。

### HIGH-4：测试数量与全量指纹声明均可机器证伪

证据：

- 固定父点与被审点分别独立 `cargo test --lib -- --list`：`2080 → 2096`，净增 **16** 个可执行 lib test，不是 commit message 的“+25”。
- 从固定 commit 的独立 `git archive` 快照复跑：
  - debug：`1957 passed / 4 failed / 135 ignored`；
  - release：`1960 passed / 1 failed / 135 ignored`。
- T3 `treasury-reverify-20260727.md:900-915` 与 T2
  `treasury-reverify-t2-armD-20260727.md:569-581` 登记的是 release `1966/1/135`、debug `1963/4/135`，两档都各多报 6 个通过测试。
- 失败集本身未新增：debug 仍为 lee_m3、lee_m4×2、digest；release 仍只有 digest。

独立复核：

```bash
# 分别把两个 ref git archive 到 /tmp 后：
cd <snapshot>/rust
CARGO_TARGET_DIR=/tmp/rev423_target cargo test --lib -- --list
CARGO_TARGET_DIR=/tmp/rev423_target cargo test --lib
CARGO_TARGET_DIR=/tmp/rev423_target cargo test --release --lib
```

判定：失败指纹“未劣化”成立，但测试数量与通过数声明不成立；按用户标准为 HIGH。

### MED-1：“唯一消费者”未限定生产面

证据：

- `rust/src/theta_v0/venue_fee.rs:216-218` 无限定声称
  `constant_effective_rate` 的“唯一消费者 = scalar_cost_rate_opt”。
- 生产代码确实只有 `treasury.rs:164` 消费；但同文件测试仍在
  `venue_fee.rs:798,820,827,840,857,861` 六处直接调用。

独立复核：

```bash
git grep -n 'constant_effective_rate()' c0c1a74b16 -- rust/src
```

判定：应写“唯一生产消费者”；现文属于可消歧的 MED，不影响分叉本体。

## Spec

### HIGH-S1：验收②仅部分兑现——数值正确，但最终输出形态没有补跑落盘

`wverify_run.rs:358-399,1588-1592,1665-1690` 已把随机对照三列纳入 #423 的层4最终输出；九份现存 `/tmp/423_arm*report*` 却均为旧 19 列，两份补遗还把三值登记成“未产出”。因此 LCB/三态数字可以验收，但 final commit 所声明的层4输出形态没有对应的跑批交付物。证据与独立复核命令同 Standards HIGH-1。

### 核心分叉：PASS

- `treasury.rs:161-167`：未标定 `Some(fee_rate)`；标定档委托结构判定，成功时合成
  `datum + slippage + tax`。
- `venue_fee.rs:256-271`：只接受 `FeeUnit::Notional`，以 `to_bits` 要求 maker/taker 逐位对称，并以 `production_bps.is_finite()` 拒 NaN/Inf；per-share 返回 `None`。
- `venue_fee.rs:111` 固定生产角色为 Taker；`:312-337` 的角色版方法私有，对外生产报价不接角色。
- 全仓成交点穷查：`fill.rs:153-186` 三处与 `overlay_state.rs:63-85` 一处均只调用无角色参数的 `production_rate_or_fallback`；两文件生产区无 `LiquidityRole` import。直接绕过 `FeeQuoter` 调 schedule 的路径未被编译期封死，但 `venue_fee.rs:103-110,327-330` 已照实登记。
- 在独立 `/tmp` commit 快照中临时给 `fill.rs:160` 的生产报价加第 4 个角色实参后，
  `cargo check --tests` 以 `E0061` / exit `101` 失败，并指向 `venue_fee.rs:331` 的三参数签名；“传角色即编译错误”已实际复跑，不是转述。

独立复核：

```bash
git grep -n -E 'LiquidityRole|production_rate_or_fallback|rate_with_role|\.fee_usd\(|\.effective_rate\(' \
  c0c1a74b16 -- rust/src/theta_v0/backtest/fill.rs \
  rust/src/theta_v0/strategy/overlay_state.rs rust/src/theta_v0/venue_fee.rs
```

### bit-exact 与 fail-loud：PASS

- `python3 scripts/check_armR_trades_digest.py` 独立复跑 exit `0`：
  p3fold `149/0x6bf47daf0aa737cd`、wf7 `196/0x282c28ca8ca65e16`、
  wf8 `168/0xcafa7c4846c762cd`。
- 三窗 × `{trades,tower_events}` 六组 `cmp` 全部 `SAME`。
- 旧防线从“所有标定档”收窄为“无良定义档”：`treasury.rs:396-424` 分别以 per-share `[A]`、非对称 notional `[B]` 保留 panic，且消息分辨成因；不是删锁。

独立复核：

```bash
python3 scripts/check_armR_trades_digest.py
for w in p3fold wf7 wf8; do
  for f in trades tower_events; do
    cmp "/tmp/423_backup_m8_win_gate/$w/$f.jsonl" "/tmp/m8_win_gate/$w/$f.jsonl"
  done
done
```

### 恢复读数：PASS（仅就 LCB/三态）

- T3 `treasury-reverify-20260727.md:815-825` 的九格 R/LCB/三态逐项等于
  `/tmp/423_arm{R,D,C}_report_{p3fold,wf7,wf8}.md`。
- T2 `treasury-reverify-t2-armD-20260727.md:499-503` 的 D 三窗同样逐项相等。
- 九格 LCB 全 `≤0`；p3fold/wf7 为 `无(R≤0)`，wf8 为 `INCONCLUSIVE`，没有冒充 confirmed alpha。
- `/tmp/423_backup_m8_win_gate/...` 六件、三份
  `/tmp/423_backup_388_armD_report_*.md` 与
  `/tmp/423_backup_m8_e2e_all_systems_oos.md` 均真实存在。

### 测试覆盖：核心红测 PASS，数量声明 FAIL（见 Standards HIGH-4）

固定 commit 独立快照定点复跑：

- `scalar_cost_rate_opt`：5 passed；
- `constant_rate`：3 passed；
- `layer4_notice`：5 passed；
- `production_quote_takes_role_from_single_constant`：1 passed。

`treasury.rs:322-342` 的确枚举 `4 qty × 4 px × 2 side = 32` 组，并用 `assert_eq!` 将 scalar 与生产 quoter 逐位锁同；`:348-393` 覆盖非对称、per-share、真实 Binance/IBKR 分叉。

### L3 消费面与认识论：PASS

- `l3_fullwindow.rs:132-137`、`l3_pi_falsify.rs:149-154` 均从
  `ThetaConfig::default()` 且无 datum 注入通道推出当前 `expect` 不可达，并保留未来 fail-loud。
- `l3_delta_r_alpha.rs:860-877` 把唯一调用面限定为 `#[ignore]` 合成鞅守卫，明确不在 m8 路径。
- 合成/契约测试标 L0，真实 datum 装载与算术标 L1，BTC OOS 三窗补遗标 L2；与
  `.claude/rules/formalization-validity-domain.md` 一致，未发现等级膨胀。

### #447 尾扫六项

1. MED-1 三处语义收窄：**核销**。`runner.rs:135-160`、`treasury.rs:41-70`、
   `fill.rs:3848-3864` 均改为三分叉/按股有效域，不再把所有标定档说成无良定义。
2. 1.8 倍归因：**核销**。T2 `:125-134` 明确只有最低佣金托底触发，上限/TAF 封顶均未触发。
3. 正则消费方：**核销**。`fill.rs:4050-4060` 改为未来/外部格式契约，不虚构仓内消费方。
4. 断言重言：**核销**。`fill.rs:4075-4082` 明说该断言不证明哈希正确性。
5. 9 列 vs 10 列：**核销**。T3 `:234-260` 并列保留旧 9 列表与脚本当前 10 列补遗。
6. config 声明处 vs treasury 强制处：**部分核销后仍失败**。`config.rs:265-277` 的语义归属正确；
   但脚本的强制处行号错误，且宣称上游改变会自动纳入为假（Standards HIGH-2/HIGH-3）。

残留 grep 中未发现新的“标定档全体无良定义”误用；命中均为订正/历史引文，或对 per-share、非对称 notional 的合法“无良定义”描述。

## Cargo 边界

- 被审 commit 独立快照：debug `1957/4/135`，release `1960/1/135`；见 Standards HIGH-4。
- 当前共享工作树（HEAD `f695e6a8c8`，保留并未触碰用户列明的 M/未跟踪文件）：
  `1980 passed / 4 failed / 135 ignored`，失败集恰为 lee_m3、lee_m4×2、digest，与用户预期一致。
- 上述当前工作树结果只作回归边界，不归因给 #423。

## 未覆盖声明

- 未重跑 m8 九臂窗：票面原则允许以现存产物复核；但现存 `/tmp/423_*` 是新增随机对照三列之前的 19 列产物。因此本评审只确认其中 LCB/三态与账本列，不确认最终 22 列输出的九窗真实 `Θ>随机/p_shift/p_indep` 值。
- “向 FeeQuoter 生产报价传角色会编译错误”已在临时快照实际得到 E0061；但仓内没有持久化的 trybuild/compile-fail 回归用例。
- 未验证直接绕过 `FeeQuoter` 的未来调用不会出现；代码已明确承认该路径没有编译期屏障。
- 未对 LCB 臂间差做费用/规模/帽形态份额分解；两份补遗已登记此缺席。
