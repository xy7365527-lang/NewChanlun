# 影子评审：#446 活动集唯一性修复（`f838540eff`）

日期：2026-07-28  
工作区：`/tmp/kimi-nest-mainline`  
评审方式：只读；未改仓内文件、未做 git mutation；跑批及复核产物仅写 `/tmp/rev446_*`

## 结论

**打回。**

| 轴 | 结论 | HIGH | MED | LOW |
|---|---|---:|---:|---:|
| Standards | **打回** | 0 | 3 | 2 |
| Spec | **打回** | 0 | 2 | 0 |
| 合计（去重） | **打回** | **0** | **5** | **2** |

没有发现修复语义错误、索引失同步、重验数字不符或逐笔分类错误，因此没有 HIGH。打回原因是：

1. release 唯一性探针不是 coverage/fill 入口的 fail-loud 防线，只在 m8 ignored 测试缝事后硬断言；
2. canonical 恢复后，T3 当前态声明和逐笔 diff 机器件的旧侧路径仍指向过期位置；
3. golden 的 `--regen` 不能再生当前提交的 provenance schema；
4. `strategy_target_legs` 的生产消费面和已发布读数订正登记没有穷尽。

## 0. 边界与固定点

- `git rev-parse HEAD` 独立复核为
  `f838540eff66c211c6e0f1e5b247bbc459003a48`。
- `git show --stat f838540eff` 为票面指定的 14 文件、`+765/-111`。
- `6e15ceffee` 是 `f838540eff` 的祖先及跑批最终核验点；`f838540eff` 的直接父提交是
  `ce2038b779`。这与 golden 明示的“最终核验点”而非“被审提交态”一致，不判伪装。
- 评审开始和结束时仓内仅观察到既有
  `M chanlun/agent-roster-2026-07-21.md`；未触碰、未归因。
- `git diff f838540eff^ f838540eff --check` exit 0。

## 1. Findings

### Standards MED-1：release 计数不是生产 fail-loud 防线，且在错误值已计算后才检查

**影响**

`duplicate_active_id_violations` 的计数点位于 coverage 层，所有进入
`coverage_step_from_buckets_sep` 的路径都会扫描；但 release 下发现重复只累加 thread-local
计数，仍返回已经双计过的 `p_tilde/sep_legs`。硬断言仅接在 m8 ignored 测试缝，未覆盖全部生产
fill/诊断消费面。

这不否定本次注册生产者修复本身，但“release 防线”能力不能外推为生产 fail-loud。

**证据**

- `rust/src/theta_v0/strategy/coverage/step.rs:282-297`：先由 `next_idx` 计算
  `strategy_target_legs` 和 `p_tilde`。
- `rust/src/theta_v0/strategy/coverage/step.rs:317-345`：发现重复后只 bump 计数并
  `debug_assert!(false)`。
- `rust/src/theta_v0/strategy/coverage/step.rs:346-362`：release 继续组装 `sep_legs` 并返回。
- `rust/src/theta_v0/backtest/wverify_run.rs:1776-1788`：唯一逐窗 release 硬断言位于
  `m8_e2e_all_systems_oos`。
- `rust/src/theta_v0/backtest/l3_fullwindow.rs:449-467`、
  `rust/src/theta_v0/backtest/incremental.rs:1778-1791`、
  `rust/src/theta_v0/strategy/coverage/sizing.rs:609-675`：同一 coverage 生产消费面的其他入口，
  没有对应 reset/snapshot/assert。
- `rust/src/theta_v0/backtest/runner.rs:5246-5265`：另一个 snapshot 是 ignored 诊断测试，且不
  断言 `duplicate_active_id_violations`。
- `.chanlun/genealogy/settled/090-strictness-grammar-rule.md:39-46`：
  声明须与实际能力一致，边界须显式。
- `.claude/rules/common/coding-style.md:23-37`：错误不得静默吞掉，边界应 fail fast。

**独立复核**

```bash
rg -n 'ancok_probe_(reset|snapshot)\(\)|duplicate_active_id_violations' rust/src
rg -n 'coverage_step_(classification|prebuilt|from_buckets)' rust/src/theta_v0
```

结果：生产层扫描是共享的；reset/snapshot 后对 duplicate 做硬断言的只有
`wverify_run.rs:1776-1788`。以 release 编译同构的
“bump + `debug_assert!(false)` + return”最小程序，得到
`violations=1; returned=true; exit=0`。

**应修**

在 coverage/fill 的生产边界建立 release 可见的错误返回或 fail-loud 终止；若只承诺 m8
观测，则将所有“release 防线”表述收窄为“m8 逐窗事后断言”，并明确其他入口未覆盖。

### Standards MED-2：T3 的 canonical 当前态声明与实装、T1 验收订正相反

**证据**

- `scripts/check_armR_trades_digest.py:40-43`：当前默认根是 `/tmp/m8_win_gate`。
- `chanlun/review-results/treasury-reverify-t1-armR-20260727.md:733-737`：编排者验收轮明确
  恢复 `/tmp/m8_win_gate`，旧产物移到 `/tmp/423_backup_m8_win_gate`。
- `chanlun/review-results/treasury-reverify-20260727.md:1078-1080`：仍称默认根已统一为
  `/tmp/446_armR_dump`。
- 同文件 `:1085` 继续给出无条件 Standards/Spec PASS，没有把上一条标为被后续验收覆盖的历史态。

这违反 090 的声明—能力一致与注释诚实。

**独立复核**

```bash
python3 scripts/check_armR_trades_digest.py
```

exit 0，实际读取 `/tmp/m8_win_gate`，输出：

```text
p3fold  n=141  digest=0xac5952cfcd8b8746  bytes=195220
wf7     n=184  digest=0x981a0560b8db0b40  bytes=253658
wf8     n=159  digest=0x18291f8ba8f1d40f  bytes=216129
```

三窗 current canonical 与 `/tmp/446_armR_dump` 的 `trades.jsonl`、`tower_events.jsonl`
共 6/6 `cmp -s` exit 0。

**应修**

在 T3 §15.5 明示该中间态已被 T1 验收轮覆盖，当前默认根以
`/tmp/m8_win_gate` 为准。

### Standards MED-3：`--regen` 会改写/丢失 golden provenance，而正常校验不会发现

**证据**

- 当前 golden
  `chanlun/review-results/treasury-reverify-t1-armR-trades-golden-20260727.json:3-6`
  保存：
  - `_source_base_head=a12a...`
  - `_final_verification_head=6e15...`
  - `_parallel_head_note`
  - `_source_worktree`
- `scripts/check_armR_trades_digest.py:132-151` 的 `--regen` 却生成：
  - `_source_base_head=6e15...`
  - 新字段 `_source_lineage_start=a12a...`
  - 不生成 `_final_verification_head`
  - 不生成 `_parallel_head_note`
- `scripts/check_armR_trades_digest.py:157-162` 的正常校验只读取 `windows`，provenance
  漂移不会使门失败。

**独立复核**

将模块内 `GOLDEN_PATH` 临时重定向到 `/tmp/rev446_regen_probe.json` 后，真实调用
`main --dump-dir /tmp/446_armR_dump --regen`。产物
`/tmp/rev446_regen_probe.json:3-5` 精确复现上述 schema/语义差异；仓内 golden 未改。

当前已提交 golden 本身与 git log 对拍，**不判元数据伪装**；缺口在机器再生路径。

**应修**

让生成器单源生成当前 provenance schema，并把关键 provenance 字段纳入校验；不得以硬编码的
历史 HEAD 覆盖当前真值。

### Spec MED-1：逐笔 diff 机器件的“旧侧路径”在 canonical 恢复后已失真

**证据**

- `/tmp/446_armR_trades_diff.json:2`：
  `_scope` 仍写“旧=`/tmp/m8_win_gate`，新=`/tmp/446_armR_dump`”。
- `/tmp/446_armR_trades_diff.md:3`：同一旧侧路径。
- `chanlun/review-results/treasury-reverify-t1-armR-20260727.md:733-737`：
  真实旧侧现为 `/tmp/423_backup_m8_win_gate`。

按机器件自述路径直接复核会变成新对新并得到零差异；必须先读 T1 才能恢复真实旧侧。因此
分类内容虽正确，机器证据入口标签不再自足。

**独立复核**

- `/tmp/m8_win_gate/<tag>` 与 `/tmp/446_armR_dump/<tag>`：三窗 trades 3/3 SAME，
  tower 3/3 SAME。
- `/tmp/423_backup_m8_win_gate/<tag>/trades.jsonl` 与 current：三窗 3/3 DIFFER。
- backup 三窗 SHA-256 精确命中
  `/tmp/446_armR_trades_diff.json:15-16` 及对应 wf7/wf8 summary 的 `old_sha256`；
  `/tmp/446_armR_dump` 精确命中 `new_sha256`。

**应修**

同步修正 diff JSON/MD 的旧证据根为 `/tmp/423_backup_m8_win_gate`，或冻结一份带内容寻址的
不可变证据清单。

### Spec MED-2：污染面登记未穷尽；只订正了 T1/T3 内部表，其他已发布读数未标为 superseded

**证据**

- `strategy_target_legs` 的唯一生产调用点是
  `rust/src/theta_v0/strategy/coverage/step.rs:282-297`；它位于所有
  `coverage_step_from_buckets[_sep]` 共享路径，不限 m8/LEE。
- 生产/诊断消费至少还包括：
  - `rust/src/theta_v0/strategy/coverage/sizing.rs:609-675`
  - `rust/src/theta_v0/backtest/l3_fullwindow.rs:449-467`
  - `rust/src/theta_v0/backtest/incremental.rs:1778-1791`
- T3 的订正声明
  `chanlun/review-results/treasury-reverify-20260727.md:994-997`
  只点名本文件 §4.1、§5.3、§14.4、§14.8。
- 仍存在未加 #446 superseded 标记的已发布旧数，例如
  `chanlun/review-results/treasury-reverify-t2-armD-20260727.md:76-97,262-273`：
  wf8 D/R 仍为旧 `+4504490/+4980320` 与 `+6346975/+6851062`；
  当前 T3 `treasury-reverify-20260727.md:1025-1035` 已是
  `+4463727/+4938316` 与 `+6281284/+6782644`。
- 更早的同路径报告还包括
  `chanlun/review-results/v4-three-window-typed-chain-acceptance-20260721.md:49-61`、
  `chanlun/review-results/v4-three-arm-acceptance-20260720.md:38-49`、
  `chanlun/review-results/w2-dual-ledger-m8-impl-20260719.md:30-61`。
- 非本次九窗的生产路径读数/结论也有公开消费者，例如
  `chanlun/review-results/dual-open-implementation-audit-20260719.md:67-116`
  和 L3/run-theta 族；本轮未重跑，不能证明每份旧产物是否实际命中碰撞。

**独立复核**

```bash
rg -n 'strategy_target_legs\(' rust/src
rg -n 'coverage_step_(classification|prebuilt|from_buckets)' rust/src/theta_v0
rg -l 'coverage_step|strategy_target_legs|p_tilde|p̃|run_theta_v0_pi' \
  chanlun/review-results --glob '*.md'
```

前两条证明共享代码消费面；第三条给出远大于 T1/T3 的已发布文档集合。

**边界**

这条 finding 不声称所有历史报告的每个数字都已被碰撞污染；能确认的是订正/作废登记没有覆盖
全部消费面，且至少 T2 的旧 m8 数仍以未 superseded 形式存在。非 m8/LEE 历史读数本轮未重跑，
照实列为未覆盖。

**应修**

建立受影响报告清单：能由新跑批替代的逐份加 current/superseded 指针；不能重跑的明确标
“受 #446 影响可能性未排除，不得作为当前代码态读数”。

### Standards LOW-1：继续扩大已超过 800 行上限的 `wverify_run.rs`

- `.claude/rules/common/coding-style.md:15-21,39-45`：文件上限 800 行。
- `rust/src/theta_v0/backtest/wverify_run.rs:1776-1788` 是本提交新增段的一部分。
- 独立 `git show <rev>:<path> | wc -l`：父提交 2791 行，被审提交 2812 行，净增 21 行。

建议把 #446 window guard/report 行拆入专用 helper/module。

### Standards LOW-2：新增测试函数超过 `<50` 行并重复构造夹具

- `.claude/rules/common/coding-style.md:39-45`：函数应小于 50 行。
- `rust/src/theta_v0/strategy/coverage/step_tests.rs:386-441`：56 行。
- 同文件 `:445-535`：91 行。
- 同文件 `:247-382` 与上述函数重复构造 carrier/parent/elements/buckets/registry。

独立按函数起止行重算为 56/91 行。建议抽 `#446` 专用 fixture builder。

### Standards 其余核查：PASS

- **认识论分级**：
  `chanlun/review-results/treasury-reverify-20260727.md:453-465` 将真实 BTC 跑批限定在 L2；
  同文件 `:1043` 明确九窗数只作能力、唯一性和配置差异审计，不作 alpha 论据或策略择优输入。
  未见从机器跑批越级到策略有效性的声明。
- **守卫注释诚实**：`step.rs:317-344` 正确区分“release 累计、debug fail-fast”，并以中性文本
  描述重复 ID；问题是 m8 报告/最终闭环对生产能力的外推，已单列 MED-1。
- **格式与基本风格**：`git diff f838540eff^ f838540eff --check` exit 0；未见新增
  whitespace 错误。文件/函数体量问题已单列 LOW-1/LOW-2。

## 2. 核心语义核查：PASS

实现逐项兑现 `/tmp/446_diagnosis-20260727.md:280-306` 的 §4 方案：
open 按 ID 注册并处理同向/反向；restore 不复用候选 copy；held 恢复以持仓身份为权威；
唯一性守卫保留根因中立文本并有纯函数/集成覆盖。

### 2.1 open 同向去重、反向湮灭

- `step.rs:192-215` 先由 held/restore `raw` 构造 `raw_by_id`：
  - held/restore 身份优先占位；
  - open 同 ID 同向保留首个；
  - open 同 ID反向删除先前 open，并同步删除 `raw_by_id/open_pushed`；
  - 候选不会湮灭已有 held/restore 身份。
- `step.rs:232-248` 在 open 触发父链 restore 后，将新增 raw idx 回灌 `raw_by_id`。
- `rust/src/theta_v0/strategy/coverage/step_tests.rs:247-338` 覆盖同向首现与反向逐对湮灭；
  `:340-441` 覆盖 held/restore/candidate 交叉复用。

**反向湮灭的构造性论证**

`ElementId` 是 carrier/位置节点的结构身份，不是“交易腿序号”。同一个 ID 的两种方向是同一
位置实例的冲突 open 意图；合法的父/子、不同 carrier 或不同声部腿具有不同 ID。若没有 held
权威身份，两条同 ID 反向新意图不存在可证明的赢家，逐对湮灭是确定性且不虚构持仓的处理。
已有 held 身份不进入 `open_pushed`，因此新反向 candidate 不会误当 close 消掉合法持仓腿。

结构证据是 `rust/src/theta_v0/classifier/recursive_tower.rs:68-87` 对 `ElementId` 的
“级别 + compose 输出序号”定义，以及
`rust/src/theta_v0/strategy/interp.rs:716-741` 将 candidate `id` 绑定到 `carrier_id` 的实现。

未构造出“合法两腿必须共享同一 ElementId”的反例；若未来账户模型允许同 ID hedge 双开，则
须先改变 ElementId/活动集定义，不能在当前集合语义内偷偷承载。

### 2.2 restore/held 复用与 ID→idx 同步

- `held.rs:106-154`：
  - 只复用 `idx >= overlay_cand_end` 的持久 overlay；
  - 禁止复用初始 candidate copy；
  - 复用时由 held 权威属性覆盖；
  - 新 push 同步 `overlay_seen` 和 parent fixup。
- `held.rs:268-340`：
  - base `id_idx` 优先；
  - overlay 只接受持久区；
  - registry 新物化后同步 `overlay_seen`、fixup、raw。
- `element.rs:120-129`：覆盖 overlay 时 idx 不变。
- `step.rs:94-164` 的 held 各 raw 写入均发生在 `raw_by_id` 建表前，因此由
  `step.rs:194-197` 一次性纳入。
- `step.rs:205-214` 的 open add/remove 和 `:244-247` 的 restore 追加均同步索引。

逐个写入点复核后，未见 raw 与 ID→idx 索引失同步。

### 2.3 守卫与计数语义

- `step.rs:317-344` 的错误文本根因中立，打印重复 ID、两 idx 和来源分类。
- `ancok.rs:80-82` 的字段注释对应“出现至少一对重复的 step 次数”，实现每 step 最多 bump
  一次，声明与实现一致。
- 缺口仅是 Standards MED-1 的 release fail-loud/覆盖范围，不是计数算法错误。

## 3. 机器声明独立复核

### 3.1 wf8 臂 R 独立 release 复跑：PASS

使用票面原命令，隔离输出到：

- `M8_REPORT_PATH=/tmp/rev446_armR_wf8.md`
- `OPSEM_DUMP_DIR=/tmp/rev446_opsem_wf8`

结果：1 passed，exit 0。

- `/tmp/rev446_armR_wf8.md:23`：
  `execR=+6281284`、`R=+6782644`、`LCB=-2427669`、`INCONCLUSIVE`。
- `/tmp/rev446_armR_wf8.md:34-40`：`duplicate_id_violations=0`。
- 独立 dump SHA-256 与 `/tmp/446_armR_dump/wf8` 的 trades/tower 两文件均一致。

因此九窗表中的 wf8 R 零违规与经济读数不是只抄旧报告。

### 3.2 九窗 T3 对拍：PASS

独立解析九份 `.out` 与九份 report，对拍 T3
`treasury-reverify-20260727.md:1007-1041`：

- 9/9 `duplicate-ID=0`；
- 9/9 LEE 各列一致；
- 9/9 经济表各列一致；
- 3/3 C-D 算术一致。

wf8 三臂：

| 臂 | LEE `(dup,cap,n_cap,n_rescaled,max_abs,orders,off_clock,explained,off_delta,unexplained)` | `execR / R / LCB` |
|---|---|---|
| R | `0,false,0,518,1153,438,136,136,362,0` | `+6281284 / +6782644 / -2427669` |
| D | `0,false,0,568,1145,486,183,183,410,0` | `+4463727 / +4938316 / -3921003` |
| C | `0,true,809,265,156,448,149,149,283,0` | `+4289720 / +4739268 / -3632422` |

证据：

- `/tmp/446_armR_wf8.out:441-444`、`/tmp/446_armR_report_wf8.md:23,40`
- `/tmp/446_armD_wf8.out:440-443`、`/tmp/446_armD_report_wf8.md:33,50`
- `/tmp/446_armC_wf8.out:440-443`、`/tmp/446_armC_report_wf8.md:37,54,64`

独立算术 `C-D = -174007 / -199048 / +288581`，与 T3 `:1037-1041` 一致。

wf8 R 的 LCB 从 `-2105181` 到 `-2427669`，差 `-322488`。去重改变
`target legs → LexArgmin/目标仓位 → 后续 active → orders/fills/fees → bootstrap 样本`，
是跨 bar 非线性级联；消除非法双计不保证经济指标单调改善，LCB 的变化也不应等于单笔 PnL 的
线性差。报告只把新值用于能力/审计，不声称收益改善，机制解释自洽。

### 3.3 六文件唯一性、逐笔分类与 golden：PASS

独立脚本 `/tmp/rev446_verify_trades.py:7-66`：

- 旧侧显式使用 `/tmp/423_backup_m8_win_gate`；
- 新侧使用 `/tmp/446_armR_dump`；
- 独立解析 JSONL、按 `position_node_id` 查重；
- 忽略 `trade_id` 后重算 unchanged/changed/removed/added；
- 重算 SHA-256 并与 diff summary 对拍。

结果：

| 窗 | old/new | unchanged/changed/removed/added | old unique | new unique |
|---|---:|---:|---|---|
| p3fold | 149/141 | 29/105/15/7 | 是 | 是 |
| wf7 | 196/184 | 2/169/25/13 | 是 | 是 |
| wf8 | 168/159 | 52/106/10/1 | 是 | 是 |

进一步逐 record 重建并核验 `class`、old/new trade_id、`changed_fields`、field values、
removed/added 完整 payload：

- p3fold 127 records，0 error；
- wf7 207 records，0 error；
- wf8 117 records，0 error。

故 `/tmp/446_armR_trades_diff.json` 的分类内容正确，不触发 HIGH；问题只有
Spec MED-1 的旧侧路径标签。

独立 FNV-1a64/bytes/count/direction 重算：

| 窗 | FNV-1a64 | bytes | n | Long/Short |
|---|---|---:|---:|---:|
| p3fold | `0xac5952cfcd8b8746` | 195220 | 141 | 72/69 |
| wf7 | `0x981a0560b8db0b40` | 253658 | 184 | 99/85 |
| wf8 | `0x18291f8ba8f1d40f` | 216129 | 159 | 87/72 |

全部命中 golden `...trades-golden-20260727.json:11` 起三窗记录。

### 3.4 golden 当前元数据：PASS（再生路径另见 MED-3）

- golden `:3-6` 明示初始 base `a12a...`、最终核验点 `6e15...`、并行 #421 三文件和
  #446 跑批时未提交态。
- `git merge-base --is-ancestor a12a... 6e15...` exit 0。
- `git merge-base --is-ancestor 6e15... f838...` exit 0。
- `git show --name-status 6e15...` 恰为
  `p123_fast_replay.rs`、`level_view.rs`、`nest_lifecycle.rs`。

当前 committed golden 未伪装成 `f838` 已提交态。

## 4. 基线与点名测试

### 4.1 `cargo test --lib`

独立运行结果：

```text
1990 passed; 1 failed; 135 ignored
```

唯一失败：

- `/tmp/rev446_cargo_test_lib.log:2568-2579`
- `theta_v0::classifier::signal::tests::extract_signals_bit_exact_digest_guard`
- panic 位于 `rust/src/theta_v0/classifier/signal.rs:3382`
- 与票面预期的 #115 线一致。

### 4.2 三项 LEE 点名测试

以下三项均以 `--exact --nocapture` 独立执行，均 1 passed：

1. `lee_m3_attribution_dimension_is_readable_and_not_residual_only`
   - `n_orders=38`
   - `n_residual_bucket=0`
   - `max_concurrent_real_levels=2`
2. `lee_m4_cap_on_sparsity_has_no_unexplained_violation`
   - `off_clock=135`
   - `explained=135`
   - `cap_narrowed=147`
   - `off_clock_delta=137`
   - `unexplained=0`
3. `lee_m4_level_cap_narrows_position_when_enabled`
   - baseline `max_abs_net_units=7953, n_orders=38`
   - capped `max_abs_net_units=754, n_orders=156`

全量日志中三项也均为 green，见
`/tmp/rev446_cargo_test_lib.log:2560-2562`。

## 5. 未覆盖与不能外推

1. 只独立重跑票面指定的 wf8 臂 R；其余八窗没有再次跑 cargo，而是对九份现存
   `.out`、report 和 dump 做独立解析、算术和哈希交叉验证。
2. 没有重跑非 m8/LEE 的 L3、incremental、sizing 或更早历史实证。因此只能确认它们消费同一
   coverage 路径，不能确认每份旧产物是否实际命中过重复 ID。
3. 没有外网、没有使用 `gh`，未核票体在线版本；上下文仅来自用户给定边界、仓内文档和
   `/tmp/446_diagnosis-20260727.md`。
4. 没有把共享工位既有修改或未跟踪文件归因给 `f838540eff`。

## 6. 最小回修门

达到以下条件后可复审：

1. coverage/fill 生产边界对重复 ID 在 release fail-loud，或明确收窄承诺且为其他生产入口补等价
   断言/错误传播；
2. T3 §15.5、diff JSON/MD 的 canonical/旧侧路径统一到当前真值；
3. `--regen` 与 committed golden 使用同一 provenance schema，并校验关键元数据；
4. 给受 `strategy_target_legs` 影响的已发布报告建立完整 superseded/未复核清单；
5. 保持本轮已通过的语义、九窗数值、逐笔分类与三项 LEE 基线不退化。
