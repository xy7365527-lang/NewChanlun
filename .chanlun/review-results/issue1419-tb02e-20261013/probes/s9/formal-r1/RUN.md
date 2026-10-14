# #1419 S9（B）正式运行：FAIL（首臂浏览器，Q 冷查询撞 5 s verify 期限）

> 名分：工作草稿，绑定 #1419。只跑一次、未重试。运行 2026-10-14 11:26:34–11:33:44（+0800），harness 退出 1，用时 429.7 s。
> 头 `daac6d86616316a2c2cc7d55daf1a298de7cf0c1`（worktree 干净），写者 `s_structure_session-48e2386463` sha256 `38c61eeed8ba79a7c5ec1d74575ca46c9cf32614ded9c5d9f04294f18642245b`（`48e2386463..daac6d8661` 无 rust/、formal/ 改动，二进制与头一致，未重建）。

## 结论

首臂 `min12_up-a` 的 S 摄入、43×live/AsKnown 查询与全量回访、第 27/43 条停机恢复对拍、冻结账核对都通过，**浏览器臂失败**；harness 按设计在首个失败臂处停止，其余三臂（`min12_up-b`、`min12_down-a`、`min12_down-b`）只做了 prepare、未启动，a/b 对拍未进行。

首个失败点：`tb02e_browser.cjs:63` 等最终 cut 43 提交的 `waitForFunction`（10 s）超时。原因在 Q 侧：页面取 cut 43 的第 2 个 `/api/v2/snapshot`（`snapshot_token` 分页）请求在 5.174 s 后得到 **HTTP 503 `QueryBudgetExceeded`「查询阶段达到事前时间上限」**，即 Q verify 期限 `verify_deadline_ms=5000`（`s_session/tests/fixtures/tb01c/query-resources.json:8`，抛出点 `s_session/s_query_integrity.py:250`）。页面因此没有提交 cut 43，CC-022 两次点击都没有发生。整次运行 Q 日志只有这一条 503；harness 自己的 HTTP 采集器同期 133 次查询全为 200。F1a 探针已记过「Q 冷 worker 首次 verify 4.5 s，贴着 5 s 期限」，本次在 1 分钟负载 13.8–17.7 下越过。

## 四臂

| 臂 | 结论 | 摄入/查询 | 恢复对拍 | 冻结账 | 浏览器 |
|---|---|---|---|---|---|
| min12_up-a | **FAIL** | 43 条全收；live/AsKnown/回访各 43 次，回访与原值逐值相同 | 第 27、43 条前后 AsKnown 全量公共候选相同（`equal=true`） | passed：5 个冻结切面（26/27/38/39/43）逐字段过，Python Q 合同与 node 浏览器合同整 cut 门过 | FAIL：未提交 cut 43，0 次点击 |
| min12_up-b | NOT_RUN | — | — | — | — |
| min12_down-a | NOT_RUN | — | — | — | — |
| min12_down-b | NOT_RUN | — | — | — | — |

a/b 对拍（`compare_exact_traces`）：**未进行**（无 `COMPARISON-RESULT.json`）。

## 浏览器臂（min12_up-a）

| 项 | 读数 |
|---|---|
| 浏览器窗（READINESS elapsed） | 15.427 s，距 30 s 帽余 14.573 s |
| 外层子进程窗上界（browser.stdout 创建 → cleanup 首个 stop-q 开始） | 15.738 s，余量下界 14.262 s |
| 30 s 帽 | 未触及；失败来自页内 10 s 提交等待 |
| Chrome 启动到 navigation-start | 5.134 s（TB-02-D 冻结运行约 0.59 s） |
| snapshot#1（新查询） | 0.867 s，200 |
| snapshot#2（token 分页） | 5.174 s，**503 QueryBudgetExceeded** |
| 第一次点击（末切面 CC-022） | 未到达 |
| 第二次点击（AsKnown 第 27 条 CC-022） | 未到达 |

Chrome：`/Applications/Google Chrome.app` 155.0.8059.79（sha256 `fbeec33e…`，经 `TB02E_CHROMIUM_EXECUTABLE`），与 TB-02-D 冻结时 154.0.8037.99、F1a 时 155.0.8059.40 都不同；`browser/RESULT.json` 因失败未写出，版本取自门检时的 Info.plist。

## 计时（min12_up-a）

S 摄入回复：中位 1.04 s，最大 4.78 s（第 43 条），10 s 帽内；第 39–43 条 3.73→4.78 s，与 S0b/MIN12 曲线一致。公共查询：live 最大 8.12 s（live-42）、AsKnown 最大 5.23 s、回访最大 5.22 s，30 s 帽内。恢复：stop-s 0.20 s、recover 4.15 s、start-s 4.85 s；恢复前后两次 AsKnown 查询最大 5.03 s / 5.09 s。

## 背景负载

门检（11:26）：1 分钟负载 9.18，没有 cargo/rustc/lake/S/Q/探针/浏览器验收进程，所有工位 target 的 cargo 锁 IDLE，端口 18860–18863 与 `/private/tmp/tb1419e-*.sock` 空闲。运行期间 1 分钟负载 9.6–18.1（本机 28 个逻辑核）。背景主要是：用户自用 Chrome 154（GPU 进程 110–160%）、agent-browser 无头 Chrome（`--remote-debugging-port=0`，约 100%）、CLIProxy（至 166%）、WindowServer（约 60%），Chrome 合计 293–458%；另有 Spotlight `mds_stores` 100–288%，推断与本次外盘写入约 7 GiB / 2.6 万个 HTTP 收据文件有关（未用 fs_usage 证实）。逐 30 s 采样见 `background-samples.jsonl`。

## 回收

`SLOT-REAP.json`（本目录，与外盘 RUN 内同件）：`clean=true`，本票进程 0 个、孤儿 playwright Chrome 0 个、监听 0 个，清掉残留 socket 1 个（`/private/tmp/tb1419e-9e4683d26281adcf.sock`）。之后用 ps/lsof 复查，无 S/Q/HTTP 采集器/playwright Chrome 残留。数据卷最低 24.03 GiB 可用，未触 15 GiB 停手线。

## 产物与偏离

本目录只放门收据、驱动与抽取脚本、背景采样、harness 输出、各臂小件原件副本（`raw/`）、大件 SHA（`BIG-ARTIFACTS.json`）与 `SHA256SUMS`。每臂 HTTP 收据约 6.9 GB、轨迹 `semantic-core.jsonl` 1.86 GB 与库留在外盘 `/Volumes/AgentStorage/issue1419-s9-formal-20261014/`：harness 文首要求外盘目录，且 TB-02-D 每臂约 6.7 GiB，写数据卷会越停手线。这是对派单「输出放本目录」的唯一偏离。

驱动 `s9_drive.py`：门检 → `probe_slot.py snapshot` → `tb02e_harness.py --execute`（python 3.14.6 venv、node v24.19.0、playwright 1.62.1）→ `probe_slot.py reap --kill`。抽取 `s9_extract.py`、汇总 `s9_finalize.py` 只读外盘原件。点击耗时原计划由 HAR 与产物落盘时刻推导（脚本 READINESS 事件缓冲 128 条会被请求事件挤满），本次两次点击都未发生，故为空。

## 给根的判断材料（推断，未验证）

失败点与 S/T 层无关：S 写者产物、公共查询与恢复在首臂上都对。卡住的是浏览器冷路径上的一次 Q verify：harness 的 HTTP 采集器一路热着，页面第一次 token 分页落到冷 worker。若根要重跑，需要先决定是降背景负载（关 agent-browser 无头 Chrome、等 Spotlight 静下来）后照原帽再跑，还是另行处理 Q 冷 verify；本工位不重试、不放宽任何帽。
