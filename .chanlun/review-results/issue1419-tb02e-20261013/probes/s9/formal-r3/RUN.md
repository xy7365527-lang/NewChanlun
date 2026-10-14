# #1419 S9（B）正式运行第三轮：PASS_EXCEPT_BROWSER_DEFERRED（四臂全过、a/b 对拍通过、浏览器臂移交 #1497）

> 名分：工作草稿，绑定 #1419。依根批 `07479fd501`（`--browser-deferred-to 1497`，根裁 ROOT-RULING-S9-RERUN 第 3 条）只跑一次。运行 2026-10-14 12:21:47–12:50:59（+0800），harness 退出 0，用时 1751.7 s。
> 头 `07479fd501a72fe202dbc0b41a8250b6a5047841`（worktree 干净；开跑前已快进推送到 `origin/codex/1419-center-seed-20261013`，`daac6d8661..07479fd501`），写者 `s_structure_session-48e2386463` sha256 `38c61eeed8ba79a7c5ec1d74575ca46c9cf32614ded9c5d9f04294f18642245b`（同前两轮）。

## 结论

`COMPARISON-RESULT.json` 总状态 **`PASS_EXCEPT_BROWSER_DEFERRED`**：四臂（min12_up / min12_down × a/b 新进程）全部退出 0，摄入、逐条 live/AsKnown 查询与全量回访、第 27/43 条停机恢复对拍、冻结账核对都通过；两用例 a/b 轨迹用 `compare_exact_traces` 逐字节、严格 JSONL 比对通过。浏览器臂未执行，各臂 `browser/RESULT.json` 为 runtime 写出的 DEFERRED 收据（`deferred_to: "#1497"`、`browser_executed: false`、`planned_cuts: ["27","43"]`），总状态因此不是 PASS。

## 四臂

| 臂 | 退出 | 采集 | 公共查询 | 冻结账（5 切面） | 恢复 27/43 | 浏览器 | 用时 |
|---|---|---|---|---|---|---|---:|
| min12_up-a | 0 | 236 条核心记录，清理无错 | 133，回访逐值相同 | passed | equal | DEFERRED→#1497，未启动 | 415.5 s |
| min12_up-b | 0 | 同上 | 133 | passed | equal | DEFERRED→#1497，未启动 | 415.3 s |
| min12_down-a | 0 | 同上 | 133 | passed | equal | DEFERRED→#1497，未启动 | 414.2 s |
| min12_down-b | 0 | 同上 | 133 | passed | equal | DEFERRED→#1497，未启动 | 420.5 s |

四臂冻结账读数相同：第 26 条无完成段（1 个归属单元）；第 27、38 条完成 `segment:1:1`（2 个）；第 39、43 条完成 `segment:1:1`、`segment:1:13`（3 个）；游标均 `SEEKING_SEED`。Python Q 合同与 node 浏览器合同整切面门同过。

## a/b 新进程对拍

| 用例 | 结果 | 记录 | 比较字节 | 两侧 sha256 |
|---|---|---:|---:|---|
| min12_up | PASS | 236/236 | 1,864,380,377 | `f851c79a…` = `f851c79a…` |
| min12_down | PASS | 236/236 | 1,870,651,041 | `2f5f22f0…` = `2f5f22f0…` |

方法 `exact_bytes_and_strict_jsonl`，无排除字段、无身份改名；比较用时 87.3 s。另外，min12_up 的 `semantic-core.jsonl` 与 `database-backup.sqlite` 在首轮、第二轮（各自的 min12_up-a）与本轮 a/b 四份上 sha256 全同（`f851c79a…` / `368e6d63…`），即跨三轮也逐字节可重放。

## 恢复对拍（两用例 × a/b）

| 臂 | 切面 | 前后相同 | stop-s | recover | start-s | 恢复前/后 AsKnown 查询最大 |
|---|---|---|---:|---:|---:|---:|
| min12_up-a | 27、43 | 是 | 0.19 s | 4.18 s | 4.84 s | 5.16 s / 5.04 s |
| min12_up-b | 27、43 | 是 | 0.20 s | 4.25 s | 4.83 s | 5.26 s / 5.11 s |
| min12_down-a | 27、43 | 是 | 0.19 s | 4.20 s | 4.81 s | 5.13 s / 5.08 s |
| min12_down-b | 27、43 | 是 | 0.20 s | 4.15 s | 4.94 s | 5.04 s / 5.20 s |

## 耗时读数

S 摄入回复最大 4.65 / 4.70 / 4.70 / 4.65 s（均为第 43 条，帽 10 s）。公共查询最大均为 live-42：8.13 / 8.11 / 8.12 / 8.21 s（帽 30 s）；AsKnown 最大 5.22–5.55 s，回访最大 5.24–5.47 s。各项与前两轮首臂同量级（首轮摄入最大 4.78 s、查询最大 8.12 s）。

浏览器臂：本轮按开关未启动，30 s 帽与两次 CC-022 点击不适用；前两轮的失败原件路径与 SHA 写在每臂 DEFERRED 收据里。

## 背景负载

门检（12:21）：1 分钟负载 9.13；无 cargo/rustc/lake/S/Q/探针/浏览器验收/hdiutil 进程；所有工位 target 的 cargo 锁 IDLE；mds_stores 3%；端口与 socket 空闲；远端分支头与本地同为 `07479fd501`。

运行中背景比门检时重，而且都不属本图，照实记：
- 12:30 起 Spotlight 回升，mds_stores 最高 508%、mdworker 最高 493%；同时段别的工位在 `/private/tmp/goal1323-native-20261012/wt-1406`、`wt-1424` 新建大文件（数据卷由 24.3 降到 21.6 GiB）。本轮大件在 `.noindex` 目录。
- 12:42 起有别的工位的 rustc 构建，12:44 起有 `/Volumes/AgentStorage/cargo-targets.noindex/1424/debug/deps/newchan_rust-…` 测试二进制（最高约 6 核），主要落在 min12_down-b 臂（42 次采样中 23 次）。
- 用户自用 Chrome 154 与 agent-browser 无头 Chrome 照常，Chrome 合计 260–584%。

分臂 1 分钟负载：up-a 8.8–13.2，up-b 9.3–18.8，down-a 10.4–22.7，down-b 10.1–23.3（28 逻辑核）。四臂用时只差 6 s，负载变化没有反映到本轮读数上。逐 10 s 采样见 `background-samples.jsonl`。

## 回收

`SLOT-REAP.json`：`clean=true`，本票进程 0、孤儿 playwright Chrome 0、监听 0，清掉四臂各 1 个残留 socket（共 4 个）。之后 ps/lsof 复查无残留。数据卷最低 20.23 GiB，未触 15 GiB 停手线；外盘最低 129.5 GiB。

## 产物

本目录：门检、驱动/抽取/汇总脚本、背景采样、harness 输出、四臂小件原件副本与 `raw/COMPARISON-RESULT.json`、大件 SHA（`BIG-ARTIFACTS.json`）、`SHA256SUMS`。外盘 `/Volumes/AgentStorage/issue1419-s9-formal-r3-20261014.noindex/`：每臂 HTTP 收据约 6.9 GB、`semantic-core.jsonl` 约 1.87 GB 与库。驱动相对第二轮只改了路径、头门（`07479fd501`）、门检加 hdiutil、记录远端跟踪头，以及 harness 命令加 `--browser-deferred-to 1497`；帽、期限、页面脚本与二进制不变。
