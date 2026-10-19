# #1527 / I-1 二进制身份绑定

- 目标 SHA：`947e05be3427e0b32a96d7429aa2b7260e9492af`（分支 `codex/1527-owner-resume-20261019`，Rust 正式 I-1 候选）
- 构建主机：windows-5090 WSL2 Ubuntu（Linux 6.6.87.2-microsoft-standard-WSL2 x86_64），经 `~/.local/bin/nc-remote` 从 git bundle 检出干净 worktree；本机未跑 cargo。
- 构建命令：`cargo build --release --locked --features s_session --bin s_structure_session`（`CARGO_TARGET_DIR=~/nc-remote/target`），45.1 s，峰值 RSS 1,558,380 KiB，退出 0。
- 编译器：`rustc 1.99.1 (58cf8f9c3 2026-10-13)`，commit-hash 58cf8f9c35b355199727627d07ae34a4e2846d67，LLVM 23.1.1；`cargo 1.99.1 (5f94df478 2026-08-27)`。
- 二进制 SHA-256：`1125211c888f8539b67e95c85931919b29415242053138afe6998062ce476977`（12,816,272 字节，ELF 64-bit PIE x86-64，BuildID 14f8329e27b6c3706b3cc7f1dbc41a25c41826ea）。
- 封存位置：windows-5090 `G:\AgentStorage\issue1527-binary\s_structure_session-947e05be`（复制后再算 SHA，一致）。
- 机械锁旁证：该 release 产物 `nm -C | grep -c test_prepare` = 0（测试造锚入口不在 release 构建中）；`admission_anchor` 符号 61 处（R4 消费端在）。
- 完整构建日志：`issue1405-i1-binary-release-build.txt`。

## producer / 写入口（文件:行号，均为 947e05be）

- 二进制入口 `rust/src/bin/s_structure_session.rs:5479`（`main`，`CommandAccounting::begin` 包住全部命令的失败收据）；`init` 分派 `:5435`，S3 写命令经 `s_session_v3/writer_cli.rs:195` `run`。
- S3 写入口 `serve` / `advance` / `recover`：`writer_cli.rs:197` / `:207` / `:217`，三者都先 `admission_anchor::from_args_for_db`（`admission_anchor.rs:916`，读 `--admission-job-limits` 与 `S_CONTROL_ADMISSION_*`）再进业务。
- 调用链见 `issue1405-i1-r4-cli-chain.md`。
