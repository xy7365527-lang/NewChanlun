# #1568 PR #1656 验证与独审证据

- `red-atomic_init.log`：红对照分支 `codex/1568-red-oldimpl-20261021@17e991631c`（main 旧 `init_v3` + 新测试），远端 nc-remote 跑 `cargo test --features s_session --bin s_structure_session atomic_init`：1 过 5 败，失败点 `atomic_init_tests.rs:221/245/276` 即旧实现在目标路径留残留。
- `green-atomic_init.log`、`green-init.log`：本分支 `0a1adbffa6` 同命令（`atomic_init` 6 过 1 忽略；`init` 13 过 1 忽略，忽略项为子进程入口）。
- `prompt.md` → `review-claude-fable-5-1-0a1adbffa6.md`：claude-fable-5-1 非作者独审 `git diff 02b5b571eb 0a1adbffa6`，PASS-WITH-NOTES，无 BLOCK；M1（macOS 分支）已由手动 ci-macos.yml run 39566559321 覆盖，L1–L5 低级项去向见 #1568 关票声明。

本目录在合入前由合入执行者从工作目录补入仓（原 PR 正文已声明此路径），评审结论原文未改。工作草稿，活期绑定 #1568（宪法 §5）。
