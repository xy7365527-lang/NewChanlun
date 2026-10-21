# #1653 旧红新绿证据

票面验收：「新断言在旧实现上为红、在新实现上为绿（贴红绿两次 CI run）」。

## 红：新断言 + 旧行为

- 分支 `harvest/issue-1653-red`，提交 `cb24a79ef4`（取证专用，不合入）= PR 头 `41fb2e53ce` 之上把两处实现退回旧行为：`classify_incremental` 拒读路径改回记 `TowerTermination::Natural`；`stale_begin_on_historical_rule` 缺键分支改回显示 `rule_revision=```。测试改动全部保留。
- CI run：https://github.com/xy7365527-lang/NewChanlun/actions/runs/39571954330（workflow_dispatch）。
- 失败恰好是本票新增的三条断言：
  - `rust-test-default`：`tb1581_classifier_refuses_marked_layer_with_warm_cache`（`tb1581_tests.rs:222`，left: Natural）与 `tb1581_injected_gate_failure_blocks_downstream`（`tb1581_tests.rs:186`，left: Natural）；`test result: FAILED. 2884 passed; 2 failed`。`rust-test-backtest-bin` 同样两条红（2890 passed; 2 failed）。
  - `rust-test-s-session (shard 3)`：`v2_serve_stale_begin_with_missing_rule_revision_says_missing`（`s_session_v2/mod.rs:3401`）；`178 passed; 1 failed`。
  - `rust-check` 也红，原因是取证提交里那一行改写未经 rustfmt（`pipeline.rs:570` Diff），属于取证提交本身的格式问题，与本票无关。
- #1603 意见 1 的直写 `null` 负控锁的是既有行为，本就不该在旧实现上红，未列入。

## 绿：PR 头

- PR #1662 的 CI（head 见 PR），结论登记在 PR 与票面评论。
