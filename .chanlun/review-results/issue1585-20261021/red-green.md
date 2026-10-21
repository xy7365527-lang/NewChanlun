# #1585 负控红绿取证（M1 处置）

- 日期：2026-10-21。作者：harvest owner 子代理（fix_1585）。
- 修复提交：`7e00b153cc`（S2/S3 负控改锁主回测接线）、`3d873f0a5e`（xianduan §15 登记 X-39）。

## 取证设计

主回测与负控现在调用同一组函数：`capture_oos.rs` 的 `Replay::seg_step`（逐 bar「#1585 闸 + 段确认开平仓」）、`final_seg`（理论分母/方向对拍取段）、`train_gaps`（λ 样本取段），`segment_gn.rs` 的 `s2_samples`。取证分支 `harvest/issue-1585-red`（头 `7ef31cc11a`，基于 `3d873f0a5e`）保留新负控，把四处接线还原成旧写法：

- `seg_step` 中 `match readable_confirmed_len(layer) { … }` 还原为 `let c = layer.segments_confirmed_len;`；
- `final_seg` 与 `train_gaps` 中 `fin.segments_valid_prefix()` 还原为 `fin.segments`；
- `s2_samples` 中 `layer.segments_valid_prefix()` 还原为 `&layer.segments`。

## 红：取证分支 CI

- Run：https://github.com/xy7365527-lang/NewChanlun/actions/runs/39597916668（workflow_dispatch，`--ref harvest/issue-1585-red`）
- Job `rust-test-default`（119214761445）：`test result: FAILED. 2888 passed; 2 failed; 153 ignored`，失败的正是两条负控：
  - `theta_v0::backtest::capture_oos::tests::tb1585_s3_refuses_segment_confirmation_under_marker`，`capture_oos.rs:614`「标记 bar 高水位不动」，left 5 / right 4（还原后标记 bar 照样确认新段；行号按取证分支，PR 头上同一断言在 `:628`）；
  - `theta_v0::backtest::segment_gn::tests::tb1585_s2_reads_only_valid_prefix_under_marker`，`segment_gn.rs:165`「样本只到失败段之前」，还原后样本 30 段，期望 15 段。
- 另两条 tb1585（`wverify_run` 与 `classifier::diag`）在红分支上照常通过，与本次只还原 S2/S3 接线一致。

capture_oos 负控在逐 bar 段先红，终局取数那半段没走到。补证：本机经 `nc-remote` 在只还原 `final_seg`/`train_gaps` 的临时分支（`3daae0d053`，未推远端，已删）上跑 `cargo test --locked --lib tb1585_s3`，失败于 `capture_oos.rs:663`「失败段起无理论分母」。这条是远端 WSL 读数，不是 CI run。

## 绿：PR #1664 头 CI

- Run：https://github.com/xy7365527-lang/NewChanlun/actions/runs/39597850919（pull_request，头 `3d873f0a5e`）
- Job `rust-test-default`（119214533711）：`test result: ok. 2890 passed; 0 failed; 153 ignored`，四条 tb1585 全部 ok。整 run 结论见 PR 检查页。

取证分支结论登记后已删除远端分支，run 号同时写入 #1585 票评论。
