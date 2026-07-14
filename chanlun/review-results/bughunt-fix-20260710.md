# bughunt phase3 修复报告（20260710）

- 基线快照：`bf1854d71960`（gap3-rework-codex9-fix 一线；main tip=`3ebc4e493c` 为其祖先，未触碰）
- 修复分支：`bughunt-fix-20260710`，worktree `/tmp/bughunt-fix-work`，共 15 个 commit，**不合并**，留待人工审查
- 权威输入：`chanlun/review-results/bughunt-verify-20260710.md`（B 节排序 / A 节禁修），三份 phase1 报告 `bughunt-{parser,cert,backtest}-20260710.md` 作背景
- 全部修改仅发生在该 worktree 与该分支；主工作树、main/tip、冻结判据文件、STRICT-NEST-CHECK.md、prereg、chanlun/goals/ 均未触碰

## 修复总表（finding × commit × 测试结果）

| B 序 | finding | commit | 修复要点 | 测试结果 |
|---|---|---|---|---|
| 1 | backtest F-05 | `735d71c9fe` | 现金不足拒单返回可区分 `FillOutcome`（区分 Filled/PartialFill/Rejected），拒单不再计入 `n_orders_executed` / `voice_qty` / `held`；apply_fill/apply_order 收口 | 新增 `f05_cash_insufficient_rejection_is_distinguishable`、`f05_rejected_order_not_counted_*` 等；backtest 模块测试全绿 |
| 2 | capture_oos F-08 | `0baaeb301a` | 训练段特征构造截断至 `train_end`，消除训练特征读越界（前视） | capture_oos 模块测试全绿 |
| 2 | capture_oos F-09 | `0baaeb301a` | 窗界（PREREG_WINDOWS 边界）强制平仓并重置 open/pending/deferred 状态，窗间不再泄漏持仓 | 同上 |
| 2 | capture_oos F-10 | `0baaeb301a` | untradable bar 阻止开仓成交；止损在 untradable/跳空时按劣侧价成交；出场被顺延时挂起至下一可交易 bar | 同上；`capture_oos F-08/F-09/F-10` 训练特征截断 + 窗界平仓 + untradable 系列断言通过 |
| 3 | backtest F-01 | `3e081d07ab` | 旧入口 `run_theta_v0` 标记 `#[deprecated]`（结构确认前视），禁止产出 L2/L3 声明；`l3_fullwindow.rs:38/120` 迁移至 `run_theta_v0_pi` 无前视入口 | l3_fullwindow / runner 测试全绿；deprecated 告警按预期出现在遗留调用点并已清理 |
| 4 | backtest F-06 | `238397321b` | 部分减仓逐 fill 产出 `TradeRecord`（qty=本次减掉手数，entry_bar 延续开仓 bar），与 `trade_pnls` 逐 fill 口径对齐；修正 TradeRecord/轨迹文档口径声明 | 新增 `partial_reduce_emits_per_fill_trade_record` 回归测试；metrics/runner 测试全绿 |
| 5 | parser BUG-03 | `1f7dd342e5` | 次级别走势"结构相等"改为指针身份索引比较；仅改比较本身，未触碰 witness/bit-exact 基线（编译与既有测试直接通过，未触发 A 列跳过条件） | classifier 模块测试全绿，基线 witness 未改动 |
| 6 | cert F-01 | `1b39834146` | `NestCertificate`/`NestRung` 字段私有化 + `from_parts`/`new` 构造收口（含 ⊆ 链 `debug_assert`），全量读点迁移 getter；守住裁决②"不入链"边界 | nest / econ_positive / strict_nest_check 相关测试全绿（E0616 私有字段读点已全量迁移） |
| 6 | cert F-02（可达性面） | `3ea504c185` | 盘整背驰诊断可达性修复：非趋势门段独立调用 `judge_pan_div` 产**纯诊断**事件（judge_pan_div_reachable_in_consolidation_without_trend_gate 坐实），不影响交易门与入链 | 新增可达性测试通过；signal/recursive_tower 测试全绿 |
| 7 | parser BUG-04 | `6ea2ce8ebc` | 缓存守卫 off-by-one：`cache.macd_state_len` 守卫改按真实覆盖契约（`<= n` 分界）校验，增量缓存分支复活 | 新增 cand_delta_tower cache 守卫测试；DIAG_CANDCACHE 路径测试全绿 |
| 8 | cert F-04/F-05/F-08 | `992104469a` | strict_nest_check 脚本输入加严：i8 截断改 checked 解析报错、手写 JSON 解析换 serde 严格反序列化、fail-open env（`STRICT_NEST_TEST_FLAG` 等）改显式白名单校验 | 新增 f05_array_body_requires_octal_and_rejects_dup_nested、f08_json_raw_tolerates_ws_rejects_dup 等；bin 测试全绿 |
| 8 | cert F-06/F-07 | `0331f0071c` | 诊断层修正：runner 侧诊断计数/摘要口径与 strict_nest_check 对齐（terminal_missing / cert_missing_terminal 统计） | strict_nest_check + runner 诊断测试全绿 |
| 9 | parser BUG-09/BUG-11/BUG-12 | `ae85ffdcc9` | 边界加固：recursive_tower / signal / nest 侧空段、单元素、区间端点越界路径 | classifier 全模块测试全绿 |
| 9 | cert F-11/F-13 | `8de8441bab` | goal_events 校验边界加固（REQUIRED_FIELDS 事件类型校验、sub_goals 类型/依赖闭包检查） | `scripts/tests` pytest 全绿 |
| 10 | parser BUG-08 | `fe76289e3e` | SecondKind 路径改用方向性包含处理：`second_kind_confirmed` 复用 `feature_seq::second_seq_has_fractal`（bit-exact 对齐 Python `_apply_inclusion` 方向性合并语义，scan_window=0 全扫），替代中性外包络实现；`segment.rs` 静态层用法不动（对齐 Origin.SegmentFeatureSeq，非本 finding 范围）；`has_any_feature_fractal` 降为 `#[cfg(test)]` | second_kind 10/10 通过，无告警 |
| 10 | parser BUG-10 | `8911c647f8` | `segment_price_amplitude`/`segment_price_speed`/`segment_total_variation` 改饱和算术（saturating_sub/abs/add），防极端 Tick 差 debug 溢出 panic / release 回绕；实务域内值逐位不变，不改类型（避免触碰 ForceFeatures 消费面与 bit-exact 基线） | divergence 测试全绿 |
| 10 | cert F-15/F-16 | `aa9bac3ee2` | F-15：`TERMINAL_STATUSES` 补「已结算」（verify 实测漏网 ×91 行）；F-16：`--workstations` 判空改 `is not None`，区分「未传→全量」与「传空→空工位集」（nargs="*" 裸开关产出 `[]` falsy 原逻辑静默落入全量扫描） | `scripts/tests` pytest 129 passed |

commit 顺序（`git log --oneline --reverse bf1854d71960..HEAD`）：
`735d71c9fe` → `0baaeb301a` → `3e081d07ab` → `238397321b` → `1f7dd342e5` → `1b39834146` → `3ea504c185` → `6ea2ce8ebc` → `992104469a` → `0331f0071c` → `ae85ffdcc9` → `8de8441bab` → `fe76289e3e` → `8911c647f8` → `aa9bac3ee2`

改动文件面（17 个）：
- rust/src/bin/strict_nest_check.rs
- rust/src/theta_v0/backtest/{capture_oos,econ_positive,l3_fullwindow,metrics,runner}.rs
- rust/src/theta_v0/classifier/{cand_predicate,divergence,force_conformance,mod,nest,recursive_tower,signal}.rs
- rust/src/theta_v0/parser/{feature_seq,second_kind}.rs
- scripts/{ceremony_scan,goal_events}.py

## 跳过项与原因

| 项 | 原因 |
|---|---|
| B 列第 2 位「F-02 残留」 | 涉及重建 bit-exact 基线，按指令本轮跳过，单列待裁定（本轮仅修 F-02 的可达性面，见 `3ea504c185`，不动基线） |
| backtest F-03（Sharpe 年化） | 按指令跳过：可能触 prereg（`chanlun/review-results/prereg-typed-exit-trigger-quality-20260705.md` 相关口径） |
| parser BUG-01/BUG-02 | 按指令跳过：须先裁定（基线面在 A 列） |
| A 节全部 10 组 | 禁修：parser BUG-01/02/03 基线面、BUG-05/06/07、cert F-03、backtest F-03/F-04/F-07/F-11、cert F-09..14 死协议部分。均未触碰 |
| BUG-03 witness 面 | B 列第 8 位的 BUG-03 仅改代码中结构相等比较本身；实测无需同步改 witness/bit-exact 基线即可编译并通过，故**未**触发"跳过并记录"条款，已正常落地（`1f7dd342e5`） |

## 回归结果

- 逐项修复时均在 worktree 内跑对应模块 `cargo test`（见总表"测试结果"列），全部通过。
- 全量 `cargo test --release`（worktree 内）：**除 3 个既有 doctest 失败外全绿**。
  - 失败项：`src/theta_v0/ledger/separate.rs` doctest（line 13/20/25）——模块文档中缩进伪公式块（`P^sep = ∏...`、`Net(p_t) = Σ_...`）被 rustdoc 当作 Rust 代码块编译（E0425/E0422/E0070）。
  - 判定为**基线既有问题**：该文件本轮 diff 为 0 行（`git diff bf1854d71960 HEAD -- rust/src/theta_v0/ledger/separate.rs` 为空），基线 `bf1854d71960` 处同一文档块原样存在；与本轮全部修复无关。未按"回滚"条款处理（非本轮修复导致）。
- Python：`pytest scripts/tests/ -q` → **129 passed**（1 个 pytest 配置 warning，与修复无关）。
- 主工作树 `git status`：本工位未产生任何改动（现存 M/D 条目为蜂群环境其他工位/会话既有状态）；main tip 仍为 `3ebc4e493c`。

## 遗留与 phase4 建议

1. **F-02 残留（bit-exact 基线重建）**：待裁定。可达性面已修（`3ea504c185`），但诊断事件若要纳入基线快照比对，需要一次受控的基线重建仪式，建议 phase4 单独立项并走裁决流程。
2. **backtest F-03（Sharpe 年化口径）**：与 prereg 判据耦合，建议先出裁定书再动代码；当前代码保持原口径。
3. **parser BUG-01/BUG-02**：证据在 phase1 报告，等待裁定后可直接按本轮同款方式修（改动面小）。
4. **ledger/separate.rs doctest 既有失败**：3 个伪公式文档块建议改为 ` ```text ` 围栏或加 `ignore`，属零风险文档修复，但因不在本轮 B 列授权范围未动；建议 phase4 顺手收掉，否则全量 `cargo test` 永远带 3 个红项，掩护真回归。
5. **旧入口 `run_theta_v0` 下游清理**：本轮已 deprecated 并迁移 `l3_fullwindow.rs:38/120`；建议 phase4 全仓 grep 剩余调用点（含实验脚本），确认无人再经旧入口产出 L2/L3 类声明后择期删除。
6. **BUG-08 的 segment.rs 静态层**：`analyze_termination` 仍用 Origin 静态层外包络（对齐 Origin.SegmentFeatureSeq，非 bit-exact Python 路径）。两套语义并存是刻意的，但建议 phase4 在模块文档里显式标注双轨边界，防止后续误"统一"。
7. **capture_oos 顺延出场语义**：F-10 修复引入"出场被顺延至下一可交易 bar"的挂起路径，当前由模块测试覆盖；建议 phase4 对连续多 bar untradable 的长挂起场景补极端用例。
