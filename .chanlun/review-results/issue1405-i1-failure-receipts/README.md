# #1527 / I-1 失败留存收据

运行：2026-10-19 20:45（Asia/Shanghai），windows-5090 WSL2，整合分支 `0ade553a`（Python 控制器 + R4）+ release 二进制 SHA-256 `1125211c…6977`（`947e05be`）。脚本 `run_failure_receipts.sh`，总日志 `run.txt`。每个收据文件组：`.cmd`（完整命令）、`.env`（S_CONTROL_* 与 PATH）、`.rc`（退出码与耗时）、`.stdout`、`.stderr`、`.time`（`/usr/bin/time -v`，含峰值 RSS）。

| 收据 | 类别 | 命令 | 退出码 | 失败原因 |
|---|---|---|---|---|
| P0-init | 前置（成功） | producer `init` s-session/3 建库 | 0 | 生成失败用例所需的既存库（315,392 字节） |
| F1-cli-start-s-no-anchor | 负控 | 正式 CLI `start-s`，库存在、无锚 | 1 | R4 `AnchorMissing`，阶段 E1，实读 0 字节；未创建 owner、未建 `s-admission`、未起 Rust 子进程 |
| F1b-rust-recover-forged-limits | 负控 | Rust `recover` 直接带伪造 `--admission-job-limits` | 1 | Rust 侧 R4 `AnchorMissing`，阶段 E1，`限额可信=false`，Job 未建立 |
| F2-init-fsize-64k | 资源不足（磁盘满模拟） | `ulimit -f 64` 下 producer `init` | 1 | `init commit 失败：disk I/O error`，退出非 0；残留 4096 字节文件（不可用库，未发布） |
| F2b-cli-start-s-nofile-8 | 资源不足（FD 耗尽） | `ulimit -n 8` 下正式 CLI `start-s` | 1 | R4 `IO`，阶段 E1，实读 0 字节，脱敏输出 |

磁盘满用 RLIMIT_FSIZE 模拟（WSL 无 sudo，不能挂小容量 tmpfs）；这与真实设备满的错误码（ENOSPC vs EFBIG）不同，SQLite 均上报为 disk I/O error。
