# #1568 证据（PR #1656，head 0a1adbffa6，2026-10-21）

工作草稿，活期绑定 #1568（宪法 §5）。

## 分支与归属

- PR 分支 `codex/1568-init-atomic-main-20261021` = origin/main `02b5b571eb` + cherry-pick 原提交 `5d2e230e68`、`0d4589507f`、`f73bfa60e6`（作者与日期保留，仅把说明前缀 `fix/style(#1568)` 改为 `init(#1568)`）。
- 树与探针 `codex/1568-on-main-probe-20261021@4bb01b2cb3` 逐字一致（`git diff 4bb01b2cb3 0a1adbffa6` 为空）。

## 红绿（远端 nc-remote，WSL Ubuntu；本机未跑 cargo）

| 对象 | 命令 | 结果 | 日志 |
|---|---|---|---|
| 红：`codex/1568-red-oldimpl-20261021@17e991631c`（main 旧 `init_v3` + 逐字节相同的新文件，仅加模块声明） | `cargo test --features s_session --bin s_structure_session atomic_init` | 1 passed / **5 failed** / 1 ignored | `red-atomic_init.log` |
| 绿：`0a1adbffa6` | 同上 | 6 passed / 0 failed / 1 ignored | `green-atomic_init.log` |
| 绿：`0a1adbffa6` | `... init` | 13 passed / 0 failed / 1 ignored | `green-init.log` |

红版失败点：`atomic_init_tests.rs:221`「fsize:4096 失败后目标不得存在」、`:245`「fsize-kill:4096 被杀后目标不得存在」、`:276`「nofile:6 失败后目标不得存在：设置 WAL/FULL 失败」——三条即旧实现在目标路径留残留库（#1527 形态）；`:339` 暂存不被清扫、`:354` 外来 WAL 未被拒绝。忽略项是子进程入口 `s1568_child_init_under_rlimit`，由父测试带环境变量调用。

模拟口径：磁盘满 = `RLIMIT_FSIZE`（EFBIG），fd 耗尽 = `RLIMIT_NOFILE`，中途被杀 = SIGXFSZ（确定性）+ 定时 SIGKILL，与 #1527 收据一致。

## 验收对照

1. 三种失败后目标不存在或为完整库，中断只留可识别暂存名并在下次 init 锁内清扫：满足（红绿 + 独审逐条）。
2. 负控：残留现场原路径重试成功（`assert_retry_from_scene`）；既有目标拒绝且字节不动、decoy 与外来 WAL 不删：满足。
3. 本机不跑 cargo：满足。
范围：v1/v2 init 与 reset 同类残留在 #1570。

## 独审

`review-r1-fable-5-1.md`：claude-fable-5-1，审 `0a1adbffa6`，**PASS-WITH-NOTES**，无 BLOCK。M1（macOS 发布路径 PR CI 跳过）已手动触发 `ci-macos.yml` 补读数；L1–L5 为前两轮遗留的低级项，登记于票。提示词见 `prompt.md`。

## CI

ci.yml run 39564051169：14 项全过（macos-session 在 PR 下跳过）。ci-macos.yml 手动触发 run 39566559321（head 0a1adbffa6）：通过。
