# 2026-09-30 工位登记

本登记为活文档。对应实施票 #1394、已批准接口增补 #1340、后端总图 #1323；产品前端 #1460 单独推进。工位登记不代表代码、服务、整票或总图验收通过。

| 工位与分支 | 职责与管理者 | 当前边界 |
|---|---|---|
| `/Users/silencehan/Projects/NewChanlun-1394-content-v3`；`codex/1394-content-v3` | 根任务整合、独评与验收 | 产品 `6dcee4da72873ab323132cf2fd64271702a1d8c3`；client 与 E/B owner 已接入隔离特性栈，仍有 open #1394 工作，未合 main／未完成联合验收 |
| 主工位下 `.scratch/issue1394-s3-writer-entry-20260930/worktree`；`codex/1394-s3-writer-entry-20260930` | S3 写入口；`/root/s3_writer_entry_manager` | 工蜂 run `4e8e8944-b299-4633-9785-afe90fa494d9` 已取消、回收；原生接管至 `bf25c5ed76d660b8d12a7181f2c16f2d8eebc8c2`，管理者结束、回收；工位代码已入特性栈，票仍 open，保留 |
| 主工位下 `.scratch/issue1394-s3-financial-producer-20260930/worktree`；`codex/1394-s3-financial-producer-20260930` | S3 `financial-basis-ref`、`financial-content`；`/root/s3_financial_producer_manager` | 工蜂 run `56ee2d09-684a-4936-969d-b6cfd509ef5f` 已取消、回收；原生接管至 `424e3fae829c3fd01bc87f01ed6e6413dce7b20f`，管理者结束、回收；工位保留，非服务／经济链通过 |
| 主工位下 `.scratch/issue1394-finance-reader-transport-20260930/worktree`；`codex/1394-finance-reader-transport-20260930` | E/B reader 资源及运输；`/root/financial_reader_seams` | 外工蜂未启动；原生实现至 `ae8c0ca0fde8cf449976d5bfbb6d53f83ef490c0`；管理者结束、回收，后续消费／闭包仍归本票 |
| 主工位下 `.scratch/issue1394-sandcastle-shell-write-canary-20260930/fixture`；`codex/issue1394-shell-write-canary` | #1456 单次工具路径 canary；`/root/s3_cli_contract_inventory` | run `5cfa0631-71bf-4fe6-a3a6-c83bf9864c18` completed；外验收提交 `050b9626dc77d05048868f98b3ef2c3a4b8934dd`；仅证明 shell 单文件路径，管理者结束、回收 |
| 主工位下 `.scratch/issue1394-financial-content-client-20260930/worktree`；`codex/1394-financial-content-client-20260930` | 当前财务完整内容客户端；`/root/financial_content_client_manager` | 固定 `268f864ed48ef0540637a4073d31e75d04af8705`，已双轴独评并按相同 Rust 树整合；管理者结束、interrupt；不代表整票完成 |
| 主工位下 `.scratch/issue1394-eb-content-owner-20260930/worktree`；`codex/1394-eb-content-owner-20260930` | E/B 新域、本地闭包、统一预算和入队期限；`/root/eb_content_owner_manager` | 固定 `d8b4b713e4645d4b7833aca0705b4e35f22ab198`，已双轴独评，根特性栈 Rust 树相同；管理者结束、interrupt；完整进程故障与正式 CLI 尚待验收 |
| `/Users/silencehan/Projects/NewChanlun-1394-eb-acceptance-20260930`；detached `d8b4b713e4645d4b7833aca0705b4e35f22ab198` | 根任务临时验收夹具承载 | 本票临时验收与输入回归，不交付产品代码；已运行24个SIGKILL、真实半导入、正式3CLI和离线recover，受控前置与资源范围另绑定；所有本票映像已卸载，工位保留复用 |

同范围实施授权持续有效；不改主仓未提交改动、GOAL_SET/SUPERSEDE 或暂停排程。main 合入及真实外效另按明确批准门。S writer、E/B reader 的具名资源缺项须照实保留，不以 Q16 资源替代。

本轮独评／补审工位均已结束并 interrupt 回收。前一入口封包保留；当前财务消费、E/B owner、实际失败／通过范围及未验项统一见 [#1394 财务owner封包](review-results/issue1394-financial-owner-20260930/EVIDENCE.md)。
