# #1419 S9（B）正式运行第二轮：FAIL，与首轮失败在同一冷 verify 点

> 名分：工作草稿，绑定 #1419。依根裁 `ROOT-RULING-S9-RERUN.md`（本目录副本）只跑一次、未重试；帽、期限、harness、页面脚本不改，二进制同首轮。运行 2026-10-14 11:57:06–12:04:15（+0800），harness 退出 1，用时 428.7 s。
> 头 `daac6d86616316a2c2cc7d55daf1a298de7cf0c1`（worktree 干净），写者 `s_structure_session-48e2386463` sha256 `38c61eeed8ba79a7c5ec1d74575ca46c9cf32614ded9c5d9f04294f18642245b`。

## 结论

第二轮与首轮在同一处失败：首臂 `min12_up-a` 浏览器步骤中，页面取第 43 条切面的第 2 个 `/api/v2/snapshot`（`snapshot_token` 分页）请求在 **5.156 s** 后得到 **HTTP 503 `QueryBudgetExceeded`「查询阶段达到事前时间上限」**（首轮 5.174 s），页面未提交第 43 条切面，`tb02e_browser.cjs:63` 的 10 s 等待超时，两次 CC-022 点击都没有发生。harness 在首个失败臂处停止，其余三臂未启动，a/b 对拍未进行。根裁第 3 条的触发条件（第二轮仍在同一冷 verify 点失败）成立。

首臂失败点之前的部分与首轮一样全过：43 条输入全收；live、AsKnown、回访各 43 次，回访与原值逐值相同；第 27、43 条停机恢复前后 AsKnown 全量公共结果相同；`tb02e_verify` 对 5 个冻结切面（26/27/38/39/43）及 Python Q 合同、node 浏览器合同整切面门全过。

## 冷 verify 点读数（供根按裁定第 3 条处理）

| 项 | 读数 |
|---|---|
| 请求 | 第 43 条切面页面第 2 个快照请求（token 分页，`query-2`），12:04:04.145–12:04:09.300 |
| 请求总耗时 | 5.156 s（HAR wait 5.154 s），503 |
| verify 耗时 | **≥ 5.000 s，耗尽 `verify_deadline_ms=5000` 未完成**，真实完成时间不可知；其余约 0.156 s 是 capture 与往返 |
| 期限出处 | `s_session/tests/fixtures/tb01c/query-resources.json:8`；`max_query_workers=2`；抛出点 `s_session/s_query_integrity.py:250`（`_deadline`） |
| 当时负载 | 1 分钟负载 8.36–8.49（28 逻辑核）；Spotlight（mds_stores/mds/mdworker）0–0.5%；Chrome 合计 444–500% |
| Q 进程 | `s_readonly_server` pid 5480 单核 95.3% → 99.1% → 98.8%，RSS 675 → 763 → 817 MB（12:04:04/06/08 三次采样） |
| 同页第 1 个快照请求 | 0.863 s，200（首轮 0.867 s） |

读数含义：请求窗内机器空闲度高，Q 进程单线程满负荷做 verify，到 5 s 期限仍未完成。从这些读数看不出资源争用。

## 与首轮对照

| 项 | 首轮 | 第二轮 |
|---|---:|---:|
| 运行中 1 分钟负载 | 9.6–18.1 | 7.3–9.1 |
| mds_stores 峰值 | 288% | 15%（浏览器窗内 ≤1.5%） |
| 503 请求耗时 | 5.174 s | 5.156 s |
| 浏览器窗（READINESS） | 15.427 s | 15.312 s |
| Chrome 启动到 navigation-start | 5.134 s | 5.025 s |
| S 摄入最大回复 | 4.777 s | 4.699 s |
| 公共查询最大 | 8.123 s（live-42） | 8.042 s（live-42） |
| harness 用时 | 429.7 s | 428.7 s |

负载与 Spotlight 两项背景都大幅下降，失败点和各项耗时几乎不变。

## 四臂

| 臂 | 结论 | 摄入/查询 | 恢复对拍 | 冻结账 | 浏览器 |
|---|---|---|---|---|---|
| min12_up-a | **FAIL** | 43 条全收；live/AsKnown/回访各 43 次，回访逐值相同 | 第 27、43 条 `equal=true` | passed（5 切面 + Py/JS 整 cut 门） | FAIL：未提交第 43 条切面，0 次点击 |
| min12_up-b | NOT_RUN | — | — | — | — |
| min12_down-a | NOT_RUN | — | — | — | — |
| min12_down-b | NOT_RUN | — | — | — | — |

a/b 对拍（`compare_exact_traces`）：**未进行**。

## 浏览器臂（min12_up-a）

浏览器窗（READINESS elapsed）15.312 s，距 30 s 帽余 14.688 s；外层子进程窗上界 15.604 s，余量下界 14.396 s；30 s 帽未触及，失败来自页内 10 s 提交等待。末切面 CC-022 与 AsKnown 第 27 条 CC-022 两次点击都未到达，没有耗时。Chrome 155.0.8059.79（`/Applications/Google Chrome.app`，经 `TB02E_CHROMIUM_EXECUTABLE`）。

## 计时与恢复（min12_up-a）

S 摄入回复中位 1.00 s、最大 4.70 s（第 43 条），10 s 帽内。公共查询 live 最大 8.04 s、AsKnown 最大 5.20 s、回访最大 5.25 s，30 s 帽内。恢复：stop-s 0.19 s、recover 4.18 s、start-s 4.85 s；恢复前后 AsKnown 查询最大 5.05 s / 5.01 s。

## 背景负载与门检

门检（11:57）：1 分钟负载 7.42；mds_stores、mds、mdworker 均 0%；无 cargo/rustc/lake/S/Q/探针/浏览器验收进程；所有工位 target 的 cargo 锁 IDLE；端口 18860–18863 与 `/private/tmp/tb1419e-*.sock` 空闲；数据卷 24.57 GiB 可用。背景照实：用户自用 Chrome 154（GPU 进程 136–166%）、agent-browser 无头 Chrome（约 100%）、WindowServer（60–76%）、ChatGPT/Codex 渲染进程，Chrome 合计 388–524%。大件写 `.noindex` 目录后 Spotlight 只有 12:02:10 一次 mds_stores 15.4% / mdworker 16.8%、12:03:51 一次 mdworker 12.4%，其余均 <2%。采样平时每 10 s、浏览器窗内每 2 s，见 `background-samples.jsonl`。

## 回收

`SLOT-REAP.json`：`clean=true`，本票进程 0 个、孤儿 playwright Chrome 0 个、监听 0 个，清掉残留 socket 1 个（`/private/tmp/tb1419e-f380b555fdebcc16.sock`）。之后用 ps/lsof 复查无残留。数据卷最低 24.51 GiB 可用，未触 15 GiB 停手线。

## 产物

本目录：门检、驱动与抽取/汇总脚本、背景采样、harness 输出、首臂小件原件副本（`raw/`）、大件 SHA（`BIG-ARTIFACTS.json`）、`SHA256SUMS`。外盘 `/Volumes/AgentStorage/issue1419-s9-formal-r2-20261014.noindex/`：首臂 HTTP 收据 26,240 个文件约 6.9 GB、`semantic-core.jsonl` 约 1.86 GB 与库。驱动相对首轮只改了路径、门检加 mds_stores < 20% 条件、背景采样加 Spotlight 字段与浏览器窗内 2 s 采样，harness/runtime/页面脚本/帽/二进制不变。
