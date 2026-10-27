# #1488 a5s6-0076：A⑤ 测试客户端核对 + S6 第 46 条 trace

终面 3ee05a27137fd4a655cc83a8503d989b62a28f15 生产构建（`--features s_session`，`--locked`，rustc 1.99.1，
sha256 d2f96f044e2e4d76…，G:/AgentStorage/1488-accept/a5s6/bin/s_structure_session-3ee05a2713）；
对照 main 6badaa6049（c944bd6bed40dd7e…，G:/AgentStorage/1488-accept/s2/bin/s_structure_session-6badaa6049）。
oracle = d65/min12d-main1/RUN.json（main v3 臂，sha256 8706096e3d37…）。全部运行在 WSL（NC_TARGET=target-a76）。

## 结论速览

| 项 | 结论 |
|---|---|
| S6 第 46 条台阶 | **不存在**。安静臂 op45（g46）×reply 1.06 = ×S CPU 1.06，×rchar 1.09–1.10；旧 Mac 台阶（5420→9471 ms）未复现 |
| 与 Δ_R / 读量同步 | 回复耗时 ≈ S 进程 CPU（r 0.98–1.00），CPU 与确定性读量（rchar/syscr，r 0.96–0.98）同步平滑增长；与 ΔR 仅 r≈0.5，与累计 R r≈0.92–0.96。CPU ∝ R^1.9–2.2，rchar ∝ R^3.1：累计历史读放大 → **按 D11 属 R2**，R2 达标后销项 |
| 10 s 帽 | **两臂都失守**：首条 >10 s 终面 op12/13，main op15/22；S CPU 自身 >10 s 自 op18–22 起（非争用）。安静臂 op51：终面 87.6 s，main 85.3 s |
| A⑤ 功能档逐代正确性 | 终面 g1/g8/g16、main g1/g8/g16 全部六项核对通过（re-encode 根、qcut3=服务端、oracle digest/counts/structure_cut/rule_revision） |
| A⑤ 冻结档 | 全库（52 代）冻结 Q3 `/api/ready` 503 QueryBudgetExceeded（verify 5 s 门）——终面、main 同形；前缀库仅 head≤2 低负载可全过 |
| #1497 | 票面交付物（Python Q 冷 verify 降阶）不阻塞本票 v3 入口；但主控 10-20 登记的扩展现场（冻结 Q 公开读超时、客户端校验成本超帽）在 Rust Q3 上复现，阻塞“冻结档全库”形态的 A⑤。非终面回归（main 同形）。待主控裁定 |

## 运行与环境

| 臂 | 二进制 | UTC | load1（全程 / op40–51） | 合计回复 | 驱动 rc / stop |
|---|---|---|---|---|---|
| smoke1 | 终面 | 01:06–01:14 | ~8.9 | 3 条 | 1（stop 误报，见下） |
| final1 | 终面 | 01:19–02:44 | 6.8–22.6 / 8.5–13.5 | 2065 s | 1（stop 12 次误报，后手动 launcher stop 成功，LAUNCHER-STOP-AFTER.log） |
| main1 | main | 02:48–04:15 | 3.6–15.9 / 7.7–14.8 | 1842 s | 0 / 1 次 |
| final2 | 终面 | 04:23–05:03 | 2.0–16.4 / 2.0–5.5 | 1364 s | 0 / 1 次 |
| main2 | main | 05:05–05:33 | 1.7–6.1 / 4.5–5.8 | 1290 s | 0 / 1 次 |

宿主 8 核；他工位常驻 serve/probe 使 WSL 无法独占。final2 的 op38–51 与 main2 全程为安静窗口（main2 min cpu/wall 0.923），
第 46 条判读以这两臂为准；final1/main1 为争用臂，作旁证。每臂 HOST.jsonl.gz（/proc/loadavg、stat 逐 250 ms）、
INGEST.jsonl 的 load1_before/host_busy_cores 即宿主负载读数。

入口：`s_session/launch_s.sh service start|stop|status`（s-launcher/3：S 写者 serve s-session/3 + Rust Q3 query-serve，
冻结资源 tools/q3-frozen.json：verify 5000 / capture 2000 / lease 10000 ms）。功能档 Q3（tools/q3-functional.json，三项 600000）为诊断档。
生产构建无埋点，阶段 trace 取 S 进程 /proc（utime+stime、io 的 rchar/syscr/read_bytes/wchar），TRACE.jsonl.gz 逐 250 ms。

确定性：同二进制两臂 syscr/rchar/wchar 52/52 相等、session.sqlite sha256 相等（终面 6f954ac3…，main 5c893f3f…）；
终面与 main 的 publication/receipt 52/52 相等，main 读量多约 12%（op51 rchar 96.9 GB vs 86.2 GB）。

## S6 第 46 条邻域（op-k ⇒ g k+1）

| 臂 | op45 ×reply | ×cpu | ×rchar | op46 ×reply | 备注 |
|---|---|---|---|---|---|
| final2（安静） | 1.059 | 1.059 | 1.103 | 1.196（cpu ×1.153，rchar ×1.104；op47 ×0.935） | 读吞吐 0.86→0.79 GB/s，单点抖动 |
| main2（安静） | 1.064 | 1.064 | 1.088 | 1.06 | 平滑 |
| final1（争用） | 0.875 | 1.0 | 1.103 | — | op43 ×1.58 伴 cpu/wall 0.70 |
| main1（争用） | 0.946 | 1.029 | 1.088 | 1.48 | 伴 cpu/wall 0.60 |

争用臂的跳变全部伴随 cpu/wall 下跌（被抢核），CPU 与读量本身无跳变。TRACE 显示 op43–49 每条几乎全程处于持续读
（rchar 吞吐 0.8–1.05 GB/s，read_bytes 极小 = 页缓存）。逐条全表见 analysis/ANALYSIS.md。

## A⑤ 收据

客户端 tools/a5_client.cjs：tb01c-client.js `Client`（s-session/3：Q3Consumer + s-content-v3 + s-content-job-v3），
资源档与 TestOnly measurement() 取 tests/q3_complete_fixture.cjs 原样（fixture：timeoutMs 30 s、max_elapsed 60 s；
relaxed 仅放长两项时间，诊断档）。每代 loadReady("AsKnown", g) → s-content-v3 独立重编码 → 根、qcut3、oracle。
tb03a 为经济域消费者：Q3 结构入口 `/api/economic/ready` 404，不适用（五臂一致）。

- 冻结 Q3 + 全库 52 代：A5-formal 终面 7/7、main 7/7 失败于 Ready（query_validation ← 503 QueryBudgetExceeded ≈5.3 s）。
- 冻结 Q3 `/api/ready` 逐代探测（READY-PROBE.jsonl）：main2（安静）g16 4.74 s → g17 起 503；main1 g13 起 503（load 15）；final2 g11 起 503（load 15）。
- 前缀库 + 冻结 Q3 + fixture 客户端：main1 head2 g1/g2/current 全过（47/58/53 s）；head4/head8 仅 g1 过，余 job_deadline；
  smoke1（终面 head3）g1 过（51 s），g3/current job_deadline；final2 head2/4/8 全 job_deadline（load 11–15）。
- 功能档 Q3 + relaxed 客户端：终面 g1 205 s/2054 req、g8 581 s/3922、g16 1871 s/7143 全过；main1 同代全过（163/488/1869 s，请求数相同）；
  final2/main2 g1 116 s 过。g16 之后成本超线性，未续跑更高代（限制）。

## #1497 书面结论

票面正文交付物是 Python Q（s_readonly_server / s_query_integrity）冷 verify 降阶，依赖 #1419，不在本票 v3 正式入口（Rust Q3）
的测试客户端路径上——按 D3，这一部分**不阻塞**。但 2026-10-20 主控在 #1497 登记的扩展现场（#1493 HEAD 复跑：正式 S 入口在冻结 Q
配置下公开读超时、客户端校验成本超帽，疑 #1394 校验并入后的成本，“本票验收须覆盖以上现场”）在本次 Rust Q3 上同形复现：
ready 在 g11–17 越过 verify 5 s 门，fixture 客户端 60 s job 期限在 head 3–4 起即撞上；main 同样如此，非终面回归。
因此 A⑤ 的“冻结档全库”形态被该现场**阻塞**。建议主控裁定：A⑤ 按“功能档逐代正确性（g1/8/16）+ 前缀库冻结档（head≤2）”销项，
全库冻结档挂 #1497 或另拆 Q3 成本票。

## 环境缺陷（非产品判据）

ssh→wsl 无 tty，procps 向 stderr 写 “your 131072x1 screen size is bogus”；s_service_control.py:151-163 process_identity
对已退出 PID 把非空 stderr 判“无法确认进程状态”，stop 等满期限报错、S 不停（smoke1/final1）。tools/run_wsl.sh `export LINES=50` 修复；
设 COLUMNS 会截断 ps 命令行、触发“服务完整参数与本次启动命令不符”，不可用。

## 目录

- tools/：a5s6_run.py（驱动）、a5_client.cjs、analyze.py、run_wsl.sh、q3-frozen.json、q3-functional.json
- smoke1/ final1/ main1/ final2/ main2/：RUN.json、INGEST.jsonl、READY-PROBE.jsonl、A5-*.jsonl、TRACE/HOST.jsonl.gz、
  config.json、control/、日志、DRIVER-EXIT、SHA256SUMS（覆盖 G: 全量，含不入仓的 session.sqlite*）
- 大件：G:/AgentStorage/1488-accept/a5s6/<label>/（WSL /mnt/g/…）
- analysis/：`python3 tools/analyze.py --oracle <min12d-main1/RUN.json> --arm final1=final1 --arm main1=main1 --arm final2=final2 --arm main2=main2 --out analysis`

限制：功能档只到 g16；前缀冻结档对宿主负载敏感（同一 head2 在 load 4 过、load 11–15 不过）；ΔR/R 取 main v3 oracle 的 counts.total
（终面与 oracle 逐代一致已由功能档 g1/8/16 与 52/52 publication 相等支撑，非逐代全核）。
