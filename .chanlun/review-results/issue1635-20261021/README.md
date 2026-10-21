# #1635 留证（2026-10-21）

- `probe-main-red-f1cdfb173b.log`：ForceL 锁只移位置、断言不动（分支 issue1635/probe-main，基于 main da537bfbcb），远端 Linux 执行为红。
- `fixed-green-bf40b72711.txt`：订正后远端 Linux 读数（fmt --check、42 个相关测试、default 与 s_session 两档 -D warnings cargo check）。
- `review-forcel-claude-opus-5-5.md`：非作者独审，PASS-WITH-NITS；nits 已在 c4ac3c1357 处理（锁注释措辞、stroke_velocity 前提、macOS 作业 paths 与 if: always()）。
- CI：run 39554793584（workflow_dispatch，ref issue1635/debt @ bf40b72711）。rust-test-default 中 `confirm_divergence_forcel_semantics ... ok`；macOS 作业新增的 -D warnings check 通过、0 告警。
