# #481 三轮复审：#492 列名域微修核销（commit `7dbae797fc`）

评审日期：2026-07-27  
工作区：`/tmp/kimi-nest-mainline`  
HEAD：`7dbae797fcbc984b5e505ef1c4d3598c519c083b fix(rust): #492 审计表 fill 列名域显式化——#481 二轮复审残留 MED`  
固定对照：`d772ea8effcdcaf64fb6ec199aac8687f302ab17`  
复审面：`rust/src/theta_v0/backtest/wverify_run.rs`，`+32/-10`

## 结论

**PASS。**

#492 已完整核销 #481 二轮复审唯一残留 MED：

- `VOICE_EXEC=1`：表头为“声部投影trades / 净额影子fill”；
- 默认域：表头为“净额trades / 净额fill”；
- 两种域均由单一 helper 生成，生产报告按同一 tuple 的顺序消费；
- 数据行第 2、3 列仍分别是投影 trades 与净额 `FeeAudit.n_fills`；
- 定点测试、BTC/OKLO 两个真实产物均锁住该对应关系；
- 审计节旧列名 `真实fill` 已清理，OKLO 正式报告未引用该旧列名；
- 固定 diff 的 Standards / Spec 两轴均未发现新引入问题。

`cargo test --lib` 仍为在案的非绿状态，照实为 exit 101、
`1980 passed / 4 failed / 135 ignored`；四个失败与二轮复审指纹完全一致，
不是 #492 新增失败。

## 核销证据

### 1. 静态接线与列对应

`wverify_run.rs:1570-1577` 新增 `m8_fee_audit_headings`：

```text
voice_exec_is_some=true  -> 声部投影trades / 净额影子fill
voice_exec_is_some=false -> 净额trades / 净额fill
```

生产接线：

- `wverify_run.rs:1967-1968` 一次解构出 trades/fill 两个标题；
- `wverify_run.rs:1970-1976` 按第 2、3 列顺序消费这两个标题；
- `format_m8_fee_audit_row` 在 `:1591-1595` 按相同顺序输出
  `projected_trades`、`fee.n_fills`；
- 章节标题和引言在 `:1970-1973` 明确写成“净额账本 fill”及
  “本表的 fill 来自净额账本”；
- `:1777-1780` 仍逐窗硬断言 `r.voice_exec.is_some()` 与报告 gate 一致，
  因而动态标题不是仅由环境变量猜测实际返回域。

定点测试：

- `m8_fee_audit_headings_match_voice_exec_state` 锁住两个域的完整二元组；
- `m8_fee_row_renders_unclassified_venue` 把“净额影子fill”与 formatter
  第 3 列的 `fee.n_fills=1` 定点绑定；
- 独立运行 `cargo test --lib m8_fee_ -- --nocapture`：
  **exit 0，2 passed / 0 failed**。

默认域与声部域共用同一个 row formatter；其列位不随 gate 分叉。两个真实产物又分别覆盖
`voice_exec=None/Some`，因此不是只锁 helper、未锁生产渲染。

### 2. BTC 臂 R wf8 独立复跑

严格按票体指定的隔离命令运行：

```text
M8_WIN_FILTER=wf8 VOICE_EXEC=1 THETA_NEST_CERT_GATE=1
M8_REPORT_PATH=/tmp/rev492_armR_wf8.md
OPSEM_DUMP_DIR=/tmp/rev492_opsem_wf8
cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

结果：

- cargo test：**exit 0，1 passed / 0 failed**，测试体 34.10s；
- `/tmp/rev492_armR_wf8.md:38` 实产物表头：
  `声部投影trades / 净额影子fill`；
- `/tmp/rev492_opsem_wf8/trades.jsonl` 对
  `/tmp/m8_win_gate/wf8/trades.jsonl`：`cmp` **SAME**；
- `/tmp/rev492_opsem_wf8/tower_events.jsonl` 对
  `/tmp/m8_win_gate/wf8/tower_events.jsonl`：`cmp` **SAME**。

两侧目录都只有上述两个 dump 文件，没有漏比同目录第三个文件。

### 3. OKLO 独立复跑

严格按票体指定的隔离命令运行：

```text
M8_SYMBOL=OKLO M8_WIN_FILTER=oklo_oos
M8_FEE_DATUM=venue_fee_ibkr_pro_20260726.json:OKLO:PRO_TIERED_LE_300K_SHARES
M8_REPORT_PATH=/tmp/rev492_oklo.md
OPSEM_DUMP_DIR=/tmp/rev492_oklo_opsem
cargo test --release --lib theta_v0::backtest::wverify_run::m8_e2e_all_systems_oos -- --ignored --nocapture
```

结果：

- cargo test：**exit 0，1 passed / 0 failed**，测试体 10.03s；
- `/tmp/rev492_oklo.md:52` 实产物表头：
  `净额trades / 净额fill`；
- 从新报告与 `/tmp/484_490_oklo_treasury.md` 分别提取全部
  `^\| oklo_oos \|` 行后 `cmp`：**SAME**；
- 两条数据行（主表行和 fee audit 行）均完全相同，#490 已增加的
  `unclassified_venue` 列也保持原位；本次只有预期的标题/引言/列名渲染变化。

## 残留检查

1. 全仓（含 hidden、排除 `target`，不含仓外 `/tmp` shadow 证词）精确检索
   `真实fill`：**0 命中**。
2. `wverify_run.rs` 中带空格的“真实 fill”只剩 `:1785` 一处源码注释：
   “净额账本的真实 fill”。它已经显式限定净额账本域，不是报告审计节标题或列名。
3. 审计节旧标题 `Treasury 真实 fill` 与旧列 `| 真实fill |`：
   `wverify_run.rs`、OKLO 正式报告均 **0 命中**。
4. `chanlun/review-results/oklo-treasury-three-stage-20260727.md` 未引用旧 M8 列名。
   其中 `:150` 的“真实 fills / orders”是该正式报告自身的历史读数项，
   不是对旧列名 `真实fill` 的引用；不应为 #492 添加无依据订正块。

## 新引入排查

### Standards

**PASS，0 finding。**

- `git diff d772ea8eff..HEAD` 只有一个文件、三处相互关联的微修：
  单一标题 helper、生产消费、定点测试；
- 命名直接表达域，章节说明与实际 `FeeAudit` 域一致，核销 090 模糊地带；
- 未见新增 Mysterious Name、Duplicated Code、Feature Envy、Data Clumps、
  Primitive Obsession、Repeated Switches、Shotgun Surgery、Speculative Generality
  等 smell；
- `git diff --check d772ea8eff..HEAD`：**exit 0**。

`wverify_run.rs` 当前 2791 行，超过 `coding-style.md` 的 800 行上限；这是 T3 起在案且
已单独挂拆分票的既有结构债，不由 #492 引入，本轮不重复登记为新 finding。

### Spec

**PASS，0 finding。**

- 二轮复审要求的两个动态域逐字符实现；
- 双域 helper、生产表头、formatter 列位、实际 `voice_exec` 对拍和真实跑批形成闭环；
- 旧审计列名已清理；
- OKLO 正式报告不需要订正；
- 未见遗漏、实现错误或 scope creep。

## 全量与回归门

### `cargo test --lib`

**exit 101**：

```text
1980 passed / 4 failed / 135 ignored / 0 measured / 0 filtered out
```

四个失败与二轮复审相同：

1. `lee_m3_attribution_dimension_is_readable_and_not_residual_only`
2. `lee_m4_cap_on_sparsity_has_no_unexplained_violation`
3. `lee_m4_level_cap_narrows_position_when_enabled`
4. `extract_signals_bit_exact_digest_guard`

本轮通过数从二轮的 1979 增为 1980，正好对应 #492 新增的列名双域测试；失败集未扩大。

### `python3 scripts/check_armR_trades_digest.py`

**exit 0**：

```text
p3fold: n_trades=149 digest=0x6bf47daf0aa737cd
wf7:    n_trades=196 digest=0x282c28ca8ca65e16
wf8:    n_trades=168 digest=0xcafa7c4846c762cd
```

脚本结论：`臂R trades 逐位无漂移。`

## 共享工位边界

评审前后 `git status --short` 的既有脏面保持为：

- `M chanlun/agent-roster-2026-07-21.md`
- `M rust/src/theta_v0/classifier/level_view.rs`
- `M rust/src/theta_v0/classifier/nest_lifecycle.rs`
- 既有未跟踪 review/probe 文件若干。

这些文件均未触碰、未归因、未纳入 #492 diff。仓内无任何评审修改；本轮只写票体授权的
`/tmp/rev492_*` 跑批产物和本报告。

## 未覆盖声明

1. 未复跑 BTC p3fold/wf7、OKLO 之外其他品种的完整 ignored M8；票体指定的 BTC wf8
   与 OKLO 已覆盖两个列名域。
2. 只读边界下未做源码故障注入；列位由源码、定点测试和两个真实产物交叉验证。
3. 未处理 `cargo test --lib` 四个既有失败，也未调查共享工位脏文件对其他测试的影响。
4. 未使用外网或 `gh`。
5. 未修改仓内文件；唯一正式评审产出为
   `/tmp/shadow-review-481-recheck3-20260727.md`。
