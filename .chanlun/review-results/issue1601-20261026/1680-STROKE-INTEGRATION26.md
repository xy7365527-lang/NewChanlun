# PR1680 当前 main 集成

已按主控裁决解决唯一冲突并普通推送。精确候选 `06a095112fb6fbf94f90d55c19e027ad76c63830`，父一 `49b71870634dec950fd077b9fe3ac02a3fd20438`，父二 `3a6866ad15c9e3f4f51ba26c7dc7b63a749c9107`。PR base 保持 main，回读 MERGEABLE，未合入或关票。

## 合并意图与差异

唯一人工决断在 `rust/src/recursive_t/stream.rs`。保留 main 的 Stroke 已否证对照臂注释，同时保留 #1601 的 q_total 资金记录注释、三参 new_with_a0 签名和 TPositionEngine::new(q_total)。旧研发-生产同源说明未恢复。

已逐项检查 ffi、rec_stream、trading/config、nested_fugue、positional、positional_fusion 自动合并，新增均为 #1654 名分注释；资金接口保持原 PR。其余 main 新增六份既有评审/交付文档随合并进入，未改写原件。

实际 `git show --remerge-diff` 只含上述一个冲突块，原件在 `1680-STROKE-REMERGE26.diff`。自动合并检查片段在 `1680-STROKE-AUTOMERGE26.diff`。相对精确 main 的 diff-check 通过，actionlint 通过。

## 验证

nc-remote 在 Windows WSL 对精确提交执行 cargo fmt --all -- --check，通过。首次定向 recursive_t::stream::tests 选中 0 项，原因是 lib.rs 的 recursive_t 明确 cfg(not(test))；不计作测试通过。

随后对同一提交执行 `CARGO_BUILD_JOBS=1 cargo check --lib`，覆盖非 test 的 recursive_t 构造接口，成功；`CARGO_BUILD_JOBS=1 cargo test --lib funding_record:: -- --nocapture` 实跑 6 PASS。原件持久保存于 `1680-integration-validation26/20261026-082155-06a095112f.log`，fmt/零选中记录在同目录 `20261026-081938-06a095112f.log`。未改 cfg 以强行运行旧臂测试，未在 macOS 编译。

首次使用 detached HEAD 作为 nc-remote ref 未生成远端日志，改用具名恢复分支 `codex/1601-stroke-integration-20261026` 后执行成功。未据首次工具失败判断代码失败。

## 交接

GitHub 精确 head 已回读。CI `40249141867`、DevSkim `40249141833` 均 queued，尚未通过。主控下一步非作者尾审本次冲突决断并消费精确 CI 终态。

工位 `/Volumes/AgentStorage/pr1680-consume1723-20261026` 保持独占、干净、detached 在候选，恢复分支保留。推送前远端仍为父一，普通快进 push 完成，无 force、retarget、main 操作或旧工位处置。
