# #1585 第二轮非作者独审（claude-fable-5-1，只读）

- 审查对象：PR #1664 新提交 `94c0b35bc5..a952c39e16`（7e00b153cc、3d873f0a5e、a952c39e16）及上一轮 M1/M2 处置。
- 路由：`~/.local/bin/claude-proxy --model claude-fable-5-1 -p`，工具限 Read/Grep/Glob 与只读 git/sed/grep/rg；前台等待结束。作者为 fix_1585 子代理（非本审查者）。日期：2026-10-21。
- 审查后处置：L-a 已在 red-green.md 补注 PR 头行号（审查者文中所写 `:620` 经 grep 核为 `:628`，以 `:628` 为准）；L-b 维持上一轮判定不改。

---

结论：PASS

三个新提交把上一轮 M1、M2 处置到位，功能语义与 94c0b35bc5 版本逐行等价，负控真正锁住了主回测接线；本地存在取证分支提交 7ef31cc11a 与临时提交 3daae0d053，其 diff 与 red-green.md 的叙述一致。无 HIGH、无 MEDIUM。

## 逐项核对

**1. M1 处置（接线锁）——到位**

- 逐 bar「闸 + 段确认开平仓」已抽成 `Replay::seg_step`（`rust/src/theta_v0/backtest/capture_oos.rs:235-325`），主回测循环 `s3_capture_oos_real_btc` 在 `capture_oos.rs:481` 调用它，且它是循环体最后一条语句，因此旧版 Err 分支的 `continue` 换成 `return` 后控制流不变。
- 终局 λ 样本经 `train_gaps`（`:367`），理论分母与方向对拍两处都经 `final_seg`（`:497`、`:501`）。`segment_gn.rs` 主统计经 `s2_samples`（`segment_gn.rs:97`）。
- 负控 `tb1585_s3_refuses_segment_confirmation_under_marker`（`capture_oos.rs:579-668`）直接拿带标记的层调 `seg_step` / `final_seg` / `train_gaps`；`tb1585_s2_reads_only_valid_prefix_under_marker`（`segment_gn.rs:140-170`）直接拿带标记的层调 `s2_samples`。我静态推演了四处还原：`seg_step` 改回 `layer.segments_confirmed_len` 则 `neg.hw == hw0`（`:620`）红；`final_seg` 改回 `fin.segments` 则 `:663` 红；`train_gaps` 改回则 `:664-668` 长度不等于 k 红；`s2_samples` 改回则 `segment_gn.rs:165` 样本长度不等于 k 红。与取证分支 7ef31cc11a（还原四处）和 3daae0d053（只还原终局取数）的 diff、以及 red-green.md 记录的失败点（取证分支的 `capture_oos.rs:614`「标记 bar 高水位不动」left 5/right 4、`segment_gn.rs:165`、临时分支 `capture_oos.rs:663`）一一对得上——我 `git show` 核过这三个行号在对应提交里确实是那几条断言。

**2. 功能语义——等价**

对比 94c0b35bc5 的主循环：窗界强平 → 顺延出场 → 止损 → 闸/段确认 的顺序不变；`retreats`、`deferred_exits`、`untradable_entry_skips`、`invalid_runs` 的计数位置与条件逐行相同；`last_tradable_close` 在 Err 分支内与正常路径末尾两处更新，与旧版一致；开仓时 `win_of_day(day(t))` 与 `lambda_of_win[wi]` 的取值改由 `entry_ctx(t)` 一次返回，仍只在开仓时求值。λ gap：旧版 `(0..fin_segs.len()).filter(day in train).map(seg_gap(si, cap))` 与新版 `segments_valid_prefix().iter().filter(in_train(start_index)).map(...)` 公式逐字相同，F-08 截断 `end_index.min(cap)` 保留。S2 的 `s2_samples` 循环体只把 `ds.bars` 换成参数 `bars`；`assert!(!segs.is_empty())` 挪成 `assert!(!ratios.is_empty())`，一段一样本，等价。

**3. M2 处置（X-38 → X-39）——到位**

7 处注释（`parser/mod.rs:140`、`capture_oos.rs:39/169/178/183/573`、`segment_gn.rs:20/38/50/137`、`wverify_run.rs:3062/3232/3662`、`cand_delta.rs:152`、`cp_recall_audit.rs:349`、`tb1585_tests.rs:1`）全改为 X-39；grep 整个 `rust/src/theta_v0` 无残留 X-38（仅上一轮审查记录里有）。`.chanlun/definitions/xianduan.md:525` 新增 X-39 行，五列（编号/文件:行号/要改什么/为什么/归票）与 X-37 体例一致；diff 中无删除行，X-38（#1600）那行未动。行号逐个 sed 核对：`mod.rs:140-151`、`cand_delta.rs:152-157`、`cp_recall_audit.rs:349-353`、`wverify_run.rs:3062-3066`、`:3232-3241`、`:3665`、`segment_gn.rs:38-80`、`:140`、`capture_oos.rs:169-326`、`:579`、`p89_dual_core_audit.rs:147` 全部准确。

**4. L2 / L4——到位**

两把见证锁 `center_strict_zd_eq_zg_full_btc_cascade_witness`（`wverify_run.rs:3360`）与 `center_strict_zd_eq_zg_change_block_witness`（`:3446`）现在先调 `c327_assert_readable` 再断言 `C327_*_DATA_ID`；`c327_run_witness` 里的位置（`:3261`）仍在读数之前。三个提交均带「编号：」声明行，首行无 fix/close/resolve；两个代码/文档提交符合 `<type>(<scope>): #N——` 格式，`review-results:` 提交沿用本分支既有的 4 条同类前缀惯例。

**5. 负控无奖励作弊**

S3 负控的正控不是摆设：先在同一 bar 用无标记层 `seg_step` 断言 `pos.hw > hw0`（`capture_oos.rs:607`），证明该 bar 确有新确认段，负控的「高水位不动」才非空转；`replay_to(t)` 从零重放到 t 前并断言 `(hw, trades.len()) == (hw0, n0)`（`:605`）保证两次重放状态一致。负控五条断言（高水位、交易数、持仓段号、污染标记、`invalid_runs == [(t,t,0)]`）在还原接线下各自独立会红。S2 负控把标记前后的样本切片做逐元素比较而非只比长度。无恒真条件。

## LOW

- **L-a** `.chanlun/review-results/issue1585-20261021/red-green.md:17` 引的 `capture_oos.rs:614` 是取证分支 7ef31cc11a 的行号；在 PR 头 a952c39e16 上第 614 行是 `rp.seg_step(u, b, &l, no_win)`，对应断言在 `:620`。文中已说明是取证分支读数，只是读者在 PR 头查会偏 6 行。不影响结论。
- **L-b** 上一轮 L3（标记消失后首 bar 积压确认不可辨）未处理，`seg_step` 的 `for j in self.hw..c` 仍一次性处理；上一轮已判定非静默使用失败结构，本轮不升级。上一轮 L5（`.map(...).last()` 驱动副作用）随负控改成 `for` 循环已顺带消失。

## 看过什么 / 未核实什么

**看过**：`git diff 94c0b35bc5..a952c39e16` 全文与三条提交信息；`capture_oos.rs` 当前版本 `:40-668` 全段与 94c0b35bc5 版本的 λ 定参、主循环逐行对照；`segment_gn.rs:36-170`；`wverify_run.rs` 的 `c327_assert_readable`、`c327_run_witness` 调用位置、两把见证锁开头；`xianduan.md:487-525` 表头与 X-35~X-39 行及其引用的全部行号；`red-green.md` 全文；本地取证提交 7ef31cc11a 与 3daae0d053 的 diff 和被引行号；`.sandcastle/CODING_STANDARDS.md` 提交规范；上一轮审查记录。

**未核实**：未跑 cargo（只读约束），新负控能否编译、`tb1581_bars(0x1581_0000, 4000)` 下 probe 必能命中「有持仓且有新确认」的 bar，依赖 red-green.md 记录的 CI run 39597850919 绿（2890 passed）与 39597916668 红（恰两条负控失败），未调 gh 复核这两个 run；未读 #1585 票面与票评论；未检查 `tb1581_bars` 是否含 untradable bar（不影响断言成立，只影响走哪条分支）。
